use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn at_end_of_turn(owner: EntityId, power_amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if power_amount == 1 {
        actions.push(Action::Damage(DamageInfo {
            source: owner,
            target: 0,
            base: 30,
            output: 30,
            damage_type: DamageType::Thorns,
            is_modified: false,
        }));
        actions.push(Action::Suicide { target: owner });
    } else {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Explosive,
            amount: -1,
        });
    }

    actions
}
