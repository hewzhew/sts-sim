use crate::content::cards::{self, CardTarget};
use crate::content::powers::PowerId;
use crate::sim::combat::CombatStepResult;
#[cfg(test)]
use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::action_effects::summarize_play_card_effects;
use super::*;

mod payload;
mod types;
use payload::resolved_card_action_payload_facts;
pub use types::{
    CombatSearchV2ActionCardFacts, CombatSearchV2ActionExactDeltaFacts, CombatSearchV2ActionFacts,
    CombatSearchV2ActionImmediateFacts, CombatSearchV2ActionMechanicsFacts,
    CombatSearchV2ActionTargetFacts,
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
        exhaust: card.exhaust_override.unwrap_or(def.exhaust),
        ethereal: def.ethereal,
        innate: def.innate,
    })
}

fn target_facts(
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
    let effects = summarize_play_card_effects(combat, runtime_card, target);
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
                .saturating_add(effects.reactive_player_block),
            target_progress_hint: target_progress,
            all_enemy_progress_hint: all_enemy_progress,
            exhausts_card: card_facts.exhaust,
            creates_pending_choice_after_one_step: false,
        },
        CombatSearchV2ActionMechanicsFacts {
            persistent_enemy_strength_down: effects.persistent_enemy_strength_down,
            temporary_enemy_strength_down: effects.temporary_enemy_strength_down,
            visible_attack_mitigation_hint: effects.visible_attack_mitigation_hint,
            enemy_weak: effects.enemy_weak,
            enemy_vulnerable: effects.enemy_vulnerable,
            enemy_strength_gain: effects.enemy_strength_gain,
            visible_attack_pressure_hint: effects.visible_attack_pressure_hint,
            reactive_player_hp_loss: effects.reactive_player_hp_loss,
            reactive_player_block: effects.reactive_player_block,
            reactive_enemy_damage: effects.reactive_enemy_damage,
            reactive_bad_draw_cards: effects.reactive_bad_draw_cards,
            reactive_forced_turn_end: effects.reactive_forced_turn_end,
        },
    )
}

fn exact_delta_facts_from_step(
    combat: &CombatState,
    step: &CombatStepResult,
) -> CombatSearchV2ActionExactDeltaFacts {
    let before_enemy_hp = total_monster_hp(combat);
    let before_enemy_block = total_monster_block(combat);
    let after = &step.position.combat;
    let phase = combat_search_phase_profile(&step.position.engine, after);
    CombatSearchV2ActionExactDeltaFacts {
        status: step_status(&step),
        terminal: terminal_label(&step.position.engine, after),
        engine_steps: step.engine_steps,
        player_hp_delta: after.entities.player.current_hp - combat.entities.player.current_hp,
        player_block_delta: after.entities.player.block - combat.entities.player.block,
        energy_delta: i32::from(after.turn.energy) - i32::from(combat.turn.energy),
        hand_delta: len_delta(after.zones.hand.len(), combat.zones.hand.len()),
        draw_delta: len_delta(after.zones.draw_pile.len(), combat.zones.draw_pile.len()),
        discard_delta: len_delta(
            after.zones.discard_pile.len(),
            combat.zones.discard_pile.len(),
        ),
        exhaust_delta: len_delta(
            after.zones.exhaust_pile.len(),
            combat.zones.exhaust_pile.len(),
        ),
        limbo_delta: len_delta(after.zones.limbo.len(), combat.zones.limbo.len()),
        queued_cards_delta: len_delta(
            after.zones.queued_cards.len(),
            combat.zones.queued_cards.len(),
        ),
        total_enemy_hp_delta: total_monster_hp(after) - before_enemy_hp,
        total_enemy_block_delta: total_monster_block(after) - before_enemy_block,
        pending_choice_present: phase.pending_choice.present,
        pending_choice_estimated_action_fanout: phase.pending_choice.estimated_action_fanout,
    }
}

fn target_progress_hint(
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

fn all_enemy_progress_hint(combat: &CombatState, target_kind: CardTarget, damage: i32) -> i32 {
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

fn total_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn total_monster_block(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.block.max(0))
        .sum()
}

fn len_delta(after: usize, before: usize) -> i32 {
    after as i32 - before as i32
}

fn action_kind(input: &ClientInput) -> &'static str {
    match input {
        ClientInput::PlayCard { .. } => "play_card",
        ClientInput::UsePotion { .. } => "use_potion",
        ClientInput::DiscardPotion(_) => "discard_potion",
        ClientInput::EndTurn => "end_turn",
        ClientInput::SubmitCardChoice(_)
        | ClientInput::SubmitDiscoverChoice(_)
        | ClientInput::SubmitScryDiscard(_)
        | ClientInput::SubmitSelection(_)
        | ClientInput::SubmitHandSelect(_)
        | ClientInput::SubmitGridSelect(_)
        | ClientInput::SubmitDeckSelect(_)
        | ClientInput::SubmitRelicChoice(_) => "pending_choice",
        _ => "other",
    }
}

fn step_status(step: &crate::sim::combat::CombatStepResult) -> &'static str {
    if step.timed_out {
        "timed_out"
    } else if step.truncated {
        "engine_step_limit"
    } else if !step.alive {
        "player_dead"
    } else {
        "stable"
    }
}

#[cfg(test)]
mod tests;
