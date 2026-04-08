use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Self-Forming Clay: Whenever you lose HP, gain 3 Block next turn.
pub fn on_lose_hp() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::NextTurnBlock, // Need to define if missing
            amount: 3,
        },
        insertion_mode: AddTo::Top,
    });

    actions
}
