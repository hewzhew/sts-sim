use super::super::*;
use super::support::*;

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
        hexaghost: Default::default(),
        darkling: Default::default(),
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