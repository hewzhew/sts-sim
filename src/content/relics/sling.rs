use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Sling: At the start of each combat, if it is your first turn, gain 2 Strength.
/// (Effectively an at_battle_start relic.)
pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::Strength,
            amount: 2,
        },
        insertion_mode: AddTo::Top, // Java: addToTop
    });
    actions
}
