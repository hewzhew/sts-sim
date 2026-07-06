#[path = "quality_lanes/feedback.rs"]
mod feedback;
#[cfg(test)]
#[path = "quality_lanes/feedback_tests.rs"]
mod feedback_tests;
#[path = "quality_lanes/quality.rs"]
mod quality;
#[path = "quality_lanes/specs.rs"]
mod specs;
#[path = "quality_lanes/types.rs"]
mod types;

use sts_simulator::eval::combat_case::CombatCase;

pub(crate) use quality::{combat_line_quality, compare_quality, witness_line_from_trajectory};
pub(crate) use specs::quality_lane_specs;
pub(crate) use types::CombatLineQuality;
pub(super) use types::CombatQualityLaneReview;

use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use feedback::{
    estimated_rollout_feedback_rank, estimated_rollout_feedback_witness,
    run_success_feedback_rerun, CombatSuccessFeedbackSource,
};
use types::{CombatQualityLaneResult, CombatSuccessFeedbackMetrics};

pub(super) fn run_quality_lanes(
    options: &ReviewOptions,
    case: &CombatCase,
) -> CombatQualityLaneReview {
    let specs = quality_lane_specs();
    let lane_count = specs.len().max(1);
    let total_nodes = options
        .quality_lane_total_nodes
        .unwrap_or(options.slow_nodes)
        .max(1);
    let total_wall_ms = options
        .quality_lane_total_ms
        .unwrap_or(options.slow_ms)
        .max(1);
    let per_lane_nodes = (total_nodes / lane_count).max(1);
    let per_lane_wall_ms = (total_wall_ms / lane_count as u64).max(1);
    let mut lanes = Vec::new();
    let mut complete_feedback_source: Option<(CombatLineQuality, CombatSuccessFeedbackSource)> =
        None;
    let mut estimated_feedback_source: Option<((i32, i32, i32, i32), CombatSuccessFeedbackSource)> =
        None;
    for lane in specs {
        let (review, report) = run_configured_search(
            lane.label,
            case,
            lane.config(per_lane_nodes, per_lane_wall_ms),
            options.action_preview_limit,
        );
        let quality = combat_line_quality(&report);
        if let (Some(quality), Some(trajectory)) =
            (quality.as_ref(), report.best_win_trajectory.as_ref())
        {
            if complete_feedback_source
                .as_ref()
                .is_none_or(|(current, _)| !compare_quality(quality, current).is_lt())
            {
                complete_feedback_source = Some((
                    quality.clone(),
                    CombatSuccessFeedbackSource {
                        spec: lane,
                        baseline: CombatSuccessFeedbackMetrics::from_review(&review),
                        witness: witness_line_from_trajectory(lane.label, trajectory),
                        source_kind: "complete_win",
                    },
                ));
            }
        } else if let Some(progress) = review.facts.diagnostic_progress.as_ref() {
            if let Some(witness) = estimated_rollout_feedback_witness(lane.label, progress) {
                let rank = estimated_rollout_feedback_rank(progress);
                if estimated_feedback_source
                    .as_ref()
                    .is_none_or(|(current, _)| rank > *current)
                {
                    estimated_feedback_source = Some((
                        rank,
                        CombatSuccessFeedbackSource {
                            spec: lane,
                            baseline: CombatSuccessFeedbackMetrics::from_review(&review),
                            witness,
                            source_kind: "estimated_rollout_frontier",
                        },
                    ));
                }
            }
        }
        lanes.push(CombatQualityLaneResult {
            lane: lane.label,
            intent: lane.intent,
            review,
            quality,
        });
    }
    let selected_lane = lanes
        .iter()
        .enumerate()
        .filter_map(|(index, lane)| lane.quality.as_ref().map(|quality| (index, quality)))
        .max_by(|(_, left), (_, right)| compare_quality(left, right))
        .map(|(index, _)| lanes[index].lane);
    let feedback_source = complete_feedback_source
        .map(|(_, source)| source)
        .or_else(|| estimated_feedback_source.map(|(_, source)| source));
    let success_feedback_rerun = feedback_source.and_then(|source| {
        run_success_feedback_rerun(
            case,
            source,
            per_lane_nodes,
            per_lane_wall_ms,
            options.action_preview_limit,
        )
    });

    CombatQualityLaneReview {
        schema: "combat_quality_lane_review_v0",
        contract: "case_level_experiment_only_same_total_budget_split_across_lanes_no_runner_policy_change",
        total_nodes,
        total_wall_ms,
        per_lane_nodes,
        per_lane_wall_ms,
        selected_lane,
        selected_reason: if selected_lane.is_some() {
            "best_complete_win_by_persistent_adjusted_hp_then_potion_conservation"
        } else {
            "no_lane_found_complete_win"
        },
        success_feedback_rerun,
        lanes,
    }
}
