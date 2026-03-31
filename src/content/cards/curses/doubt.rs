use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::content::powers::PowerId;

pub fn on_end_turn_in_hand(_state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::ApplyPower {
                target: 0,
                source: 0,
                power_id: PowerId::Weak,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
