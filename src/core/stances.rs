//! Stance System (Watcher mechanic)
//!
//! Implements the 4 stances from Java's `com.megacrit.cardcrawl.stances.*`.
//! Stances modify damage dealt/received and have on-enter/on-exit effects.

use serde::{Deserialize, Serialize};

/// The four possible stances in the game.
///
/// Java: `AbstractStance.getStanceFromName(String name)`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stance {
    /// No stance — default state. No modifiers.
    Neutral,
    /// Wrath: Deal and receive double NORMAL damage.
    /// Java: `WrathStance.atDamageGive` returns `damage * 2.0`
    Wrath,
    /// Calm: No damage modifiers. On exit: gain 2 Energy.
    /// Java: `CalmStance.onExitStance` → `GainEnergyAction(2)`
    Calm,
    /// Divinity: Deal triple NORMAL damage. On enter: gain 3 Energy.
    /// At start of turn: automatically return to Neutral.
    /// Java: `DivinityStance.atDamageGive` returns `damage * 3.0`
    Divinity,
}

impl Default for Stance {
    fn default() -> Self {
        Stance::Neutral
    }
}

impl Stance {
    /// Parse a stance name string into a Stance enum.
    ///
    /// Java: `AbstractStance.getStanceFromName(String name)`
    pub fn from_str(s: &str) -> Self {
        match s {
            "Wrath" => Stance::Wrath,
            "Calm" => Stance::Calm,
            "Divinity" => Stance::Divinity,
            "Neutral" | "None" | "" => Stance::Neutral,
            _ => {
                game_log!("⚠️ Unknown stance: '{}', defaulting to Neutral", s);
                Stance::Neutral
            }
        }
    }

    /// Modify outgoing NORMAL damage based on current stance.
    ///
    /// Java: `AbstractStance.atDamageGive(float damage, DamageType type)`
    /// - Wrath: ×2 for NORMAL damage
    /// - Divinity: ×3 for NORMAL damage
    /// - Others: no modification
    pub fn at_damage_give(&self, damage: f32) -> f32 {
        match self {
            Stance::Wrath => damage * 2.0,
            Stance::Divinity => damage * 3.0,
            _ => damage,
        }
    }

    /// Modify incoming NORMAL damage based on current stance.
    ///
    /// Java: `AbstractStance.atDamageReceive(float damage, DamageType type)`
    /// - Wrath: ×2 for NORMAL damage
    /// - Others: no modification
    pub fn at_damage_receive(&self, damage: f32) -> f32 {
        match self {
            Stance::Wrath => damage * 2.0,
            _ => damage,
        }
    }

    /// Effects to apply when entering this stance.
    ///
    /// Returns (energy_gain,) for now.
    /// Java: `onEnterStance()`
    /// - Divinity: +3 Energy
    pub fn on_enter_energy(&self) -> i32 {
        match self {
            Stance::Divinity => 3,
            _ => 0,
        }
    }

    /// Effects to apply when exiting this stance.
    ///
    /// Returns energy gained on exit.
    /// Java: `onExitStance()`
    /// - Calm: +2 Energy
    pub fn on_exit_energy(&self) -> i32 {
        match self {
            Stance::Calm => 2,
            _ => 0,
        }
    }

    /// Whether this stance auto-transitions to Neutral at start of turn.
    ///
    /// Java: `DivinityStance.atStartOfTurn()` → ChangeStanceAction("Neutral")
    pub fn auto_exit_on_turn_start(&self) -> bool {
        matches!(self, Stance::Divinity)
    }

    /// Display name for logging.
    pub fn name(&self) -> &'static str {
        match self {
            Stance::Neutral => "Neutral",
            Stance::Wrath => "Wrath",
            Stance::Calm => "Calm",
            Stance::Divinity => "Divinity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrath_double_damage() {
        assert_eq!(Stance::Wrath.at_damage_give(10.0), 20.0);
        assert_eq!(Stance::Wrath.at_damage_receive(10.0), 20.0);
    }

    #[test]
    fn test_divinity_triple_damage() {
        assert_eq!(Stance::Divinity.at_damage_give(10.0), 30.0);
        assert_eq!(Stance::Divinity.at_damage_receive(10.0), 10.0); // only outgoing
    }

    #[test]
    fn test_calm_no_damage_mod() {
        assert_eq!(Stance::Calm.at_damage_give(10.0), 10.0);
        assert_eq!(Stance::Calm.at_damage_receive(10.0), 10.0);
    }

    #[test]
    fn test_calm_exit_energy() {
        assert_eq!(Stance::Calm.on_exit_energy(), 2);
        assert_eq!(Stance::Wrath.on_exit_energy(), 0);
    }

    #[test]
    fn test_divinity_enter_energy() {
        assert_eq!(Stance::Divinity.on_enter_energy(), 3);
        assert_eq!(Stance::Wrath.on_enter_energy(), 0);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Stance::from_str("Wrath"), Stance::Wrath);
        assert_eq!(Stance::from_str("Calm"), Stance::Calm);
        assert_eq!(Stance::from_str("Divinity"), Stance::Divinity);
        assert_eq!(Stance::from_str("Neutral"), Stance::Neutral);
        assert_eq!(Stance::from_str("None"), Stance::Neutral);
    }
}
