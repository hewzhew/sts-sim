use crate::action::Action;
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    actions.push(Action::ApplyPower {
        source: owner,
        target: owner,
        power_id: PowerId::Strength,
        amount: -amount,
    });
    actions.push(Action::RemovePower {
        target: owner,
        power_id: PowerId::LoseStrength,
    });
    actions
}
