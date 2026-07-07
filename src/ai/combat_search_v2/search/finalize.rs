use super::super::*;
use super::finish_coverage::{coverage_status_for_finished_search, coverage_status_reason};
use super::finish_evidence::evidence_reliability_report;
use super::finish_frontier::frontier_sample_states;
use super::finish_outcome::outcome_report;
use super::finish_policy::{budget_report, search_policy_report};
use super::finish_trajectories::trajectory_reports;
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
    let rollout = rollout_cache.finish(trajectories.best_frontier.as_ref());
    let trajectory_reports = trajectory_reports(trajectories);

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
            trajectory_reports.best_complete_trajectory.is_some(),
            trajectory_reports.best_win_trajectory.is_some(),
            exhaustive,
        ),
        best_complete_trajectory: trajectory_reports.best_complete_trajectory,
        best_win_trajectory: trajectory_reports.best_win_trajectory,
        win_candidate_trajectories: trajectory_reports.win_candidate_trajectories,
        best_frontier_trajectory: trajectory_reports.best_frontier_trajectory,
        frontier: CombatSearchV2FrontierReport {
            remaining_states: frontier.len(),
            unresolved_leaf_count,
            max_actions_cut_count,
            engine_step_limit_count,
            potion_budget_cut_count,
            best_estimated_value: trajectory_reports.best_frontier_value,
            sample_states,
        },
        rollout,
        diagnostics,
        stats,
        performance,
        evidence_reliability: evidence_reliability_report(
            invalid_card_identity_observed,
            exhaustive,
        ),
    }
}
