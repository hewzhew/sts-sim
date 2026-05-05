use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::Action;

pub fn on_calculate_damage_received_from_attack(_damage: i32, amount: i32) -> i32 {
    // Intangible caps attack damage received to 1
    if amount > 0 && _damage > 1 {
        1
    } else {
        _damage
    }
}

pub fn at_damage_final_receive(
    damage: i32,
    amount: i32,
    _damage_type: crate::runtime::action::DamageType,
) -> i32 {
    if amount > 0 && damage > 1 {
        1
    } else {
        damage
    }
}

pub fn at_end_of_turn(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if amount > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Intangible,
            amount: -1,
        });
    }

    actions
}

pub fn at_end_of_round(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if amount > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::IntangiblePlayer,
            amount: -1,
        });
    }

    actions
}
