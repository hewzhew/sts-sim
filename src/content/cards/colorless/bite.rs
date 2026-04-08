use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn bite_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Bite requires a valid target!");
    // Exact mechanics: Deal 7 (8) damage. Heal 2 (3) HP. Exhaust.
    // The exhaust and damage output values will naturally adapt based on card definition mapping and upgrades.
    let heal_amount = if card.upgrades > 0 { 3 } else { 2 };

    smallvec![
        ActionInfo {
            action: Action::Damage(DamageInfo {
                source: card.uuid as usize,
                target,
                base: 7, // Base dmg, but the action engine modifies it with `card.base_damage_mut` automatically
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: card.base_damage_mut != 7,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::Heal {
                target: 0, // 0 is player entity ID
                amount: heal_amount,
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
