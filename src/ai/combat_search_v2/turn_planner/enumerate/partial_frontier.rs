use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::ai::combat_search_v2::rollout_pending_choice::RolloutPendingChoiceProgress;
use crate::ai::combat_search_v2::turn_planner::types::TurnPlannerConfigV1;
use crate::ai::combat_search_v2::value::{combat_eval_from_rollout_estimate, CombatEvalV2};
use crate::ai::combat_search_v2::{RolloutNodeEstimate, RolloutStopReason};

use super::work::TurnPlanWorkNode;

const PARTIAL_FRONTIER_MIN_WIDTH: usize = 8;
const PARTIAL_FRONTIER_MAX_WIDTH: usize = 32;
const PARTIAL_FRONTIER_END_STATE_MULTIPLIER: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct PartialActionScore {
    sustained_mitigation: i32,
    visible_mitigation: i32,
    debuff_setup: i32,
    progress_hint: i32,
    focused_target_progress: i32,
    access_gain: i32,
    resource_timing: i32,
    reactive_safety: i32,
}

struct ScoredWorkNode {
    work: Option<TurnPlanWorkNode>,
    first_action_key: String,
    eval: CombatEvalV2,
    action: PartialActionScore,
    living_enemy_score: i32,
}

pub(super) fn select_partial_frontier(
    candidates: Vec<TurnPlanWorkNode>,
    config: &TurnPlannerConfigV1,
    root_action_len: usize,
) -> Vec<TurnPlanWorkNode> {
    let width = partial_frontier_width(config);
    if candidates.len() <= width {
        return candidates;
    }

    let mut scored = candidates
        .into_iter()
        .map(|work| score_work_node(work, root_action_len))
        .collect::<Vec<_>>();
    let balanced = ordered_indexes(&scored, compare_balanced);
    let survival = ordered_indexes(&scored, compare_survival);
    let progress = ordered_indexes(&scored, compare_progress);
    let setup = ordered_indexes(&scored, compare_setup);

    let mut selected = Vec::with_capacity(width);
    let mut selected_flags = vec![false; scored.len()];
    let mut covered_first_actions = HashSet::new();
    for &index in &balanced {
        if covered_first_actions.insert(scored[index].first_action_key.clone()) {
            select_index(index, &mut selected, &mut selected_flags, width);
            if selected.len() >= width {
                break;
            }
        }
    }

    let lanes = [&survival, &progress, &setup, &balanced];
    let mut lane_offsets = [0usize; 4];
    while selected.len() < width {
        let mut added = false;
        for (lane_index, lane) in lanes.iter().enumerate() {
            while lane_offsets[lane_index] < lane.len() {
                let index = lane[lane_offsets[lane_index]];
                lane_offsets[lane_index] = lane_offsets[lane_index].saturating_add(1);
                if selected_flags[index] {
                    continue;
                }
                select_index(index, &mut selected, &mut selected_flags, width);
                added = true;
                break;
            }
            if selected.len() >= width {
                break;
            }
        }
        if !added {
            break;
        }
    }

    selected
        .into_iter()
        .filter_map(|index| scored[index].work.take())
        .collect()
}

fn partial_frontier_width(config: &TurnPlannerConfigV1) -> usize {
    config
        .max_end_states
        .max(1)
        .saturating_mul(PARTIAL_FRONTIER_END_STATE_MULTIPLIER)
        .clamp(PARTIAL_FRONTIER_MIN_WIDTH, PARTIAL_FRONTIER_MAX_WIDTH)
        .min(config.max_inner_nodes.max(1))
}

fn score_work_node(work: TurnPlanWorkNode, root_action_len: usize) -> ScoredWorkNode {
    let actions_simulated = work.node.actions.len().saturating_sub(root_action_len);
    let estimate = RolloutNodeEstimate::from_node(
        &work.node,
        actions_simulated,
        RolloutStopReason::MaxActions,
        None,
        RolloutPendingChoiceProgress::default(),
    );
    let eval = combat_eval_from_rollout_estimate(&estimate);
    let action = partial_action_score(&work);
    let first_action_key = work
        .node
        .actions
        .get(root_action_len)
        .map(|action| action.action_key.clone())
        .unwrap_or_default();
    let living_enemy_score = -(work
        .node
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count() as i32);
    ScoredWorkNode {
        work: Some(work),
        first_action_key,
        eval,
        action,
        living_enemy_score,
    }
}

fn partial_action_score(work: &TurnPlanWorkNode) -> PartialActionScore {
    let mut score = PartialActionScore {
        sustained_mitigation: 0,
        visible_mitigation: 0,
        debuff_setup: 0,
        progress_hint: 0,
        focused_target_progress: 0,
        access_gain: 0,
        resource_timing: 0,
        reactive_safety: 0,
    };
    let mut target_progress = HashMap::<usize, i32>::new();
    for facts in &work.action_facts {
        score.sustained_mitigation = score
            .sustained_mitigation
            .saturating_add(facts.mechanics.direct.persistent_enemy_strength_down);
        score.visible_mitigation = score
            .visible_mitigation
            .saturating_add(facts.mechanics.direct.temporary_enemy_strength_down)
            .saturating_add(facts.mechanics.direct.visible_attack_mitigation_hint)
            .saturating_add(facts.mechanics.derived.enemy_weak);
        score.debuff_setup = score
            .debuff_setup
            .saturating_add(facts.mechanics.derived.enemy_vulnerable);
        score.progress_hint = score
            .progress_hint
            .saturating_add(
                facts
                    .immediate
                    .target_progress_hint
                    .max(facts.immediate.all_enemy_progress_hint),
            )
            .saturating_add(facts.mechanics.reactive.enemy_damage);
        if let Some(target) = facts.target.as_ref() {
            let progress = facts
                .immediate
                .target_progress_hint
                .saturating_add(facts.mechanics.reactive.enemy_damage);
            let accumulated = target_progress.entry(target.target_slot).or_default();
            *accumulated = accumulated.saturating_add(progress);
        }
        score.access_gain = score.access_gain.saturating_add(
            facts
                .exact_one_step_delta
                .hand_delta
                .saturating_add(1)
                .max(0)
                .saturating_add(facts.exact_one_step_delta.energy_delta.max(0)),
        );
        score.resource_timing = score
            .resource_timing
            .saturating_add(facts.mechanics.resource_timing.ordering_score);
        score.reactive_safety = score.reactive_safety.saturating_sub(
            facts
                .mechanics
                .derived
                .enemy_strength_gain
                .saturating_add(facts.mechanics.derived.visible_attack_pressure_hint)
                .saturating_add(facts.mechanics.reactive.player_hp_loss)
                .saturating_add(facts.mechanics.reactive.bad_draw_cards)
                .saturating_add(i32::from(facts.mechanics.reactive.forced_turn_end)),
        );
    }
    score.focused_target_progress = target_progress.values().copied().max().unwrap_or_default();
    score
}

fn ordered_indexes(
    scored: &[ScoredWorkNode],
    compare: fn(&ScoredWorkNode, &ScoredWorkNode) -> Ordering,
) -> Vec<usize> {
    let mut indexes = (0..scored.len()).collect::<Vec<_>>();
    indexes.sort_by(|left, right| {
        compare(&scored[*right], &scored[*left]).then_with(|| left.cmp(right))
    });
    indexes
}

fn compare_balanced(left: &ScoredWorkNode, right: &ScoredWorkNode) -> Ordering {
    left.eval
        .cmp(&right.eval)
        .then_with(|| left.action.cmp(&right.action))
}

fn compare_survival(left: &ScoredWorkNode, right: &ScoredWorkNode) -> Ordering {
    left.eval
        .outcome_class()
        .cmp(&right.eval.outcome_class())
        .then_with(|| {
            left.eval
                .survival_bucket()
                .cmp(&right.eval.survival_bucket())
        })
        .then_with(|| left.eval.risk_margin().cmp(&right.eval.risk_margin()))
        .then_with(|| left.living_enemy_score.cmp(&right.living_enemy_score))
        .then_with(|| {
            left.action
                .sustained_mitigation
                .cmp(&right.action.sustained_mitigation)
        })
        .then_with(|| {
            left.action
                .visible_mitigation
                .cmp(&right.action.visible_mitigation)
        })
        .then_with(|| left.eval.cmp(&right.eval))
}

fn compare_progress(left: &ScoredWorkNode, right: &ScoredWorkNode) -> Ordering {
    left.eval
        .outcome_class()
        .cmp(&right.eval.outcome_class())
        .then_with(|| {
            left.eval
                .progress_bucket()
                .cmp(&right.eval.progress_bucket())
        })
        .then_with(|| left.living_enemy_score.cmp(&right.living_enemy_score))
        .then_with(|| {
            left.action
                .focused_target_progress
                .cmp(&right.action.focused_target_progress)
        })
        .then_with(|| left.eval.enemy_progress().cmp(&right.eval.enemy_progress()))
        .then_with(|| left.action.progress_hint.cmp(&right.action.progress_hint))
        .then_with(|| {
            left.eval
                .survival_bucket()
                .cmp(&right.eval.survival_bucket())
        })
        .then_with(|| left.eval.risk_margin().cmp(&right.eval.risk_margin()))
        .then_with(|| left.eval.cmp(&right.eval))
}

fn compare_setup(left: &ScoredWorkNode, right: &ScoredWorkNode) -> Ordering {
    left.eval
        .outcome_class()
        .cmp(&right.eval.outcome_class())
        .then_with(|| left.action.access_gain.cmp(&right.action.access_gain))
        .then_with(|| {
            left.action
                .sustained_mitigation
                .cmp(&right.action.sustained_mitigation)
        })
        .then_with(|| {
            left.action
                .visible_mitigation
                .cmp(&right.action.visible_mitigation)
        })
        .then_with(|| left.action.debuff_setup.cmp(&right.action.debuff_setup))
        .then_with(|| {
            left.action
                .resource_timing
                .cmp(&right.action.resource_timing)
        })
        .then_with(|| {
            left.action
                .reactive_safety
                .cmp(&right.action.reactive_safety)
        })
        .then_with(|| left.eval.cmp(&right.eval))
}

fn select_index(
    index: usize,
    selected: &mut Vec<usize>,
    selected_flags: &mut [bool],
    width: usize,
) {
    if selected.len() >= width || selected_flags[index] {
        return;
    }
    selected_flags[index] = true;
    selected.push(index);
}
