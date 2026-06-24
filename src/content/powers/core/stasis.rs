use crate::runtime::action::Action;
use crate::EntityId;

pub struct StasisPower;

pub fn on_death(
    state: &crate::runtime::combat::CombatState,
    _owner: EntityId,
    card_uuid: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    if !state
        .zones
        .limbo
        .iter()
        .any(|card| card.uuid == card_uuid as u32)
    {
        return actions;
    }

    actions.push(Action::ReturnStasisCard {
        card_uuid: card_uuid as u32,
        to_hand: state.zones.hand.len() != 10,
    });
    actions
}
