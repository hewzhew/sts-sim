// No dependencies

/// Golden Idol: Enemies drop 25% more Gold.
/// Handled statically at reward generation time inside `run.rs`'s `generate_combat_rewards`.
/// We only export an empty struct or identifying marker for consistency.

pub fn on_gold_gain(amount: i32) -> i32 {
    amount + (amount as f32 * 0.25) as i32
}
