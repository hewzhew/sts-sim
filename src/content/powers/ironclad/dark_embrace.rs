use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn on_exhaust(state: &CombatState, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    if !state
        .entities
        .monsters
        .iter()
        .any(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped && !m.half_dead)
    {
        return actions;
    }
    actions.push(Action::DrawCards(amount as u32));
    actions
}
