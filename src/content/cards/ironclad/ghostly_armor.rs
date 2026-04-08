use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn ghostly_armor_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: card.base_block_mut as i32
        },
        insertion_mode: AddTo::Bottom,
    }]
}
