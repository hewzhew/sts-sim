use crate::action::{Action, DamageType};
use smallvec::SmallVec;

pub fn on_block_gained(amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::AttackDamageRandomEnemy {
        base_damage: amount,
        damage_type: DamageType::Thorns,
        applies_target_modifiers: false,
    });
    actions
}
