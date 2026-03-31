use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

pub struct BagOfMarbles;

impl BagOfMarbles {
    pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // apply 1 vulnerable to all enemies
        for monster in &state.monsters {
            if !monster.is_escaped && !monster.is_dying {
                actions.push(ActionInfo {
                    action: Action::ApplyPower {
                        source: state.player.id,
                        target: monster.id,
                        power_id: crate::content::powers::PowerId::Vulnerable,
                        amount: 1,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
        }
        actions
    }
}
