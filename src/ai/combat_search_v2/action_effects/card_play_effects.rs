use super::*;
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatCard;

mod attack_retaliation_observation;
mod monster_signals;
mod observation;
mod reactive_observation;
use monster_signals::{
    is_living_monster_id, monster_attack_relevance, visible_strength_down_mitigation_hint,
    visible_strength_gain_pressure_hint,
};
use observation::{observe_card_play_effects, CardPlayEffectAccumulator};

pub(super) fn card_play_effect_facts(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> CardPlayEffectFacts {
    let actions = crate::content::cards::resolve_card_play_with_context(
        card.id,
        combat,
        card,
        target,
        crate::content::cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let accumulator =
        observe_card_play_effects(combat, card, actions.into_iter().map(|info| info.action));
    card_play_facts_from_accumulator(combat, accumulator)
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

fn card_play_facts_from_accumulator(
    combat: &CombatState,
    accumulator: CardPlayEffectAccumulator,
) -> CardPlayEffectFacts {
    let mut facts = CardPlayEffectFacts {
        direct: DirectCardPlayEffectFacts {
            declared_draw_cards: accumulator.direct.declared_draw_cards,
            conditional_draw_cards: accumulator.direct.conditional_draw_cards,
            enemy_weak: accumulator.direct.enemy_weak,
            enemy_vulnerable: accumulator.direct.enemy_vulnerable,
            player_strength_gain: accumulator.direct.player_strength_gain,
            player_temporary_strength_gain: accumulator
                .direct
                .player_strength_gain
                .min(accumulator.direct.player_lose_strength),
            ..DirectCardPlayEffectFacts::default()
        },
        reactive: accumulator.reactive,
    };

    for (target, amount) in accumulator.direct.enemy_strength_down_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        if accumulator.direct.shackled_targets.contains(&target) {
            facts.direct.temporary_enemy_strength_down = facts
                .direct
                .temporary_enemy_strength_down
                .saturating_add(weighted_amount);
        } else {
            facts.direct.persistent_enemy_strength_down = facts
                .direct
                .persistent_enemy_strength_down
                .saturating_add(weighted_amount);
        }
        facts.direct.visible_attack_mitigation_hint =
            facts.direct.visible_attack_mitigation_hint.saturating_add(
                visible_strength_down_mitigation_hint(combat, target, amount),
            );
    }
    for (target, amount) in accumulator.direct.enemy_strength_gain_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        facts.direct.enemy_strength_gain = facts
            .direct
            .enemy_strength_gain
            .saturating_add(weighted_amount);
        facts.direct.visible_attack_pressure_hint = facts
            .direct
            .visible_attack_pressure_hint
            .saturating_add(visible_strength_gain_pressure_hint(combat, target, amount));
    }

    facts
}
