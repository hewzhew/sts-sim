pub mod corrupt_heart;
pub mod spire_shield;
pub mod spire_spear;

use crate::content::powers::{store, PowerId};
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

pub(super) fn surrounded_cleanup_actions(state: &CombatState) -> Vec<Action> {
    let mut actions = Vec::new();

    if store::has_power(state, 0, PowerId::Surrounded) {
        actions.push(Action::RemovePower {
            target: 0,
            power_id: PowerId::Surrounded,
        });
    }

    for monster in &state.entities.monsters {
        if monster.current_hp > 0
            && !monster.is_dying
            && store::has_power(state, monster.id, PowerId::BackAttack)
        {
            actions.push(Action::RemovePower {
                target: monster.id,
                power_id: PowerId::BackAttack,
            });
        }
    }

    actions
}
