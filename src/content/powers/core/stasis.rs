use crate::action::Action;
use crate::core::EntityId;

pub struct StasisPower;

pub fn on_death(_owner: EntityId, power_amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    // In our implementation, `power_amount` stores the UUID of the stolen card.
    actions.push(Action::MoveCard {
        card_uuid: power_amount as u32,
        from: crate::state::PileType::Limbo,
        to: crate::state::PileType::Hand,
    });
    actions
}
