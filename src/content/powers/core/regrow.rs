use crate::action::Action;
use crate::core::EntityId;

// Regrow power. Note: For the basic engine, triggering this requires the monster to not be flagged
// as removed entirely. Currently mocked as a simplified hook since multi-monster sync requires more engine support.
pub fn at_end_of_turn(_owner: EntityId, _amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![]
}
