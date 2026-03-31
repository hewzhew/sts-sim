use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn exhume_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::SuspendForGridSelect {
                source_pile: crate::state::PileType::Exhaust,
                min: 1,
                max: 1,
                can_cancel: false,
                reason: crate::state::GridSelectReason::Exhume { upgrade: card.upgrades > 0 },
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
