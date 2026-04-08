use crate::content::powers::PowerId;

/// Turnip: You can no longer become Frail.
pub fn is_turnip() -> bool {
    true
}

pub fn check_immunity(power: PowerId) -> bool {
    power == PowerId::Frail
}

pub fn on_receive_power_modify(power_id: PowerId, amount: i32) -> i32 {
    if power_id == PowerId::Frail {
        return 0;
    }
    amount
}
