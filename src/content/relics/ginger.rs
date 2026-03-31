use crate::combat::CombatState;
use crate::action::{ActionInfo};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

/// Ginger: You can no longer become Weakened.
/// The game checks this at power application time. We hook into `on_apply_power` (to be created/used depending on Power architecture)
/// or we handle it in `powers/mod.rs` where Immunities are typically checked (like Artifact).
/// For the Relic architecture, we'll expose a passive bool check.

pub fn check_immunity(power: PowerId) -> bool {
    power == PowerId::Weak
}

pub fn on_apply_power(_state: &CombatState, _power_id: PowerId) -> SmallVec<[ActionInfo; 4]> {
    // Currently relying on `check_immunity` being called by the Power application logic.
    SmallVec::new()
}
