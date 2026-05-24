use super::*;

pub(super) const FRONTIER_SAMPLE_LIMIT: usize = 8;

#[derive(Default)]
pub(super) struct SearchDiagnosticsCollector {
    states_queried: u64,
    states_with_legal_actions: u64,
    legal_actions_total: u64,
    legal_actions_max: usize,
    expansion: ActionExpansionDiagnosticsCollector,
    target_fanout: TargetFanoutDiagnosticsCollector,
    equivalence: ActionEquivalenceDiagnosticsCollector,
    ordering: ActionOrderingDiagnosticsCollector,
    turn_branching: TurnBranchingDiagnosticsCollector,
    pending_choice: PendingChoiceDiagnosticsCollector,
    turn_prefix: TurnPrefixDiagnosticsCollector,
    turn_sequence: TurnSequenceDiagnosticsCollector,
    card_identity: CardIdentityDiagnosticsCollector,
    turn_local_dominance: TurnLocalDominanceDiagnosticsCollector,
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
    pub(super) potion_budget_cut_count: u64,
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

    pub(super) fn observe_action_ordering(&mut self, ordering: &ActionOrderingSummary) {
        self.ordering.observe(ordering);
    }

    pub(super) fn observe_target_fanout(&mut self, target_fanout: &TargetFanoutSummary) {
        self.target_fanout.observe(target_fanout);
    }

    pub(super) fn observe_action_equivalence(&mut self, equivalence: &ActionEquivalenceSummary) {
        self.equivalence.observe(equivalence);
    }

    pub(super) fn observe_turn_branching(&mut self, observation: &TurnBranchingStateObservation) {
        self.turn_branching.observe(observation);
    }

    pub(super) fn observe_pending_choice(&mut self, profile: Option<&PendingChoiceProfile>) {
        self.pending_choice.observe(profile);
    }

    pub(super) fn observe_turn_prefix(&mut self, summary: &TurnPrefixSummary) {
        self.turn_prefix.observe(summary);
    }

    pub(super) fn observe_turn_sequence(
        &mut self,
        summary: &TurnSequenceSummary,
        node: &SearchNode,
    ) {
        self.turn_sequence.observe_with_node(summary, node);
    }

    pub(super) fn observe_card_identity(&mut self, summary: &CardIdentitySummary) {
        self.card_identity.observe(summary);
    }

    pub(super) fn observe_turn_local_dominance(
        &mut self,
        observation: &TurnLocalDominanceStateObservation,
    ) {
        self.turn_local_dominance.observe(observation);
    }

    pub(super) fn run_discard_order_exact_shadow_audit(
        &mut self,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
    ) {
        self.turn_sequence
            .run_discard_order_exact_shadow_audit(stepper, config);
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
            turn_local_dominance_prunes: input.stats.turn_local_dominance_prunes,
            terminal_wins: input.stats.terminal_wins,
            terminal_losses: input.stats.terminal_losses,
            unresolved_leaf_count: input.unresolved_leaf_count,
            max_actions_cut_count: input.max_actions_cut_count,
            engine_step_limit_count: input.engine_step_limit_count,
            potion_budget_cut_count: input.potion_budget_cut_count,
        };
        let frontier = CombatSearchV2DiagnosticsFrontier {
            remaining_states: input.frontier_remaining_states,
            sample_limit: FRONTIER_SAMPLE_LIMIT,
            sampled_states: input.frontier_sample_count,
        };
        let expansion = self.expansion.finish();
        let target_fanout = self.target_fanout.finish();
        let equivalence = self.equivalence.finish();
        let ordering = self.ordering.finish();
        let turn_branching = self.turn_branching.finish();
        let pending_choice = self.pending_choice.finish();
        let turn_prefix = self.turn_prefix.finish();
        let turn_sequence = self.turn_sequence.finish();
        let card_identity = self.card_identity.finish();
        let turn_local_dominance = self.turn_local_dominance.finish();
        let diagnosis = diagnosis_tags(
            input.proof_status,
            input.stats,
            &branching,
            &expansion,
            &target_fanout,
            &equivalence,
            &ordering,
            &turn_branching,
            &pending_choice,
            &turn_prefix,
            &turn_sequence,
            &card_identity,
            &turn_local_dominance,
            &pruning,
            frontier.remaining_states,
        );

        CombatSearchV2DiagnosticsReport {
            schema_version: 10,
            mode: "summary",
            tables,
            branching,
            expansion,
            target_fanout,
            equivalence,
            ordering,
            turn_branching,
            pending_choice,
            turn_prefix,
            turn_sequence,
            card_identity,
            turn_local_dominance,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_ratio_uses_bounded_precision() {
        assert_eq!(rounded_ratio(0, 0), 0.0);
        assert_eq!(rounded_ratio(10, 3), 3.33);
    }
}
