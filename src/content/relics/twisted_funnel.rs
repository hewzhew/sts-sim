use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// TwistedFunnel: At the start of combat, apply 4 Poison to ALL enemies.
pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    for monster in &state.monsters {
        if !monster.is_escaped && !monster.is_dying {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: state.player.id,
                    target: monster.id,
                    power_id: crate::content::powers::PowerId::Poison,
                    amount: 4,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }
    actions
}
