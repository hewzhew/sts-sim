use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn prepared_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut.max(0) as u32),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::SuspendForHandSelect {
                min: card.base_magic_num_mut.max(0) as u8,
                max: card.base_magic_num_mut.max(0) as u8,
                can_cancel: false,
                filter: crate::state::HandSelectFilter::Any,
                reason: crate::state::HandSelectReason::Discard,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
