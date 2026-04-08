use crate::action::Action;
use crate::combat::CombatState;
use crate::core::EntityId;

pub fn on_calculate_damage_from_player(mut damage: f32, amount: i32) -> f32 {
    if amount > 0 {
        // Flight reduces damage by exactly 50% (damage / 2.0f) before block
        damage = damage / 2.0;
    }
    damage
}

pub fn at_damage_final_receive(
    damage: i32,
    amount: i32,
    _damage_type: crate::action::DamageType,
) -> i32 {
    if amount > 0 {
        // Java: this.output = Math.round(this.output / 2.0F)
        (damage as f32 / 2.0).round() as i32
    } else {
        damage
    }
}

pub fn on_attacked(
    state: &CombatState,
    owner: EntityId,
    damage: i32,
    _source: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    let owner_survived = state
        .monsters
        .iter()
        .find(|m| m.id == owner)
        .is_some_and(|m| m.current_hp > 0 && !m.is_dying);

    if damage > 0 && amount > 0 && owner_survived {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: crate::content::powers::PowerId::Flight,
            amount: -1,
        });

        // If this attack will reduce Flight to 0, stun the monster.
        // The Java code triggers "GROUNDED" ChangeStateAction onRemove().
        // In our engine, we handle the Stun transition when Flight drops to 0.
        if amount == 1 {
            actions.push(Action::SetMonsterMove {
                monster_id: owner,
                next_move_byte: 4, // 4 corresponds to STUNNED in Byrd
                intent: crate::combat::Intent::Stun,
            });
        }
    }

    actions
}

pub fn at_turn_start(
    state: &CombatState,
    owner: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let stored_amount = state
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
