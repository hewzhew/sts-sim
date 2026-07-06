use super::super::super::action_effects::CardPlayEffectFacts;
use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CombatEvent, InstalledRule, Mechanic, PlayEffect,
    TriggeredEffect,
};
use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::{CombatCard, CombatState};

pub(super) fn key_setup_card_online_candidate(card: CardId, upgrades: u8) -> bool {
    let definition = card_definition_with_upgrades(card, upgrades);
    definition.play_effects.iter().any(|effect| {
        matches!(
            effect,
            PlayEffect::Provide(
                Mechanic::Strength | Mechanic::TemporaryStrength | Mechanic::StrengthMultiplier
            )
        )
    }) || definition.event_handlers.iter().any(|handler| {
        matches!(
            handler.effect,
            TriggeredEffect::Provide(
                Mechanic::Strength | Mechanic::TemporaryStrength | Mechanic::StrengthMultiplier
            )
        )
    }) || definition
        .installed_rules
        .contains(&InstalledRule::SkillCardsCostZeroAndExhaust)
        || definition.event_handlers.iter().any(|handler| {
            handler.on == CombatEvent::CardExhausted
                && matches!(
                    handler.effect,
                    TriggeredEffect::Provide(Mechanic::Block | Mechanic::CardDraw)
                )
        })
}

pub(super) fn current_turn_attack_setup_score(
    combat: &CombatState,
    card_index: usize,
    card: &CombatCard,
    effects: CardPlayEffectFacts,
) -> i32 {
    if effects.direct.player_strength_gain <= 0 {
        return 0;
    }

    let setup_cost = card.cost_for_turn_java().max(0);
    let available_energy = i32::from(combat.turn.energy);
    if setup_cost > available_energy {
        return 0;
    }
    let remaining_energy = available_energy - setup_cost;
    let playable_attacks = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            *index != card_index
                && cards::get_card_definition(candidate.id).card_type == CardType::Attack
                && cards::can_play_card(candidate, combat).is_ok()
                && attack_cost_is_payable_after_setup(candidate, remaining_energy)
        })
        .count() as i32;

    effects
        .direct
        .player_strength_gain
        .saturating_mul(playable_attacks)
}

fn attack_cost_is_payable_after_setup(card: &CombatCard, remaining_energy: i32) -> bool {
    let cost = card.cost_for_turn_java();
    if cost < 0 {
        return remaining_energy > 0;
    }
    cost <= remaining_energy
}
