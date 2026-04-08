use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

pub struct Brimstone;

impl Brimstone {
    pub fn at_turn_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();

        // Give player 2 strength
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: state.player.id,
                target: state.player.id,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 2,
            },
            insertion_mode: AddTo::Bottom,
        });

        // Give all enemies 1 strength
        for monster in &state.monsters {
            if !monster.is_escaped && !monster.is_dying {
                actions.push(ActionInfo {
                    action: Action::ApplyPower {
                        source: state.player.id,
                        target: monster.id,
                        power_id: crate::content::powers::PowerId::Strength,
                        amount: 1,
                    },
                    insertion_mode: AddTo::Top,
                });
            }
        }

        actions
    }
}
