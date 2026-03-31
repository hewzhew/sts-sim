use crate::action::Action;
use smallvec::SmallVec;

pub fn on_exhaust(owner: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::GainBlock { target: owner, amount });
    actions
}
