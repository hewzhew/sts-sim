// Oracle: Java source + invariant checks
// Evidence:
// - cardcrawl/actions/unique/ApplyStasisAction.java
// - cardcrawl/powers/StasisPower.java
//
// This file currently covers the first stable Stasis guarantees:
// - Bronze Orb captures a real draw/discard card into limbo
// - the Stasis power tracks the captured card UUID
// - on death, the captured card returns as a copy to hand or discard

use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::factory::EncounterId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::powers::store;
use sts_simulator::engine::action_handlers::{check_and_trigger_monster_death, execute_action};
use sts_simulator::fixtures::combat_start_spec::build_natural_start_state;
use sts_simulator::map::node::RoomType;
use sts_simulator::runtime::action::Action;
use sts_simulator::runtime::combat::{CombatCard, CombatState, PowerId};
use sts_simulator::state::run::RunState;

fn automaton_combat() -> CombatState {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 80;
    run_state.max_hp = 80;
    run_state.master_deck = vec![
        CombatCard::new(CardId::Strike, 10_000),
        CombatCard::new(CardId::Strike, 10_001),
        CombatCard::new(CardId::Strike, 10_002),
        CombatCard::new(CardId::Strike, 10_003),
        CombatCard::new(CardId::Strike, 10_004),
        CombatCard::new(CardId::Defend, 10_005),
        CombatCard::new(CardId::Defend, 10_006),
        CombatCard::new(CardId::Defend, 10_007),
        CombatCard::new(CardId::Defend, 10_008),
        CombatCard::new(CardId::Bash, 10_009),
    ];

    let (_engine_state, combat) =
        build_natural_start_state(&mut run_state, EncounterId::Automaton, RoomType::MonsterRoomBoss)
            .expect("compile automaton spec");
    combat
}

fn bronze_orb_id(state: &CombatState) -> usize {
    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.monster_type == EnemyId::BronzeOrb as usize)
        .map(|monster| monster.id)
        .expect("bronze automaton encounter should contain a BronzeOrb")
}

fn automaton_id(state: &CombatState) -> usize {
    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.monster_type == EnemyId::BronzeAutomaton as usize)
        .map(|monster| monster.id)
        .expect("automaton encounter should contain BronzeAutomaton")
}

fn drain_action_queue(state: &mut CombatState) {
    while let Some(action) = state.engine.action_queue.pop_front() {
        execute_action(action, state);
    }
}

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn spawn_bronze_orbs(state: &mut CombatState) {
    if state
        .entities
        .monsters
        .iter()
        .any(|monster| monster.monster_type == EnemyId::BronzeOrb as usize)
    {
        return;
    }

    let automaton = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == automaton_id(state))
        .expect("automaton should exist")
        .clone();
    let actions = sts_simulator::content::monsters::resolve_monster_turn(state, &automaton);
    for action in actions {
        state.engine.action_queue.push_back(action);
    }
    drain_action_queue(state);
}

#[test]
fn stasis_captures_a_real_card_into_limbo_and_tracks_its_uuid() {
    let mut combat = automaton_combat();
    spawn_bronze_orbs(&mut combat);
    let orb_id = bronze_orb_id(&combat);

    combat.zones.hand.clear();
    combat.zones.draw_pile = vec![card(CardId::Strike, 20_001)];
    combat.zones.discard_pile.clear();
    combat.zones.exhaust_pile.clear();
    combat.zones.limbo.clear();
    combat.zones.card_uuid_counter = 30_000;

    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);

    assert!(combat.zones.draw_pile.is_empty());
    assert!(combat.zones.discard_pile.is_empty());
    assert_eq!(combat.zones.limbo.len(), 1);

    let captured = &combat.zones.limbo[0];
    assert_eq!(captured.id, CardId::Strike);
    assert_eq!(captured.uuid, 20_001);

    let stasis = store::powers_for(&combat, orb_id)
        .and_then(|powers| powers.iter().find(|power| power.power_type == PowerId::Stasis))
        .expect("BronzeOrb should gain Stasis");
    assert_eq!(stasis.amount, -1);
    assert_eq!(stasis.extra_data, 20_001);
}

#[test]
fn stasis_returns_a_copy_to_hand_when_orb_dies_and_hand_has_space() {
    let mut combat = automaton_combat();
    spawn_bronze_orbs(&mut combat);
    let orb_id = bronze_orb_id(&combat);

    combat.zones.hand.clear();
    combat.zones.draw_pile = vec![card(CardId::Strike, 20_001)];
    combat.zones.discard_pile.clear();
    combat.zones.exhaust_pile.clear();
    combat.zones.limbo.clear();
    combat.zones.card_uuid_counter = 30_000;

    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);

    let orb = combat
        .entities
        .monsters
        .iter_mut()
        .find(|monster| monster.id == orb_id)
        .expect("orb should exist");
    orb.current_hp = 0;

    check_and_trigger_monster_death(&mut combat, orb_id);
    drain_action_queue(&mut combat);

    assert!(combat.zones.limbo.is_empty());
    assert_eq!(combat.zones.hand.len(), 1);
    let returned = &combat.zones.hand[0];
    assert_eq!(returned.id, CardId::Strike);
    assert_ne!(returned.uuid, 20_001);
    assert!(combat.zones.discard_pile.is_empty());
}

#[test]
fn stasis_returns_a_copy_to_discard_when_orb_dies_and_hand_is_full() {
    let mut combat = automaton_combat();
    spawn_bronze_orbs(&mut combat);
    let orb_id = bronze_orb_id(&combat);

    combat.zones.hand = vec![
        card(CardId::Strike, 21_000),
        card(CardId::Strike, 21_001),
        card(CardId::Strike, 21_002),
        card(CardId::Strike, 21_003),
        card(CardId::Strike, 21_004),
        card(CardId::Defend, 21_005),
        card(CardId::Defend, 21_006),
        card(CardId::Defend, 21_007),
        card(CardId::Defend, 21_008),
        card(CardId::Bash, 21_009),
    ];
    combat.zones.draw_pile = vec![card(CardId::Strike, 20_001)];
    combat.zones.discard_pile.clear();
    combat.zones.exhaust_pile.clear();
    combat.zones.limbo.clear();
    combat.zones.card_uuid_counter = 30_000;

    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);

    let orb = combat
        .entities
        .monsters
        .iter_mut()
        .find(|monster| monster.id == orb_id)
        .expect("orb should exist");
    orb.current_hp = 0;

    check_and_trigger_monster_death(&mut combat, orb_id);
    drain_action_queue(&mut combat);

    assert!(combat.zones.limbo.is_empty());
    assert_eq!(combat.zones.hand.len(), 10);
    assert_eq!(combat.zones.discard_pile.len(), 1);
    let returned = &combat.zones.discard_pile[0];
    assert_eq!(returned.id, CardId::Strike);
    assert_ne!(returned.uuid, 20_001);
}

#[test]
fn stasis_prefers_rare_then_uncommon_then_common_when_selecting_from_draw_pile() {
    let mut combat = automaton_combat();
    spawn_bronze_orbs(&mut combat);
    let orb_id = bronze_orb_id(&combat);

    combat.zones.hand.clear();
    combat.zones.discard_pile.clear();
    combat.zones.exhaust_pile.clear();
    combat.zones.limbo.clear();
    combat.zones.card_uuid_counter = 30_000;

    combat.zones.draw_pile = vec![
        card(CardId::Clash, 22_001),
        card(CardId::Inflame, 22_002),
        card(CardId::DemonForm, 22_003),
    ];
    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);
    assert_eq!(combat.zones.limbo.len(), 1);
    assert_eq!(combat.zones.limbo[0].id, CardId::DemonForm);
    assert_eq!(combat.zones.limbo[0].uuid, 22_003);
    assert_eq!(combat.zones.draw_pile.len(), 2);
    combat.zones.limbo.clear();
    store::retain_entity_powers(&mut combat, orb_id, |power| power.power_type != PowerId::Stasis);

    combat.zones.draw_pile = vec![card(CardId::Clash, 22_011), card(CardId::Inflame, 22_012)];
    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);
    assert_eq!(combat.zones.limbo.len(), 1);
    assert_eq!(combat.zones.limbo[0].id, CardId::Inflame);
    assert_eq!(combat.zones.limbo[0].uuid, 22_012);
    assert_eq!(combat.zones.draw_pile.len(), 1);
    combat.zones.limbo.clear();
    store::retain_entity_powers(&mut combat, orb_id, |power| power.power_type != PowerId::Stasis);

    combat.zones.draw_pile = vec![card(CardId::Clash, 22_021)];
    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);
    assert_eq!(combat.zones.limbo.len(), 1);
    assert_eq!(combat.zones.limbo[0].id, CardId::Clash);
    assert_eq!(combat.zones.limbo[0].uuid, 22_021);
    assert!(combat.zones.draw_pile.is_empty());
}

#[test]
fn stasis_uses_discard_pile_when_draw_pile_is_empty() {
    let mut combat = automaton_combat();
    spawn_bronze_orbs(&mut combat);
    let orb_id = bronze_orb_id(&combat);

    combat.zones.hand.clear();
    combat.zones.draw_pile.clear();
    combat.zones.discard_pile = vec![card(CardId::Inflame, 23_001)];
    combat.zones.exhaust_pile.clear();
    combat.zones.limbo.clear();
    combat.zones.card_uuid_counter = 30_000;

    execute_action(Action::ApplyStasis { target_id: orb_id }, &mut combat);
    drain_action_queue(&mut combat);

    assert!(combat.zones.draw_pile.is_empty());
    assert!(combat.zones.discard_pile.is_empty());
    assert_eq!(combat.zones.limbo.len(), 1);
    assert_eq!(combat.zones.limbo[0].id, CardId::Inflame);
    assert_eq!(combat.zones.limbo[0].uuid, 23_001);

    let stasis = store::powers_for(&combat, orb_id)
        .and_then(|powers| powers.iter().find(|power| power.power_type == PowerId::Stasis))
        .expect("BronzeOrb should gain Stasis from discard");
    assert_eq!(stasis.extra_data, 23_001);
}
