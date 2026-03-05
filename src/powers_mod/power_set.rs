//! # Power System
//!
//! Data-driven buff/debuff system for Slay the Spire.
//!
//! Instead of hardcoding status effects, this module loads power definitions from JSON
//! and provides a unified API for applying, querying, and triggering powers.
//!
//! ## Architecture
//!
//! - **PowerDefinition**: Static data loaded from `powers.json` (name, type, stack behavior)
//! - **PowerInstance**: Runtime state (current stacks, metadata)
//! - **PowerLibrary**: Global registry of all power definitions
//! - **PowerSet**: Per-entity collection of active powers (replaces `statuses` HashMap)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// Power Definition (loaded from JSON)
// ============================================================================

/// The category of a power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerType {
    Buff,
    Debuff,
}

/// How stacks of this power behave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackType {
    /// Stacks increase intensity (Strength, Poison)
    Intensity,
    /// Stacks decrease by 1 each turn (Vulnerable, Weak)
    Duration,
    /// Combination: intensity AND duration (Regen, Poison)
    IntensityAndDuration,
    /// Stacks consumed on trigger (Artifact, Buffer)
    Counter,
    /// Does not stack, only presence matters (Barricade, Corruption)
    NoStack,
}

/// When a power triggers its effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerTrigger {
    /// Constant effect while active
    Passive,
    /// Triggers at the start of each turn
    TurnStart,
    /// Triggers at the end of each turn  
    TurnEnd,
    /// Triggers when damage is calculated (modifies outgoing damage)
    OnCalculateDamage,
    /// Triggers when block is calculated
    OnCalculateBlock,
    /// Triggers when receiving damage (modifies incoming damage)
    OnDamageReceived,
    /// Triggers when HP is lost
    OnLoseHP,
    /// Triggers when a card is played
    OnPlayCard,
    /// Triggers when an Attack card is played
    OnPlayAttack,
    /// Triggers when a Skill card is played
    OnPlaySkill,
    /// Triggers when a Power card is played
    OnPlayPower,
    /// Triggers when a card is exhausted
    OnExhaust,
    /// Triggers when a card is drawn
    OnDraw,
    /// Triggers when attacked (for Thorns, Flame Barrier)
    OnAttacked,
    /// Triggers when dealing unblocked damage
    OnUnblockedDamage,
    /// Triggers when gaining block
    OnGainBlock,
    /// Triggers when a debuff is applied
    OnDebuffApplied,
    /// Triggers when the entity dies
    OnDeath,
    /// Triggers when changing stance (Watcher)
    OnStanceChange,
    /// Triggers when Scrying (Watcher)
    OnScry,
    /// Triggers at the start of combat
    BattleStart,
    /// Triggers when orb effects occur (Defect)
    OnOrbEffect,
}

/// Static definition of a power, loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerDefinition {
    /// Unique identifier (e.g., "Strength", "Vulnerable")
    pub id: String,
    /// Display name
    pub name: String,
    /// Buff or Debuff
    #[serde(rename = "type")]
    pub power_type: PowerType,
    /// How stacks behave
    pub stack_type: StackType,
    /// User-facing description
    pub description: String,
    /// Does this power decrement over time?
    #[serde(default)]
    pub duration_based: bool,
    /// When does duration decrement? (TurnStart or TurnEnd)
    #[serde(default)]
    pub decrements_at: Option<String>,
    /// Can the power have negative stacks? (Strength, Dexterity)
    #[serde(default)]
    pub can_be_negative: bool,
    /// Is this power specific to a character class?
    #[serde(default)]
    pub class_specific: Option<String>,
    /// Is this power only for enemies?
    #[serde(default)]
    pub enemy_only: bool,
    /// The effect logic (parsed but not used for data-driven execution yet)
    #[serde(default)]
    pub logic: Option<serde_json::Value>,
}

// ============================================================================
// Power Library (global registry)
// ============================================================================

/// Global registry of all power definitions.
#[derive(Debug, Clone, Default)]
pub struct PowerLibrary {
    /// All power definitions indexed by ID
    powers: HashMap<String, PowerDefinition>,
}

impl PowerLibrary {
    /// Create an empty power library.
    pub fn new() -> Self {
        Self {
            powers: HashMap::new(),
        }
    }

    /// Load powers from a JSON file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read powers file: {}", e))?;
        
        let powers: Vec<PowerDefinition> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse powers JSON: {}", e))?;
        
        let mut library = Self::new();
        for power in powers {
            library.powers.insert(power.id.clone(), power);
        }
        
        Ok(library)
    }

    /// Get a power definition by ID.
    pub fn get(&self, id: &str) -> Option<&PowerDefinition> {
        self.powers.get(id)
    }

    /// Check if a power exists.
    pub fn contains(&self, id: &str) -> bool {
        self.powers.contains_key(id)
    }

    /// Get all power IDs.
    pub fn all_ids(&self) -> impl Iterator<Item = &String> {
        self.powers.keys()
    }

    /// Get the number of powers in the library.
    pub fn len(&self) -> usize {
        self.powers.len()
    }

    /// Check if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.powers.is_empty()
    }

    /// Check if a power is a buff.
    pub fn is_buff(&self, id: &str) -> bool {
        self.powers.get(id).map_or(false, |p| p.power_type == PowerType::Buff)
    }

    /// Check if a power is a debuff.
    pub fn is_debuff(&self, id: &str) -> bool {
        self.powers.get(id).map_or(false, |p| p.power_type == PowerType::Debuff)
    }

    /// Check if a power is duration-based.
    pub fn is_duration_based(&self, id: &str) -> bool {
        self.powers.get(id).map_or(false, |p| p.duration_based)
    }

    /// Get the stack type for a power.
    pub fn get_stack_type(&self, id: &str) -> Option<StackType> {
        self.powers.get(id).map(|p| p.stack_type)
    }
}

// ============================================================================
// Power Set (per-entity runtime state)
// ============================================================================

/// A collection of active powers on an entity (player or monster).
///
/// This is a wrapper around HashMap that provides type-safe power operations
/// and integrates with the PowerLibrary for validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerSet {
    /// Active powers: power_id -> current stacks
    powers: HashMap<String, i32>,
    /// Tracks powers applied this turn to prevent immediate decay
    #[serde(default)]
    pub just_applied: std::collections::HashSet<String>,
}

impl PowerSet {
    /// Create an empty power set.
    pub fn new() -> Self {
        Self {
            powers: HashMap::new(),
            just_applied: std::collections::HashSet::new(),
        }
    }

    /// Apply (add) stacks of a power. Returns the new stack count.
    ///
    /// For NoStack powers, this sets the stacks to 1 if amount > 0.
    /// For other powers, this adds to existing stacks.
    pub fn apply(&mut self, power_id: &str, amount: i32, library: Option<&PowerLibrary>) -> i32 {
        // Check if this is a NoStack power
        let is_no_stack = library
            .and_then(|lib| lib.get_stack_type(power_id))
            .map_or(false, |st| st == StackType::NoStack);

        if is_no_stack {
            // NoStack powers: just set to 1 if applying positive amount
            if amount > 0 {
                self.powers.insert(power_id.to_string(), 1);
                self.just_applied.insert(power_id.to_string());
                1
            } else {
                self.powers.remove(power_id);
                0
            }
        } else {
            // Normal stacking behavior
            let entry = self.powers.entry(power_id.to_string()).or_insert(0);
            *entry += amount;
            
            if amount > 0 {
                self.just_applied.insert(power_id.to_string());
            }
            
            // Check if this power can be negative
            let can_be_negative = library
                .and_then(|lib| lib.get(power_id))
                .map_or(false, |p| p.can_be_negative);
            
            // Remove if stacks <= 0 and can't be negative
            if *entry <= 0 && !can_be_negative {
                self.powers.remove(power_id);
                0
            } else {
                *entry
            }
        }
    }

    /// Set stacks of a power to a specific value.
    ///
    /// Java: `AbstractPower.amount` defaults to -1 for non-stackable powers
    /// (Barricade, Confusion, No Draw, etc.). We must preserve these values.
    /// Use `remove()` to explicitly remove a power.
    pub fn set(&mut self, power_id: &str, stacks: i32) {
        self.powers.insert(power_id.to_string(), stacks);
    }

    /// Set stacks of a power, even if negative.
    /// Use this for powers like Strength that can legitimately be negative.
    pub fn force_set(&mut self, power_id: &str, stacks: i32) {
        if stacks == 0 {
            self.powers.remove(power_id);
        } else {
            self.powers.insert(power_id.to_string(), stacks);
        }
    }

    /// Get the current stacks of a power.
    pub fn get(&self, power_id: &str) -> i32 {
        *self.powers.get(power_id).unwrap_or(&0)
    }

    /// Check if a power is present (any value, including -1 for flag powers).
    ///
    /// Java: `owner.hasPower("X")` checks existence, not amount.
    pub fn has(&self, power_id: &str) -> bool {
        self.powers.contains_key(power_id)
    }

    /// Remove a power entirely.
    pub fn remove(&mut self, power_id: &str) -> i32 {
        self.powers.remove(power_id).unwrap_or(0)
    }

    /// Remove a specific number of stacks. Returns the number removed.
    pub fn remove_stacks(&mut self, power_id: &str, amount: i32) -> i32 {
        if let Some(current) = self.powers.get_mut(power_id) {
            let to_remove = amount.min(*current);
            *current -= to_remove;
            if *current <= 0 {
                self.powers.remove(power_id);
            }
            to_remove
        } else {
            0
        }
    }

    /// Decrement all duration-based powers by 1. Removes those that reach 0.
    pub fn decrement_durations(&mut self, library: &PowerLibrary) {
        let to_decrement: Vec<String> = self.powers.keys()
            .filter(|id| library.is_duration_based(id))
            .cloned()
            .collect();

        for id in to_decrement {
            if let Some(stacks) = self.powers.get_mut(&id) {
                *stacks -= 1;
                if *stacks <= 0 {
                    self.powers.remove(&id);
                }
            }
        }
    }

    /// Decrement specific powers by 1 (for turn-end decay).
    pub fn decrement_specific(&mut self, power_ids: &[&str]) {
        for id in power_ids {
            if let Some(stacks) = self.powers.get_mut(*id) {
                *stacks -= 1;
                if *stacks <= 0 {
                    self.powers.remove(*id);
                }
            }
        }
    }

    /// Java STS equivalent: `AbstractPower.atEndOfRound()`.
    /// Called after all turn actions have concluded (player + enemies).
    /// Decays specific turn-based debuffs while respecting the just_applied flag.
    pub fn on_round_end(&mut self) {
        let turn_statuses = [power_ids::VULNERABLE, power_ids::WEAK, power_ids::FRAIL];
        
        for status in &turn_statuses {
            if self.has(status) {
                // If it was just applied this turn, skip the decay but clear the flag
                if self.just_applied.contains(*status) {
                    self.just_applied.remove(*status);
                } else {
                    // Normal decay
                    self.remove_stacks(status, 1);
                }
            }
        }
        
        // Ensure all just_applied flags are fully cleared at the true end of round
        self.just_applied.clear();
    }

    /// Get all active powers as an iterator.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &i32)> {
        self.powers.iter()
    }

    /// Get all active power IDs.
    pub fn active_powers(&self) -> impl Iterator<Item = &String> {
        self.powers.keys()
    }

    /// Clear all powers.
    pub fn clear(&mut self) {
        self.powers.clear();
        self.just_applied.clear();
    }

    /// Check if there are any active powers.
    pub fn is_empty(&self) -> bool {
        self.powers.is_empty()
    }

    /// Get the number of active powers.
    pub fn len(&self) -> usize {
        self.powers.len()
    }

    /// Get the internal HashMap (for backwards compatibility).
    pub fn as_map(&self) -> &HashMap<String, i32> {
        &self.powers
    }

    /// Get a mutable reference to the internal HashMap.
    pub fn as_map_mut(&mut self) -> &mut HashMap<String, i32> {
        &mut self.powers
    }
}

// ============================================================================
// Common Power IDs (constants for type safety)
// ============================================================================

pub mod power_ids {
    // === Shared Buffs ===
    pub const STRENGTH: &str = "Strength";
    pub const DEXTERITY: &str = "Dexterity";
    pub const FOCUS: &str = "Focus";
    pub const ARTIFACT: &str = "Artifact";
    pub const BARRICADE: &str = "Barricade";
    pub const BUFFER: &str = "Buffer";
    pub const INTANGIBLE: &str = "Intangible";
    pub const METALLICIZE: &str = "Metallicize";
    pub const PLATED_ARMOR: &str = "PlatedArmor";
    pub const REGEN: &str = "Regen";
    pub const REGENERATE: &str = "Regenerate";
    pub const RITUAL: &str = "Ritual";
    pub const THORNS: &str = "Thorns";
    pub const VIGOR: &str = "Vigor";
    pub const ENERGIZED: &str = "Energized";
    pub const DRAW_CARD: &str = "DrawCard";
    pub const NEXT_TURN_BLOCK: &str = "NextTurnBlock";
    pub const MANTRA: &str = "Mantra";
    pub const BLUR: &str = "Blur";
    
    // === Shared Debuffs ===
    pub const VULNERABLE: &str = "Vulnerable";
    pub const WEAK: &str = "Weak";
    pub const FRAIL: &str = "Frail";
    pub const POISON: &str = "Poison";
    pub const STRENGTH_DOWN: &str = "StrengthDown";
    pub const DEXTERITY_DOWN: &str = "DexterityDown";
    pub const SHACKLED: &str = "Shackled";
    pub const CONSTRICTED: &str = "Constricted";
    pub const ENTANGLED: &str = "Entangled";
    pub const NO_DRAW: &str = "NoDraw";
    pub const NO_BLOCK: &str = "NoBlock";
    pub const SLOW: &str = "Slow";
    pub const HEX: &str = "Hex";
    pub const CONFUSED: &str = "Confused";
    pub const DRAW_REDUCTION: &str = "DrawReduction";
    
    // === Duplication Effects ===
    pub const DOUBLE_TAP: &str = "DoubleTap";
    pub const BURST: &str = "Burst";
    pub const AMPLIFY: &str = "Amplify";
    pub const DUPLICATION: &str = "Duplication";
    pub const ECHO_FORM: &str = "EchoForm";
    
    // === Ironclad ===
    pub const DEMON_FORM: &str = "DemonForm";
    pub const FLAME_BARRIER: &str = "FlameBarrier";
    pub const CORRUPTION: &str = "Corruption";
    pub const BERSERK: &str = "Berserk";
    pub const BRUTALITY: &str = "Brutality";
    pub const COMBUST: &str = "Combust";
    pub const DARK_EMBRACE: &str = "DarkEmbrace";
    pub const EVOLVE: &str = "Evolve";
    pub const FEEL_NO_PAIN: &str = "FeelNoPain";
    pub const FIRE_BREATHING: &str = "FireBreathing";
    pub const JUGGERNAUT: &str = "Juggernaut";
    pub const RAGE: &str = "Rage";
    pub const RUPTURE: &str = "Rupture";
    
    // === Silent ===
    pub const ACCURACY: &str = "Accuracy";
    pub const AFTER_IMAGE: &str = "AfterImage";
    pub const ENVENOM: &str = "Envenom";
    pub const NOXIOUS_FUMES: &str = "NoxiousFumes";
    pub const THOUSAND_CUTS: &str = "ThousandCuts";
    pub const WRAITH_FORM: &str = "WraithForm";
    pub const CHOKED: &str = "Choked";
    pub const CORPSE_EXPLOSION: &str = "CorpseExplosion";
    
    // === Defect ===
    pub const CREATIVE_AI: &str = "CreativeAI";
    pub const HEATSINK: &str = "Heatsink";
    pub const HELLO_WORLD: &str = "HelloWorld";
    pub const LOOP: &str = "Loop";
    pub const STORM: &str = "Storm";
    pub const STATIC_DISCHARGE: &str = "StaticDischarge";
    pub const ELECTRO: &str = "Electro";
    pub const BIAS: &str = "Bias";
    pub const LOCK_ON: &str = "LockOn";
    
    // === Watcher ===
    pub const MENTAL_FORTRESS: &str = "MentalFortress";
    pub const RUSHDOWN: &str = "Rushdown";
    pub const LIKE_WATER: &str = "LikeWater";
    pub const NIRVANA: &str = "Nirvana";
    pub const FORESIGHT: &str = "Foresight";
    pub const BATTLE_HYMN: &str = "BattleHymn";
    pub const DEVOTION: &str = "Devotion";
    pub const ESTABLISHMENT: &str = "Establishment";
    pub const FASTING: &str = "Fasting";
    pub const MARK: &str = "Mark";
    
    // === Enemy Powers ===
    pub const ANGRY: &str = "Angry";
    pub const ENRAGE: &str = "Enrage";
    pub const CURIOSITY: &str = "Curiosity";
    pub const CURL_UP: &str = "CurlUp";
    pub const SPORE_CLOUD: &str = "SporeCloud";
    pub const SPLIT: &str = "Split";
    pub const FLYING: &str = "Flying";
    pub const MALLEABLE: &str = "Malleable";
    pub const MODE_SHIFT: &str = "ModeShift";
    pub const SHARP_HIDE: &str = "SharpHide";
    pub const INVINCIBLE: &str = "Invincible";
    pub const BEAT_OF_DEATH: &str = "BeatOfDeath";
    pub const PAINFUL_STABS: &str = "PainfulStabs";
    pub const TIME_WARP: &str = "TimeWarp";
    pub const THIEVERY: &str = "Thievery";
    pub const STRENGTH_UP: &str = "StrengthUp";
    pub const MINION: &str = "Minion";
    pub const FADING: &str = "Fading";
    
    // === Special ===
    pub const PEN_NIB: &str = "PenNib";
    pub const DOUBLE_DAMAGE: &str = "DoubleDamage";
    pub const BLOCK_RETURN: &str = "BlockReturn";
    
    // === Player-Only Temporary Effects ===
    pub const FREE_ATTACK: &str = "FreeAttack";
    pub const REBOUND: &str = "Rebound";
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_set_basic_operations() {
        let mut powers = PowerSet::new();
        
        // Apply strength
        powers.apply("Strength", 3, None);
        assert_eq!(powers.get("Strength"), 3);
        
        // Stack more strength
        powers.apply("Strength", 2, None);
        assert_eq!(powers.get("Strength"), 5);
        
        // Apply vulnerable
        powers.apply("Vulnerable", 2, None);
        assert!(powers.has("Vulnerable"));
        assert_eq!(powers.get("Vulnerable"), 2);
        
        // Remove stacks
        powers.remove_stacks("Vulnerable", 1);
        assert_eq!(powers.get("Vulnerable"), 1);
        
        // Remove completely
        powers.remove("Strength");
        assert!(!powers.has("Strength"));
        assert_eq!(powers.get("Strength"), 0);
    }



    #[test]
    fn test_duration_decrement() {
        let mut powers = PowerSet::new();
        
        powers.apply("Vulnerable", 3, None);
        powers.apply("Weak", 2, None);
        powers.apply("Strength", 5, None);
        
        // Decrement specific durations
        powers.decrement_specific(&["Vulnerable", "Weak"]);
        
        assert_eq!(powers.get("Vulnerable"), 2);
        assert_eq!(powers.get("Weak"), 1);
        assert_eq!(powers.get("Strength"), 5); // Not decremented
        
        // Decrement again
        powers.decrement_specific(&["Vulnerable", "Weak"]);
        assert_eq!(powers.get("Vulnerable"), 1);
        assert!(!powers.has("Weak")); // Removed at 0
    }
}
