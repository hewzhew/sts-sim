use crate::combat::CombatState;
use crate::core::EntityId;
use crate::action::Action;

// Confusion randomizes card costs when drawn. This modifies cost tracking in combat state dynamically. 
// A placeholder is added for integration.
pub fn on_card_drawn(
    _state: &mut CombatState,
    _owner: EntityId,
    _card_uuid: u32,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![]
}
