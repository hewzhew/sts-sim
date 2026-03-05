//! Orb system for the Defect class.
//!
//! Java reference:
//! - AbstractOrb.java — base class (applyFocus, onEvoke, onEndOfTurn)
//! - Lightning.java — passive: 3+Focus damage to random enemy; evoke: 8+Focus damage
//! - Frost.java — passive: 2+Focus block; evoke: 5+Focus block
//! - Dark.java — passive: accumulate 6+Focus to evokeAmount; evoke: deal accumulated to lowest HP
//! - Plasma.java — passive: 1 energy (start of turn, no Focus); evoke: 2 energy (no Focus)
//!
//! Key differences from Java:
//! - Plasma passive fires onStartOfTurn, others fire onEndOfTurn
//! - Dark.applyFocus() only applies to passiveAmount, not evokeAmount
//! - Focus does NOT affect Plasma at all

use serde::{Deserialize, Serialize};

/// The type of an orb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrbType {
    Lightning,
    Frost,
    Dark,
    Plasma,
}

impl OrbType {
    /// Parse from a string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lightning" => Some(OrbType::Lightning),
            "frost" => Some(OrbType::Frost),
            "dark" => Some(OrbType::Dark),
            "plasma" => Some(OrbType::Plasma),
            _ => None,
        }
    }

    /// Get the display name.
    pub fn name(&self) -> &'static str {
        match self {
            OrbType::Lightning => "Lightning",
            OrbType::Frost => "Frost",
            OrbType::Dark => "Dark",
            OrbType::Plasma => "Plasma",
        }
    }

    /// Base passive amount (before Focus).
    pub fn base_passive(&self) -> i32 {
        match self {
            OrbType::Lightning => 3,
            OrbType::Frost => 2,
            OrbType::Dark => 6,
            OrbType::Plasma => 1,
        }
    }

    /// Base evoke amount (before Focus).
    pub fn base_evoke(&self) -> i32 {
        match self {
            OrbType::Lightning => 8,
            OrbType::Frost => 5,
            OrbType::Dark => 6, // starting evoke; grows via passive accumulation
            OrbType::Plasma => 2,
        }
    }

    /// Whether Focus affects this orb's passive/evoke amounts.
    /// Java: Plasma does NOT call applyFocus (it overrides with no-op).
    pub fn affected_by_focus(&self) -> bool {
        !matches!(self, OrbType::Plasma)
    }

    /// Whether this orb's passive fires at start of turn (vs end of turn).
    /// Java: Plasma.onStartOfTurn() vs Lightning/Frost/Dark.onEndOfTurn()
    pub fn passive_at_start_of_turn(&self) -> bool {
        matches!(self, OrbType::Plasma)
    }
}

/// A single orb slot containing an orb instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbSlot {
    /// The type of orb in this slot.
    pub orb_type: OrbType,
    /// Current passive amount (affected by Focus).
    pub passive_amount: i32,
    /// Current evoke amount (affected by Focus for Lightning/Frost; accumulates for Dark).
    pub evoke_amount: i32,
}

impl OrbSlot {
    /// Create a new orb slot with Focus applied.
    pub fn new(orb_type: OrbType, focus: i32) -> Self {
        let (passive, evoke) = Self::calculate_amounts(orb_type, focus);
        Self {
            orb_type,
            passive_amount: passive,
            evoke_amount: evoke,
        }
    }

    /// Calculate passive and evoke amounts with Focus applied.
    /// Java: AbstractOrb.applyFocus() → passiveAmount = basePassiveAmount + focus; evokeAmount = baseEvokeAmount + focus
    /// Dark.applyFocus() → only passiveAmount gets focus, NOT evokeAmount
    /// Plasma: Focus does not apply at all
    fn calculate_amounts(orb_type: OrbType, focus: i32) -> (i32, i32) {
        if !orb_type.affected_by_focus() {
            return (orb_type.base_passive(), orb_type.base_evoke());
        }

        match orb_type {
            OrbType::Dark => {
                // Dark: Focus only applies to passive, not evoke
                // Java: Dark.applyFocus() overrides to only modify passiveAmount
                let passive = std::cmp::max(0, orb_type.base_passive() + focus);
                let evoke = orb_type.base_evoke(); // evoke starts at base, grows via passive
                (passive, evoke)
            }
            _ => {
                // Lightning, Frost: Focus applies to both passive and evoke
                let passive = std::cmp::max(0, orb_type.base_passive() + focus);
                let evoke = std::cmp::max(0, orb_type.base_evoke() + focus);
                (passive, evoke)
            }
        }
    }

    /// Re-apply Focus when Focus value changes.
    /// For Dark orbs, only passive is recalculated; evoke retains accumulated value.
    pub fn reapply_focus(&mut self, focus: i32) {
        if !self.orb_type.affected_by_focus() {
            return;
        }

        match self.orb_type {
            OrbType::Dark => {
                // Only recalculate passive; evoke is accumulated
                self.passive_amount = std::cmp::max(0, self.orb_type.base_passive() + focus);
            }
            _ => {
                let (passive, evoke) = Self::calculate_amounts(self.orb_type, focus);
                self.passive_amount = passive;
                self.evoke_amount = evoke;
            }
        }
    }

    /// Execute the passive effect at end of turn.
    /// Returns a PassiveEffect describing what happened.
    ///
    /// Java:
    /// - Lightning.onEndOfTurn() → deal passiveAmount to random enemy
    /// - Frost.onEndOfTurn() → gain passiveAmount block
    /// - Dark.onEndOfTurn() → evokeAmount += passiveAmount (accumulate)
    pub fn on_end_of_turn(&mut self) -> PassiveEffect {
        match self.orb_type {
            OrbType::Lightning => PassiveEffect::DamageRandom(self.passive_amount),
            OrbType::Frost => PassiveEffect::GainBlock(self.passive_amount),
            OrbType::Dark => {
                // Dark accumulates: evokeAmount += passiveAmount each turn
                self.evoke_amount += self.passive_amount;
                PassiveEffect::DarkAccumulate(self.passive_amount, self.evoke_amount)
            }
            OrbType::Plasma => PassiveEffect::None, // Plasma fires at start of turn
        }
    }

    /// Execute the passive effect at start of turn (Plasma only).
    /// Java: Plasma.onStartOfTurn() → gain 1 energy
    pub fn on_start_of_turn(&self) -> PassiveEffect {
        match self.orb_type {
            OrbType::Plasma => PassiveEffect::GainEnergy(self.passive_amount),
            _ => PassiveEffect::None, // Other orbs fire at end of turn
        }
    }

    /// Execute the evoke effect (when orb is evoked/pushed out).
    /// Returns an EvokeEffect describing what happened.
    ///
    /// Java:
    /// - Lightning.onEvoke() → deal evokeAmount to random (or all with Electro)
    /// - Frost.onEvoke() → gain evokeAmount block
    /// - Dark.onEvoke() → deal accumulated evokeAmount to lowest HP enemy
    /// - Plasma.onEvoke() → gain evokeAmount energy
    pub fn on_evoke(&self, has_electro: bool) -> EvokeEffect {
        match self.orb_type {
            OrbType::Lightning => {
                if has_electro {
                    EvokeEffect::DamageAll(self.evoke_amount)
                } else {
                    EvokeEffect::DamageRandom(self.evoke_amount)
                }
            }
            OrbType::Frost => EvokeEffect::GainBlock(self.evoke_amount),
            OrbType::Dark => EvokeEffect::DamageLowestHp(self.evoke_amount),
            OrbType::Plasma => EvokeEffect::GainEnergy(self.evoke_amount),
        }
    }
}

/// Result of an orb's passive effect (each turn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassiveEffect {
    /// No effect (e.g., Plasma at end of turn).
    None,
    /// Deal damage to a random enemy (Lightning).
    DamageRandom(i32),
    /// Gain block (Frost).
    GainBlock(i32),
    /// Dark accumulated energy (amount added, new total).
    DarkAccumulate(i32, i32),
    /// Gain energy (Plasma at start of turn).
    GainEnergy(i32),
}

/// Result of an orb's evoke effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvokeEffect {
    /// Deal damage to a random enemy (Lightning without Electro).
    DamageRandom(i32),
    /// Deal damage to ALL enemies (Lightning with Electro).
    DamageAll(i32),
    /// Gain block (Frost).
    GainBlock(i32),
    /// Deal damage to the lowest HP enemy (Dark).
    DamageLowestHp(i32),
    /// Gain energy (Plasma).
    GainEnergy(i32),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightning_base_values() {
        let orb = OrbSlot::new(OrbType::Lightning, 0);
        assert_eq!(orb.passive_amount, 3);
        assert_eq!(orb.evoke_amount, 8);
    }

    #[test]
    fn test_lightning_with_focus() {
        let orb = OrbSlot::new(OrbType::Lightning, 2);
        assert_eq!(orb.passive_amount, 5); // 3 + 2
        assert_eq!(orb.evoke_amount, 10); // 8 + 2
    }

    #[test]
    fn test_frost_base_values() {
        let orb = OrbSlot::new(OrbType::Frost, 0);
        assert_eq!(orb.passive_amount, 2);
        assert_eq!(orb.evoke_amount, 5);
    }

    #[test]
    fn test_dark_focus_only_passive() {
        let orb = OrbSlot::new(OrbType::Dark, 3);
        assert_eq!(orb.passive_amount, 9); // 6 + 3
        assert_eq!(orb.evoke_amount, 6);   // Focus does NOT affect Dark's evoke
    }

    #[test]
    fn test_dark_accumulation() {
        let mut orb = OrbSlot::new(OrbType::Dark, 0);
        assert_eq!(orb.evoke_amount, 6);

        // After 1 turn: evoke += passive (6)
        let eff = orb.on_end_of_turn();
        assert_eq!(eff, PassiveEffect::DarkAccumulate(6, 12));
        assert_eq!(orb.evoke_amount, 12);

        // After 2 turns: evoke += passive (6) again
        let eff2 = orb.on_end_of_turn();
        assert_eq!(eff2, PassiveEffect::DarkAccumulate(6, 18));
        assert_eq!(orb.evoke_amount, 18);
    }

    #[test]
    fn test_plasma_no_focus() {
        // Focus should NOT affect Plasma
        let orb = OrbSlot::new(OrbType::Plasma, 5);
        assert_eq!(orb.passive_amount, 1); // unchanged
        assert_eq!(orb.evoke_amount, 2);   // unchanged
    }

    #[test]
    fn test_plasma_passive_at_start() {
        let orb = OrbSlot::new(OrbType::Plasma, 0);
        // End of turn: no effect
        assert_eq!(orb.on_start_of_turn(), PassiveEffect::GainEnergy(1));
    }

    #[test]
    fn test_lightning_evoke_with_electro() {
        let orb = OrbSlot::new(OrbType::Lightning, 0);
        assert_eq!(orb.on_evoke(false), EvokeEffect::DamageRandom(8));
        assert_eq!(orb.on_evoke(true), EvokeEffect::DamageAll(8));
    }

    #[test]
    fn test_frost_evoke() {
        let orb = OrbSlot::new(OrbType::Frost, 0);
        assert_eq!(orb.on_evoke(false), EvokeEffect::GainBlock(5));
    }

    #[test]
    fn test_dark_evoke_after_accumulation() {
        let mut orb = OrbSlot::new(OrbType::Dark, 0);
        orb.on_end_of_turn(); // evoke: 6 → 12
        orb.on_end_of_turn(); // evoke: 12 → 18
        assert_eq!(orb.on_evoke(false), EvokeEffect::DamageLowestHp(18));
    }

    #[test]
    fn test_negative_focus_clamped() {
        // Focus can't make amounts negative (clamped to 0)
        let orb = OrbSlot::new(OrbType::Lightning, -10);
        assert_eq!(orb.passive_amount, 0);
        assert_eq!(orb.evoke_amount, 0);
    }

    #[test]
    fn test_reapply_focus() {
        let mut orb = OrbSlot::new(OrbType::Lightning, 0);
        assert_eq!(orb.passive_amount, 3);
        assert_eq!(orb.evoke_amount, 8);

        orb.reapply_focus(3);
        assert_eq!(orb.passive_amount, 6);
        assert_eq!(orb.evoke_amount, 11);
    }

    #[test]
    fn test_dark_reapply_focus_preserves_evoke() {
        let mut orb = OrbSlot::new(OrbType::Dark, 0);
        orb.on_end_of_turn(); // evoke: 6 → 12

        // Reapply focus shouldn't reset accumulated evoke
        orb.reapply_focus(3);
        assert_eq!(orb.passive_amount, 9); // 6 + 3
        assert_eq!(orb.evoke_amount, 12);  // preserved accumulated value
    }
}
