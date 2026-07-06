use sts_simulator::runtime::combat::{CombatCard, CombatState};

use super::types::KeyCardCounterfactualPlacement;

pub(crate) fn move_key_card(
    combat: &mut CombatState,
    uuid: u32,
    placement: KeyCardCounterfactualPlacement,
) -> Option<()> {
    if matches!(placement, KeyCardCounterfactualPlacement::OpeningHand)
        && combat.zones.hand.iter().any(|card| card.uuid == uuid)
    {
        return Some(());
    }
    if matches!(placement, KeyCardCounterfactualPlacement::DrawTop)
        && combat
            .zones
            .draw_pile
            .first()
            .is_some_and(|card| card.uuid == uuid)
    {
        return Some(());
    }

    let card = take_card_by_uuid(combat, uuid)?;
    match placement {
        KeyCardCounterfactualPlacement::OpeningHand => combat.zones.hand.push(card),
        KeyCardCounterfactualPlacement::DrawTop => combat.zones.add_to_draw_pile_top(card),
    }
    Some(())
}

fn take_card_by_uuid(combat: &mut CombatState, uuid: u32) -> Option<CombatCard> {
    CombatState::remove_card_by_uuid(&mut combat.zones.hand, uuid)
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.draw_pile, uuid))
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.discard_pile, uuid))
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.exhaust_pile, uuid))
}
