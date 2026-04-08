// Odd Mushroom: Vulnerable now increases damage by 25% instead of 50%.
// This requires a behavior modifier in the `powers::vulnerable` damage calculation logic.

pub fn on_calculate_vulnerable_multiplier() -> f32 {
    VULNERABLE_MULTIPLIER
}

pub const VULNERABLE_MULTIPLIER: f32 = 1.25;

pub fn has_odd_mushroom() -> bool {
    true
}
