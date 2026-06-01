use std::collections::HashSet;
use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::super::rollout_pending_choice::RolloutPendingChoiceProgress;
use super::super::turn_planner::{enumerate_turn_plans, TurnPlannerConfigV1};
use super::super::value::combat_eval_from_rollout_estimate;
use super::super::*;

const TURN_BEAM_ACTION_REASON: &str = "turn_beam_no_potion_selected_turn_plan_end_state";
const TURN_BEAM_NO_PLAN_REASON: &str = "turn_beam_no_potion_no_turn_plan_available";

#[derive(Clone)]
struct TurnBeamState {
    node: SearchNode,
    progress: RolloutPendingChoiceProgress,
    last_action_reason: Option<&'static str>,
}

pub(in crate::ai::combat_search_v2) fn turn_beam_no_potion_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    let root_action_count = node.actions.len();
    let beam_width = config.rollout_beam_width.max(1);
    let mut beam = vec![TurnBeamState {
        node: node.clone_for_rollout(),
        progress: RolloutPendingChoiceProgress::default(),
        last_action_reason: None,
    }];

    loop {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            return best_estimate(&beam, root_action_count, RolloutStopReason::Deadline);
        }

        if let Some(winner) = beam
            .iter()
            .filter(|state| {
                terminal_label(&state.node.engine, &state.node.combat) == SearchTerminalLabel::Win
            })
            .max_by_key(|state| state.node.combat.entities.player.current_hp)
        {
            return state_estimate(winner, root_action_count, RolloutStopReason::TerminalState);
        }
        if beam.iter().all(|state| {
            terminal_label(&state.node.engine, &state.node.combat)
                != SearchTerminalLabel::Unresolved
        }) {
            return best_estimate(&beam, root_action_count, RolloutStopReason::TerminalState);
        }

        let max_simulated = beam
            .iter()
            .map(|state| state.node.actions.len().saturating_sub(root_action_count))
            .max()
            .unwrap_or_default();
        if max_simulated >= max_actions {
            return best_estimate(&beam, root_action_count, RolloutStopReason::MaxActions);
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
                });
                continue;
            }
            if !matches!(state.node.engine, EngineState::CombatPlayerTurn) {
                stalled_stop_reason = RolloutStopReason::PolicyDeclined;
                stalled.push(TurnBeamState {
                    node: state.node.clone(),
                    progress,
                    last_action_reason: state.last_action_reason,
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
                        }),
                );
                if next.len() == before {
                    stalled_stop_reason = RolloutStopReason::MaxActions;
                    stalled.push(TurnBeamState {
                        node: state.node.clone(),
                        progress,
                        last_action_reason: state.last_action_reason,
                    });
                }
            }
        }

        if next.is_empty() {
            if !stalled.is_empty() {
                return best_estimate(&stalled, root_action_count, stalled_stop_reason);
            }
            return best_estimate(&beam, root_action_count, RolloutStopReason::NoLegalActions);
        }

        beam = select_beam(next, beam_width);
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
    RolloutNodeEstimate::from_node(
        &state.node,
        state.node.actions.len().saturating_sub(root_action_count),
        stop_reason,
        state.last_action_reason,
        state.progress,
    )
}
