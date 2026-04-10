use crate::action::Action;
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn at_turn_start(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    if amount <= 0 {
        return smallvec::smallvec![Action::RemovePower {
            target: owner,
            power_id: PowerId::Poison,
        }];
    }

    smallvec::smallvec![
        Action::LoseHp {
            target: owner,
            amount,
            triggers_rupture: false,
        },
        Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Poison,
            amount: -1,
        },
    ]
}
