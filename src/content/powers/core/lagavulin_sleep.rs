use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn on_hp_lost(owner: EntityId) -> SmallVec<[Action; 2]> {
    smallvec![
        Action::RemovePower { target: owner, power_id: PowerId::LagavulinSleep },
        // Java uses ReducePowerAction(Metallicize, 8) — reduce, don't remove entirely
        Action::ApplyPower { target: owner, source: owner, power_id: PowerId::Metallicize, amount: -8 },
        Action::SetMonsterMove {
            monster_id: owner,
            next_move_byte: 4, // 4 = STUN
            intent: crate::combat::Intent::Stun,
        }
    ]
}
