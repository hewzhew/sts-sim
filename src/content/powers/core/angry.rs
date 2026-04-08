use crate::action::Action;
use crate::combat::{CombatState, PowerId};
use smallvec::{smallvec, SmallVec};

pub fn on_attacked(
    _state: &CombatState,
    owner: crate::core::EntityId,
    _damage: i32,
    _source: crate::core::EntityId,
    power_amount: i32,
) -> SmallVec<[Action; 2]> {
    // Note: Slay the Spire usually triggers this even if attack damage is 0 or fully blocked.
    // We assume the caller (engine.rs) filters by DamageType::Normal.
    smallvec![Action::ApplyPower {
        target: owner,
        source: owner,
        power_id: PowerId::Strength,
        amount: power_amount,
    }]
}
