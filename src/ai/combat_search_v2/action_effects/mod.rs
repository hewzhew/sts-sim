use super::*;
use crate::runtime::combat::CombatCard;

mod card_play_effects;
mod types;

pub(in crate::ai::combat_search_v2) use types::*;

pub(super) fn card_play_effect_facts(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> CardPlayEffectFacts {
    card_play_effects::card_play_effect_facts(combat, card, target)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    card_play_effects::state_sustained_mitigation_score(combat)
}

#[cfg(test)]
mod tests;
