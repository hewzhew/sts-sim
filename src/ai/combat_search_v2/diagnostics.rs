use super::*;

pub(super) const FRONTIER_SAMPLE_LIMIT: usize = 8;

#[derive(Default)]
pub(super) struct SearchDiagnosticsCollector {
    states_queried: u64,
    states_with_legal_actions: u64,
    legal_actions_total: u64,
    legal_actions_max: usize,
    expansion: ActionExpansionDiagnosticsCollector,
}

pub(super) struct SearchDiagnosticsFinish<'a> {
    pub(super) exact_transpositions: &'a HashMap<CombatExactStateKey, Vec<ResourceVector>>,
    pub(super) dominance: &'a HashMap<CombatDominanceKey, Vec<ResourceVector>>,
    pub(super) frontier_remaining_states: usize,
    pub(super) frontier_sample_count: usize,
    pub(super) stats: &'a CombatSearchV2Stats,
    pub(super) proof_status: SearchProofStatus,
    pub(super) unresolved_leaf_count: u64,
    pub(super) max_actions_cut_count: u64,
    pub(super) engine_step_limit_count: u64,
}

impl SearchDiagnosticsCollector {
    pub(super) fn observe_legal_actions(&mut self, expansion: &ActionExpansionSummary) {
        let action_count = expansion.action_count;
        self.states_queried = self.states_queried.saturating_add(1);
        if action_count > 0 {
            self.states_with_legal_actions = self.states_with_legal_actions.saturating_add(1);
        }
        self.legal_actions_total = self.legal_actions_total.saturating_add(action_count as u64);
        self.legal_actions_max = self.legal_actions_max.max(action_count);
        self.expansion.observe(expansion);
    }

    pub(super) fn finish(
        &self,
        input: SearchDiagnosticsFinish<'_>,
    ) -> CombatSearchV2DiagnosticsReport {
        let tables = CombatSearchV2DiagnosticsTables {
            exact_keys: input.exact_transpositions.len(),
            exact_resource_vectors: resource_vector_count(input.exact_transpositions),
            dominance_buckets: input.dominance.len(),
            dominance_resource_vectors: resource_vector_count(input.dominance),
        };
        let branching = CombatSearchV2DiagnosticsBranching {
            states_queried: self.states_queried,
            states_with_legal_actions: self.states_with_legal_actions,
            legal_actions_total: self.legal_actions_total,
            legal_actions_avg: rounded_ratio(self.legal_actions_total, self.states_queried),
            legal_actions_max: self.legal_actions_max,
            nodes_generated_per_expanded: rounded_ratio(
                input.stats.nodes_generated,
                input.stats.nodes_expanded,
            ),
        };
        let pruning = CombatSearchV2DiagnosticsPruning {
            transposition_prunes: input.stats.transposition_prunes,
            dominance_prunes: input.stats.dominance_prunes,
            terminal_wins: input.stats.terminal_wins,
            terminal_losses: input.stats.terminal_losses,
            unresolved_leaf_count: input.unresolved_leaf_count,
            max_actions_cut_count: input.max_actions_cut_count,
            engine_step_limit_count: input.engine_step_limit_count,
        };
        let frontier = CombatSearchV2DiagnosticsFrontier {
            remaining_states: input.frontier_remaining_states,
            sample_limit: FRONTIER_SAMPLE_LIMIT,
            sampled_states: input.frontier_sample_count,
        };
        let expansion = self.expansion.finish();
        let diagnosis = diagnosis_tags(
            input.proof_status,
            input.stats,
            &branching,
            &expansion,
            &pruning,
            frontier.remaining_states,
        );

        CombatSearchV2DiagnosticsReport {
            schema_version: 2,
            mode: "summary",
            tables,
            branching,
            expansion,
            pruning,
            frontier,
            diagnosis,
        }
    }
}

fn resource_vector_count<K>(table: &HashMap<K, Vec<ResourceVector>>) -> usize {
    table.values().map(Vec::len).sum()
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

fn diagnosis_tags(
    proof_status: SearchProofStatus,
    stats: &CombatSearchV2Stats,
    branching: &CombatSearchV2DiagnosticsBranching,
    expansion: &CombatSearchV2DiagnosticsExpansion,
    pruning: &CombatSearchV2DiagnosticsPruning,
    frontier_remaining_states: usize,
) -> Vec<&'static str> {
    let mut tags = Vec::new();

    match proof_status {
        SearchProofStatus::Exhaustive => tags.push("frontier_exhausted"),
        SearchProofStatus::BudgetExhausted => {
            if frontier_remaining_states > 0 {
                tags.push("budget_exhausted_with_unresolved_frontier");
            } else {
                tags.push("budget_exhausted");
            }
        }
        SearchProofStatus::DeadlineHit => {
            if frontier_remaining_states > 0 {
                tags.push("deadline_hit_with_unresolved_frontier");
            } else {
                tags.push("deadline_hit");
            }
        }
        SearchProofStatus::FrontierUnresolved => tags.push("frontier_unresolved"),
    }

    if stats.terminal_wins > 0 {
        tags.push("terminal_wins_found");
    } else {
        tags.push("no_terminal_wins_found");
    }
    if stats.terminal_losses > 0 {
        tags.push("terminal_losses_found");
    }
    if stats.transposition_prunes > 0 {
        tags.push("transposition_pruning_active");
    } else {
        tags.push("transposition_pruning_inactive");
    }
    if stats.dominance_prunes > 0 {
        tags.push("dominance_pruning_active");
    } else {
        tags.push("dominance_pruning_inactive");
    }
    if pruning.engine_step_limit_count > 0 {
        tags.push("engine_step_limit_truncated_children");
    }
    if pruning.max_actions_cut_count > 0 {
        tags.push("max_actions_per_line_cutoffs");
    }
    if pruning.unresolved_leaf_count > 0 {
        tags.push("unresolved_leaf_states");
    }
    if branching.states_queried > 0 && branching.legal_actions_max == 0 {
        tags.push("no_legal_actions_observed");
    }
    if expansion.states_observed > 0 {
        tags.push("action_expansion_diagnostics_active");
    }
    if expansion.max_group_size > 1 {
        tags.push("action_fanout_groups_observed");
    }
    if frontier_remaining_states > 0 {
        tags.push("frontier_remaining");
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_ratio_uses_bounded_precision() {
        assert_eq!(rounded_ratio(0, 0), 0.0);
        assert_eq!(rounded_ratio(10, 3), 3.33);
    }

    #[test]
    fn diagnosis_tags_budgeted_unresolved_frontier() {
        let stats = CombatSearchV2Stats {
            terminal_wins: 1,
            transposition_prunes: 2,
            ..CombatSearchV2Stats::default()
        };
        let branching = CombatSearchV2DiagnosticsBranching {
            states_queried: 1,
            states_with_legal_actions: 0,
            legal_actions_total: 0,
            legal_actions_avg: 0.0,
            legal_actions_max: 0,
            nodes_generated_per_expanded: 0.0,
        };
        let pruning = CombatSearchV2DiagnosticsPruning {
            transposition_prunes: 2,
            dominance_prunes: 0,
            terminal_wins: 1,
            terminal_losses: 0,
            unresolved_leaf_count: 1,
            max_actions_cut_count: 0,
            engine_step_limit_count: 0,
        };

        let tags = diagnosis_tags(
            SearchProofStatus::BudgetExhausted,
            &stats,
            &branching,
            &CombatSearchV2DiagnosticsExpansion {
                grouping_policy: "typed_fanout_groups_with_no_action_merge",
                behavioral_effect: "diagnostic_only_search_expansion_unchanged",
                states_observed: 1,
                total_atomic_actions: 0,
                total_fanout_groups: 0,
                fanout_groups_avg: 0.0,
                fanout_groups_max: 0,
                max_group_size: 0,
                action_kind_counts: Vec::new(),
                largest_groups: Vec::new(),
                notes: Vec::new(),
            },
            &pruning,
            4,
        );

        assert!(tags.contains(&"budget_exhausted_with_unresolved_frontier"));
        assert!(tags.contains(&"terminal_wins_found"));
        assert!(tags.contains(&"transposition_pruning_active"));
        assert!(tags.contains(&"dominance_pruning_inactive"));
        assert!(tags.contains(&"unresolved_leaf_states"));
        assert!(tags.contains(&"no_legal_actions_observed"));
        assert!(tags.contains(&"action_expansion_diagnostics_active"));
        assert!(tags.contains(&"frontier_remaining"));
    }
}
