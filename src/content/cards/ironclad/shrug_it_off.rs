use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn shrug_it_off_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: card.base_block_mut
            },
            insertion_mode: AddTo::Bottom
        },
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut as u32),
            insertion_mode: AddTo::Bottom
        }
    ]
}
