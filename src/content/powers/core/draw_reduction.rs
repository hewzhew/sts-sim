use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_round(
    owner: EntityId,
    amount: i32,
    just_applied: bool,
) -> smallvec::SmallVec<[Action; 2]> {
    if amount <= 0 || just_applied {
        return smallvec::smallvec![];
    }

    if amount <= 1 {
        smallvec::smallvec![Action::RemovePower {
            target: owner,
            power_id: PowerId::DrawReduction,
        }]
    } else {
        smallvec::smallvec![Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::DrawReduction,
            amount: -1,
        }]
    }
}
