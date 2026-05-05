use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn battle_trance_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut as u32),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::NoDraw,
                amount: -1, // Java NoDrawPower uses a sentinel amount of -1
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
