use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::content::cards::CardId;
use smallvec::SmallVec;

pub fn cloak_and_dagger_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: card.base_block_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInHand {
                card_id: CardId::Shiv,
                amount: card.base_magic_num_mut.max(0) as u8,
                upgraded: false,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
