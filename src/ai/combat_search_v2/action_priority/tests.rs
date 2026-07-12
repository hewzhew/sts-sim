use super::*;
use crate::ai::combat_search_v2::{
    CombatSearchActionOrderingPlugins, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2SetupBiasPolicy,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{ClientInput, EngineState};
use crate::state::selection::{SelectionResolution, SelectionScope};
use crate::test_support::{blank_test_combat, test_monster};

fn grid_select(uuids: impl IntoIterator<Item = u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(SelectionScope::Grid, uuids))
}

fn add_explosive(combat: &mut crate::runtime::combat::CombatState, entity_id: usize, amount: i32) {
    combat
        .entities
        .power_db
        .entry(entity_id)
        .or_default()
        .push(crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Explosive,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        });
}

fn priority_for_input(
    engine: &EngineState,
    combat: &crate::runtime::combat::CombatState,
    input: &ClientInput,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
    setup_bias_policy: CombatSearchV2SetupBiasPolicy,
) -> ActionOrderingPriority {
    priority_for_input_with_plugins(
        engine,
        combat,
        input,
        CombatSearchActionOrderingPlugins {
            root_action_prior: None,
            phase_guard: phase_guard_policy.into(),
            action_prior: setup_bias_policy.into(),
        },
    )
}

#[test]
fn non_player_turn_priority_is_neutral() {
    let combat = blank_test_combat();

    let priority = priority_for_input(
        &EngineState::CombatProcessing,
        &combat,
        &ClientInput::EndTurn,
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(priority.role, ActionOrderingRole::Neutral);
}

#[test]
fn lethal_play_card_gets_lethal_role() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::Cultist);
    monster.current_hp = 6;
    monster.max_hp = 6;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let priority = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(priority.role, ActionOrderingRole::LethalCard);
}

#[test]
fn timed_threat_damage_progress_orders_before_neutral_target() {
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
    add_explosive(&mut combat, 2, 3);

    let neutral = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let timed = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(neutral.targets_timed_threat, 0);
    assert_eq!(timed.targets_timed_threat, 1);
    assert_eq!(timed.timed_threat_urgency, -3);
    assert_eq!(timed.timed_threat_raw_damage, 30);
    assert!(timed > neutral);
}

#[test]
fn timed_threat_damage_progress_prefers_shorter_countdown() {
    let mut combat = blank_test_combat();
    let mut slower = test_monster(EnemyId::Exploder);
    slower.id = 1;
    slower.current_hp = 40;
    slower.max_hp = 40;
    let mut sooner = test_monster(EnemyId::Exploder);
    sooner.id = 2;
    sooner.current_hp = 40;
    sooner.max_hp = 40;
    combat.entities.monsters = vec![slower, sooner];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    add_explosive(&mut combat, 1, 3);
    add_explosive(&mut combat, 2, 1);

    let slower = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let sooner = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(slower.timed_threat_urgency, -3);
    assert_eq!(sooner.timed_threat_urgency, -1);
    assert!(sooner > slower);
}

#[test]
fn timed_threat_damage_progress_is_neutral_without_power() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::Repulsor);
    first.id = 1;
    first.current_hp = 40;
    first.max_hp = 40;
    let mut second = test_monster(EnemyId::Exploder);
    second.id = 2;
    second.current_hp = 40;
    second.max_hp = 40;
    combat.entities.monsters = vec![first, second];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let first = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let second = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(first.targets_timed_threat, 0);
    assert_eq!(second.targets_timed_threat, 0);
    assert_eq!(first.timed_threat_urgency, 0);
    assert_eq!(second.timed_threat_urgency, 0);
    assert_eq!(first.timed_threat_raw_damage, 0);
    assert_eq!(second.timed_threat_raw_damage, 0);
    assert_eq!(first, second);
}

#[test]
fn pending_choice_priority_uses_structured_selection_role() {
    let mut combat = blank_test_combat();
    combat.zones.discard_pile = vec![CombatCard::new(CardId::Carnage, 20)];
    let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::Discard,
        candidate_uuids: vec![20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: crate::state::core::GridSelectReason::MoveToDrawPile,
    });

    let priority = priority_for_input(
        &engine,
        &combat,
        &grid_select([20]),
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(
        priority.role,
        ActionOrderingRole::PendingChoiceValueSelection
    );
    assert!(priority.pending_choice_primary > 0);
}

#[test]
fn sleeping_lagavulin_wake_damage_has_phase_penalty() {
    let mut combat = blank_test_combat();
    let mut lagavulin = test_monster(EnemyId::Lagavulin);
    lagavulin.id = 1;
    lagavulin.lagavulin.is_out = false;
    combat.entities.monsters = vec![lagavulin];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let priority = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert!(priority.phase_hint.has_signal());
    assert!(priority.phase_setup < 0);
    assert!(priority.phase_transition_safety < 0);
}

#[test]
fn key_card_setup_bias_promotes_strength_scaling_power() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 50;
    monster.max_hp = 50;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::DemonForm, 11),
    ];

    let default_setup = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    assert_eq!(default_setup.role, ActionOrderingRole::DeferredSetup);

    let biased_setup = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::KeyCardOnline,
    );
    let strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::KeyCardOnline,
    );

    assert_eq!(biased_setup.role, ActionOrderingRole::KeySetupCard);
    assert!(biased_setup > strike);
}

#[test]
fn collector_boss_race_prior_targets_collector_before_torch_head() {
    let mut combat = blank_test_combat();
    let mut collector = test_monster(EnemyId::TheCollector);
    collector.id = 1;
    collector.current_hp = 282;
    collector.max_hp = 282;
    let mut torch = test_monster(EnemyId::TorchHead);
    torch.id = 2;
    torch.current_hp = 40;
    torch.max_hp = 40;
    combat.entities.monsters = vec![collector, torch];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let collector_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorBossRace,
    );
    let torch_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorBossRace,
    );

    assert!(collector_target.collector_tactic > torch_target.collector_tactic);
    assert!(collector_target > torch_target);
}

#[test]
fn collector_control_prior_focuses_weaker_head_while_two_live() {
    let mut combat = blank_test_combat();
    let mut collector = test_monster(EnemyId::TheCollector);
    collector.id = 1;
    collector.current_hp = 282;
    collector.max_hp = 282;
    let mut weak_torch = test_monster(EnemyId::TorchHead);
    weak_torch.id = 2;
    weak_torch.current_hp = 20;
    weak_torch.max_hp = 40;
    let mut healthy_torch = test_monster(EnemyId::TorchHead);
    healthy_torch.id = 3;
    healthy_torch.current_hp = 40;
    healthy_torch.max_hp = 40;
    combat.entities.monsters = vec![collector, weak_torch, healthy_torch];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let weak_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorSingleHeadControl,
    );
    let healthy_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(3),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorSingleHeadControl,
    );

    assert!(weak_target.collector_tactic > healthy_target.collector_tactic);
    assert!(weak_target > healthy_target);
}

#[test]
fn collector_control_prior_preserves_last_head_and_targets_collector() {
    let mut combat = blank_test_combat();
    let mut collector = test_monster(EnemyId::TheCollector);
    collector.id = 1;
    collector.current_hp = 282;
    collector.max_hp = 282;
    let mut torch = test_monster(EnemyId::TorchHead);
    torch.id = 2;
    torch.current_hp = 5;
    torch.max_hp = 40;
    combat.entities.monsters = vec![collector, torch];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let collector_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorSingleHeadControl,
    );
    let torch_target = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::CollectorSingleHeadControl,
    );

    assert!(collector_target.collector_tactic > torch_target.collector_tactic);
    assert!(collector_target > torch_target);
}

#[test]
fn premature_fiend_fire_waits_behind_access_when_it_would_burn_key_hand_resources() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 80;
    monster.max_hp = 80;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::FiendFire, 10),
        CombatCard::new(CardId::Offering, 11),
        CombatCard::new(CardId::SpotWeakness, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let fiend_fire = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let offering = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert!(fiend_fire.resource_timing < 0);
    assert!(fiend_fire.role_rank < constants::ROLE_DAMAGE_PROGRESS);
    assert!(
        offering > fiend_fire,
        "premature Fiend Fire should not outrank access while burning Offering/Spot Weakness"
    );
}

#[test]
fn lethal_fiend_fire_keeps_finisher_priority_even_when_it_consumes_hand_resources() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 20;
    monster.max_hp = 20;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::FiendFire, 10),
        CombatCard::new(CardId::Offering, 11),
        CombatCard::new(CardId::SpotWeakness, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let fiend_fire = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let offering = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(fiend_fire.role, ActionOrderingRole::LethalCard);
    assert!(fiend_fire > offering);
}
