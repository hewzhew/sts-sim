use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::Action;

pub fn at_end_of_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if amount <= 1 {
        actions.push(Action::Suicide { target: owner });
    } else {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Fading,
            amount: -1,
        });
    }

    actions
}
