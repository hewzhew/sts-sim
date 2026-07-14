use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) fn promote_replayable_terminal_rollout(
    loop_state: &mut SearchLoopState,
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
) {
    if loop_state.trajectories.best_win.is_some() {
        return;
    }
    let Some(witness) = loop_state
        .rollout_cache
        .best_replayable_terminal_win
        .clone()
    else {
        return;
    };
    let Some(node) = replay_terminal_witness(root, &witness, stepper, config) else {
        return;
    };

    if loop_state.remember_win(node, config) {
        loop_state.mark_accepted_complete_candidate();
    }
}

fn replay_terminal_witness(
    root: &SearchNode,
    witness: &RolloutNodeEstimate,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
) -> Option<SearchNode> {
    if !witness.is_replayable_terminal_win() {
        return None;
    }

    let mut node = root.clone();
    node.rollout_estimate = RolloutNodeEstimate::unevaluated();
    for action in &witness.action_preview {
        if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Unresolved {
            return None;
        }
        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let (action_id, choice) = filtered_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &node.combat,
        )
        .into_iter()
        .enumerate()
        .find(|(_, choice)| {
            choice.input == action.input && choice.action_key == action.action_key
        })?;
        let step = stepper.apply_to_stable(
            &position,
            choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return None;
        }

        let mut child = node.clone_for_child(step.position.engine, step.position.combat);
        let transition = classify_turn_branch_transition(
            &node.engine,
            &node.combat,
            &choice.input,
            &child.engine,
            &child.combat,
        );
        child.note_turn_prefix(&node.combat, &choice.input, transition);
        child.note_input(&choice.input);
        child.note_turn_branch_priority(transition.frontier_priority_hint());
        child.actions.push(CombatSearchV2ActionTrace {
            step_index: node.actions.len(),
            action_id,
            action_key: choice.action_key,
            action_debug: choice.action_debug,
            input: choice.input,
        });
        node = child;
    }

    exact_terminal_matches_witness(&node, witness).then_some(node)
}

fn exact_terminal_matches_witness(node: &SearchNode, witness: &RolloutNodeEstimate) -> bool {
    let phase_profile = combat_search_phase_profile(&node.engine, &node.combat);
    terminal_label(&node.engine, &node.combat) == SearchTerminalLabel::Win
        && witness.terminal == SearchTerminalLabel::Win
        && node.actions.len() == witness.total_actions
        && node.combat.entities.player.current_hp == witness.final_hp
        && node.combat.turn.turn_count == witness.turns
        && node.potions_used == witness.potions_used
        && node.potions_discarded == witness.potions_discarded
        && node.cards_played == witness.cards_played
        && living_enemy_count(&node.combat) == witness.living_enemy_count
        && phase_profile.enemy_phase.raw_living_enemy_hp == witness.total_enemy_hp
        && phase_profile.enemy_phase.raw_living_enemy_block == witness.total_enemy_block
}
