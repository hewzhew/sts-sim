use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;

pub fn on_end_turn_in_hand(_state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::Damage(crate::action::DamageInfo {
            source: 0,
            target: 0,
            base: 2,
            output: 2,
            damage_type: crate::action::DamageType::Thorns,
            is_modified: false,
        }),
        insertion_mode: AddTo::Bottom,
    }]
}
