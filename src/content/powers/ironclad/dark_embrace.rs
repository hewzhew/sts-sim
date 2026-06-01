use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn on_exhaust(state: &CombatState, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    if state.are_monsters_basically_dead_java() {
        return actions;
    }
    actions.push(Action::DrawCards(amount as u32));
    actions
}
