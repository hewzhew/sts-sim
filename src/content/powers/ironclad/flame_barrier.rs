use crate::action::{Action, DamageType, DamageInfo};
use smallvec::SmallVec;

pub fn on_attacked(source: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::Damage(DamageInfo {
        source: 0,
        target: source,
        base: amount,
        output: amount,
        damage_type: DamageType::Thorns,
        is_modified: false,
    }));
    actions
}
