use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn entrench_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let current_block = state.entities.player.block;
    if current_block > 0 {
        smallvec::smallvec![ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: current_block
            },
            insertion_mode: AddTo::Bottom,
        }]
    } else {
        smallvec::smallvec![]
    }
}
