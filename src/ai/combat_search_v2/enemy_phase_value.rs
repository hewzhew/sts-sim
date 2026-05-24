use super::*;
use crate::content::powers::{store, PowerId};

// Java large slimes and Slime Boss use move byte 3 for the Split move.
const SPLIT_MOVE_ID: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EnemyPhaseValueV1 {
    pub(super) raw_living_enemy_hp: i32,
    pub(super) raw_living_enemy_block: i32,
    pub(super) raw_living_enemy_effort: i32,
    pub(super) phase_adjusted_living_enemy_hp: i32,
    pub(super) phase_adjusted_living_enemy_effort: i32,
    pub(super) split_pending_count: usize,
    pub(super) split_debt_hp: i32,
    pub(super) guardian_defensive_count: usize,
    pub(super) guardian_defensive_block: i32,
}

pub(super) fn enemy_phase_value(combat: &CombatState) -> EnemyPhaseValueV1 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .fold(EnemyPhaseValueV1::default(), |mut value, monster| {
            let raw_hp = monster.current_hp.max(0);
            let raw_block = monster.block.max(0);
            let adjusted_hp = phase_adjusted_enemy_hp(combat, monster);
            let adjusted_effort = adjusted_hp.saturating_add(raw_block);
            value.raw_living_enemy_hp += raw_hp;
            value.raw_living_enemy_block += raw_block;
            value.raw_living_enemy_effort += raw_hp.saturating_add(raw_block);
            value.phase_adjusted_living_enemy_hp += adjusted_hp;
            value.phase_adjusted_living_enemy_effort += adjusted_effort;
            if adjusted_hp > raw_hp {
                value.split_pending_count += 1;
                value.split_debt_hp += adjusted_hp - raw_hp;
            }
            if is_guardian_defensive(monster) {
                value.guardian_defensive_count += 1;
                value.guardian_defensive_block += raw_block;
            }
            value
        })
}

fn phase_adjusted_enemy_hp(combat: &CombatState, monster: &MonsterEntity) -> i32 {
    let raw_hp = monster.current_hp.max(0);
    if is_split_pending_or_triggered(combat, monster) {
        raw_hp.saturating_mul(2)
    } else {
        raw_hp
    }
}

fn is_split_pending_or_triggered(combat: &CombatState, monster: &MonsterEntity) -> bool {
    has_split_power(combat, monster)
        && (monster.planned_move_id() == SPLIT_MOVE_ID
            || monster.current_hp <= monster.max_hp.saturating_div(2))
}

fn has_split_power(combat: &CombatState, monster: &MonsterEntity) -> bool {
    store::has_power(combat, monster.id, PowerId::Split)
}

fn is_guardian_defensive(monster: &MonsterEntity) -> bool {
    EnemyId::from_id(monster.monster_type) == Some(EnemyId::TheGuardian)
        && !monster.guardian.is_open
}

#[cfg(test)]
mod tests {
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
}
