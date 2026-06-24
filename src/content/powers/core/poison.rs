use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::EntityId;

pub fn at_turn_start(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    if amount <= 0 {
        return smallvec::smallvec![Action::RemovePower {
            target: owner,
            power_id: PowerId::Poison,
        }];
    }

    smallvec::smallvec![Action::PoisonLoseHp {
        target: owner,
        amount,
    }]
}
