use crate::ai::card_semantics_v1::{card_mechanics_profile_v1, CombatExternalPayoffV1};
use crate::runtime::combat::{CombatCard, CombatState};

pub(crate) fn has_external_payoff_opportunity(combat: &CombatState) -> bool {
    combat
        .meta
        .master_deck_snapshot
        .iter()
        .chain(combat.zones.hand.iter())
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .chain(combat.zones.queued_cards.iter().map(|queued| &queued.card))
        .any(|card| card_has_external_payoff_opportunity(card, combat))
}

fn card_has_external_payoff_opportunity(card: &CombatCard, combat: &CombatState) -> bool {
    match card_mechanics_profile_v1(card.id).combat_external_payoff {
        Some(CombatExternalPayoffV1::PersistentOrReward) => true,
        Some(CombatExternalPayoffV1::HealingIfDamaged) => {
            combat.entities.player.current_hp < combat.entities.player.max_hp
        }
        None => false,
    }
}
