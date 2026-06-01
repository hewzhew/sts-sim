use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::super::rollout_pending_choice::RolloutPendingChoiceProgress;
use super::super::turn_planner::{
    enumerate_turn_plans, TurnPlanBucket, TurnPlanEnumeration, TurnPlannerConfigV1,
};
use super::super::value::combat_eval_from_rollout_estimate;
use super::super::*;

const TURN_BEAM_ACTION_REASON: &str = "turn_beam_no_potion_selected_turn_plan_end_state";
const TURN_BEAM_NO_PLAN_REASON: &str = "turn_beam_no_potion_no_turn_plan_available";
const TURN_BEAM_CONSERVATIVE_ANCHOR_REASON: &str = "turn_beam_no_potion_conservative_anchor";
const TURN_BEAM_BOUNDARY_FALLBACK_REASON: &str =
    "turn_beam_no_potion_conservative_boundary_fallback";

#[derive(Clone)]
struct TurnBeamState {
    node: SearchNode,
    progress: RolloutPendingChoiceProgress,
    last_action_reason: Option<&'static str>,
    estimate_override: Option<RolloutNodeEstimate>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::ai::combat_search_v2) struct TurnBeamExtensionAttribution {
    pub(in crate::ai::combat_search_v2) turn_plan_calls: u64,
    pub(in crate::ai::combat_search_v2) turn_plan_inner_nodes_expanded: u64,
    pub(in crate::ai::combat_search_v2) turn_plan_inner_nodes_generated: u64,
    pub(in crate::ai::combat_search_v2) turn_plans_kept: u64,
    pub(in crate::ai::combat_search_v2) turn_plans_kept_by_bucket: BTreeMap<&'static str, u64>,
    pub(in crate::ai::combat_search_v2) terminal_candidates_kept: u64,
    pub(in crate::ai::combat_search_v2) best_pv_len: usize,
    pub(in crate::ai::combat_search_v2) best_pv_terminal: Option<SearchTerminalLabel>,
}

impl TurnBeamExtensionAttribution {
    fn observe_turn_plan_enumeration(&mut self, enumeration: &TurnPlanEnumeration) {
        self.turn_plan_calls = self.turn_plan_calls.saturating_add(1);
        self.turn_plan_inner_nodes_expanded = self
            .turn_plan_inner_nodes_expanded
            .saturating_add(enumeration.nodes_expanded as u64);
        self.turn_plan_inner_nodes_generated = self
            .turn_plan_inner_nodes_generated
            .saturating_add(enumeration.nodes_generated as u64);
        self.turn_plans_kept = self
            .turn_plans_kept
            .saturating_add(enumeration.plans.len() as u64);
        for plan in &enumeration.plans {
            *self
                .turn_plans_kept_by_bucket
                .entry(plan.bucket.label())
                .or_default() += 1;
            if plan.bucket == TurnPlanBucket::TerminalWin {
                self.terminal_candidates_kept = self.terminal_candidates_kept.saturating_add(1);
            }
        }
    }

    fn observe_best_estimate(&mut self, estimate: RolloutNodeEstimate) {
        if self.best_pv_terminal.is_none() || estimate.actions_simulated > self.best_pv_len {
            self.best_pv_len = estimate.actions_simulated;
            self.best_pv_terminal = Some(estimate.terminal);
        }
    }
}

#[cfg(test)]
pub(in crate::ai::combat_search_v2) fn turn_beam_no_potion_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    if !matches!(node.engine, EngineState::CombatPlayerTurn) {
        return turn_beam_extension_rollout(node, stepper, config, max_actions, deadline);
    }
    let anchor =
        turn_beam_conservative_anchor_rollout(node, stepper, config, max_actions, deadline);
    if anchor.terminal == SearchTerminalLabel::Win {
        return anchor;
    }
    let beam = turn_beam_extension_rollout(node, stepper, config, max_actions, deadline);
    better_estimate(beam, anchor)
}

pub(in crate::ai::combat_search_v2) fn turn_beam_conservative_anchor_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    let mut estimate =
        super::conservative_no_potion_rollout(node, stepper, config, max_actions, deadline);
    estimate.last_action_reason = Some(TURN_BEAM_CONSERVATIVE_ANCHOR_REASON);
    estimate
}

#[cfg(test)]
fn turn_beam_extension_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    turn_beam_extension_rollout_with_attribution(node, stepper, config, max_actions, deadline).0
}

pub(in crate::ai::combat_search_v2) fn turn_beam_extension_rollout_with_attribution(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> (RolloutNodeEstimate, TurnBeamExtensionAttribution) {
    let root_action_count = node.actions.len();
    let beam_width = config.rollout_beam_width.max(1);
    let mut attribution = TurnBeamExtensionAttribution::default();
    let mut beam = vec![TurnBeamState {
        node: node.clone_for_rollout(),
        progress: RolloutPendingChoiceProgress::default(),
        last_action_reason: None,
        estimate_override: None,
    }];

    loop {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            return finish_with_attribution(
                best_estimate(&beam, root_action_count, RolloutStopReason::Deadline),
                attribution,
            );
        }

        if let Some(winner) = beam
            .iter()
            .filter(|state| {
                terminal_label(&state.node.engine, &state.node.combat) == SearchTerminalLabel::Win
            })
            .max_by_key(|state| state.node.combat.entities.player.current_hp)
        {
            return finish_with_attribution(
                state_estimate(winner, root_action_count, RolloutStopReason::TerminalState),
                attribution,
            );
        }
        if beam.iter().all(|state| {
            terminal_label(&state.node.engine, &state.node.combat)
                != SearchTerminalLabel::Unresolved
        }) {
            return finish_with_attribution(
                best_estimate(&beam, root_action_count, RolloutStopReason::TerminalState),
                attribution,
            );
        }

        let max_simulated = beam
            .iter()
            .map(|state| state.node.actions.len().saturating_sub(root_action_count))
            .max()
            .unwrap_or_default();
        if max_simulated >= max_actions {
            return finish_with_attribution(
                best_estimate(&beam, root_action_count, RolloutStopReason::MaxActions),
                attribution,
            );
        }

        let mut next = Vec::new();
        let mut stalled = Vec::new();
        let mut stalled_stop_reason = RolloutStopReason::NoLegalActions;
        for state in &beam {
            let mut progress = state.progress;
            let phase_profile = combat_search_phase_profile(&state.node.engine, &state.node.combat);
            progress.observe_boundary(phase_profile.pending_choice);
            if phase_profile.pending_choice.high_fanout {
                stalled_stop_reason = RolloutStopReason::HighFanoutPendingChoice;
                stalled.push(TurnBeamState {
                    node: state.node.clone(),
                    progress,
                    last_action_reason: state.last_action_reason,
                    estimate_override: None,
                });
                continue;
            }
            if !matches!(state.node.engine, EngineState::CombatPlayerTurn) {
                let simulated = state.node.actions.len().saturating_sub(root_action_count);
                let remaining_actions = max_actions.saturating_sub(simulated);
                let mut estimate = super::conservative_no_potion_rollout(
                    &state.node,
                    stepper,
                    config,
                    remaining_actions,
                    deadline,
                );
                estimate.actions_simulated = estimate.actions_simulated.saturating_add(simulated);
                estimate.last_action_reason = Some(TURN_BEAM_BOUNDARY_FALLBACK_REASON);
                merge_prior_pending_choice_progress(&mut estimate, state.progress);
                stalled_stop_reason = estimate.stop_reason;
                stalled.push(TurnBeamState {
                    node: state.node.clone(),
                    progress,
                    last_action_reason: state.last_action_reason,
                    estimate_override: Some(estimate),
                });
                continue;
            }

            let remaining_actions = max_actions
                .saturating_sub(state.node.actions.len().saturating_sub(root_action_count));
            if remaining_actions == 0 {
                stalled_stop_reason = RolloutStopReason::MaxActions;
                stalled.push(TurnBeamState {
                    node: state.node.clone(),
                    progress,
                    last_action_reason: state.last_action_reason,
                    estimate_override: None,
                });
                continue;
            }

            let turn_config = TurnPlannerConfigV1 {
                max_end_states: beam_width.saturating_mul(2),
                per_bucket_limit: beam_width,
                potion_policy: CombatSearchV2PotionPolicy::Never,
                max_engine_steps_per_action: config.max_engine_steps_per_action,
                ..TurnPlannerConfigV1::default()
            };
            let plans = enumerate_turn_plans(&state.node, stepper, &turn_config, deadline);
            attribution.observe_turn_plan_enumeration(&plans);
            if plans.plans.is_empty() {
                let mut no_plan = state.node.clone();
                no_plan.rollout_estimate = RolloutNodeEstimate::from_node(
                    &no_plan,
                    no_plan.actions.len().saturating_sub(root_action_count),
                    RolloutStopReason::NoLegalActions,
                    Some(TURN_BEAM_NO_PLAN_REASON),
                    progress,
                );
                stalled_stop_reason = RolloutStopReason::NoLegalActions;
                stalled.push(TurnBeamState {
                    node: no_plan,
                    progress,
                    last_action_reason: Some(TURN_BEAM_NO_PLAN_REASON),
                    estimate_override: None,
                });
            } else {
                let before = next.len();
                next.extend(
                    plans
                        .plans
                        .into_iter()
                        .filter(|plan| {
                            plan.end_node
                                .actions
                                .len()
                                .saturating_sub(root_action_count)
                                <= max_actions
                        })
                        .map(|plan| TurnBeamState {
                            node: plan.end_node,
                            progress,
                            last_action_reason: Some(TURN_BEAM_ACTION_REASON),
                            estimate_override: None,
                        }),
                );
                if next.len() == before {
                    stalled_stop_reason = RolloutStopReason::MaxActions;
                    stalled.push(TurnBeamState {
                        node: state.node.clone(),
                        progress,
                        last_action_reason: state.last_action_reason,
                        estimate_override: None,
                    });
                }
            }
        }

        if next.is_empty() {
            if !stalled.is_empty() {
                return finish_with_attribution(
                    best_estimate(&stalled, root_action_count, stalled_stop_reason),
                    attribution,
                );
            }
            return finish_with_attribution(
                best_estimate(&beam, root_action_count, RolloutStopReason::NoLegalActions),
                attribution,
            );
        }

        beam = select_beam(next, beam_width);
    }
}

fn finish_with_attribution(
    estimate: RolloutNodeEstimate,
    mut attribution: TurnBeamExtensionAttribution,
) -> (RolloutNodeEstimate, TurnBeamExtensionAttribution) {
    attribution.observe_best_estimate(estimate);
    (estimate, attribution)
}

#[cfg(test)]
fn better_estimate(left: RolloutNodeEstimate, right: RolloutNodeEstimate) -> RolloutNodeEstimate {
    let left_eval = combat_eval_from_rollout_estimate(left);
    let right_eval = combat_eval_from_rollout_estimate(right);
    if right_eval > left_eval {
        right
    } else {
        left
    }
}

fn select_beam(mut candidates: Vec<TurnBeamState>, beam_width: usize) -> Vec<TurnBeamState> {
    let mut seen = HashSet::new();
    candidates.retain(|state| {
        seen.insert(combat_exact_state_key(
            &state.node.engine,
            &state.node.combat,
        ))
    });
    candidates.sort_by(|left, right| {
        let left_eval = combat_eval_from_rollout_estimate(left.node.rollout_estimate);
        let right_eval = combat_eval_from_rollout_estimate(right.node.rollout_estimate);
        right_eval.cmp(&left_eval)
    });
    candidates.truncate(beam_width.max(1));
    candidates
}

fn best_estimate(
    beam: &[TurnBeamState],
    root_action_count: usize,
    stop_reason: RolloutStopReason,
) -> RolloutNodeEstimate {
    let Some(best) = beam.iter().max_by_key(|state| {
        combat_eval_from_rollout_estimate(state_estimate(state, root_action_count, stop_reason))
    }) else {
        return RolloutNodeEstimate::unevaluated();
    };
    state_estimate(best, root_action_count, stop_reason)
}

fn state_estimate(
    state: &TurnBeamState,
    root_action_count: usize,
    stop_reason: RolloutStopReason,
) -> RolloutNodeEstimate {
    if let Some(estimate) = state.estimate_override {
        return estimate;
    }
    RolloutNodeEstimate::from_node(
        &state.node,
        state.node.actions.len().saturating_sub(root_action_count),
        stop_reason,
        state.last_action_reason,
        state.progress,
    )
}

fn merge_prior_pending_choice_progress(
    estimate: &mut RolloutNodeEstimate,
    prior: RolloutPendingChoiceProgress,
) {
    estimate.pending_choices_seen = estimate
        .pending_choices_seen
        .saturating_add(prior.pending_choices_seen);
    estimate.pending_choice_actions_simulated = estimate
        .pending_choice_actions_simulated
        .saturating_add(prior.pending_choice_actions_simulated);
    estimate.max_pending_choice_candidate_count = estimate
        .max_pending_choice_candidate_count
        .max(prior.max_pending_choice_candidate_count);
    estimate.max_pending_choice_estimated_action_fanout = estimate
        .max_pending_choice_estimated_action_fanout
        .max(prior.max_pending_choice_estimated_action_fanout);
    if estimate.last_pending_choice_kind.is_none() {
        estimate.last_pending_choice_kind = prior.last_pending_choice_kind_label();
    }
    estimate.stopped_on_high_fanout_pending_choice |= prior.stopped_on_high_fanout_pending_choice;
    estimate.high_fanout_pending_choice |= prior.stopped_on_high_fanout_pending_choice;
}
