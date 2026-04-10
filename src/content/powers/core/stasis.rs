use crate::action::Action;
use crate::core::EntityId;

pub struct StasisPower;

pub fn on_death(_owner: EntityId, card_uuid: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    // Java StasisPower displays amount = -1 and keeps the captured card separately.
    // Rust stores the captured UUID in Power.extra_data and passes it here on death.
    actions.push(Action::MoveCard {
        card_uuid: card_uuid as u32,
        from: crate::state::PileType::Limbo,
        to: crate::state::PileType::Hand,
    });
    actions
}
