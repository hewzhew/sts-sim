use super::*;
use crate::runtime::combat::{Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn guardian_profile_reports_mode_shift_remaining() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 7;
    guardian.guardian.is_open = true;
    combat.entities.monsters = vec![guardian];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::ModeShift,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.guardian_open_count, 1);
    assert_eq!(profile.guardian_min_mode_shift_remaining, Some(4));
}

#[test]
fn gremlin_nob_profile_reports_anger_amount() {
    let mut combat = blank_test_combat();
    let mut nob = test_monster(EnemyId::GremlinNob);
    nob.id = 9;
    combat.entities.monsters = vec![nob];
    combat.entities.power_db.insert(
        9,
        vec![Power {
            power_type: PowerId::Anger,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.gremlin_nob_enrage_count, 1);
    assert_eq!(profile.gremlin_nob_anger_amount_total, 2);
}

#[test]
fn bronze_automaton_profile_reports_spawn_and_stasis_pressure() {
    let mut combat = blank_test_combat();
    let mut automaton = test_monster(EnemyId::BronzeAutomaton);
    automaton.id = 1;
    automaton.bronze_automaton.first_turn = true;
    let mut orb = test_monster(EnemyId::BronzeOrb);
    orb.id = 2;
    orb.bronze_orb.used_stasis = false;
    orb.set_planned_move_id(3);
    combat.entities.monsters = vec![automaton, orb];

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.bronze_automaton_count, 1);
    assert_eq!(profile.bronze_automaton_spawn_orbs_pending_count, 1);
    assert_eq!(profile.bronze_orb_count, 1);
    assert_eq!(profile.bronze_orb_stasis_pending_count, 1);
}

#[test]
fn healer_profile_reports_support_enemy() {
    let mut combat = blank_test_combat();
    let mut healer = test_monster(EnemyId::Healer);
    healer.id = 2;
    combat.entities.monsters = vec![healer];

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.healer_support_count, 1);
    assert_eq!(profile.tracked_monsters, 1);
}
