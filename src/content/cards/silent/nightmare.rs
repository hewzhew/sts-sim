use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatCard, CombatState};

pub fn nightmare_play(
    _state: &CombatState,
    card: &CombatCard,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::Nightmare {
            amount: card.base_magic_num_mut.max(0).min(u8::MAX as i32) as u8,
        },
        insertion_mode: crate::runtime::action::AddTo::Bottom,
    }]
}
