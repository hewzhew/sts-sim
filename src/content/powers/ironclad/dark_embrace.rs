use crate::action::Action;
use smallvec::SmallVec;

pub fn on_exhaust(amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::DrawCards(amount as u32));
    actions
}
