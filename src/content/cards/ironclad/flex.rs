use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn flex_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::ApplyPower { source: 0, target: 0, power_id: crate::content::powers::PowerId::Strength, amount: _card.base_magic_num_mut },
            insertion_mode: AddTo::Bottom
        },
        ActionInfo {
            action: Action::ApplyPower { source: 0, target: 0, power_id: crate::content::powers::PowerId::LoseStrength, amount: _card.base_magic_num_mut },
            insertion_mode: AddTo::Bottom
        }
    ]
}
