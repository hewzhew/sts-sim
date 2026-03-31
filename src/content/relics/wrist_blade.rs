// WristBlade: Attacks that cost 0 deal 3 additional damage.
// This is a passive damage modifier evaluated in the damage calculation pipeline.
// The engine's damage calc checks for this relic and adds 3 to base damage
// when the played card is an Attack with effective cost 0.

/// Returns additional damage for 0-cost Attacks.
pub fn bonus_damage() -> i32 {
    3
}
