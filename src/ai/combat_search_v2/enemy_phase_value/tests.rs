use super::*;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn split_pending_counts_inherited_child_hp_as_phase_debt() {
    let mut combat = blank_test_combat();
    let mut slime = test_monster(EnemyId::AcidSlimeL);
    slime.id = 7;
    slime.current_hp = 30;
    slime.max_hp = 65;
    slime.set_planned_move_id(SPLIT_MOVE_ID);
    combat.entities.monsters = vec![slime];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Split,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let value = enemy_phase_value(&combat);

    assert_eq!(value.raw_living_enemy_hp, 30);
    assert_eq!(value.raw_living_enemy_block, 0);
    assert_eq!(value.raw_living_enemy_effort, 30);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 60);
    assert_eq!(value.phase_adjusted_living_enemy_effort, 60);
    assert_eq!(value.split_pending_count, 1);
    assert_eq!(value.split_debt_hp, 30);
}

#[test]
fn split_power_above_threshold_keeps_raw_hp_until_split_is_pending() {
    let mut combat = blank_test_combat();
    let mut slime = test_monster(EnemyId::AcidSlimeL);
    slime.id = 8;
    slime.current_hp = 40;
    slime.max_hp = 65;
    combat.entities.monsters = vec![slime];
    combat.entities.power_db.insert(
        8,
        vec![Power {
            power_type: PowerId::Split,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let value = enemy_phase_value(&combat);

    assert_eq!(value.raw_living_enemy_hp, 40);
    assert_eq!(value.raw_living_enemy_block, 0);
    assert_eq!(value.raw_living_enemy_effort, 40);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 40);
    assert_eq!(value.phase_adjusted_living_enemy_effort, 40);
    assert_eq!(value.split_pending_count, 0);
    assert_eq!(value.split_debt_hp, 0);
}

#[test]
fn guardian_defensive_block_counts_as_phase_effort() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 9;
    guardian.current_hp = 180;
    guardian.max_hp = 240;
    guardian.block = 20;
    guardian.guardian.is_open = false;
    combat.entities.monsters = vec![guardian];

    let value = enemy_phase_value(&combat);

    assert_eq!(value.raw_living_enemy_hp, 180);
    assert_eq!(value.raw_living_enemy_block, 20);
    assert_eq!(value.raw_living_enemy_effort, 200);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 180);
    assert_eq!(value.phase_adjusted_living_enemy_effort, 200);
    assert_eq!(value.guardian_defensive_count, 1);
    assert_eq!(value.guardian_defensive_block, 20);
}

#[test]
fn awakened_one_form_one_includes_full_rebirth_phase_debt() {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 10;
    awakened.current_hp = 1;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = true;
    combat.entities.monsters = vec![awakened];

    let value = enemy_phase_value(&combat);

    assert_eq!(value.phase_adjusted_living_enemy_count, 1);
    assert_eq!(value.raw_living_enemy_hp, 1);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 301);
    assert_eq!(value.phase_adjusted_living_enemy_effort, 301);
    assert_eq!(value.split_pending_count, 0);
    assert_eq!(value.split_debt_hp, 0);
    assert_eq!(value.awakened_rebirth_pending_count, 1);
    assert_eq!(value.awakened_rebirth_debt_hp, 300);
}

#[test]
fn awakened_one_rebirth_window_preserves_enemy_count_and_phase_debt() {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 12;
    awakened.current_hp = 0;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = true;
    combat.entities.monsters = vec![awakened];

    let value = enemy_phase_value(&combat);

    assert_eq!(value.phase_adjusted_living_enemy_count, 1);
    assert_eq!(value.raw_living_enemy_hp, 0);
    assert_eq!(value.raw_living_enemy_effort, 0);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 300);
    assert_eq!(value.phase_adjusted_living_enemy_effort, 300);
    assert_eq!(value.awakened_rebirth_pending_count, 1);
    assert_eq!(value.awakened_rebirth_debt_hp, 300);
}

#[test]
fn awakened_one_form_two_has_no_remaining_rebirth_debt() {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 11;
    awakened.current_hp = 300;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = false;
    combat.entities.monsters = vec![awakened];

    let value = enemy_phase_value(&combat);

    assert_eq!(value.phase_adjusted_living_enemy_count, 1);
    assert_eq!(value.phase_adjusted_living_enemy_hp, 300);
    assert_eq!(value.awakened_rebirth_pending_count, 0);
    assert_eq!(value.awakened_rebirth_debt_hp, 0);
}
