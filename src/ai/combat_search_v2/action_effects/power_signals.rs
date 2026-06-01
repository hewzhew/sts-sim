use super::*;
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatCard;

mod monster_signals;
mod observation;
mod reactive_observation;
use monster_signals::{
    is_living_monster_id, monster_attack_relevance, visible_strength_down_mitigation_hint,
    visible_strength_gain_pressure_hint,
};
use observation::{observe_card_power_effects, RawPowerEffects};

pub(super) fn summarize_play_card_power_effects(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> PlayCardEffectSummary {
    let actions = crate::content::cards::resolve_card_play_with_context(
        card.id,
        combat,
        card,
        target,
        crate::content::cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let raw = observe_card_power_effects(combat, card, actions.into_iter().map(|info| info.action));
    summarize_power_effects(combat, raw)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            let strength = combat.get_power(monster.id, PowerId::Strength);
            if strength >= 0 {
                return 0;
            }
            (-strength).saturating_mul(monster_attack_relevance(combat, monster.id))
        })
        .sum()
}

fn summarize_power_effects(combat: &CombatState, raw: RawPowerEffects) -> PlayCardEffectSummary {
    let mut summary = PlayCardEffectSummary {
        reactive_player_hp_loss: raw.reactive_player_hp_loss,
        reactive_player_block: raw.reactive_player_block,
        reactive_enemy_damage: raw.reactive_enemy_damage,
        reactive_bad_draw_cards: raw.reactive_bad_draw_cards,
        reactive_forced_turn_end: raw.reactive_forced_turn_end,
        enemy_weak: raw.enemy_weak,
        enemy_vulnerable: raw.enemy_vulnerable,
        ..PlayCardEffectSummary::default()
    };

    for (target, amount) in raw.enemy_strength_down_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        if raw.shackled_targets.contains(&target) {
            summary.temporary_enemy_strength_down = summary
                .temporary_enemy_strength_down
                .saturating_add(weighted_amount);
        } else {
            summary.persistent_enemy_strength_down = summary
                .persistent_enemy_strength_down
                .saturating_add(weighted_amount);
        }
        summary.visible_attack_mitigation_hint =
            summary.visible_attack_mitigation_hint.saturating_add(
                visible_strength_down_mitigation_hint(combat, target, amount),
            );
    }
    for (target, amount) in raw.enemy_strength_gain_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        summary.enemy_strength_gain = summary.enemy_strength_gain.saturating_add(weighted_amount);
        summary.visible_attack_pressure_hint = summary
            .visible_attack_pressure_hint
            .saturating_add(visible_strength_gain_pressure_hint(combat, target, amount));
    }

    summary
}
