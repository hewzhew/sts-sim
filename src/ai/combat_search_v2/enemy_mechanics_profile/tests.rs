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
fn profile_reports_timed_enemy_threat_aggregates() {
    let mut combat = blank_test_combat();
    let mut exploder = test_monster(EnemyId::Exploder);
    exploder.id = 7;
    combat.entities.monsters = vec![exploder];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Explosive,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);
    let report = enemy_mechanics_profile_report(profile);

    assert_eq!(profile.timed_threat_count, 1);
    assert_eq!(profile.timed_threat_min_owner_turns, Some(3));
    assert_eq!(profile.timed_threat_total_raw_damage, 30);
    assert_eq!(
        report.profiling_policy,
        "typed_enemy_mechanics_fact_profile_no_direct_score"
    );
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

#[test]
fn fungi_profile_reports_swarm_count() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::FungiBeast);
    first.id = 1;
    let mut second = test_monster(EnemyId::FungiBeast);
    second.id = 2;
    let mut third = test_monster(EnemyId::FungiBeast);
    third.id = 3;
    combat.entities.monsters = vec![first, second, third];

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.fungi_beast_count, 3);
    assert_eq!(profile.tracked_monsters, 3);
}
