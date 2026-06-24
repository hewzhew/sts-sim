use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::EntityId;

pub fn during_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if amount <= 1 {
        actions.push(Action::Suicide {
            target: owner,
            trigger_relics: true,
        });
    } else {
        actions.push(Action::ReducePower {
            target: owner,
            power_id: PowerId::Fading,
            amount: 1,
        });
    }

    actions
}
