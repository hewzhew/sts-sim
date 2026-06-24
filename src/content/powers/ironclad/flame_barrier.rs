use crate::runtime::action::{Action, DamageInfo, DamageType};
use smallvec::SmallVec;

pub fn on_attacked(
    owner: crate::EntityId,
    source: crate::EntityId,
    damage_type: DamageType,
    amount: i32,
) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    if source == crate::runtime::action::NO_SOURCE
        || source == owner
        || matches!(damage_type, DamageType::Thorns | DamageType::HpLoss)
    {
        return actions;
    }
    actions.push(Action::Damage(DamageInfo {
        source: owner,
        target: source,
        base: amount,
        output: amount,
        damage_type: DamageType::Thorns,
        is_modified: false,
    }));
    actions
}
