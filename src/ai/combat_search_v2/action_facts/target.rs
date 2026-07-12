use crate::content::cards::CardTarget;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatState;
use crate::sim::combat_projection::monster_preview_total_damage_in_combat;
use crate::state::core::ClientInput;

use super::super::timed_enemy_threat::timed_enemy_threat_for_target;
use super::types::{CombatSearchV2ActionTargetFacts, CombatSearchV2TimedEnemyThreatTargetFacts};

pub(super) fn target_facts(
    combat: &CombatState,
    input: &ClientInput,
) -> Option<CombatSearchV2ActionTargetFacts> {
    let ClientInput::PlayCard {
        target: Some(entity_id),
        ..
    } = *input
    else {
        return None;
    };
    let (slot, monster) = combat
        .entities
        .monsters
        .iter()
        .enumerate()
        .find(|(_, monster)| monster.id == entity_id)?;
    Some(CombatSearchV2ActionTargetFacts {
        target_slot: slot,
        entity_id: monster.id,
        enemy_id: EnemyId::from_id(monster.monster_type)
            .map(|id| format!("{id:?}"))
            .unwrap_or_else(|| format!("MonsterType{}", monster.monster_type)),
        hp: monster.current_hp,
        block: monster.block,
        visible_incoming_damage: monster_preview_total_damage_in_combat(combat, monster),
        vulnerable: combat.get_power(monster.id, PowerId::Vulnerable),
        weak: combat.get_power(monster.id, PowerId::Weak),
        strength: combat.get_power(monster.id, PowerId::Strength),
        timed_enemy_threat: timed_enemy_threat_for_target(combat, monster.id).map(|threat| {
            CombatSearchV2TimedEnemyThreatTargetFacts {
                kind: threat.kind.label(),
                owner_turns_until_trigger: threat.owner_turns_until_trigger,
                raw_player_damage: threat.raw_player_damage,
                canceled_by_owner_death: threat.canceled_by_owner_death,
            }
        }),
    })
}

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
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|entity_id| {
                combat
                    .entities
                    .monsters
                    .iter()
                    .find(|monster| monster.id == entity_id)
            })
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
            .unwrap_or_default(),
        CardTarget::AllEnemy => all_enemy_progress_hint(combat, target_kind, damage),
        _ => 0,
    }
}

pub(super) fn all_enemy_progress_hint(
    combat: &CombatState,
    target_kind: CardTarget,
    damage: i32,
) -> i32 {
    if damage <= 0 || target_kind != CardTarget::AllEnemy {
        return 0;
    }
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
        .sum()
}
