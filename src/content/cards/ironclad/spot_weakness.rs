use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn spot_weakness_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Spot Weakness requires a valid target!");
    let mut actions = smallvec::SmallVec::new();

    // Check if target intends to attack
    let is_attacking =
        if let Some(target_monster) = state.entities.monsters.iter().find(|m| m.id == target) {
            crate::projection::combat::monster_has_visible_attack_in_combat(state, target_monster)
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
