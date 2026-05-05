use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn acrobatics_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::DrawCards(card.base_magic_num_mut as u32),
        insertion_mode: AddTo::Bottom,
    }];

    if !_state.zones.hand.is_empty() {
        actions.push(ActionInfo {
            action: Action::SuspendForHandSelect {
                min: 1,
                max: 1,
                can_cancel: false,
                filter: crate::state::HandSelectFilter::Any,
                reason: crate::state::HandSelectReason::Discard,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
