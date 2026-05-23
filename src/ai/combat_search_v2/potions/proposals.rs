use super::semantics::{potion_semantics, PotionArea, PotionSemanticKind, PotionUncertainty};
use super::*;

pub(in crate::ai::combat_search_v2) fn semantic_potion_action_allowed(
    combat: &CombatState,
    input: &ClientInput,
) -> bool {
    let ClientInput::UsePotion {
        potion_index,
        target,
    } = input
    else {
        return false;
    };
    let Some(Some(potion)) = combat.entities.potions.get(*potion_index) else {
        return false;
    };

    let semantics = potion_semantics(combat, potion.id);
    if matches!(semantics.uncertainty, PotionUncertainty::PassiveOnly) {
        return false;
    }

    match semantics.kind {
        PotionSemanticKind::DirectDamage { amount, area } => {
            direct_damage_is_tactically_relevant(combat, *target, amount, area)
        }
        PotionSemanticKind::EnemyPower => {
            target_points_to_live_enemy(combat, *target)
                && (incoming_hp_loss(combat) || current_hand_lacks_visible_lethal(combat))
        }
        PotionSemanticKind::PlayerBlock => incoming_hp_loss(combat),
        PotionSemanticKind::PlayerHeal => {
            combat.entities.player.current_hp < combat.entities.player.max_hp
                && (incoming_hp_loss(combat) || current_hand_lacks_visible_lethal(combat))
        }
        PotionSemanticKind::TemporaryPlayerPower => {
            incoming_hp_loss(combat) || current_hand_lacks_visible_lethal(combat)
        }
        PotionSemanticKind::PlayerMaxHp => {
            combat.entities.player.current_hp < combat.entities.player.max_hp
                || incoming_hp_loss(combat)
        }
        PotionSemanticKind::PlayerEnergy
        | PotionSemanticKind::PlayerDraw
        | PotionSemanticKind::CardDiscovery
        | PotionSemanticKind::CardGeneration
        | PotionSemanticKind::HandOrPileSelection
        | PotionSemanticKind::PlayTopCards
        | PotionSemanticKind::UpgradeHand
        | PotionSemanticKind::DuplicateNextCard
        | PotionSemanticKind::Stance => {
            has_living_enemy(combat) && current_hand_lacks_visible_lethal(combat)
        }
        PotionSemanticKind::PlayerPower | PotionSemanticKind::Orb => {
            has_living_enemy(combat)
                && (incoming_hp_loss(combat) || current_hand_lacks_visible_lethal(combat))
        }
        PotionSemanticKind::Escape
        | PotionSemanticKind::RandomPotionGeneration
        | PotionSemanticKind::PassiveDeathPrevention => false,
    }
}

fn direct_damage_is_tactically_relevant(
    combat: &CombatState,
    target: Option<usize>,
    amount: i32,
    area: PotionArea,
) -> bool {
    match area {
        PotionArea::SingleEnemy => {
            let Some(target) = target else {
                return false;
            };
            let Some(monster) = combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == target && monster.is_alive_for_action())
            else {
                return false;
            };
            amount >= monster.current_hp + monster.block
                || incoming_hp_loss(combat)
                || current_hand_lacks_visible_lethal(combat)
        }
        PotionArea::AllEnemies => {
            combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .any(|monster| amount >= monster.current_hp + monster.block)
                || incoming_hp_loss(combat)
                || current_hand_lacks_visible_lethal(combat)
        }
    }
}

fn incoming_hp_loss(combat: &CombatState) -> bool {
    visible_incoming_damage(combat) > combat.entities.player.block
}

fn has_living_enemy(combat: &CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .any(MonsterEntity::is_alive_for_action)
}

fn target_points_to_live_enemy(combat: &CombatState, target: Option<usize>) -> bool {
    let Some(target) = target else {
        return false;
    };
    combat
        .entities
        .monsters
        .iter()
        .any(|monster| monster.id == target && monster.is_alive_for_action())
}

fn current_hand_lacks_visible_lethal(combat: &CombatState) -> bool {
    playable_hand_damage(combat) < total_living_enemy_hp_with_block(combat)
}

fn total_living_enemy_hp_with_block(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn playable_hand_damage(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            card.base_damage_override
                .unwrap_or(def.base_damage + def.upgrade_damage * card.upgrades as i32)
                .max(0)
        })
        .sum()
}
