//! # ID Mapping: CommunicationMod ↔ Rust Engine
//!
//! Java Slay the Spire uses internal IDs (`public static final String ID/POWER_ID`)
//! that differ from class names. CommunicationMod sends these Java IDs, but our Rust
//! engine uses class-derived names in match patterns.
//!
//! This module provides a **single source of truth** for all ID translations.
//!
//! ## Design
//!
//! **Generic rule**: Most mismatches are just space removal (`"Time Warp" → "TimeWarp"`).
//! The default fallback strips spaces from the CommunicationMod ID.
//!
//! **Explicit exceptions**: A small table handles non-trivial renames (e.g.,
//! `"Weakened" → "Weak"`, `"Yang" → "Duality"`).

// ============================================================================
// Power ID Mapping
// ============================================================================

/// Explicit power ID mappings: (CommunicationMod Java ID, Rust engine ID).
///
/// Only entries where space-removal is NOT sufficient are listed here.
/// All other IDs use the generic `remove_spaces` fallback.
const POWER_SPECIAL_MAP: &[(&str, &str)] = &[
    // Java WeakPower.POWER_ID = "Weakened" → Rust uses "Weak"
    ("Weakened", "Weak"),
    // Java IntangiblePlayerPower.POWER_ID = "IntangiblePlayer" → Rust uses "Intangible"
    ("IntangiblePlayer", "Intangible"),
    // Java RushdownPower.POWER_ID = "Adaptation" → Rust uses "Rushdown"
    ("Adaptation", "Rushdown"),
    // Java MentalFortressPower.POWER_ID = "Controlled" → Rust uses "MentalFortress"
    ("Controlled", "MentalFortress"),
    // Java ForesightPower.POWER_ID = "WireheadingPower" → Rust uses "Foresight"
    ("WireheadingPower", "Foresight"),
    // Java MarkPower.POWER_ID = "PathToVictoryPower" → Rust uses "Mark" 
    ("PathToVictoryPower", "Mark"),
    // Java WraithFormPower.POWER_ID = "Wraith Form v2" → Rust uses "WraithForm"
    // Note: space removal would give "WraithFormv2", so explicit entry needed
    ("Wraith Form v2", "WraithForm"),
    // Java RegenPower.POWER_ID = "Regeneration" → Rust uses "Regen"
    ("Regeneration", "Regen"),
    // Java LoseDexterityPower.POWER_ID = "DexLoss" → keep as "DexLoss"
    // (No mapping needed — identity, but listed for documentation)
    // Java LoseStrengthPower.POWER_ID = "Flex" → keep as "Flex"  
    // (No mapping needed — identity)
    // Java LockOnPower.POWER_ID = "Lockon" → Rust uses "LockOn" (capitalization)
    ("Lockon", "LockOn"),
];

/// Convert a CommunicationMod power ID to the Rust engine's internal power ID.
///
/// Strategy:
/// 1. Check explicit exception table
/// 2. Fallback: remove spaces (`"Time Warp" → "TimeWarp"`)
pub fn commod_to_engine_power_id(commod_id: &str) -> String {
    // Check explicit exceptions first
    for &(java_id, rust_id) in POWER_SPECIAL_MAP {
        if commod_id == java_id {
            return rust_id.to_string();
        }
    }
    // Generic fallback: strip spaces (handles 24+ mappings automatically)
    // e.g. "Time Warp" → "TimeWarp", "Pen Nib" → "PenNib", etc.
    commod_id.replace(' ', "")
}

/// Convert a Rust engine power ID back to the CommunicationMod (Java) power ID.
///
/// This is the inverse of `commod_to_engine_power_id()`.
/// Used by `snapshot_from_game_state()` to convert actual state back to
/// CommunicationMod format for comparison.
pub fn engine_to_commod_power_id(rust_id: &str) -> String {
    // Check explicit exceptions (reverse direction)
    for &(java_id, engine_id) in POWER_SPECIAL_MAP {
        if rust_id == engine_id {
            return java_id.to_string();
        }
    }
    // For the generic space-removal cases, the Rust ID == the no-space version.
    // CommunicationMod may send either "TimeWarp" or "Time Warp" for the same power.
    // Since our comparison normalizes both sides, returning as-is works.
    rust_id.to_string()
}

// ============================================================================
// Relic ID Mapping
// ============================================================================

/// Explicit relic ID mappings: (CommunicationMod Java ID, Rust engine ID).
///
/// Only entries where space-removal is NOT sufficient are listed here.
const RELIC_SPECIAL_MAP: &[(&str, &str)] = &[
    // Java Duality.ID = "Yang" → Rust uses "Duality"
    ("Yang", "Duality"),
    // Java SnakeRing.ID = "Ring of the Snake" → Rust uses "SnakeRing"
    // (space removal gives "RingoftheSnake", wrong)
    ("Ring of the Snake", "SnakeRing"),
    // Java RingOfTheSerpent.ID = "Ring of the Serpent" → Rust uses "RingOfTheSerpent"
    // Space removal gives "RingoftheSerpent" (lowercase "of", "the") — wrong
    ("Ring of the Serpent", "RingOfTheSerpent"),
    // Java WingBoots.ID = "WingedGreaves" → Rust uses "WingBoots"
    ("WingedGreaves", "WingBoots"),
    // Java GoldPlatedCables.ID = "Cables" → Rust uses "GoldPlatedCables"
    ("Cables", "GoldPlatedCables"),
    // Java Abacus.ID = "TheAbacus" → Rust uses "Abacus" (if/when implemented)
    ("TheAbacus", "Abacus"),
];

/// Convert a CommunicationMod relic ID to the Rust engine's internal relic ID.
///
/// Strategy:
/// 1. Check explicit exception table
/// 2. Fallback: remove spaces (`"Art of War" → "ArtofWar"`)
///
/// Note: The hydrator also does `.replace(' ', "")` after this function, so
/// the fallback here just returns the ID unchanged (spaces handled downstream).
pub fn commod_to_engine_relic_id(commod_id: &str) -> String {
    // Check explicit exceptions first
    for &(java_id, rust_id) in RELIC_SPECIAL_MAP {
        if commod_id == java_id {
            return rust_id.to_string();
        }
    }
    // Return as-is; the hydrator's `.replace(' ', "")` handles space removal
    commod_id.to_string()
}

/// Convert a Rust engine relic ID back to the CommunicationMod (Java) relic ID.
pub fn engine_to_commod_relic_id(rust_id: &str) -> String {
    for &(java_id, engine_id) in RELIC_SPECIAL_MAP {
        if rust_id == engine_id {
            return java_id.to_string();
        }
    }
    rust_id.to_string()
}

// ============================================================================
// Card ID Mapping
// ============================================================================

/// Convert a CommunicationMod card ID to the card library's internal ID.
///
/// CommunicationMod sends the Java card class's `ID` field, which for Watcher
/// cards often uses CamelCase (e.g., "FlurryOfBlows", "CrushJoints").
/// The Rust card library uses underscore_case (e.g., "flurry_of_blows").
///
/// This function first checks explicit overrides, then applies a generic
/// CamelCase → underscore_case normalization.
pub fn commod_to_library_card_id(commod_id: &str) -> String {
    // Explicit overrides for non-standard mappings
    match commod_id {
        // Add explicit card ID overrides here if CamelCase→underscore fails
        _ => {}
    }
    
    // Generic CamelCase → underscore_case normalization
    // "FlurryOfBlows" → "flurry_of_blows"
    let mut result = String::with_capacity(commod_id.len() + 8);
    for (i, ch) in commod_id.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_space_removal() {
        assert_eq!(commod_to_engine_power_id("Time Warp"), "TimeWarp");
        assert_eq!(commod_to_engine_power_id("Pen Nib"), "PenNib");
        assert_eq!(commod_to_engine_power_id("Fire Breathing"), "FireBreathing");
        assert_eq!(commod_to_engine_power_id("Plated Armor"), "PlatedArmor");
        assert_eq!(commod_to_engine_power_id("No Draw"), "NoDraw");
        assert_eq!(commod_to_engine_power_id("Double Tap"), "DoubleTap");
    }

    #[test]
    fn test_power_special_cases() {
        assert_eq!(commod_to_engine_power_id("Weakened"), "Weak");
        assert_eq!(commod_to_engine_power_id("Adaptation"), "Rushdown");
        assert_eq!(commod_to_engine_power_id("Controlled"), "MentalFortress");
        assert_eq!(commod_to_engine_power_id("WireheadingPower"), "Foresight");
        assert_eq!(commod_to_engine_power_id("PathToVictoryPower"), "Mark");
        assert_eq!(commod_to_engine_power_id("Wraith Form v2"), "WraithForm");
        assert_eq!(commod_to_engine_power_id("Regeneration"), "Regen");
        assert_eq!(commod_to_engine_power_id("Lockon"), "LockOn");
        assert_eq!(commod_to_engine_power_id("IntangiblePlayer"), "Intangible");
    }

    #[test]
    fn test_power_identity() {
        // Powers that don't need mapping
        assert_eq!(commod_to_engine_power_id("Strength"), "Strength");
        assert_eq!(commod_to_engine_power_id("Dexterity"), "Dexterity");
        assert_eq!(commod_to_engine_power_id("Vulnerable"), "Vulnerable");
        assert_eq!(commod_to_engine_power_id("Frail"), "Frail");
        assert_eq!(commod_to_engine_power_id("Poison"), "Poison");
        assert_eq!(commod_to_engine_power_id("DexLoss"), "DexLoss");
    }

    #[test]
    fn test_power_roundtrip_special() {
        // Special cases must roundtrip
        for &(java_id, rust_id) in POWER_SPECIAL_MAP {
            let forward = commod_to_engine_power_id(java_id);
            assert_eq!(forward, rust_id, "Forward: {} → {}", java_id, rust_id);
            let reverse = engine_to_commod_power_id(rust_id);
            assert_eq!(reverse, java_id, "Reverse: {} → {}", rust_id, java_id);
        }
    }

    #[test]
    fn test_relic_special_cases() {
        assert_eq!(commod_to_engine_relic_id("Yang"), "Duality");
        assert_eq!(commod_to_engine_relic_id("Ring of the Snake"), "SnakeRing");
        assert_eq!(commod_to_engine_relic_id("WingedGreaves"), "WingBoots");
        assert_eq!(commod_to_engine_relic_id("Cables"), "GoldPlatedCables");
    }

    #[test]
    fn test_relic_passthrough() {
        // Relics with space-only differences pass through (hydrator strips spaces)
        assert_eq!(commod_to_engine_relic_id("Art of War"), "Art of War");
        assert_eq!(commod_to_engine_relic_id("Pen Nib"), "Pen Nib");
    }

    #[test]
    fn test_card_camelcase() {
        assert_eq!(commod_to_library_card_id("FlurryOfBlows"), "flurry_of_blows");
        assert_eq!(commod_to_library_card_id("CrushJoints"), "crush_joints");
        assert_eq!(commod_to_library_card_id("ClearTheMind"), "clear_the_mind");
        assert_eq!(commod_to_library_card_id("Strike_P"), "strike__p"); // underscore preserved
    }
}
