use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use smallvec::SmallVec;

/// Tingsha: Whenever you discard a card, deal 3 damage to a random enemy.
pub fn on_discard(_state: &crate::runtime::combat::CombatState) -> SmallVec<[ActionInfo; 4]> {
    // Java: addToBot(DamageRandomEnemyAction(DamageInfo(player, 3, THORNS)))
    // Target selection and RNG handled inside the engine's AttackDamageRandomEnemy handler.
    smallvec::smallvec![ActionInfo {
        action: Action::AttackDamageRandomEnemy {
            base_damage: 3,
            damage_type: DamageType::Thorns,
            applies_target_modifiers: false,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
