use crate::runtime::action::Action;
use crate::runtime::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    if amount > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Strength,
            amount,
        });
    }
    actions.push(Action::RemovePower {
        target: owner,
        power_id: PowerId::Shackled,
    });
    actions
}
