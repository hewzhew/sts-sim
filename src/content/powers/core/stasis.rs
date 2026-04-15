use crate::action::Action;
use crate::core::EntityId;

pub struct StasisPower;

pub fn on_death(
    state: &crate::combat::CombatState,
    _owner: EntityId,
    card_uuid: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    let Some(card) = state
        .zones
        .limbo
        .iter()
        .find(|card| card.uuid == card_uuid as u32)
        .cloned()
    else {
        return actions;
    };

    // Java StasisPower returns a fresh copy of the captured card to hand, or to
    // discard if hand is full, then the stasis-held runtime copy ceases to exist.
    if state.zones.hand.len() < 10 {
        actions.push(Action::MakeCopyInHand {
            original: Box::new(card),
            amount: 1,
        });
    } else {
        actions.push(Action::MakeCopyInDiscard {
            original: Box::new(card),
            amount: 1,
        });
    }
    actions.push(Action::RemoveCardFromPile {
        card_uuid: card_uuid as u32,
        from: crate::state::PileType::Limbo,
    });
    actions
}
