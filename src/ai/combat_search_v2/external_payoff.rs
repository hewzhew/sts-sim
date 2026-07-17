use crate::ai::card_semantics_v1::{card_mechanics_profile_v1, CombatExternalPayoffV1};
use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, CombatState, MetaChange};
use std::collections::HashMap;

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

pub(super) fn persistent_run_value(combat: &CombatState) -> i32 {
    combat.entities.player.max_hp
        + combat
            .entities
            .player
            .gold_delta_this_combat
            .saturating_div(5)
        + persistent_card_value(combat, CardId::RitualDagger)
        + persistent_card_value(combat, CardId::GeneticAlgorithm)
}

pub(super) fn persistent_card_value(combat: &CombatState, card_id: CardId) -> i32 {
    let mut misc_delta_by_uuid = HashMap::<u32, i32>::new();
    for change in &combat.meta.meta_changes {
        if let MetaChange::ModifyCardMisc { card_uuid, amount } = change {
            let delta = misc_delta_by_uuid.entry(*card_uuid).or_default();
            *delta = delta.saturating_add(*amount);
        }
    }
    combat
        .meta
        .master_deck_snapshot
        .iter()
        .filter(|card| card.id == card_id)
        .map(|card| {
            card.misc_value
                .saturating_add(
                    misc_delta_by_uuid
                        .get(&card.uuid)
                        .copied()
                        .unwrap_or_default(),
                )
                .max(0)
        })
        .sum()
}
