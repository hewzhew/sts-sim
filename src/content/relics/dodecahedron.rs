use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub struct Dodecahedron;

impl Dodecahedron {
    pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        if state.entities.player.current_hp == state.entities.player.max_hp {
            actions.push(ActionInfo {
                action: Action::GainEnergy { amount: 1 },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
