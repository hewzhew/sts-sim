use super::super::*;
use super::support::*;

#[test]
fn silent_starter_neutralize_deals_damage_and_applies_weak() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Neutralize, 100));

    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );

    assert!(ok);
    assert_eq!(combat.entities.monsters[0].current_hp, 37);
    assert_eq!(combat.get_power(1, PowerId::Weak), 1);
}

#[test]
fn survivor_requires_discard_and_moves_selected_card_to_discard() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Survivor, 100));
    combat.zones.hand.push(CombatCard::new(CardId::Strike, 101));

    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(ok);
    assert_eq!(combat.entities.player.block, 8);
    assert!(matches!(engine_state, EngineState::CombatPlayerTurn));
    assert!(combat.zones.hand.is_empty());
    assert!(combat.zones.discard_pile.iter().any(|c| c.uuid == 101));
}

#[test]
fn acrobatics_draws_then_discards_selected_card() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 200),
        CombatCard::new(CardId::Defend, 201),
        CombatCard::new(CardId::Bash, 202),
    ];
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Acrobatics, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Survivor, 101));

    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(ok);
    assert!(matches!(engine_state, EngineState::PendingChoice(_)));
    assert!(combat.zones.hand.iter().any(|c| c.uuid == 101));
    assert_eq!(combat.zones.hand.len(), 4);

    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::SubmitHandSelect(vec![101]),
    );

    assert!(ok);
    assert!(matches!(engine_state, EngineState::CombatPlayerTurn));
    assert_eq!(combat.zones.hand.len(), 3);
    assert!(combat.zones.discard_pile.iter().any(|c| c.uuid == 101));
}

#[test]
fn blade_dance_and_cloak_and_dagger_create_shivs() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::BladeDance, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::CloakAndDagger, 101));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(
        combat
            .zones
            .hand
            .iter()
            .filter(|c| c.id == CardId::Shiv)
            .count(),
        3
    );

    let cloak_index = combat
        .zones
        .hand
        .iter()
        .position(|c| c.id == CardId::CloakAndDagger)
        .expect("cloak and dagger should still be in hand");
    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: cloak_index,
            target: None,
        },
    ));
    assert_eq!(combat.entities.player.block, 6);
    assert_eq!(
        combat
            .zones
            .hand
            .iter()
            .filter(|c| c.id == CardId::Shiv)
            .count(),
        4
    );
}

#[test]
fn poison_chain_and_tick_work_for_silent_cards() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.turn.energy = 5;
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::DeadlyPoison, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::BouncingFlask, 101));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Catalyst, 102));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert_eq!(combat.get_power(1, PowerId::Poison), 5);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.get_power(1, PowerId::Poison), 14);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert_eq!(combat.get_power(1, PowerId::Poison), 28);

    let poison_actions =
        crate::content::powers::resolve_power_at_turn_start(PowerId::Poison, &combat, 1, 28);
    for action in poison_actions {
        combat.engine.action_queue.push_back(action);
    }
    drain_processing(&mut engine_state, &mut combat);

    assert_eq!(combat.entities.monsters[0].current_hp, 12);
    assert_eq!(combat.get_power(1, PowerId::Poison), 27);
}

#[test]
fn noxious_fumes_hits_all_enemies_after_post_draw() {
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
        hexaghost: Default::default(),
        darkling: Default::default(),
    });
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::NoxiousFumes, 100));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.get_power(0, PowerId::NoxiousFumes), 2);

    let actions =
        crate::content::powers::resolve_power_on_post_draw(PowerId::NoxiousFumes, &combat, 0, 2);
    for action in actions {
        combat.engine.action_queue.push_back(action);
    }
    drain_processing(&mut engine_state, &mut combat);

    assert_eq!(combat.get_power(1, PowerId::Poison), 2);
    assert_eq!(combat.get_power(2, PowerId::Poison), 2);
}

#[test]
fn prepared_draws_then_discards_selected_card() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 200)];
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Prepared, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Survivor, 101));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert!(matches!(engine_state, EngineState::PendingChoice(_)));
    assert_eq!(combat.zones.hand.len(), 2);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::SubmitHandSelect(vec![101]),
    ));
    assert!(matches!(engine_state, EngineState::CombatPlayerTurn));
    assert_eq!(combat.zones.hand.len(), 1);
    assert!(combat.zones.discard_pile.iter().any(|c| c.uuid == 101));
}

#[test]
fn dagger_throw_deals_damage_draws_and_discards() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 200)];
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::DaggerThrow, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Survivor, 101));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert!(matches!(engine_state, EngineState::PendingChoice(_)));
    assert_eq!(combat.entities.monsters[0].current_hp, 31);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::SubmitHandSelect(vec![101]),
    ));
    assert!(matches!(engine_state, EngineState::CombatPlayerTurn));
    assert_eq!(combat.zones.hand.len(), 1);
    assert!(combat.zones.discard_pile.iter().any(|c| c.uuid == 101));
}

#[test]
fn poisoned_stab_deals_damage_and_applies_poison() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::PoisonedStab, 100));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert_eq!(combat.entities.monsters[0].current_hp, 34);
    assert_eq!(combat.get_power(1, PowerId::Poison), 3);
}

#[test]
fn dagger_spray_hits_all_enemies_twice() {
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
        hexaghost: Default::default(),
        darkling: Default::default(),
    });
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::DaggerSpray, 100));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.entities.monsters[0].current_hp, 32);
    assert_eq!(combat.entities.monsters[1].current_hp, 27);
}

#[test]
fn adrenaline_gains_energy_draws_and_exhausts() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.turn.energy = 1;
    combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 200),
        CombatCard::new(CardId::Defend, 201),
    ];
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Adrenaline, 100));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.turn.energy, 2);
    assert_eq!(combat.zones.hand.len(), 2);
    assert!(combat
        .zones
        .exhaust_pile
        .iter()
        .any(|c| c.id == CardId::Adrenaline));
}

#[test]
fn after_image_grants_block_on_followup_card() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::AfterImage, 100));
    combat.zones.hand.push(CombatCard::new(CardId::Strike, 101));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.get_power(0, PowerId::AfterImage), 1);
    assert_eq!(combat.entities.player.block, 0);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert_eq!(combat.entities.player.block, 1);
    assert_eq!(combat.entities.monsters[0].current_hp, 34);
}

#[test]
fn burst_duplicates_next_skill() {
    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = test_combat();
    combat.zones.hand.push(CombatCard::new(CardId::Burst, 100));
    combat
        .zones
        .hand
        .push(CombatCard::new(CardId::DeadlyPoison, 101));

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    ));
    assert_eq!(combat.get_power(0, PowerId::Burst), 1);

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ));
    assert_eq!(combat.get_power(0, PowerId::Burst), 0);
    assert_eq!(combat.get_power(1, PowerId::Poison), 10);
}