use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn sword_boomerang_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // In Java, AttackDamageRandomEnemyAction is resolved during the actual queue execution.
    // To achieve strict 1:1, we ideally need `Action::DamageRandomEnemy` and resolve the RNG target
    // inside the engine. For now, since we haven't added RandomTarget to Action, we can evaluate it
    // synchronously here if we simulate it, but the CORRECT way is to add an action.
    // Let's implement `Action::AttackDamageRandomEnemy` in the engine.
    for _ in 0..card.base_magic_num_mut {
        actions.push(ActionInfo {
            action: Action::AttackDamageRandomEnemy {
                base_damage: card.base_damage_mut,
                damage_type: DamageType::Normal,
                applies_target_modifiers: true,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
