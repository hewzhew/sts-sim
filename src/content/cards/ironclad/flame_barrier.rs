use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn flame_barrier_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: evaluated.base_block_mut as i32
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::FlameBarrier,
                amount: evaluated.base_magic_num_mut as i32,
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
