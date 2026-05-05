use crate::runtime::combat::CombatState;

pub fn at_turn_start(
    _state: &CombatState,
) -> smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    // Java: player.applyStartOfTurnOrbs()
    smallvec::smallvec![]
}

pub fn trigger_end_of_turn_orbs(
    _state: &CombatState,
) -> smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    // Java: TriggerEndOfTurnOrbsAction() -> Evoke/trigger passive for all orbs
    smallvec::smallvec![]
}
