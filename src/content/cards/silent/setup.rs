use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::{HandSelectFilter, HandSelectReason};
use smallvec::SmallVec;

pub fn setup_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: HandSelectFilter::Any,
            reason: HandSelectReason::Setup,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
