use super::*;
use crate::content::powers::{store, PowerId};

// Java large slimes and Slime Boss use move byte 3 for the Split move.
const SPLIT_MOVE_ID: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EnemyPhaseValueV1 {
    pub(super) phase_adjusted_living_enemy_count: usize,
    pub(super) raw_living_enemy_hp: i32,
    pub(super) raw_living_enemy_block: i32,
    pub(super) raw_living_enemy_effort: i32,
    pub(super) phase_adjusted_living_enemy_hp: i32,
    pub(super) phase_adjusted_living_enemy_effort: i32,
    pub(super) split_pending_count: usize,
    pub(super) split_debt_hp: i32,
    pub(super) awakened_rebirth_pending_count: usize,
    pub(super) awakened_rebirth_debt_hp: i32,
    pub(super) guardian_defensive_count: usize,
    pub(super) guardian_defensive_block: i32,
}

pub(super) fn enemy_phase_value(combat: &CombatState) -> EnemyPhaseValueV1 {
    combat
        .entities
        .monsters
        .iter()
        .fold(EnemyPhaseValueV1::default(), |mut value, monster| {
            let alive_for_action = monster.is_alive_for_action();
            let awakened_rebirth_debt_hp = awakened_rebirth_debt_hp(monster);
            if !alive_for_action && awakened_rebirth_debt_hp == 0 {
                return value;
            }
            value.phase_adjusted_living_enemy_count =
                value.phase_adjusted_living_enemy_count.saturating_add(1);
            let raw_hp = if alive_for_action {
                monster.current_hp.max(0)
            } else {
                0
            };
            let raw_block = if alive_for_action {
                monster.block.max(0)
            } else {
                0
            };
            let split_debt_hp = if is_split_pending_or_triggered(combat, monster) {
                raw_hp
            } else {
                0
            };
            let adjusted_hp = raw_hp
                .saturating_add(split_debt_hp)
                .saturating_add(awakened_rebirth_debt_hp);
            let adjusted_effort = adjusted_hp.saturating_add(raw_block);
            value.raw_living_enemy_hp += raw_hp;
            value.raw_living_enemy_block += raw_block;
            value.raw_living_enemy_effort += raw_hp.saturating_add(raw_block);
            value.phase_adjusted_living_enemy_hp += adjusted_hp;
            value.phase_adjusted_living_enemy_effort += adjusted_effort;
            if split_debt_hp > 0 {
                value.split_pending_count += 1;
                value.split_debt_hp += split_debt_hp;
            }
            if awakened_rebirth_debt_hp > 0 {
                value.awakened_rebirth_pending_count += 1;
                value.awakened_rebirth_debt_hp += awakened_rebirth_debt_hp;
            }
            if is_guardian_defensive(monster) {
                value.guardian_defensive_count += 1;
                value.guardian_defensive_block += raw_block;
            }
            value
        })
}

fn awakened_rebirth_debt_hp(monster: &MonsterEntity) -> i32 {
    if EnemyId::from_id(monster.monster_type) == Some(EnemyId::AwakenedOne)
        && monster.awakened_one.form1
    {
        monster.max_hp.max(0)
    } else {
        0
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
mod tests;
