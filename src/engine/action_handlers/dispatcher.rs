mod card_route;
mod damage_route;
mod orb_route;
mod power_route;
mod spawn_route;
mod unhandled;

use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

/// Executes one queued action by delegating to the relevant domain handler.
pub fn execute_action(action: Action, state: &mut CombatState) {
    let action = match damage_route::try_execute(action, state) {
        Ok(()) => return,
        Err(action) => action,
    };
    let action = match power_route::try_execute(action, state) {
        Ok(()) => return,
        Err(action) => action,
    };
    let action = match card_route::try_execute(action, state) {
        Ok(()) => return,
        Err(action) => action,
    };
    let action = match spawn_route::try_execute(action, state) {
        Ok(()) => return,
        Err(action) => action,
    };
    let action = match orb_route::try_execute(action, state) {
        Ok(()) => return,
        Err(action) => action,
    };
    unhandled::execute(action, state);
}
