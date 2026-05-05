use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn combust_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::Combust,
            amount: card.base_magic_num_mut, // Note: Java Combust also stores HP loss (1) separately. We might hardcode 1 in the power tick hook.
        },
        insertion_mode: AddTo::Bottom,
    }]
}
