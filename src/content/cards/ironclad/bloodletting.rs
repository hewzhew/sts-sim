use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn bloodletting_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::LoseHp {
                target: 0,
                amount: 3,
                triggers_rupture: true,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainEnergy {
                amount: card.base_magic_num_mut
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
