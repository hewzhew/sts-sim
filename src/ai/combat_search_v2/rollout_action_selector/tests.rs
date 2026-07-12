use super::*;
use crate::ai::combat_search_v2::rollout_profile::RolloutPerformanceCounters;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
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
        CombatSearchRolloutPluginId::ConservativeNoPotion,
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
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut RolloutPerformanceCounters::default(),
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
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut RolloutPerformanceCounters::default(),
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

#[test]
fn conservative_rollout_probe_does_not_rescore_fallback_candidate() {
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
    let mut performance = RolloutPerformanceCounters::default();

    let selection = choose_rollout_action(
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut performance,
    )
    .expect("probe should select an action");

    assert_eq!(
        selection.reason,
        ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PROBE
    );
    assert!(selection.cached_step.is_some());
    assert_eq!(performance.no_potion_probe_score_calls, 2);
    assert_eq!(performance.no_potion_probe_actions_evaluated, 2);
}

#[test]
fn conservative_rollout_reuses_timed_threat_ordering() {
    let mut combat = blank_test_combat();
    let mut neutral = test_monster(EnemyId::Repulsor);
    neutral.id = 1;
    neutral.current_hp = 40;
    neutral.max_hp = 40;
    let mut timed = test_monster(EnemyId::Exploder);
    timed.id = 2;
    timed.current_hp = 40;
    timed.max_hp = 40;
    combat.entities.monsters = vec![neutral, timed];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    combat.entities.power_db.insert(
        2,
        vec![Power {
            power_type: PowerId::Explosive,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    let legal = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
        ),
    ];

    let selection = choose_rollout_action(
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut RolloutPerformanceCounters::default(),
    )
    .expect("rollout should select an action");

    assert!(matches!(
        selection.choice.choice.input,
        ClientInput::PlayCard {
            target: Some(2),
            ..
        }
    ));
}

#[test]
fn conservative_rollout_reuses_attack_retaliation_ordering() {
    let mut combat = blank_test_combat();
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 1;
    spiker.current_hp = 40;
    spiker.max_hp = 40;
    combat.entities.monsters = vec![spiker];
    combat.zones.hand = vec![
        CombatCard::new(CardId::TwinStrike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Thorns,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    let legal = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let selection = choose_rollout_action(
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut RolloutPerformanceCounters::default(),
    )
    .expect("rollout should select an action");

    assert!(matches!(
        selection.choice.choice.input,
        ClientInput::PlayCard { card_index: 1, .. }
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
        action_prior_score: None,
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
        stop_on_win_hp_loss_at_most: None,
        min_win_candidates_before_stop: 1,
        input_label: None,
        potion_policy: CombatSearchV2PotionPolicy::Never,
        max_potions_used: None,
        rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
        rollout_max_evaluations: 10,
        rollout_max_actions: 10,
        rollout_beam_width: 3,
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        frontier_policy: CombatSearchV2FrontierPolicy::SingleQueue,
        phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        setup_bias_policy: CombatSearchV2SetupBiasPolicy::Default,
        turn_plan_probe_max_inner_nodes: None,
        turn_plan_probe_max_end_states: None,
        turn_plan_probe_per_bucket_limit: None,
        root_action_prior: None,
        turn_plan_prior: None,
    }
}
