use crate::combat::CombatState;
use crate::core::EntityId;
use crate::action::Action;

// Thievery power tracks the amount of gold stolen. Doesn't heavily impact combat interactions.
pub fn on_monster_turn_ended(
    _state: &CombatState,
    _owner: EntityId,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![]
}
