use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn noxious_fumes_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::NoxiousFumes,
            amount: card.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
