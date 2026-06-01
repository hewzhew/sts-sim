use super::super::frontier::QueueEntry;
use super::super::*;

pub(super) struct SearchFinishInput {
    pub(super) config: CombatSearchV2Config,
    pub(super) stats: CombatSearchV2Stats,
    pub(super) diagnostics: SearchDiagnosticsCollector,
    pub(super) exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>>,
    pub(super) dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>>,
    pub(super) frontier: BinaryHeap<QueueEntry>,
    pub(super) best_complete: Option<SearchNode>,
    pub(super) best_frontier: Option<SearchNode>,
    pub(super) rollout_cache: RolloutCache,
    pub(super) unresolved_leaf_count: u64,
    pub(super) max_actions_cut_count: u64,
    pub(super) engine_step_limit_count: u64,
    pub(super) potion_budget_cut_count: u64,
    pub(super) exhausted: bool,
}

pub(super) fn finish_combat_search_report(input: SearchFinishInput) -> CombatSearchV2Report {
    let SearchFinishInput {
        config,
        stats,
        diagnostics,
        exact_transpositions,
        dominance,
        frontier,
        best_complete,
        best_frontier,
        rollout_cache,
        unresolved_leaf_count,
        max_actions_cut_count,
        engine_step_limit_count,
        potion_budget_cut_count,
        exhausted,
    } = input;

    let exhaustive = !exhausted && frontier.is_empty();
    let proof_status = proof_status_for_finished_search(&stats, exhaustive);
    let top_terminal = top_terminal_for_finished_search(exhaustive, best_complete.as_ref());
    let reason = proof_status_reason(proof_status);
    let sample_states = frontier_sample_states(&frontier);
    let diagnostics = diagnostics.finish(SearchDiagnosticsFinish {
        exact_transpositions: &exact_transpositions,
        dominance: &dominance,
        frontier_remaining_states: frontier.len(),
        frontier_sample_count: sample_states.len(),
        stats: &stats,
        proof_status,
        unresolved_leaf_count,
        max_actions_cut_count,
        engine_step_limit_count,
        potion_budget_cut_count,
    });
    let invalid_card_identity_observed =
        diagnostics.card_identity.states_with_uuid_card_id_conflict > 0;
    let evidence_warnings = evidence_warnings(invalid_card_identity_observed);

    CombatSearchV2Report {
        schema_name: "CombatSearchV2Report",
        schema_version: 4,
        input_label: config.input_label,
        information_boundary: "engine_state_snapshot_truth_v0",
        search_policy: CombatSearchV2PolicyReport {
            kind: "best_first_atomic_action_graph_search_v2",
            terminal_policy: "whole_combat_terminal_only",
            expansion_order:
                "conservative_duplicate_action_equivalence_then_semantic_turn_action_ordering_then_frontier_value_v1",
            frontier_value: COMBAT_SEARCH_FRONTIER_VALUE_POLICY,
            turn_branching: "turn_transition_classification_with_late_frontier_tie_break",
            turn_plan_policy: config.turn_plan_policy.label(),
            potion_policy: config.potion_policy.label(),
            transposition_table: "exact_runtime_state_key_with_resource_coverage",
            dominance_pruning: "global_dominance_bucket_resource_vector_plus_same_parent_same_turn_sibling_coverage",
            rollout_value: "combat_eval_v2_risk_bucketed_unresolved_estimate_used_for_frontier_priority_only_not_terminal_claims",
            llm_authority: "none",
        },
        budget: CombatSearchV2BudgetReport {
            max_nodes: config.max_nodes,
            max_actions_per_line: config.max_actions_per_line,
            max_engine_steps_per_action: config.max_engine_steps_per_action,
            wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
            max_potions_used: config.max_potions_used,
            rollout_max_evaluations: config.rollout_max_evaluations,
            rollout_max_actions: config.rollout_max_actions,
        },
        outcome: CombatSearchV2OutcomeReport {
            terminal: top_terminal,
            proof_status,
            reason,
            complete_trajectory_found: best_complete.is_some(),
            exhaustive,
        },
        best_complete_trajectory: best_complete
            .as_ref()
            .map(|node| trajectory_report(node, false)),
        best_frontier_trajectory: best_frontier.as_ref().map(|node| {
            trajectory_report(
                node,
                terminal_label(&node.engine, &node.combat) == SearchTerminalLabel::Unresolved,
            )
        }),
        frontier: CombatSearchV2FrontierReport {
            remaining_states: frontier.len(),
            unresolved_leaf_count,
            max_actions_cut_count,
            engine_step_limit_count,
            potion_budget_cut_count,
            best_estimated_value: best_frontier
                .as_ref()
                .map(combat_search_frontier_value_report),
            sample_states,
        },
        rollout: rollout_cache.finish(best_frontier.as_ref()),
        diagnostics,
        stats,
        evidence_reliability: CombatSearchV2EvidenceReport {
            hidden_info_policy: "uses_only_the_supplied_engine_state; if that state contains hidden draw/rng truth, the report is engine-evidence rather than public-agent evidence",
            random_policy: "rng state is part of the transposition key; belief particles are not implemented in this first runner",
            estimate_policy: "unresolved frontier summaries are estimates/partial evidence and are never reported as terminal outcomes",
            reliability: if invalid_card_identity_observed {
                "invalid_input_or_rollout_state_duplicate_card_uuid_conflict_observed"
            } else if exhaustive {
                "exact_under_supplied_state_and_engine_semantics"
            } else {
                "partial_budgeted_evidence"
            },
            warnings: evidence_warnings,
        },
    }
}

fn proof_status_for_finished_search(
    stats: &CombatSearchV2Stats,
    exhaustive: bool,
) -> SearchProofStatus {
    if stats.deadline_hit {
        SearchProofStatus::DeadlineHit
    } else if stats.node_budget_hit {
        SearchProofStatus::BudgetExhausted
    } else if exhaustive {
        SearchProofStatus::Exhaustive
    } else {
        SearchProofStatus::FrontierUnresolved
    }
}

fn top_terminal_for_finished_search(
    exhaustive: bool,
    best_complete: Option<&SearchNode>,
) -> SearchTerminalLabel {
    if exhaustive {
        best_complete
            .map(|node| terminal_label(&node.engine, &node.combat))
            .unwrap_or(SearchTerminalLabel::Unresolved)
    } else {
        SearchTerminalLabel::Unresolved
    }
}

fn proof_status_reason(proof_status: SearchProofStatus) -> String {
    match proof_status {
        SearchProofStatus::Exhaustive => {
            "frontier exhausted; best_complete_trajectory is the best complete trajectory found by this exact-state search".to_string()
        }
        SearchProofStatus::BudgetExhausted => {
            "node budget exhausted; unresolved frontier remains, so no optimality claim is made".to_string()
        }
        SearchProofStatus::DeadlineHit => {
            "wall-clock deadline hit; unresolved frontier remains, so no optimality claim is made".to_string()
        }
        SearchProofStatus::FrontierUnresolved => {
            "frontier unresolved under current safety limits; no optimality claim is made".to_string()
        }
    }
}

fn frontier_sample_states(frontier: &BinaryHeap<QueueEntry>) -> Vec<CombatSearchV2StateSummary> {
    frontier
        .iter()
        .take(FRONTIER_SAMPLE_LIMIT)
        .map(|entry| summarize_state(&entry.node.engine, &entry.node.combat))
        .collect()
}

fn evidence_warnings(invalid_card_identity_observed: bool) -> Vec<&'static str> {
    let mut warnings = vec![
        "unresolved_cannot_be_claimed_better_than_a_complete_baseline",
        "no_stepwise_human_action_agreement_objective",
        "no_llm_control_path",
        "combat_only_runner_does_not_validate_out_of_combat_strategy_quality",
        "default_potion_policy_disables_potions_until_a_real_potion_option_planner_exists",
    ];
    if invalid_card_identity_observed {
        warnings.push(
            "duplicate_active_card_uuid_with_conflicting_card_ids_observed_input_or_rollout_state_invalid_until_investigated",
        );
    }
    warnings
}
