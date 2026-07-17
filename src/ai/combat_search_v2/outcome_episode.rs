use super::pending_choice_action_prefix::canonical_pending_choice_inputs;
use super::rollout_profile::RolloutPerformanceCounters;
use super::*;

const POLICY: CombatSearchRolloutPluginId = CombatSearchRolloutPluginId::PhaseAwareNoPotion;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CombatOutcomePolicyEpisodeStopV1 {
    Terminal,
    MaxActions,
    NoLegalAction,
    PolicyDeclined,
    EngineStepLimit,
}

pub(crate) struct CombatOutcomePolicyEpisodeV1 {
    pub(crate) terminal: CombatTerminal,
    pub(crate) observed_player_turns: Vec<CombatPosition>,
    pub(crate) final_hp: i32,
    pub(crate) final_max_hp: i32,
    pub(crate) stop: CombatOutcomePolicyEpisodeStopV1,
}

/// Executes one deterministic behavior-policy episode. It never branches and
/// never treats a budget stop as a loss. Structured selections choose the
/// first canonical legal transaction without materializing the full powerset.
pub(crate) fn run_combat_outcome_policy_episode_v1(
    root: &CombatPosition,
    max_actions: usize,
) -> CombatOutcomePolicyEpisodeV1 {
    let stepper = EngineCombatStepper;
    let config = CombatSearchV2Config::default();
    let mut node = SearchNode::root(root.engine.clone(), root.combat.clone());
    let mut observed_player_turns = vec![root.clone()];
    let mut last_observed_turn = root.combat.turn.turn_count;
    let mut performance = RolloutPerformanceCounters::default();

    for actions_simulated in 0..=max_actions {
        let terminal = combat_terminal(&node.engine, &node.combat);
        if terminal != CombatTerminal::Unresolved {
            return episode(
                &node,
                terminal,
                observed_player_turns,
                CombatOutcomePolicyEpisodeStopV1::Terminal,
            );
        }
        if actions_simulated == max_actions {
            return episode(
                &node,
                terminal,
                observed_player_turns,
                CombatOutcomePolicyEpisodeStopV1::MaxActions,
            );
        }

        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let canonical_selection = if let EngineState::PendingChoice(choice) = &node.engine {
            canonical_pending_choice_inputs(choice)
                .and_then(|mut inputs| {
                    inputs.find_map(|input| stepper.choice_for_legal_input(&position, &input))
                })
                .map(|choice| (choice, None))
        } else {
            None
        };
        let selection = if canonical_selection.is_some() {
            canonical_selection
        } else {
            let legal = filtered_rollout_legal_actions(
                POLICY,
                stepper.atomic_action_choices(&position),
                &node.combat,
            );
            if legal.is_empty() {
                return episode(
                    &node,
                    terminal,
                    observed_player_turns,
                    CombatOutcomePolicyEpisodeStopV1::NoLegalAction,
                );
            }
            choose_rollout_action(
                POLICY,
                &node,
                &stepper,
                &config,
                None,
                &node.engine,
                &node.combat,
                legal,
                &mut performance,
            )
            .map(|selection| (selection.choice.choice, selection.cached_step))
        };
        let Some((choice, cached_step)) = selection else {
            return episode(
                &node,
                terminal,
                observed_player_turns,
                CombatOutcomePolicyEpisodeStopV1::PolicyDeclined,
            );
        };
        let step = cached_step.unwrap_or_else(|| {
            stepper.apply_to_stable(
                &position,
                choice.input.clone(),
                CombatStepLimits {
                    max_engine_steps: config.max_engine_steps_per_action,
                    deadline: None,
                },
            )
        });
        let mut child = node.clone_for_child(step.position.engine, step.position.combat);
        child.note_input(&choice.input);
        child.push_action(CombatSearchV2ActionTrace {
            step_index: node.actions.len(),
            action_id: 0,
            action_key: choice.action_key,
            action_debug: choice.action_debug,
            input: choice.input,
        });
        node = child;
        if node.combat.turn.turn_count != last_observed_turn
            && matches!(node.engine, EngineState::CombatPlayerTurn)
        {
            observed_player_turns.push(CombatPosition::new(
                node.engine.clone(),
                node.combat.clone(),
            ));
            last_observed_turn = node.combat.turn.turn_count;
        }
        if step.truncated || step.timed_out {
            return episode(
                &node,
                combat_terminal(&node.engine, &node.combat),
                observed_player_turns,
                CombatOutcomePolicyEpisodeStopV1::EngineStepLimit,
            );
        }
    }
    unreachable!("bounded episode loop always returns")
}

fn episode(
    node: &SearchNode,
    terminal: CombatTerminal,
    observed_player_turns: Vec<CombatPosition>,
    stop: CombatOutcomePolicyEpisodeStopV1,
) -> CombatOutcomePolicyEpisodeV1 {
    CombatOutcomePolicyEpisodeV1 {
        terminal,
        observed_player_turns,
        final_hp: node.combat.entities.player.current_hp,
        final_max_hp: node.combat.entities.player.max_hp,
        stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn action_budget_stop_remains_unknown_instead_of_becoming_a_loss() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
        let root = CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let episode = run_combat_outcome_policy_episode_v1(&root, 0);

        assert_eq!(episode.stop, CombatOutcomePolicyEpisodeStopV1::MaxActions);
        assert_eq!(episode.terminal, CombatTerminal::Unresolved);
    }
}
