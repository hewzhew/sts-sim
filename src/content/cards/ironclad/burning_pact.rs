use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn burning_pact_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::SuspendForHandSelect {
                min: 1,
                max: 1,
                reason: crate::state::HandSelectReason::Exhaust,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut as u32),
            insertion_mode: AddTo::Bottom,
        }
    ]
}
