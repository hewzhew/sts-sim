use super::super::*;
use super::best_trajectories::SearchTrajectoryBook;
use super::finish_coverage::{coverage_status_for_finished_search, coverage_status_reason};
use super::finish_evidence::evidence_reliability_report;
use super::finish_frontier::frontier_sample_states;
use super::finish_outcome::outcome_report;
use super::finish_policy::{budget_report, search_policy_report};
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
    let search_policy = search_policy_report(&config);
    let budget = budget_report(&config);

    CombatSearchV2Report {
        schema_name: "CombatSearchV2Report",
        schema_version: 11,
        input_label: config.input_label,
        information_boundary: "engine_state_snapshot_truth_v0",
        policy_evidence,
        search_policy,
        budget,
        outcome: outcome_report(
            coverage_status,
            coverage_reason,
            best_complete.is_some(),
            best_win.is_some(),
            exhaustive,
        ),
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
        evidence_reliability: evidence_reliability_report(
            invalid_card_identity_observed,
            exhaustive,
        ),
    }
}
