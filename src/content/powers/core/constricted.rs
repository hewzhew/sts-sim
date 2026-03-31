use crate::action::{Action, DamageType, DamageInfo};
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    
    // In Java, Constricted visually attacks the player from the monster, but functionally it's just normal damage to target.
    actions.push(Action::Damage(DamageInfo {
        source: owner,
        target: owner,
        base: amount,
        output: amount,
        damage_type: DamageType::Thorns,
        is_modified: false,
    }));

    actions
}
