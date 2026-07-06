use crate::content::cards::CardTarget;
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::runtime::combat::CombatState;

pub(super) fn target_progress_hint(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> i32 {
    if damage <= 0 {
        return 0;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
            .sum(),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .map(|hp| damage.min(hp).max(0))
            .unwrap_or_default(),
        _ => 0,
    }
}

pub(super) fn target_progress_kills(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> bool {
    if damage <= 0 {
        return false;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .any(|monster| damage >= monster.current_hp + monster.block),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .is_some_and(|hp| damage >= hp),
        _ => false,
    }
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

pub(super) fn target_enemy_id(combat: &CombatState, target: Option<usize>) -> Option<EnemyId> {
    target
        .and_then(|entity_id| {
            combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        })
        .and_then(|monster| EnemyId::from_id(monster.monster_type))
}

pub(super) fn target_has_stasis_card(combat: &CombatState, target: Option<usize>) -> bool {
    target.is_some_and(|entity_id| store::has_power(combat, entity_id, PowerId::Stasis))
}
