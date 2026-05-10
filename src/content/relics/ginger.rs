use crate::content::powers::PowerId;

/// Java ApplyPowerAction blocks player-targeted Weakened before Artifact.
pub fn on_receive_power_modify(power_id: PowerId, amount: i32) -> i32 {
    if power_id == PowerId::Weak {
        return 0;
    }
    amount
}
