use crate::combat::{CombatState, CombatCard, Intent};
use crate::action::{Action, ActionInfo, AddTo};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn spot_weakness_play(state: &CombatState, card: &CombatCard, target: EntityId) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    
    // Check if target intends to attack
    let is_attacking = if let Some(target_monster) = state.monsters.iter().find(|m| m.id == target) {
        match target_monster.current_intent {
            Intent::Attack { .. } | Intent::AttackDefend { .. } | Intent::AttackBuff { .. } | Intent::AttackDebuff { .. } => true,
            _ => false,
        }
    } else {
        false
    };

    if is_attacking {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Strength,
                amount: card.base_magic_num_mut,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
