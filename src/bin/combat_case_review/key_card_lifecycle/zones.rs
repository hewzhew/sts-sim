use sts_simulator::runtime::combat::CombatState;

use super::types::CardZoneLabel;

pub(super) fn zone_for_uuid(combat: &CombatState, uuid: u32) -> CardZoneLabel {
    if combat.zones.hand.iter().any(|card| card.uuid == uuid) {
        CardZoneLabel::Hand
    } else if combat.zones.draw_pile.iter().any(|card| card.uuid == uuid) {
        CardZoneLabel::Draw
    } else if combat
        .zones
        .discard_pile
        .iter()
        .any(|card| card.uuid == uuid)
    {
        CardZoneLabel::Discard
    } else if combat
        .zones
        .exhaust_pile
        .iter()
        .any(|card| card.uuid == uuid)
    {
        CardZoneLabel::Exhaust
    } else if combat.zones.limbo.iter().any(|card| card.uuid == uuid) {
        CardZoneLabel::Limbo
    } else if combat
        .zones
        .queued_cards
        .iter()
        .any(|queued| queued.card.uuid == uuid)
    {
        CardZoneLabel::Queued
    } else if combat
        .meta
        .master_deck_snapshot
        .iter()
        .any(|card| card.uuid == uuid)
    {
        CardZoneLabel::MasterOnly
    } else {
        CardZoneLabel::Missing
    }
}
