use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_round(
    owner: EntityId,
    amount: i32,
    just_applied: bool,
) -> smallvec::SmallVec<[Action; 2]> {
    if just_applied {
        return smallvec::smallvec![];
    }
    if amount <= 1 {
        smallvec::smallvec![Action::RemovePower {
            target: owner,
            power_id: PowerId::NoBlock,
        }]
    } else {
        smallvec::smallvec![Action::ReducePower {
            target: owner,
            power_id: PowerId::NoBlock,
            amount: 1,
        }]
    }
}

pub fn on_calculate_block(_block: f32, _amount: i32) -> f32 {
    0.0
}
