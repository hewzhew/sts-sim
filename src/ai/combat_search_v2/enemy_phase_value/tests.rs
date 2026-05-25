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
