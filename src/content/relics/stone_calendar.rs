use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// StoneCalendar: At the end of turn 7, deal 52 damage to ALL enemies.
pub fn at_end_of_turn(state: &crate::combat::CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if state.turn_count == 7 {
        for monster in &state.monsters {
            if !monster.is_escaped && !monster.is_dying && monster.current_hp > 0 {
                actions.push(ActionInfo {
                    action: Action::LoseHp { target: monster.id, amount: 52 },
                    insertion_mode: AddTo::Bottom,
                });
            }
        }
    }
    actions
}
