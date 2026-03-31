use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn second_wind_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::BlockPerNonAttack {
                block_per_card: card.base_block_mut,
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
