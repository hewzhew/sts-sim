use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;
use crate::state::HandSelectReason;

pub fn dual_wield_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let amount = card.base_magic_num_mut as u8; // 1, upgraded 2
    
    actions.push(ActionInfo {
        action: Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            reason: HandSelectReason::Copy { amount },
        },
        insertion_mode: AddTo::Bottom,
    });
    
    actions
}
