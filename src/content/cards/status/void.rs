use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;

pub fn on_drawn(_state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainEnergy { amount: -1 },
            insertion_mode: AddTo::Top,
        }
    ]
}
