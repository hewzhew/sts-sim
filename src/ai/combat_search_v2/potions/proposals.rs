use super::context::PotionPlanningContext;
use super::decision::{PotionGateDecision, PotionGateReason};
use super::semantics::{potion_semantics, PotionArea, PotionSemanticKind, PotionUncertainty};
use super::*;

pub(in crate::ai::combat_search_v2) fn semantic_potion_action_allowed(
    combat: &CombatState,
    input: &ClientInput,
) -> bool {
    semantic_potion_gate_decision(combat, input).allowed
}

pub(super) fn semantic_potion_gate_decision(
    combat: &CombatState,
    input: &ClientInput,
) -> PotionGateDecision {
    let ClientInput::UsePotion {
        potion_index,
        target,
    } = input
    else {
        return PotionGateDecision::reject(PotionGateReason::InvalidPotionAction);
    };
    let Some(Some(potion)) = combat.entities.potions.get(*potion_index) else {
        return PotionGateDecision::reject(PotionGateReason::PotionSlotMissing);
    };

    let semantics = potion_semantics(combat, potion.id);
    if matches!(semantics.uncertainty, PotionUncertainty::PassiveOnly) {
        return PotionGateDecision::reject(PotionGateReason::PassiveOnly);
    }

    let context = PotionPlanningContext::from_combat(combat);
    match semantics.kind {
        PotionSemanticKind::DirectDamage { amount, area } => {
            direct_damage_gate(combat, context, *target, amount, area)
        }
        PotionSemanticKind::EnemyPower => enemy_power_gate(combat, context, *target),
        PotionSemanticKind::PlayerBlock => block_gate(context),
        PotionSemanticKind::PlayerHeal => wounded_resource_gate(context),
        PotionSemanticKind::TemporaryPlayerPower => pressure_gate(context),
        PotionSemanticKind::PlayerMaxHp => max_hp_gate(context),
        PotionSemanticKind::PlayerEnergy
        | PotionSemanticKind::PlayerDraw
        | PotionSemanticKind::CardDiscovery
        | PotionSemanticKind::CardGeneration
        | PotionSemanticKind::HandOrPileSelection
        | PotionSemanticKind::PlayTopCards
        | PotionSemanticKind::UpgradeHand
        | PotionSemanticKind::DuplicateNextCard
        | PotionSemanticKind::Stance => {
            if !context.has_living_enemy() {
                PotionGateDecision::reject(PotionGateReason::NoLivingEnemy)
            } else if context.lacks_visible_lethal() {
                PotionGateDecision::allow(PotionGateReason::NoVisibleHandLethal)
            } else {
                PotionGateDecision::reject(PotionGateReason::NoTacticalPressure)
            }
        }
        PotionSemanticKind::PlayerPower | PotionSemanticKind::Orb => pressure_gate(context),
        PotionSemanticKind::Escape => PotionGateDecision::reject(PotionGateReason::EscapeNotWin),
        PotionSemanticKind::RandomPotionGeneration => {
            PotionGateDecision::reject(PotionGateReason::RandomPotionGenerationUnsupported)
        }
        PotionSemanticKind::PassiveDeathPrevention => {
            PotionGateDecision::reject(PotionGateReason::PassiveOnly)
        }
    }
}

fn direct_damage_gate(
    combat: &CombatState,
    context: PotionPlanningContext,
    target: Option<usize>,
    amount: i32,
    area: PotionArea,
) -> PotionGateDecision {
    match area {
        PotionArea::SingleEnemy => {
            let Some(target) = target else {
                return PotionGateDecision::reject(PotionGateReason::InvalidTarget);
            };
            let Some(monster) = combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == target && monster.is_alive_for_action())
            else {
                return PotionGateDecision::reject(PotionGateReason::InvalidTarget);
            };
            if amount >= monster.current_hp + monster.block {
                PotionGateDecision::allow(PotionGateReason::DirectDamageCanKill)
            } else {
                pressure_gate(context)
            }
        }
        PotionArea::AllEnemies => {
            let can_kill = combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .any(|monster| amount >= monster.current_hp + monster.block);
            if can_kill {
                PotionGateDecision::allow(PotionGateReason::DirectDamageCanKill)
            } else {
                pressure_gate(context)
            }
        }
    }
}

fn enemy_power_gate(
    combat: &CombatState,
    context: PotionPlanningContext,
    target: Option<usize>,
) -> PotionGateDecision {
    if !target_points_to_live_enemy(combat, target) {
        return PotionGateDecision::reject(PotionGateReason::InvalidTarget);
    }
    pressure_gate(context)
}

fn block_gate(context: PotionPlanningContext) -> PotionGateDecision {
    if context.visible_hp_loss {
        PotionGateDecision::allow(PotionGateReason::VisibleIncomingHpLoss)
    } else {
        PotionGateDecision::reject(PotionGateReason::NoVisibleIncomingHpLoss)
    }
}

fn wounded_resource_gate(context: PotionPlanningContext) -> PotionGateDecision {
    if !context.player_is_wounded() {
        return PotionGateDecision::reject(PotionGateReason::NotWounded);
    }
    if context.visible_hp_loss {
        PotionGateDecision::allow(PotionGateReason::VisibleIncomingHpLoss)
    } else if context.lacks_visible_lethal() {
        PotionGateDecision::allow(PotionGateReason::NoVisibleHandLethal)
    } else {
        PotionGateDecision::reject(PotionGateReason::NoTacticalPressure)
    }
}

fn max_hp_gate(context: PotionPlanningContext) -> PotionGateDecision {
    if context.player_is_wounded() {
        PotionGateDecision::allow(PotionGateReason::PlayerWounded)
    } else if context.visible_hp_loss {
        PotionGateDecision::allow(PotionGateReason::VisibleIncomingHpLoss)
    } else {
        PotionGateDecision::reject(PotionGateReason::NoTacticalPressure)
    }
}

fn pressure_gate(context: PotionPlanningContext) -> PotionGateDecision {
    if !context.has_living_enemy() {
        PotionGateDecision::reject(PotionGateReason::NoLivingEnemy)
    } else if context.visible_hp_loss {
        PotionGateDecision::allow(PotionGateReason::VisibleIncomingHpLoss)
    } else if context.lacks_visible_lethal() {
        PotionGateDecision::allow(PotionGateReason::NoVisibleHandLethal)
    } else {
        PotionGateDecision::reject(PotionGateReason::NoTacticalPressure)
    }
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
