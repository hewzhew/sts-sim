use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn impervious_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: card.base_block_mut as i32
        },
        insertion_mode: AddTo::Bottom,
    }]
}
