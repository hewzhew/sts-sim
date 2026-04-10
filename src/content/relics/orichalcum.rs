use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// Orichalcum: If you end your turn without Block, gain 6 Block.
pub fn at_end_of_turn(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // We must check if the player currently has NO block at the end of the turn.
    if state.entities.player.block == 0 {
        actions.push(ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: 6,
            },
            insertion_mode: AddTo::Top,
        });
    }

    actions
}
