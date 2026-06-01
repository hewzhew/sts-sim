use super::*;
use crate::content::monsters::EnemyId;
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy)]
struct PotionWinStepper;

impl CombatStepper for PotionWinStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        vec![
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
            ClientInput::EndTurn,
        ]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::UsePotion { .. }) {
            EngineState::GameOver(crate::state::core::RunResult::Victory)
        } else {
            position.engine.clone()
        };
        let position = CombatPosition::new(engine, position.combat.clone());
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[derive(Clone, Copy)]
struct ReversePotionWinStepper;

impl CombatStepper for ReversePotionWinStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        vec![
            ClientInput::EndTurn,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        ]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::UsePotion { .. }) {
            EngineState::GameOver(crate::state::core::RunResult::Victory)
        } else {
            position.engine.clone()
        };
        let position = CombatPosition::new(engine, position.combat.clone());
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[derive(Clone, Copy)]
struct PendingChoiceResolveStepper;

impl CombatStepper for PendingChoiceResolveStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if matches!(position.engine, EngineState::PendingChoice(_)) {
            vec![
                ClientInput::SubmitDiscoverChoice(0),
                ClientInput::SubmitDiscoverChoice(1),
            ]
        } else {
            Vec::new()
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::SubmitDiscoverChoice(_)) {
            EngineState::CombatPlayerTurn
        } else {
            position.engine.clone()
        };
        let position = CombatPosition::new(engine, position.combat.clone());
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[test]
fn max_potions_used_cuts_potion_branches_without_disabling_policy_all() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let mut config = CombatSearchV2Config {
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(0),
        max_nodes: 8,
        ..CombatSearchV2Config::default()
    };

    let blocked = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config.clone(),
        &PotionWinStepper,
    );

    assert!(!blocked.outcome.complete_trajectory_found);
    assert!(blocked.frontier.potion_budget_cut_count > 0);
    assert!(blocked
        .diagnostics
        .diagnosis
        .contains(&"potion_budget_cutoffs"));

    config.max_potions_used = Some(1);
    let allowed = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config,
        &PotionWinStepper,
    );

    assert!(allowed.outcome.complete_trajectory_found);
    assert_eq!(
        allowed
            .best_complete_trajectory
            .as_ref()
            .map(|trajectory| trajectory.potions_used),
        Some(1)
    );
}

#[test]
fn action_ordering_preserves_original_action_id_in_trace() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let config = CombatSearchV2Config {
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_nodes: 8,
        ..CombatSearchV2Config::default()
    };

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config,
        &ReversePotionWinStepper,
    );

    let first_action_id = report
        .best_complete_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.actions.first())
        .map(|action| action.action_id);

    assert_eq!(first_action_id, Some(1));
}

#[test]
fn pending_choice_contract_counts_exact_child_resolution() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let config = CombatSearchV2Config {
        max_nodes: 1,
        rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
        ..CombatSearchV2Config::default()
    };

    let report = run_combat_search_v2_with_stepper(
        &EngineState::PendingChoice(crate::state::core::PendingChoice::StanceChoice),
        &combat,
        config,
        &PendingChoiceResolveStepper,
    );

    assert_eq!(report.diagnostics.pending_choice.pending_choice_states, 1);
    assert_eq!(
        report
            .diagnostics
            .pending_choice
            .expanded_pending_choice_states,
        1
    );
    assert_eq!(
        report
            .diagnostics
            .pending_choice
            .legal_actions_from_pending_choice,
        2
    );
    assert_eq!(report.diagnostics.pending_choice.resolved_children, 2);
    assert_eq!(report.diagnostics.pending_choice.still_pending_children, 0);
    assert!(report
        .diagnostics
        .pending_choice
        .ordering_role_counts
        .iter()
        .any(|role| role.role == "pending_choice_neutral_selection"
            && role.actions == 2
            && role.first_actions == 1));
    assert!(report
        .diagnostics
        .diagnosis
        .contains(&"pending_choice_contract_observed"));
}
