use crate::combat::CombatState;
use crate::action::Action;

pub fn at_turn_start(_state: &CombatState) -> smallvec::SmallVec<[crate::action::ActionInfo; 4]> {
    // Java: player.applyStartOfTurnOrbs()
    smallvec::smallvec![]
}

pub fn trigger_end_of_turn_orbs(_state: &CombatState) -> smallvec::SmallVec<[crate::action::ActionInfo; 4]> {
    // Java: TriggerEndOfTurnOrbsAction() -> Evoke/trigger passive for all orbs
    smallvec::smallvec![]
}
