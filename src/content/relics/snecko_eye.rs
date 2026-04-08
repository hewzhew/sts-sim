use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Snecko Eye: Draw 2 additional cards each turn. Start each combat Confused.
pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Confusion
    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::Confusion,
            amount: 0, // Confusion has no numeric value
        },
        insertion_mode: AddTo::Bottom,
    });

    // Note: The +2 draw per turn is handled natively in the
    // draw phase of the player's turn state via relic check.

    actions
}
