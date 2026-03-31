use crate::action::Action;
use smallvec::SmallVec;

pub fn on_post_draw(owner: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::LoseHp { target: owner, amount: 1 });
    actions.push(Action::DrawCards(amount as u32));
    actions
}
