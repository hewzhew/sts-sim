use crate::content::cards::{self, CardTarget};
#[cfg(test)]
use crate::content::powers::PowerId;
use crate::sim::combat::CombatStepResult;
#[cfg(test)]
use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::action_effects::card_play_effect_facts;
use super::*;

mod delta;
mod payload;
mod target;
mod types;
use delta::{action_kind, exact_delta_facts_from_step};
use payload::resolved_card_action_payload_facts;
use target::{all_enemy_progress_hint, target_facts, target_progress_hint};
pub use types::{
    CombatSearchV2ActionAccessMechanicsFacts, CombatSearchV2ActionCardFacts,
    CombatSearchV2ActionDerivedMechanicsFacts, CombatSearchV2ActionDirectMechanicsFacts,
    CombatSearchV2ActionExactDeltaFacts, CombatSearchV2ActionFacts,
    CombatSearchV2ActionImmediateFacts, CombatSearchV2ActionMechanicsFacts,
    CombatSearchV2ActionReactiveMechanicsFacts, CombatSearchV2ActionTargetFacts,
};

#[cfg(test)]
pub(super) fn summarize_action_facts(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    stepper: &impl CombatStepper,
    max_engine_steps: usize,
) -> CombatSearchV2ActionFacts {
    let step = stepper.apply_to_stable(
        &CombatPosition::new(engine.clone(), combat.clone()),
        input.clone(),
        CombatStepLimits {
            max_engine_steps,
            deadline: None,
        },
    );
    summarize_action_facts_from_step(combat, input, &step)
}

pub(super) fn summarize_action_facts_from_step(
    combat: &CombatState,
    input: &ClientInput,
    step: &CombatStepResult,
) -> CombatSearchV2ActionFacts {
    let card = card_facts(combat, input);
    let target = target_facts(combat, input);
    let (immediate, mechanics) = immediate_and_mechanics_facts(combat, input, card.as_ref());
    let exact_one_step_delta = exact_delta_facts_from_step(combat, step);

    CombatSearchV2ActionFacts {
        action_kind: action_kind(input),
        card,
        target,
        immediate: CombatSearchV2ActionImmediateFacts {
            creates_pending_choice_after_one_step: exact_one_step_delta.pending_choice_present,
            ..immediate
        },
        mechanics,
        exact_one_step_delta,
    }
}

fn card_facts(combat: &CombatState, input: &ClientInput) -> Option<CombatSearchV2ActionCardFacts> {
    let ClientInput::PlayCard { card_index, target } = *input else {
        return None;
    };
    let card = combat.zones.hand.get(card_index)?;
    let def = cards::get_card_definition(card.id);
    let evaluated = cards::evaluate_card_for_play(card, combat, target);

    Some(CombatSearchV2ActionCardFacts {
        hand_index: card_index,
        uuid: card.uuid,
        card_id: format!("{:?}", card.id),
        name: def.name,
        upgraded: card.upgrades > 0,
        card_type: def.card_type,
        definition_target: def.target,
        effective_target: cards::effective_target(card),
        cost_for_turn: card.cost_for_turn_java(),
        base_cost: def.cost,
        evaluated_damage: evaluated.base_damage_mut.max(0),
        evaluated_block: evaluated.base_block_mut.max(0),
        evaluated_magic: evaluated.base_magic_num_mut.max(0),
        exhaust: card
            .exhaust_override
            .unwrap_or_else(|| cards::exhausts_when_played(card)),
        ethereal: cards::is_ethereal(card),
        innate: def.innate,
    })
}

fn immediate_and_mechanics_facts(
    combat: &CombatState,
    input: &ClientInput,
    card: Option<&CombatSearchV2ActionCardFacts>,
) -> (
    CombatSearchV2ActionImmediateFacts,
    CombatSearchV2ActionMechanicsFacts,
) {
    let Some(card_facts) = card else {
        return (
            CombatSearchV2ActionImmediateFacts::default(),
            CombatSearchV2ActionMechanicsFacts::default(),
        );
    };
    let ClientInput::PlayCard { card_index, target } = *input else {
        return (
            CombatSearchV2ActionImmediateFacts::default(),
            CombatSearchV2ActionMechanicsFacts::default(),
        );
    };
    let Some(runtime_card) = combat.zones.hand.get(card_index) else {
        return (
            CombatSearchV2ActionImmediateFacts::default(),
            CombatSearchV2ActionMechanicsFacts::default(),
        );
    };
    let payload = resolved_card_action_payload_facts(combat, runtime_card, target);
    let effects = card_play_effect_facts(combat, runtime_card, target);
    let target_progress_damage = if card_facts.effective_target == CardTarget::AllEnemy {
        card_facts.evaluated_damage
    } else {
        payload.damage_total_hint.max(card_facts.evaluated_damage)
    };
    let target_progress = target_progress_hint(
        combat,
        card_facts.effective_target,
        target,
        target_progress_damage,
    );
    let all_enemy_progress = all_enemy_progress_hint(
        combat,
        card_facts.effective_target,
        card_facts.evaluated_damage,
    );

    (
        CombatSearchV2ActionImmediateFacts {
            damage_hint: card_facts.evaluated_damage,
            action_payload_damage_hint: payload.damage_total_hint,
            action_payload_damage_hit_count_hint: payload.damage_hit_count_hint,
            block_hint: card_facts
                .evaluated_block
                .max(payload.player_block_hint)
                .saturating_add(effects.reactive.player_block),
            target_progress_hint: target_progress,
            all_enemy_progress_hint: all_enemy_progress,
            exhausts_card: card_facts.exhaust,
            creates_pending_choice_after_one_step: false,
        },
        CombatSearchV2ActionMechanicsFacts {
            direct: CombatSearchV2ActionDirectMechanicsFacts {
                persistent_enemy_strength_down: effects.direct.persistent_enemy_strength_down,
                temporary_enemy_strength_down: effects.direct.temporary_enemy_strength_down,
                visible_attack_mitigation_hint: effects.direct.visible_attack_mitigation_hint,
                enemy_weak: effects.direct.enemy_weak,
                enemy_vulnerable: effects.direct.enemy_vulnerable,
                enemy_strength_gain: effects.direct.enemy_strength_gain,
                visible_attack_pressure_hint: effects.direct.visible_attack_pressure_hint,
                player_strength_gain: effects.direct.player_strength_gain,
                player_temporary_strength_gain: effects.direct.player_temporary_strength_gain,
            },
            reactive: CombatSearchV2ActionReactiveMechanicsFacts {
                player_hp_loss: effects.reactive.player_hp_loss,
                player_block: effects.reactive.player_block,
                enemy_damage: effects.reactive.enemy_damage,
                bad_draw_cards: effects.reactive.bad_draw_cards,
                forced_turn_end: effects.reactive.forced_turn_end,
                enemy_strength_gain: effects.reactive.enemy_strength_gain,
                visible_attack_pressure_hint: effects.reactive.visible_attack_pressure_hint,
                enemy_weak: effects.reactive.enemy_weak,
                enemy_vulnerable: effects.reactive.enemy_vulnerable,
            },
            access: CombatSearchV2ActionAccessMechanicsFacts {
                declared_draw_cards: effects.direct.declared_draw_cards,
                conditional_draw_cards: effects.direct.conditional_draw_cards,
                total_draw_cards: effects.total_draw_cards(),
            },
            derived: CombatSearchV2ActionDerivedMechanicsFacts {
                mitigation_score: effects.mitigation_ordering_score(),
                enemy_scaling_risk_score: effects.enemy_scaling_risk_score(),
                reactive_risk_score: effects.reactive_risk_score(),
                net_mitigation_score: effects.net_mitigation_ordering_score(),
                enemy_weak: effects
                    .direct
                    .enemy_weak
                    .saturating_add(effects.reactive.enemy_weak),
                enemy_vulnerable: effects
                    .direct
                    .enemy_vulnerable
                    .saturating_add(effects.reactive.enemy_vulnerable),
                enemy_strength_gain: effects
                    .direct
                    .enemy_strength_gain
                    .saturating_add(effects.reactive.enemy_strength_gain),
                visible_attack_pressure_hint: effects
                    .direct
                    .visible_attack_pressure_hint
                    .saturating_add(effects.reactive.visible_attack_pressure_hint),
            },
        },
    )
}

#[cfg(test)]
mod tests;
