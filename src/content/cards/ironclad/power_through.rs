use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn power_through_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::MakeTempCardInHand { card_id: crate::content::cards::CardId::Wound, amount: 2 , upgraded: false },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainBlock { target: 0, amount: card.base_block_mut as i32 },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
