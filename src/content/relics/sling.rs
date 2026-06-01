use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

/// Sling: at battle start, gain 2 Strength only in elite combats.
pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if !state.meta.is_elite_fight {
        return actions;
    }

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
