use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_round(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    if amount > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Weak,
            amount: -1,
        });
    }
    actions
}
