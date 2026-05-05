use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn bouncing_flask_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::BouncingFlask {
            target: None,
            amount: 3,
            num_times: card.base_magic_num_mut.max(0) as u8,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
