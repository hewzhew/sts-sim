use crate::action::Action;
use crate::action::{DamageType, NO_SOURCE};
use crate::combat::CombatState;
use crate::core::EntityId;

pub fn on_calculate_damage_from_player(mut damage: f32, amount: i32) -> f32 {
    if amount > 0 {
        // Flight reduces damage by exactly 50% (damage / 2.0f) before block
        damage = damage / 2.0;
    }
    damage
}

pub fn at_damage_final_receive(damage: i32, amount: i32, damage_type: DamageType) -> i32 {
    if amount > 0 && damage_type != DamageType::HpLoss && damage_type != DamageType::Thorns {
        // Java FlightPower.atDamageFinalReceive returns a float and DamageInfo later floors it.
        // For integer inputs this is equivalent to floor(damage / 2.0).
        (damage as f32 / 2.0).floor() as i32
    } else {
        damage
    }
}

pub fn on_attacked(
    state: &CombatState,
    owner: EntityId,
    damage: i32,
    source: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    let owner_survived = state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == owner)
        .is_some_and(|m| m.current_hp > 0 && !m.is_dying);

    if damage > 0 && amount > 0 && owner_survived && source != NO_SOURCE {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: crate::content::powers::PowerId::Flight,
            amount: -1,
        });
    }

    actions
}

pub fn at_turn_start(
    state: &CombatState,
    owner: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let stored_amount = state
        .entities
        .power_db
        .get(&owner)
        .and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
                .map(|p| p.extra_data)
        })
        .unwrap_or(amount);

    if stored_amount > amount {
        smallvec::smallvec![Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: crate::content::powers::PowerId::Flight,
            amount: stored_amount - amount,
        }]
    } else {
        smallvec::smallvec![]
    }
}

pub fn on_remove(state: &CombatState, owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    if state.entities.monsters.iter().any(|m| {
        m.id == owner && m.monster_type == crate::content::monsters::EnemyId::Byrd as usize
    }) {
        smallvec::smallvec![Action::SetMonsterMove {
            monster_id: owner,
            next_move_byte: 4,
            intent: crate::combat::Intent::Stun,
        }]
    } else {
        smallvec::smallvec![]
    }
}
