use std::collections::BTreeMap;

use serde::Serialize;

use crate::ai::combat_search_v2::{
    compare_outcome_metrics, CombatSearchV2OutcomeMetrics, CombatSearchV2RolloutPolicy,
    CombatSearchV2TrajectoryReport, SearchProofStatus, SearchTerminalLabel,
};

use super::benchmark::{
    run_combat_search_v2_benchmark, CombatSearchV2BenchmarkCaseReport,
    CombatSearchV2BenchmarkReport, CombatSearchV2LoadedBenchmarkCase,
};
use super::rollout_compare_attribution::{
    first_action_diff, CombatSearchV2RolloutPolicyFirstActionDiff,
};
use super::{CombatSearchV2LoadedBenchmark, CombatSearchV2RunOptions};

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2RolloutPolicyComparisonReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub benchmark_name: String,
    pub case_count: usize,
    pub left_policy: &'static str,
    pub right_policy: &'static str,
    pub summary: CombatSearchV2RolloutPolicyComparisonSummary,
    pub notes: Vec<&'static str>,
    pub cases: Vec<CombatSearchV2RolloutPolicyComparisonCase>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2RolloutPolicyComparisonSummary {
    pub cases_compared: usize,
    pub right_better: usize,
    pub left_better: usize,
    pub tied: usize,
    pub right_only_complete: usize,
    pub left_only_complete: usize,
    pub both_inconclusive: usize,
    pub first_action_diff_cases: usize,
    pub right_minus_left_final_hp_total: i32,
    pub first_diff_action_index_histogram: BTreeMap<String, usize>,
    pub right_better_right_role_histogram: BTreeMap<String, usize>,
    pub left_better_right_role_histogram: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2RolloutPolicyComparisonCase {
    pub id: String,
    pub verdict: CombatSearchV2RolloutPolicyComparisonVerdict,
    pub right_minus_left_final_hp: Option<i32>,
    pub right_minus_left_hp_loss: Option<i32>,
    pub right_minus_left_turns: Option<i32>,
    pub right_minus_left_cards_played: Option<i32>,
    pub left: CombatSearchV2RolloutPolicyComparisonRun,
    pub right: CombatSearchV2RolloutPolicyComparisonRun,
    pub first_action_diff: Option<CombatSearchV2RolloutPolicyFirstActionDiff>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RolloutPolicyComparisonVerdict {
    RightBetter,
    LeftBetter,
    Tied,
    RightOnlyComplete,
    LeftOnlyComplete,
    BothInconclusive,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2RolloutPolicyComparisonRun {
    pub policy: &'static str,
    pub terminal: Option<SearchTerminalLabel>,
    pub proof_status: SearchProofStatus,
    pub complete_trajectory_found: bool,
    pub final_hp: Option<i32>,
    pub hp_loss: Option<i32>,
    pub turns: Option<u32>,
    pub potions_used: Option<u32>,
    pub cards_played: Option<u32>,
    pub action_count: Option<usize>,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub nodes_to_first_win: Option<u64>,
    pub deadline_hit: bool,
    pub node_budget_hit: bool,
}

pub fn compare_combat_search_v2_rollout_policies(
    loaded: &CombatSearchV2LoadedBenchmark,
    options: CombatSearchV2RunOptions,
    left_policy: CombatSearchV2RolloutPolicy,
    right_policy: CombatSearchV2RolloutPolicy,
) -> CombatSearchV2RolloutPolicyComparisonReport {
    let mut left_options = options.clone();
    left_options.rollout_policy = Some(left_policy);
    let left = run_combat_search_v2_benchmark(loaded, left_options);

    let mut right_options = options.clone();
    right_options.rollout_policy = Some(right_policy);
    let right = run_combat_search_v2_benchmark(loaded, right_options);

    build_comparison_report(loaded, &options, left_policy, right_policy, &left, &right)
}

fn build_comparison_report(
    loaded: &CombatSearchV2LoadedBenchmark,
    options: &CombatSearchV2RunOptions,
    left_policy: CombatSearchV2RolloutPolicy,
    right_policy: CombatSearchV2RolloutPolicy,
    left: &CombatSearchV2BenchmarkReport,
    right: &CombatSearchV2BenchmarkReport,
) -> CombatSearchV2RolloutPolicyComparisonReport {
    let mut summary = CombatSearchV2RolloutPolicyComparisonSummary::default();
    let cases = loaded
        .cases
        .iter()
        .zip(left.cases.iter())
        .zip(right.cases.iter())
        .map(|((loaded_case, left_case), right_case)| {
            debug_assert_eq!(loaded_case.id, left_case.id);
            debug_assert_eq!(left_case.id, right_case.id);
            let case = compare_case(
                loaded_case,
                options,
                left_policy,
                right_policy,
                left_case,
                right_case,
            );
            observe_case(&mut summary, &case);
            case
        })
        .collect::<Vec<_>>();

    CombatSearchV2RolloutPolicyComparisonReport {
        schema_name: "CombatSearchV2RolloutPolicyComparisonReport",
        schema_version: 2,
        benchmark_name: loaded.name.clone(),
        case_count: cases.len(),
        left_policy: left_policy.label(),
        right_policy: right_policy.label(),
        summary,
        notes: vec![
            "comparison uses best complete trajectories, not proof of optimality",
            "first_action_diff identifies where selected best complete trajectories diverge",
            "rollout policy affects frontier priority; action diffs are consequences, not direct rollout action labels",
            "first_action_diff context is reconstructed by exact replay of the common prefix and is diagnostic only",
        ],
        cases,
    }
}

fn compare_case(
    loaded: &CombatSearchV2LoadedBenchmarkCase,
    options: &CombatSearchV2RunOptions,
    left_policy: CombatSearchV2RolloutPolicy,
    right_policy: CombatSearchV2RolloutPolicy,
    left: &CombatSearchV2BenchmarkCaseReport,
    right: &CombatSearchV2BenchmarkCaseReport,
) -> CombatSearchV2RolloutPolicyComparisonCase {
    let left_trajectory = left.best_complete_trajectory.as_ref();
    let right_trajectory = right.best_complete_trajectory.as_ref();
    let verdict = compare_trajectories(left_trajectory, right_trajectory);
    CombatSearchV2RolloutPolicyComparisonCase {
        id: left.id.clone(),
        verdict,
        right_minus_left_final_hp: delta_i32(
            left_trajectory.map(|trajectory| trajectory.final_hp),
            right_trajectory.map(|trajectory| trajectory.final_hp),
        ),
        right_minus_left_hp_loss: delta_i32(
            left_trajectory.map(|trajectory| trajectory.hp_loss),
            right_trajectory.map(|trajectory| trajectory.hp_loss),
        ),
        right_minus_left_turns: delta_u32(
            left_trajectory.map(|trajectory| trajectory.turns),
            right_trajectory.map(|trajectory| trajectory.turns),
        ),
        right_minus_left_cards_played: delta_u32(
            left_trajectory.map(|trajectory| trajectory.cards_played),
            right_trajectory.map(|trajectory| trajectory.cards_played),
        ),
        left: summarize_run(left_policy, left),
        right: summarize_run(right_policy, right),
        first_action_diff: first_action_diff(
            loaded,
            options,
            left_trajectory.map(|trajectory| trajectory.actions.as_slice()),
            right_trajectory.map(|trajectory| trajectory.actions.as_slice()),
        ),
    }
}

fn compare_trajectories(
    left: Option<&CombatSearchV2TrajectoryReport>,
    right: Option<&CombatSearchV2TrajectoryReport>,
) -> CombatSearchV2RolloutPolicyComparisonVerdict {
    match (left.filter(is_resolved), right.filter(is_resolved)) {
        (Some(left), Some(right)) => match compare_outcome_metrics(
            CombatSearchV2OutcomeMetrics::from_trajectory(right),
            CombatSearchV2OutcomeMetrics::from_trajectory(left),
        ) {
            std::cmp::Ordering::Greater => {
                CombatSearchV2RolloutPolicyComparisonVerdict::RightBetter
            }
            std::cmp::Ordering::Equal => CombatSearchV2RolloutPolicyComparisonVerdict::Tied,
            std::cmp::Ordering::Less => CombatSearchV2RolloutPolicyComparisonVerdict::LeftBetter,
        },
        (None, Some(_)) => CombatSearchV2RolloutPolicyComparisonVerdict::RightOnlyComplete,
        (Some(_), None) => CombatSearchV2RolloutPolicyComparisonVerdict::LeftOnlyComplete,
        (None, None) => CombatSearchV2RolloutPolicyComparisonVerdict::BothInconclusive,
    }
}

fn is_resolved(trajectory: &&CombatSearchV2TrajectoryReport) -> bool {
    trajectory.terminal != SearchTerminalLabel::Unresolved
}

fn summarize_run(
    policy: CombatSearchV2RolloutPolicy,
    case: &CombatSearchV2BenchmarkCaseReport,
) -> CombatSearchV2RolloutPolicyComparisonRun {
    let trajectory = case.best_complete_trajectory.as_ref();
    CombatSearchV2RolloutPolicyComparisonRun {
        policy: policy.label(),
        terminal: trajectory.map(|trajectory| trajectory.terminal),
        proof_status: case.outcome.proof_status,
        complete_trajectory_found: case.outcome.complete_trajectory_found,
        final_hp: trajectory.map(|trajectory| trajectory.final_hp),
        hp_loss: trajectory.map(|trajectory| trajectory.hp_loss),
        turns: trajectory.map(|trajectory| trajectory.turns),
        potions_used: trajectory.map(|trajectory| trajectory.potions_used),
        cards_played: trajectory.map(|trajectory| trajectory.cards_played),
        action_count: trajectory.map(|trajectory| trajectory.actions.len()),
        nodes_expanded: case.stats.nodes_expanded,
        nodes_generated: case.stats.nodes_generated,
        nodes_to_first_win: case.stats.nodes_to_first_win,
        deadline_hit: case.stats.deadline_hit,
        node_budget_hit: case.stats.node_budget_hit,
    }
}

fn observe_case(
    summary: &mut CombatSearchV2RolloutPolicyComparisonSummary,
    case: &CombatSearchV2RolloutPolicyComparisonCase,
) {
    summary.cases_compared += 1;
    match case.verdict {
        CombatSearchV2RolloutPolicyComparisonVerdict::RightBetter => summary.right_better += 1,
        CombatSearchV2RolloutPolicyComparisonVerdict::LeftBetter => summary.left_better += 1,
        CombatSearchV2RolloutPolicyComparisonVerdict::Tied => summary.tied += 1,
        CombatSearchV2RolloutPolicyComparisonVerdict::RightOnlyComplete => {
            summary.right_only_complete += 1
        }
        CombatSearchV2RolloutPolicyComparisonVerdict::LeftOnlyComplete => {
            summary.left_only_complete += 1
        }
        CombatSearchV2RolloutPolicyComparisonVerdict::BothInconclusive => {
            summary.both_inconclusive += 1
        }
    }
    if let Some(delta) = case.right_minus_left_final_hp {
        summary.right_minus_left_final_hp_total += delta;
    }
    if case.first_action_diff.is_some() {
        summary.first_action_diff_cases += 1;
    }
    if let Some(diff) = case.first_action_diff.as_ref() {
        increment_histogram(
            &mut summary.first_diff_action_index_histogram,
            diff.action_index.to_string(),
        );
        match case.verdict {
            CombatSearchV2RolloutPolicyComparisonVerdict::RightBetter
            | CombatSearchV2RolloutPolicyComparisonVerdict::RightOnlyComplete => {
                if let Some(role) = diff.right_action_role {
                    increment_histogram(&mut summary.right_better_right_role_histogram, role);
                }
            }
            CombatSearchV2RolloutPolicyComparisonVerdict::LeftBetter
            | CombatSearchV2RolloutPolicyComparisonVerdict::LeftOnlyComplete => {
                if let Some(role) = diff.right_action_role {
                    increment_histogram(&mut summary.left_better_right_role_histogram, role);
                }
            }
            CombatSearchV2RolloutPolicyComparisonVerdict::Tied
            | CombatSearchV2RolloutPolicyComparisonVerdict::BothInconclusive => {}
        }
    }
}

fn increment_histogram(histogram: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *histogram.entry(key.into()).or_default() += 1;
}

fn delta_i32(left: Option<i32>, right: Option<i32>) -> Option<i32> {
    Some(right? - left?)
}

fn delta_u32(left: Option<u32>, right: Option<u32>) -> Option<i32> {
    Some(right? as i32 - left? as i32)
}

#[cfg(test)]
#[path = "rollout_compare_tests.rs"]
mod tests;
