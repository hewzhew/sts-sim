// Oracle: Java source + invariant checks
// Evidence:
// - docs/protocol/GUARDIAN_THRESHOLD_TEST_MATRIX.md
// - Java Guardian / Mode Shift source inspection used during test design
//
// This file intentionally avoids exact boundary cases whose oracle has not yet
// been separately confirmed by Java source or live runtime evidence.

use sts_simulator::content::powers::store;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::fixtures::author_spec::AuthorCardSpec;
use sts_simulator::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use sts_simulator::runtime::action::{Action, DamageInfo, DamageType};
use sts_simulator::runtime::combat::{CombatState, Intent, PowerId};
use sts_simulator::EntityId;

fn guardian_combat() -> CombatState {
    let spec = CombatStartSpec {
        name: "guardian-threshold-behavior".to_string(),
        player_class: "ironclad".to_string(),
        ascension_level: 0,
        encounter_id: "guardian".to_string(),
        room_type: "boss".to_string(),
        seed: 1,
        player_current_hp: 80,
        player_max_hp: 80,
        relics: vec![],
        potions: vec![],
        master_deck: vec![
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Bash".to_string()),
        ],
    };

    let (_engine_state, combat) = compile_combat_start_spec(&spec).expect("compile guardian spec");
    combat
}

fn guardian_id(state: &CombatState) -> EntityId {
    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.monster_type == sts_simulator::content::monsters::EnemyId::TheGuardian as usize)
        .map(|monster| monster.id)
        .expect("guardian should exist")
}

fn guardian(state: &CombatState) -> &sts_simulator::runtime::combat::MonsterEntity {
    let guardian_id = guardian_id(state);
    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == guardian_id)
        .expect("guardian entity should exist")
}

fn drain_action_queue(state: &mut CombatState) {
    while let Some(action) = state.engine.action_queue.pop_front() {
        execute_action(action, state);
    }
}

fn run_guardian_turn_once(state: &mut CombatState) {
    let guardian = guardian(state).clone();
    let actions = sts_simulator::content::monsters::resolve_monster_turn(state, &guardian);
    for action in actions {
        state.engine.action_queue.push_back(action);
    }
    drain_action_queue(state);
}

fn player_hits_guardian(state: &mut CombatState, amount: i32) {
    let guardian_id = guardian_id(state);
    execute_action(
        Action::Damage(DamageInfo {
            source: 0,
            target: guardian_id,
            base: amount,
            output: amount,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        state,
    );
}

#[test]
fn guardian_below_threshold_hit_does_not_switch_modes() {
    let mut combat = guardian_combat();
    let guardian_id = guardian_id(&combat);

    player_hits_guardian(&mut combat, 9);

    let guardian = guardian(&combat);
    assert_eq!(guardian.current_hp, guardian.max_hp - 9);
    assert_eq!(guardian.block, 0);
    assert_eq!(guardian.current_intent, Intent::Defend);
    assert_eq!(guardian.next_move_byte, 6);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 21);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 30);
    assert!(combat.engine.action_queue.is_empty());
}

#[test]
fn guardian_exact_threshold_hit_triggers_queued_defensive_mode_switch() {
    let mut combat = guardian_combat();
    let guardian_id = guardian_id(&combat);

    player_hits_guardian(&mut combat, 30);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.current_hp, guardian_entity.max_hp - 30);
    assert_eq!(guardian_entity.block, 0);
    assert_eq!(guardian_entity.current_intent, Intent::Defend);
    assert_eq!(guardian_entity.next_move_byte, 6);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 30);
    assert_eq!(combat.engine.action_queue.len(), 3);

    drain_action_queue(&mut combat);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.current_hp, guardian_entity.max_hp - 30);
    assert_eq!(guardian_entity.block, 20);
    assert_eq!(guardian_entity.current_intent, Intent::Buff);
    assert_eq!(guardian_entity.next_move_byte, 1);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
}

#[test]
fn guardian_threshold_overflow_hit_applies_full_damage_before_switch_queue_resolves() {
    let mut combat = guardian_combat();
    let guardian_id = guardian_id(&combat);

    player_hits_guardian(&mut combat, 35);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.current_hp, guardian_entity.max_hp - 35);
    assert_eq!(guardian_entity.block, 0);
    assert_eq!(guardian_entity.current_intent, Intent::Defend);
    assert_eq!(guardian_entity.next_move_byte, 6);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 30);
    assert_eq!(combat.engine.action_queue.len(), 3);

    drain_action_queue(&mut combat);

    let guardian_entity = guardian(&combat);
    assert_eq!(
        guardian_entity.current_hp,
        guardian_entity.max_hp - 35
    );
    assert_eq!(guardian_entity.block, 20);
    assert_eq!(guardian_entity.current_intent, Intent::Buff);
    assert_eq!(guardian_entity.next_move_byte, 1);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
}

#[test]
fn guardian_reapplies_mode_shift_from_increased_threshold_after_defensive_cycle() {
    let mut combat = guardian_combat();
    let guardian_id = guardian_id(&combat);

    player_hits_guardian(&mut combat, 35);
    drain_action_queue(&mut combat);

    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(guardian(&combat).next_move_byte, 1);

    run_guardian_turn_once(&mut combat);
    assert_eq!(guardian(&combat).next_move_byte, 3);
    assert_eq!(guardian(&combat).current_intent, Intent::Attack { damage: 9, hits: 1 });

    run_guardian_turn_once(&mut combat);
    assert_eq!(guardian(&combat).next_move_byte, 4);
    assert_eq!(
        guardian(&combat).current_intent,
        Intent::AttackBuff { damage: 8, hits: 2 }
    );

    run_guardian_turn_once(&mut combat);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.block, 0);
    assert_eq!(guardian_entity.next_move_byte, 5);
    assert_eq!(
        guardian_entity.current_intent,
        Intent::Attack { damage: 5, hits: 4 }
    );
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 40);
    assert!(!store::has_power(&combat, guardian_id, PowerId::SharpHide));
}

#[test]
fn guardian_second_trigger_raises_threshold_to_fifty_and_reapplies_mode_shift() {
    let mut combat = guardian_combat();
    let guardian_id = guardian_id(&combat);

    player_hits_guardian(&mut combat, 35);
    drain_action_queue(&mut combat);

    run_guardian_turn_once(&mut combat);
    run_guardian_turn_once(&mut combat);
    run_guardian_turn_once(&mut combat);

    assert_eq!(guardian(&combat).next_move_byte, 5);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 40);

    player_hits_guardian(&mut combat, 40);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.current_hp, guardian_entity.max_hp - 75);
    assert_eq!(guardian_entity.block, 0);
    assert_eq!(guardian_entity.current_intent, Intent::Attack { damage: 5, hits: 4 });
    assert_eq!(guardian_entity.next_move_byte, 5);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 40);
    assert_eq!(combat.engine.action_queue.len(), 3);

    drain_action_queue(&mut combat);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.current_hp, guardian_entity.max_hp - 75);
    assert_eq!(guardian_entity.block, 20);
    assert_eq!(guardian_entity.current_intent, Intent::Buff);
    assert_eq!(guardian_entity.next_move_byte, 1);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 0);
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 50);

    run_guardian_turn_once(&mut combat);
    run_guardian_turn_once(&mut combat);
    run_guardian_turn_once(&mut combat);

    let guardian_entity = guardian(&combat);
    assert_eq!(guardian_entity.next_move_byte, 5);
    assert_eq!(
        guardian_entity.current_intent,
        Intent::Attack { damage: 5, hits: 4 }
    );
    assert_eq!(combat.get_power(guardian_id, PowerId::GuardianThreshold), 50);
    assert_eq!(combat.get_power(guardian_id, PowerId::ModeShift), 50);
    assert!(!store::has_power(&combat, guardian_id, PowerId::SharpHide));
}
