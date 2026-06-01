use super::*;
use crate::content::monsters::EnemyId;
use crate::sim::combat::{CombatPosition, CombatStepLimits};
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy)]
struct ProbeWinStepper;

impl CombatStepper for ProbeWinStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        Vec::new()
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::PlayCard { .. }) {
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

    fn terminal(&self, position: &CombatPosition) -> crate::sim::combat::CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[test]
fn conservative_rollout_policy_filters_potion_actions() {
    let combat = blank_test_combat();
    let legal = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        ),
        CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
    ];

    let filtered = filtered_rollout_legal_actions(
        CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        legal,
        &combat,
    );

    assert_eq!(filtered.len(), 1);
    assert!(matches!(filtered[0].input, ClientInput::EndTurn));
}

#[test]
fn conservative_rollout_policy_reports_selection_reason() {
    let combat = non_terminal_combat();
    let legal = vec![CombatActionChoice::from_input(
        &combat,
        ClientInput::EndTurn,
    )];

    let selection = choose_rollout_action(
        CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
    )
    .expect("single legal action should be selected");

    assert_eq!(
        selection.reason,
        ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST
    );
    assert!(matches!(
        selection.choice.choice.input,
        ClientInput::EndTurn
    ));
}

#[test]
fn conservative_rollout_probe_can_select_non_first_terminal_win() {
    let combat = non_terminal_combat();
    let legal = vec![
        CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 99,
                target: None,
            },
        ),
    ];

    let selection = choose_rollout_action(
        CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
    )
    .expect("probe should select an action");

    assert_eq!(
        selection.reason,
        ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PROBE
    );
    assert!(matches!(
        selection.choice.choice.input,
        ClientInput::PlayCard { .. }
    ));
}

fn test_node(combat: CombatState) -> SearchNode {
    SearchNode {
        engine: EngineState::CombatPlayerTurn,
        combat,
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: 80,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    }
}

fn non_terminal_combat() -> CombatState {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat
}

fn test_config() -> CombatSearchV2Config {
    CombatSearchV2Config {
        max_nodes: 100,
        max_actions_per_line: 100,
        max_engine_steps_per_action: 10,
        wall_time: None,
        input_label: None,
        potion_policy: CombatSearchV2PotionPolicy::Never,
        max_potions_used: None,
        rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        rollout_max_evaluations: 10,
        rollout_max_actions: 10,
        rollout_beam_width: 3,
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        frontier_policy: CombatSearchV2FrontierPolicy::SingleQueue,
    }
}
