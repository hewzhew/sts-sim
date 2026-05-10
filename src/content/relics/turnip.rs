use crate::content::powers::PowerId;

/// Java ApplyPowerAction blocks player-targeted Frail before Artifact.
pub fn on_receive_power_modify(power_id: PowerId, amount: i32) -> i32 {
    if power_id == PowerId::Frail {
        return 0;
    }
    amount
}
