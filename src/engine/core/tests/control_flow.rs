use super::super::*;
use super::support::*;
use crate::engine::test_support::combat_with_hand;

#[test]
fn auto_resolves_single_required_hand_choice() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    let mut card = CombatCard::new(CardId::Strike, 100);
    card.base_damage_mut = 6;
    combat.zones.hand.push(card);
    combat
        .engine
        .action_queue
        .push_back(Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Upgradeable,
            reason: crate::state::HandSelectReason::Upgrade,
        });

    assert!(tick_engine(&mut engine_state, &mut combat, None));
    assert!(!matches!(engine_state, EngineState::PendingChoice(_)));
    assert_eq!(combat.zones.hand[0].upgrades, 1);
}

#[test]
fn empty_required_hand_choice_safely_noops() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    combat
        .engine
        .action_queue
        .push_back(Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Upgradeable,
            reason: crate::state::HandSelectReason::Upgrade,
        });

    assert!(tick_engine(&mut engine_state, &mut combat, None));
    assert!(!matches!(engine_state, EngineState::PendingChoice(_)));
    assert!(combat.engine.action_queue.is_empty());
}

#[test]
fn player_turn_auto_selects_single_target() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.zones.hand.push(CombatCard::new(CardId::Strike, 100));

    let result = handle_player_turn_input(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(result.is_ok());
    assert!(!combat.engine.action_queue.is_empty());
}

#[test]
fn player_turn_rejects_missing_target_when_multiple_exist() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.entities.monsters.push(MonsterEntity {
        id: 2,
        monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
        current_hp: 35,
        max_hp: 35,
        block: 0,
        slot: 1,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        next_move_byte: 0,
        current_intent: Intent::Unknown,
        move_history: VecDeque::new(),
        intent_dmg: 0,
        logical_position: 1,
        protocol_identity: Default::default(),
        hexaghost: Default::default(),
        chosen: Default::default(),
        darkling: Default::default(),
        lagavulin: Default::default(),
    });
    combat.zones.hand.push(CombatCard::new(CardId::Strike, 100));

    let result = handle_player_turn_input(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert_eq!(
        result.unwrap_err(),
        "Multiple targets available. Must specify a target."
    );
}

#[test]
fn discovery_card_uses_class_pool_not_colorless_pool() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    combat
        .engine
        .action_queue
        .push_back(Action::SuspendForDiscovery {
            colorless: false,
            card_type: None,
            cost_for_turn: Some(0),
        });

    assert!(tick_engine(&mut engine_state, &mut combat, None));

    let EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards)) = &engine_state else {
        panic!("expected discovery pending choice");
    };

    assert_eq!(cards.len(), 3);
    for &card_id in cards {
        assert!(!crate::content::cards::COLORLESS_UNCOMMON_POOL.contains(&card_id));
        assert!(!crate::content::cards::COLORLESS_RARE_POOL.contains(&card_id));
    }
}

#[test]
fn colorless_discovery_uses_colorless_pool() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    combat
        .engine
        .action_queue
        .push_back(Action::SuspendForDiscovery {
            colorless: true,
            card_type: None,
            cost_for_turn: Some(0),
        });

    assert!(tick_engine(&mut engine_state, &mut combat, None));

    let EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards)) = &engine_state else {
        panic!("expected discovery pending choice");
    };

    assert_eq!(cards.len(), 3);
    for &card_id in cards {
        let is_colorless = crate::content::cards::COLORLESS_UNCOMMON_POOL.contains(&card_id)
            || crate::content::cards::COLORLESS_RARE_POOL.contains(&card_id);
        assert!(is_colorless);
        let def = crate::content::cards::get_card_definition(card_id);
        assert!(!def.tags.contains(&crate::content::cards::CardTag::Healing));
    }
}

#[test]
fn smoke_bomb_escape_runs_victory_hooks_before_reward_transition() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    combat.turn.current_phase = CombatPhase::TurnTransition;
    combat.turn.counters.player_escaping = true;
    combat.entities.player.current_hp = 40;
    combat.entities.player.max_hp = 80;
    combat
        .entities
        .player
        .relics
        .push(RelicState::new(RelicId::BurningBlood));
    combat.entities.player.relic_buses.on_victory.push(0);

    assert!(tick_engine(&mut engine_state, &mut combat, None));
    assert_eq!(engine_state, EngineState::CombatProcessing);
    assert_eq!(combat.entities.player.current_hp, 46);
    assert!(combat.turn.counters.escape_pending_reward);

    assert!(!tick_engine(&mut engine_state, &mut combat, None));
    assert!(matches!(engine_state, EngineState::RewardScreen(_)));
}

#[test]
fn half_dead_awakened_one_does_not_trigger_victory_transition() {
    let mut engine_state = EngineState::CombatProcessing;
    let mut combat = test_combat();
    combat.entities.monsters[0].monster_type =
        crate::content::monsters::EnemyId::AwakenedOne as usize;
    combat.entities.monsters[0].current_hp = 0;
    combat.entities.monsters[0].half_dead = true;
    combat.entities.monsters[0].is_dying = false;
    combat.entities.monsters[0].current_intent = Intent::Unknown;
    combat.turn.current_phase = CombatPhase::PlayerTurn;

    assert!(tick_engine(&mut engine_state, &mut combat, None));
    assert!(!matches!(engine_state, EngineState::RewardScreen(_)));
    assert!(!matches!(
        engine_state,
        EngineState::GameOver(RunResult::Victory)
    ));
}

#[test]
fn pain_in_hand_triggers_hp_loss_when_another_card_is_played() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = combat_with_hand(&[CardId::Strike, CardId::Pain])
        .with_player_hp(20)
        .with_energy(1);

    let _ = tick_engine(
        &mut engine_state,
        &mut combat,
        Some(ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }),
    );

    while matches!(engine_state, EngineState::CombatProcessing) {
        assert!(tick_engine(&mut engine_state, &mut combat, None));
    }

    assert_eq!(combat.entities.player.current_hp, 19);
}

#[test]
fn pain_lethal_short_circuits_remaining_card_resolution() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = combat_with_hand(&[CardId::Strike, CardId::Pain])
        .with_player_hp(1)
        .with_energy(1);
    let monster_hp_before = combat.entities.monsters[0].current_hp;

    let _ = tick_engine(
        &mut engine_state,
        &mut combat,
        Some(ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }),
    );

    while matches!(engine_state, EngineState::CombatProcessing) {
        tick_engine(&mut engine_state, &mut combat, None);
    }

    assert!(matches!(
        engine_state,
        EngineState::GameOver(RunResult::Defeat)
    ));
    assert_eq!(combat.entities.monsters[0].current_hp, monster_hp_before);
    assert!(combat.zones.discard_pile.is_empty());
    assert_eq!(combat.zones.limbo.len(), 1);
}
