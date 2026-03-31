use crate::combat::CombatState;
use crate::action::Action;

pub fn on_end_of_turn(_state: &CombatState) -> smallvec::SmallVec<[crate::action::ActionInfo; 4]> {
    // Watcher: e.g. Like Water (Gain block at end of turn if in Calm)
    smallvec::smallvec![]
}
