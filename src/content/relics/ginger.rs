use crate::action::ActionInfo;
use crate::combat::CombatState;
use crate::content::powers::PowerId;
use smallvec::SmallVec;

/// Ginger: You can no longer become Weakened.
/// The game checks this at power application time. We hook into `on_apply_power` (to be created/used depending on Power architecture)
/// or we handle it in `powers/mod.rs` where Immunities are typically checked (like Artifact).
/// For the Relic architecture, we'll expose a passive bool check.

pub fn check_immunity(power: PowerId) -> bool {
    power == PowerId::Weak
}

pub fn on_receive_power_modify(power_id: PowerId, amount: i32) -> i32 {
    if power_id == PowerId::Weak {
        return 0;
    }
    amount
}

pub fn on_apply_power(_state: &CombatState, _power_id: PowerId) -> SmallVec<[ActionInfo; 4]> {
    SmallVec::new()
}
