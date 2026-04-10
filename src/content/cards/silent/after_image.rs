use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn after_image_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::AfterImage,
            amount: 1,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
