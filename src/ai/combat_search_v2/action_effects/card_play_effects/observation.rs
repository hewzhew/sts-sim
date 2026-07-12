use std::collections::{BTreeMap, BTreeSet};

use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

#[derive(Default)]
pub(super) struct CardPlayEffectAccumulator {
    pub(super) direct: DirectCardPlayEffectAccumulator,
    pub(super) reactive: super::super::ReactiveCardPlayEffectFacts,
}

#[derive(Default)]
pub(super) struct DirectCardPlayEffectAccumulator {
    pub(super) enemy_strength_down_by_target: BTreeMap<usize, i32>,
    pub(super) enemy_strength_gain_by_target: BTreeMap<usize, i32>,
    pub(super) shackled_targets: BTreeSet<usize>,
    pub(super) player_strength_gain: i32,
    pub(super) player_lose_strength: i32,
    pub(super) declared_draw_cards: i32,
    pub(super) conditional_draw_cards: i32,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

pub(super) fn observe_card_play_effects(
    combat: &CombatState,
    card: &CombatCard,
    actions: impl IntoIterator<Item = Action>,
) -> CardPlayEffectAccumulator {
    let mut accumulator = CardPlayEffectAccumulator::default();
    for action in actions {
        super::attack_retaliation_observation::observe_attack_retaliation_action(
            combat,
            &mut accumulator,
            &action,
        );
        observe_direct_action(combat, &mut accumulator.direct, action);
    }
    super::reactive_observation::observe_card_play_reactive_actions(combat, card, &mut accumulator);
    accumulator
}

pub(super) fn observe_direct_action(
    combat: &CombatState,
    direct: &mut DirectCardPlayEffectAccumulator,
    action: Action,
) {
    match action {
        Action::ApplyPower {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerDetailed {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerWithPayload {
            target,
            power_id,
            amount,
            ..
        } => observe_apply_power(direct, target, power_id, amount),
        Action::DrawCards(amount) | Action::DrawCardsWithHistory { amount, .. } => {
            direct.declared_draw_cards = direct.declared_draw_cards.saturating_add(amount as i32);
        }
        Action::ExpertiseDraw { target_hand_size } => {
            let hand_after_play = combat.zones.hand.len().saturating_sub(1) as i32;
            direct.conditional_draw_cards = direct
                .conditional_draw_cards
                .saturating_add(target_hand_size.saturating_sub(hand_after_play).max(0));
        }
        Action::InnerPeace { draw_amount } | Action::Sanctity { draw_amount } => {
            direct.conditional_draw_cards = direct
                .conditional_draw_cards
                .saturating_add(draw_amount as i32);
        }
        Action::CalculatedGamble { draw_extra } => {
            let hand_after_play = combat.zones.hand.len().saturating_sub(1) as i32;
            direct.conditional_draw_cards = direct
                .conditional_draw_cards
                .saturating_add(hand_after_play.saturating_add(i32::from(draw_extra)).max(0));
        }
        Action::DrawForUniqueOrbTypes {
            amount_per_orb_type,
        } => {
            direct.conditional_draw_cards = direct
                .conditional_draw_cards
                .saturating_add(amount_per_orb_type as i32);
        }
        _ => {}
    }
}

fn observe_apply_power(
    direct: &mut DirectCardPlayEffectAccumulator,
    target: usize,
    power_id: PowerId,
    amount: i32,
) {
    match power_id {
        PowerId::Strength if target == 0 && amount > 0 => {
            direct.player_strength_gain = direct.player_strength_gain.saturating_add(amount);
        }
        PowerId::LoseStrength if target == 0 && amount > 0 => {
            direct.player_lose_strength = direct.player_lose_strength.saturating_add(amount);
        }
        PowerId::Strength if target != 0 && amount < 0 => {
            *direct
                .enemy_strength_down_by_target
                .entry(target)
                .or_default() += -amount;
        }
        PowerId::Strength if target != 0 && amount > 0 => {
            *direct
                .enemy_strength_gain_by_target
                .entry(target)
                .or_default() += amount;
        }
        PowerId::Shackled if amount > 0 => {
            direct.shackled_targets.insert(target);
        }
        PowerId::Weak if amount > 0 => {
            direct.enemy_weak = direct.enemy_weak.saturating_add(amount);
        }
        PowerId::Vulnerable if amount > 0 => {
            direct.enemy_vulnerable = direct.enemy_vulnerable.saturating_add(amount);
        }
        _ => {}
    }
}
