use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    compile_combat_search_witness_prior_v0, CombatSearchV2ActionPreview,
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config, CombatSearchV2FrontierPolicy,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2Report,
    CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy, CombatSearchV2WitnessLine,
    SearchTerminalLabel,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::search_runner::run_configured_search;
use super::search_types::SearchReview;
use super::Args;

#[derive(Serialize)]
pub(super) struct CombatQualityLaneReview {
    schema: &'static str,
    contract: &'static str,
    total_nodes: usize,
    total_wall_ms: u64,
    per_lane_nodes: usize,
    per_lane_wall_ms: u64,
    selected_lane: Option<&'static str>,
    selected_reason: &'static str,
    success_feedback_rerun: Option<CombatSuccessFeedbackRerun>,
    lanes: Vec<CombatQualityLaneResult>,
}

#[derive(Serialize)]
struct CombatQualityLaneResult {
    lane: &'static str,
    intent: &'static str,
    review: SearchReview,
    quality: Option<CombatLineQuality>,
}

#[derive(Serialize)]
struct CombatSuccessFeedbackRerun {
    schema: &'static str,
    contract: &'static str,
    source_lane: &'static str,
    witness_action_count: usize,
    prior_states: usize,
    duplicate_prior_hints: usize,
    baseline: CombatSuccessFeedbackMetrics,
    rerun: SearchReview,
    comparison: CombatSuccessFeedbackComparison,
}

#[derive(Clone, Serialize)]
struct CombatSuccessFeedbackMetrics {
    complete_win: bool,
    nodes_to_first_win: Option<u64>,
    terminal_wins: u64,
    final_hp: Option<i32>,
    hp_loss: Option<i32>,
    potions_used: Option<u32>,
    nodes_expanded: u64,
    nodes_generated: u64,
    elapsed_ms: u128,
}

#[derive(Serialize)]
struct CombatSuccessFeedbackComparison {
    rerun_found_win: bool,
    first_win_nodes_delta: Option<i64>,
    terminal_wins_delta: i64,
    final_hp_delta: Option<i32>,
    hp_loss_delta: Option<i32>,
    potions_used_delta: Option<i32>,
    easier_first_win: Option<bool>,
}

#[derive(Clone, Serialize)]
pub(crate) struct CombatLineQuality {
    terminal: SearchTerminalLabel,
    hp_loss: i32,
    final_hp: i32,
    persistent_run_value: i32,
    persistent_adjusted_hp: i32,
    potions_used: u32,
    turns: u32,
    cards_played: u32,
    action_count: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct QualityLaneSpec {
    pub(crate) label: &'static str,
    intent: &'static str,
    frontier_policy: CombatSearchV2FrontierPolicy,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    rollout_policy: CombatSearchV2RolloutPolicy,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
}

struct CombatSuccessFeedbackSource {
    spec: QualityLaneSpec,
    quality: CombatLineQuality,
    baseline: CombatSuccessFeedbackMetrics,
    witness: CombatSearchV2WitnessLine,
}

impl QualityLaneSpec {
    pub(crate) fn config(self, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Config {
        CombatSearchV2Config {
            max_nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            stop_on_win_hp_loss_at_most: Some(0),
            min_win_candidates_before_stop: 4,
            potion_policy: self.potion_policy,
            max_potions_used: self.max_potions_used,
            rollout_policy: self.rollout_policy,
            child_rollout_policy: self.child_rollout_policy,
            turn_plan_policy: self.turn_plan_policy,
            frontier_policy: self.frontier_policy,
            phase_guard_policy: self.phase_guard_policy,
            ..CombatSearchV2Config::default()
        }
    }
}

impl CombatSuccessFeedbackMetrics {
    fn from_review(review: &SearchReview) -> Self {
        Self {
            complete_win: review.complete_win,
            nodes_to_first_win: review.nodes_to_first_win,
            terminal_wins: review.terminal_wins,
            final_hp: review.final_hp,
            hp_loss: review.hp_loss,
            potions_used: review.potions_used,
            nodes_expanded: review.nodes_expanded,
            nodes_generated: review.nodes_generated,
            elapsed_ms: review.elapsed_ms,
        }
    }
}

pub(super) fn run_quality_lanes(args: &Args, case: &CombatCase) -> CombatQualityLaneReview {
    let specs = quality_lane_specs();
    let lane_count = specs.len().max(1);
    let total_nodes = args
        .quality_lane_total_nodes
        .unwrap_or(args.slow_nodes)
        .max(1);
    let total_wall_ms = args.quality_lane_total_ms.unwrap_or(args.slow_ms).max(1);
    let per_lane_nodes = (total_nodes / lane_count).max(1);
    let per_lane_wall_ms = (total_wall_ms / lane_count as u64).max(1);
    let mut lanes = Vec::new();
    let mut feedback_source: Option<CombatSuccessFeedbackSource> = None;
    for lane in specs {
        let (review, report) = run_configured_search(
            lane.label,
            case,
            lane.config(per_lane_nodes, per_lane_wall_ms),
            args.action_preview_limit,
        );
        let quality = combat_line_quality(&report);
        if let (Some(quality), Some(trajectory)) =
            (quality.as_ref(), report.best_win_trajectory.as_ref())
        {
            if feedback_source
                .as_ref()
                .is_none_or(|source| !compare_quality(quality, &source.quality).is_lt())
            {
                feedback_source = Some(CombatSuccessFeedbackSource {
                    spec: lane,
                    quality: quality.clone(),
                    baseline: CombatSuccessFeedbackMetrics::from_review(&review),
                    witness: witness_line_from_trajectory(lane.label, trajectory),
                });
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
    let success_feedback_rerun = feedback_source.and_then(|source| {
        run_success_feedback_rerun(
            case,
            source,
            per_lane_nodes,
            per_lane_wall_ms,
            args.action_preview_limit,
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

pub(crate) fn witness_line_from_trajectory(
    source: &'static str,
    trajectory: &sts_simulator::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> CombatSearchV2WitnessLine {
    CombatSearchV2WitnessLine {
        source,
        terminal: trajectory.terminal,
        final_hp: trajectory.final_hp,
        total_enemy_hp: trajectory
            .enemy_final_state
            .iter()
            .filter(|enemy| enemy.alive)
            .map(|enemy| enemy.hp.max(0) + enemy.block.max(0))
            .sum(),
        action_count: Some(trajectory.actions.len()),
        actions: trajectory
            .actions
            .iter()
            .map(|action| CombatSearchV2ActionPreview {
                action_key: action.action_key.clone(),
                input: action.input.clone(),
            })
            .collect(),
    }
}

fn run_success_feedback_rerun(
    case: &CombatCase,
    source: CombatSuccessFeedbackSource,
    max_nodes: usize,
    wall_ms: u64,
    action_preview_limit: usize,
) -> Option<CombatSuccessFeedbackRerun> {
    let witness_prior = compile_combat_search_witness_prior_v0(&case.position, &source.witness);
    if witness_prior.prior.is_empty() {
        return None;
    }
    let prior_states = witness_prior.prior_states;
    let duplicate_prior_hints = witness_prior.duplicate_prior_hints;
    let mut config = source.spec.config(max_nodes, wall_ms);
    config.input_label = Some(format!("success_feedback_rerun:{}", source.spec.label));
    config.root_action_prior = Some(witness_prior.prior);
    let (rerun, _report) = run_configured_search(
        "quality_success_feedback_rerun",
        case,
        config,
        action_preview_limit,
    );
    let comparison = compare_success_feedback(&source.baseline, &rerun);
    Some(CombatSuccessFeedbackRerun {
        schema: "combat_success_feedback_rerun_v0",
        contract: "best_complete_quality_lane_win_compiled_to_exact_state_action_prior_then_rerun_with_same_lane_budget",
        source_lane: source.spec.label,
        witness_action_count: source.witness.actions.len(),
        prior_states,
        duplicate_prior_hints,
        baseline: source.baseline,
        rerun,
        comparison,
    })
}

fn compare_success_feedback(
    baseline: &CombatSuccessFeedbackMetrics,
    rerun: &SearchReview,
) -> CombatSuccessFeedbackComparison {
    let first_win_nodes_delta = match (baseline.nodes_to_first_win, rerun.nodes_to_first_win) {
        (Some(base), Some(next)) => Some(next as i64 - base as i64),
        _ => None,
    };
    CombatSuccessFeedbackComparison {
        rerun_found_win: rerun.complete_win,
        first_win_nodes_delta,
        terminal_wins_delta: rerun.terminal_wins as i64 - baseline.terminal_wins as i64,
        final_hp_delta: baseline
            .final_hp
            .zip(rerun.final_hp)
            .map(|(base, next)| next - base),
        hp_loss_delta: baseline
            .hp_loss
            .zip(rerun.hp_loss)
            .map(|(base, next)| next - base),
        potions_used_delta: baseline
            .potions_used
            .zip(rerun.potions_used)
            .map(|(base, next)| next as i32 - base as i32),
        easier_first_win: first_win_nodes_delta.map(|delta| delta < 0),
    }
}

pub(crate) fn quality_lane_specs() -> [QualityLaneSpec; 4] {
    [
        QualityLaneSpec {
            label: "quality_balanced_rr",
            intent: "baseline round-robin frontier with adaptive rollout",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
        QualityLaneSpec {
            label: "quality_champ_split_guard",
            intent: "penalize crossing Champ half-hp threshold before a clear burst window",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
            max_potions_used: Some(2),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::ChampSplitGuard,
        },
        QualityLaneSpec {
            label: "quality_immediate_rescue_no_potion",
            intent: "force immediate child rollout so low-hp tactical lines are not under-sampled",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
        QualityLaneSpec {
            label: "quality_immediate_potion_rescue",
            intent:
                "try semantic potion rescue with immediate rollout before declaring a combat gap",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
            max_potions_used: Some(2),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
    ]
}

pub(crate) fn combat_line_quality(report: &CombatSearchV2Report) -> Option<CombatLineQuality> {
    let trajectory = report.best_win_trajectory.as_ref()?;
    Some(CombatLineQuality {
        terminal: trajectory.terminal,
        hp_loss: trajectory.hp_loss,
        final_hp: trajectory.final_hp,
        persistent_run_value: trajectory.persistent_run_value,
        persistent_adjusted_hp: trajectory
            .final_hp
            .saturating_add(trajectory.persistent_run_value),
        potions_used: trajectory.potions_used,
        turns: trajectory.turns,
        cards_played: trajectory.cards_played,
        action_count: trajectory.actions.len(),
    })
}

pub(crate) fn compare_quality(
    left: &CombatLineQuality,
    right: &CombatLineQuality,
) -> std::cmp::Ordering {
    (
        left.persistent_adjusted_hp,
        left.final_hp,
        left.persistent_run_value,
        -(left.potions_used as i32),
        -(left.turns as i32),
        -(left.cards_played as i32),
        -(left.action_count as i32),
    )
        .cmp(&(
            right.persistent_adjusted_hp,
            right.final_hp,
            right.persistent_run_value,
            -(right.potions_used as i32),
            -(right.turns as i32),
            -(right.cards_played as i32),
            -(right.action_count as i32),
        ))
}
