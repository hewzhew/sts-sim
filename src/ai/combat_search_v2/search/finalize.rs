use super::super::*;
use super::best_trajectories::SearchTrajectoryBook;
use super::finish_coverage::{coverage_status_for_finished_search, coverage_status_reason};
use super::finish_evidence::evidence_warnings;
use super::finish_frontier::frontier_sample_states;
use super::loop_state::SearchLoopState;

pub(super) struct SearchFinishInput {
    pub(super) config: CombatSearchV2Config,
    pub(super) policy_evidence: CombatSearchV2PolicyEvidenceReport,
    pub(super) loop_state: SearchLoopState,
}

pub(super) fn finish_combat_search_report(input: SearchFinishInput) -> CombatSearchV2Report {
    let SearchFinishInput {
        config,
        policy_evidence,
        loop_state,
    } = input;
    let SearchLoopState {
        stats,
        diagnostics,
        exact_transpositions,
        dominance,
        frontier,
        trajectories,
        rollout_cache,
        performance,
        unresolved_leaf_count,
        max_actions_cut_count,
        engine_step_limit_count,
        potion_budget_cut_count,
        exhausted,
        accepted_complete_candidate,
        ..
    } = loop_state;
    let SearchTrajectoryBook {
        best_complete,
        best_win,
        win_candidates,
        best_frontier,
    } = trajectories;

    let exhaustive = !accepted_complete_candidate && !exhausted && frontier.is_empty();
    let coverage_status =
        coverage_status_for_finished_search(&stats, exhaustive, accepted_complete_candidate);
    let coverage_reason = coverage_status_reason(coverage_status);
    let sample_states = frontier_sample_states(&frontier);
    let diagnostics = diagnostics.finish(SearchDiagnosticsFinish {
        exact_transpositions: &exact_transpositions,
        dominance: &dominance,
        frontier_remaining_states: frontier.len(),
        frontier_sample_count: sample_states.len(),
        stats: &stats,
        coverage_status,
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
        schema_version: 11,
        input_label: config.input_label,
        information_boundary: "engine_state_snapshot_truth_v0",
        policy_evidence,
        search_policy: CombatSearchV2PolicyReport {
            kind: "best_first_atomic_action_graph_search_v2",
            terminal_policy: "whole_combat_terminal_only",
            expansion_order:
                "conservative_duplicate_action_equivalence_then_semantic_turn_action_ordering_then_frontier_value_v1",
            frontier_value: COMBAT_SEARCH_FRONTIER_VALUE_POLICY,
            frontier_policy: config.frontier_policy.label(),
            turn_branching: "turn_transition_classification_with_late_frontier_tie_break",
            turn_plan_policy: config.turn_plan_policy.label(),
            potion_policy: config.potion_policy.label(),
            transposition_table: "exact_runtime_state_key_with_resource_coverage",
            dominance_pruning: "global_dominance_bucket_resource_vector_plus_same_parent_same_turn_sibling_coverage",
            rollout_value: "combat_eval_v2_risk_bucketed_unresolved_estimate_used_for_frontier_priority_only_not_terminal_claims",
            child_rollout_policy: config.child_rollout_policy.label(),
            llm_authority: "none",
        },
        budget: CombatSearchV2BudgetReport {
            max_nodes: config.max_nodes,
            max_actions_per_line: config.max_actions_per_line,
            max_engine_steps_per_action: config.max_engine_steps_per_action,
            wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
            stop_on_win_hp_loss_at_most: config.stop_on_win_hp_loss_at_most,
            min_win_candidates_before_stop: config.min_win_candidates_before_stop,
            max_potions_used: config.max_potions_used,
            rollout_max_evaluations: config.rollout_max_evaluations,
            rollout_max_actions: config.rollout_max_actions,
            rollout_beam_width: config.rollout_beam_width,
        },
        outcome: CombatSearchV2OutcomeReport {
            coverage_status,
            coverage_reason,
            complete_trajectory_found: best_complete.is_some(),
            complete_win_found: best_win.is_some(),
            exhaustive,
        },
        best_complete_trajectory: best_complete
            .as_ref()
            .map(|node| trajectory_report(node, false)),
        best_win_trajectory: best_win.as_ref().map(|node| trajectory_report(node, false)),
        win_candidate_trajectories: win_candidates
            .iter()
            .map(|node| trajectory_report(node, false))
            .collect(),
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
        performance,
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
