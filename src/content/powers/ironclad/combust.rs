use crate::action::{Action, DamageType};
use crate::combat::CombatState;
use crate::content::powers::PowerId;
use smallvec::SmallVec;

/// Java: CombustPower.atEndOfTurn(isPlayer)
///
/// The Java power has TWO state values:
///   - `amount`  → damage dealt to ALL enemies (stored in Power.amount)
///   - `hpLoss`  → HP lost by the player (stored in Power.extra_data)
///
/// On stack (playing Combust again):
///   - `amount += magicNumber`  (normal stack via ApplyPower)
///   - `hpLoss += 1`            (always +1, handled in action_handlers.rs)
///
/// At end of turn:
///   1. LoseHPAction(owner, owner, hpLoss, FIRE)
///   2. DamageAllEnemiesAction(null, createDamageMatrix(amount, true), THORNS, FIRE)
pub fn at_end_of_turn(
    state: &CombatState,
    owner: crate::core::EntityId,
    amount: i32,
) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();

    // Get hpLoss from extra_data (defaults to 1 if somehow missing)
    let hp_loss = state
        .entities
        .power_db
        .get(&owner)
        .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::Combust))
        .map(|p| p.extra_data)
        .unwrap_or(1);

    actions.push(Action::LoseHp {
        target: owner,
        amount: hp_loss,
        triggers_rupture: true,
    });
    actions.push(Action::DamageAllEnemies {
        source: owner,
        damages: crate::action::repeated_damage_matrix(state.entities.monsters.len(), amount),
        damage_type: DamageType::Thorns, // Java: DamageInfo.DamageType.THORNS
        is_modified: false,
    });
    actions
}
