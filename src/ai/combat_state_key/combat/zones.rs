use crate::runtime::combat::{CombatCard, CombatState};

use super::super::types::{CombatCardKey, CombatQueuedCardKey, CombatTargetKey, CombatZonesKey};
use super::cards::card_key;

pub(super) fn zones_key(combat: &CombatState) -> CombatZonesKey {
    CombatZonesKey {
        card_uuid_counter: combat.zones.card_uuid_counter,
        hand: zone_key(&combat.zones.hand),
        draw: zone_key(&combat.zones.draw_pile),
        discard: zone_key(&combat.zones.discard_pile),
        exhaust: zone_key(&combat.zones.exhaust_pile),
        limbo: zone_key(&combat.zones.limbo),
        queued: combat
            .zones
            .queued_cards
            .iter()
            .map(|queued| CombatQueuedCardKey {
                card: card_key(&queued.card),
                target: target_key(combat, queued.target),
                energy_on_use: queued.energy_on_use,
                ignore_energy_total: queued.ignore_energy_total,
                autoplay: queued.autoplay,
                random_target: queued.random_target,
                is_end_turn_autoplay: queued.is_end_turn_autoplay,
                purge_on_use: queued.purge_on_use,
                source: queued.source,
            })
            .collect(),
    }
}

fn zone_key(cards: &[CombatCard]) -> Vec<CombatCardKey> {
    cards.iter().map(card_key).collect()
}

pub(super) fn target_key(combat: &CombatState, target: Option<usize>) -> CombatTargetKey {
    match target {
        None => CombatTargetKey::None,
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(CombatTargetKey::MonsterSlot)
            .unwrap_or(CombatTargetKey::Entity(entity_id)),
    }
}
