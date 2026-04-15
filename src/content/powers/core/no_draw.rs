use crate::runtime::action::Action;
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![Action::RemovePower {
        target: owner,
        power_id: crate::content::powers::PowerId::NoDraw,
    }]
}
