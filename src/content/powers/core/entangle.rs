use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    // Entangle removes itself at the end of the turn
    smallvec::smallvec![
        Action::RemovePower {
            target: owner,
            power_id: PowerId::Entangle,
        }
    ]
}
