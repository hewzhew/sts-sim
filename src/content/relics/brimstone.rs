use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub struct Brimstone;

impl Brimstone {
    pub fn at_turn_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();

        // Java calls addToTop for the player strength, then addToTop once per monster.
        // The queue helper preserves returned top-action order, so emit the effective
        // execution order directly: later Java addToTop calls execute first.
        for monster in state.entities.monsters.iter().rev() {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: monster.id,
                    target: monster.id,
                    power_id: crate::content::powers::PowerId::Strength,
                    amount: 1,
                },
                insertion_mode: AddTo::Top,
            });
        }

        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: state.entities.player.id,
                target: state.entities.player.id,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 2,
            },
            insertion_mode: AddTo::Top,
        });

        actions
    }
}
