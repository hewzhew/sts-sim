use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::content::cards::CardId;
use smallvec::SmallVec;

pub fn blade_dance_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::MakeTempCardInHand {
            card_id: CardId::Shiv,
            amount: card.base_magic_num_mut.max(0) as u8,
            upgraded: false,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
