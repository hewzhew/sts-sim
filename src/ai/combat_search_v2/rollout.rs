use std::time::Instant;

use super::rollout_pending_choice::RolloutPendingChoiceProgress;
use super::*;

pub(super) const DEFAULT_ROLLOUT_MAX_EVALUATIONS: usize = 384;
pub(super) const DEFAULT_ROLLOUT_MAX_ACTIONS: usize = 80;

pub(super) fn conservative_no_potion_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    no_potion_rollout(
        CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        node,
        stepper,
        config,
        max_actions,
        deadline,
    )
}

pub(super) fn phase_aware_no_potion_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    no_potion_rollout(
        CombatSearchV2RolloutPolicy::PhaseAwareNoPotion,
        node,
        stepper,
        config,
        max_actions,
        deadline,
    )
}

fn no_potion_rollout(
    policy: CombatSearchV2RolloutPolicy,
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    let mut rollout = node.clone_for_rollout();
    let mut last_action_reason = None;
    let mut pending_choice_progress = RolloutPendingChoiceProgress::default();
    for actions_simulated in 0..=max_actions {
        if terminal_label(&rollout.engine, &rollout.combat) != SearchTerminalLabel::Unresolved {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::TerminalState,
                last_action_reason,
                pending_choice_progress,
            );
        }
        if actions_simulated == max_actions {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::MaxActions,
                last_action_reason,
                pending_choice_progress,
            );
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::Deadline,
                last_action_reason,
                pending_choice_progress,
            );
        }
        let phase_profile = combat_search_phase_profile(&rollout.engine, &rollout.combat);
        pending_choice_progress.observe_boundary(phase_profile.pending_choice);
        if phase_profile.pending_choice.high_fanout {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::HighFanoutPendingChoice,
                last_action_reason,
                pending_choice_progress,
            );
        }

        let position = CombatPosition::new(rollout.engine.clone(), rollout.combat.clone());
        let legal = filtered_rollout_legal_actions(
            policy,
            stepper.legal_action_choices(&position),
            &rollout.combat,
        );
        if legal.is_empty() {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::NoLegalActions,
                last_action_reason,
                pending_choice_progress,
            );
        }

        let Some(selection) = choose_rollout_action(
            policy,
            &rollout,
            stepper,
            config,
            deadline,
            &rollout.engine,
            &rollout.combat,
            legal,
        ) else {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::PolicyDeclined,
                last_action_reason,
                pending_choice_progress,
            );
        };
        last_action_reason = Some(selection.reason);
        let choice = selection.choice;
        pending_choice_progress.note_simulated_action(phase_profile.pending_choice);

        let step = stepper.apply_to_stable(
            &position,
            choice.choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline,
            },
        );
        let mut child = rollout.clone_for_child(step.position.engine, step.position.combat);
        child.note_input(&choice.choice.input);
        child.actions.push(CombatSearchV2ActionTrace {
            step_index: rollout.actions.len(),
            action_id: choice.original_action_id,
            action_key: choice.choice.action_key,
            action_debug: choice.choice.action_debug,
            input: choice.choice.input,
        });
        rollout = child;

        if step.truncated {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated + 1,
                RolloutStopReason::EngineStepLimit,
                last_action_reason,
                pending_choice_progress,
            );
        }
    }

    RolloutNodeEstimate::from_node(
        &rollout,
        max_actions,
        RolloutStopReason::MaxActions,
        last_action_reason,
        pending_choice_progress,
    )
}

impl SearchNode {
    fn clone_for_rollout(&self) -> Self {
        let mut clone = self.clone();
        clone.rollout_estimate = RolloutNodeEstimate::unevaluated();
        clone
    }
}

#[cfg(test)]
mod tests;
