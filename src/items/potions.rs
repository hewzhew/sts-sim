//! Potion system for Slay the Spire simulator.
//!
//! This module provides potion definitions, a library for loading potions from JSON,
//! and helpers for potion management during combat.
//!
//! # Overview
//!
//! Potions are consumable items that provide powerful one-time effects during combat.
//! Each character can hold up to 3 potion slots (2 on higher ascensions by default).
//! Potions can be obtained from combat rewards, shops, and special events.
//!
//! # Example
//!
//! ```no_run
//! use sts_sim::potions::PotionLibrary;
//!
//! let library = PotionLibrary::load("data/potions.json")?;
//! let fire_potion = library.get("FirePotion")?;
//! game_log!("Potion: {} - {}", fire_potion.name, fire_potion.description);
//! # Ok::<(), sts_sim::potions::PotionError>(())
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during potion operations.
#[derive(Error, Debug)]
pub enum PotionError {
    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse JSON from '{path}': {source}")]
    JsonParse {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Potion '{0}' not found in library")]
    PotionNotFound(String),

    #[error("Empty potion library loaded from '{0}'")]
    EmptyLibrary(String),

    #[error("Invalid potion slot index: {0}")]
    InvalidSlot(usize),

    #[error("Potion slot {0} is empty")]
    EmptySlot(usize),

    #[error("All potion slots are full")]
    SlotsFull,
}

// ============================================================================
// Enums
// ============================================================================

/// Potion rarity determines drop rates and shop prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotionRarity {
    /// Common potions (most frequent drops, lowest price)
    Common,
    /// Uncommon potions (moderate drop rate and price)
    Uncommon,
    /// Rare potions (rarest, most expensive)
    Rare,
}

impl Default for PotionRarity {
    fn default() -> Self {
        Self::Common
    }
}

/// Potion target type determines how the potion is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotionTarget {
    /// Targets the player (no target selection needed)
    #[serde(rename = "Self")]
    Player,
    /// Targets a single enemy (requires target selection)
    Enemy,
    /// Targets all enemies (no target selection needed)
    AllEnemies,
}

impl Default for PotionTarget {
    fn default() -> Self {
        Self::Player
    }
}

/// Character class restriction for potions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotionClass {
    /// Can be used by any character
    Any,
    /// Ironclad only
    Ironclad,
    /// Silent only
    Silent,
    /// Defect only
    Defect,
    /// Watcher only
    Watcher,
}

impl Default for PotionClass {
    fn default() -> Self {
        Self::Any
    }
}

/// Command hint for potion effects (used by engine to determine action type).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotionCommand {
    // ---- Damage effects ----
    /// Deal damage to a single target
    DealDamage,
    /// Deal damage to all enemies
    DealDamageAll,

    // ---- Block effects ----
    /// Gain block
    GainBlock,

    // ---- Healing effects ----
    /// Heal HP (flat amount)
    Heal,
    /// Heal percentage of max HP (Blood Potion)
    HealPercent,
    /// Gain Max HP (Fruit Juice)
    GainMaxHP,

    // ---- Buff/Debuff effects ----
    /// Apply Weak to target
    ApplyWeak,
    /// Apply Vulnerable to target
    ApplyVulnerable,
    /// Apply Poison to target
    ApplyPoison,
    /// Gain Strength (may be temporary)
    GainStrength,
    /// Gain Dexterity (may be temporary)
    GainDexterity,
    /// Gain Artifact
    GainArtifact,
    /// Gain Plated Armor
    GainPlatedArmor,
    /// Gain Thorns
    GainThorns,
    /// Gain Metallicize
    GainMetallicize,
    /// Gain Ritual
    GainRitual,
    /// Gain Regeneration
    GainRegeneration,
    /// Gain Intangible
    GainIntangible,

    // ---- Resource effects ----
    /// Gain Energy
    GainEnergy,
    /// Draw cards
    DrawCards,

    // ---- Discover/Add card effects ----
    /// Discover an Attack card
    DiscoverAttack,
    /// Discover a Skill card
    DiscoverSkill,
    /// Discover a Power card
    DiscoverPower,
    /// Discover a Colorless card
    DiscoverColorless,
    /// Discover a Rare card
    DiscoverRare,
    /// Add Miracles to hand
    AddMiracles,
    /// Add Shivs to hand
    AddShivs,

    // ---- Defect-specific effects ----
    /// Gain Focus
    GainFocus,
    /// Gain Orb slots
    GainOrbSlots,
    /// Channel Dark orbs (Essence of Darkness)
    ChannelDark,

    // ---- Watcher-specific effects ----
    /// Enter a stance (Calm, Wrath, Divinity)
    EnterStance,

    // ---- Special effects ----
    /// Recall a card from discard pile
    RecallFromDiscard,
    /// Exhaust cards from hand
    ExhaustCards,
    /// Upgrade cards in hand
    UpgradeCards,
    /// Double next card played (like Double Tap)
    DoubleTap,
    /// Fill empty potion slots with random potions
    FillPotions,
    /// Escape from non-boss combat
    Escape,
    /// Fairy in a Bottle - revive on death
    FairyRevive,
    /// Play cards from top of draw pile (Distilled Chaos)
    PlayFromDraw,
    /// Discard any number, draw that many (Gambler's Brew)
    GamblerDraw,
    /// Draw cards + randomize costs (Snecko Oil)
    SneckoEffect,
}

impl Default for PotionCommand {
    fn default() -> Self {
        Self::Heal
    }
}

// ============================================================================
// Potion Definition
// ============================================================================

/// Definition of a potion loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotionDefinition {
    /// Unique identifier (e.g., "FirePotion")
    pub id: String,

    /// Display name (e.g., "Fire Potion")
    pub name: String,

    /// Potion rarity
    pub rarity: PotionRarity,

    /// Character class restriction
    #[serde(rename = "class")]
    pub potion_class: PotionClass,

    /// Target type
    pub target: PotionTarget,

    /// Description text
    pub description: String,

    /// Command hint for the engine
    pub command_hint: PotionCommand,

    /// Potency value (base numeric value, e.g., damage, block)
    #[serde(default)]
    pub potency: Option<i32>,

    /// Potency when upgraded by Sacred Bark relic
    #[serde(default)]
    pub potency_upgraded: Option<i32>,

    /// Potency as percentage (for heal/max HP potions)
    #[serde(default)]
    pub potency_percent: Option<i32>,

    /// Percentage potency when upgraded
    #[serde(default)]
    pub potency_percent_upgraded: Option<i32>,
}

impl PotionDefinition {
    /// Get the effective potency, considering Sacred Bark upgrade.
    pub fn get_potency(&self, has_sacred_bark: bool) -> i32 {
        if has_sacred_bark {
            self.potency_upgraded.unwrap_or(self.potency.unwrap_or(0))
        } else {
            self.potency.unwrap_or(0)
        }
    }

    /// Get the effective percentage potency, considering Sacred Bark upgrade.
    pub fn get_potency_percent(&self, has_sacred_bark: bool) -> i32 {
        if has_sacred_bark {
            self.potency_percent_upgraded
                .unwrap_or(self.potency_percent.unwrap_or(0))
        } else {
            self.potency_percent.unwrap_or(0)
        }
    }

    /// Check if this potion requires a target selection.
    pub fn requires_target(&self) -> bool {
        matches!(self.target, PotionTarget::Enemy)
    }

    /// Check if this potion can be used by the given character class.
    pub fn can_use(&self, character: PotionClass) -> bool {
        match self.potion_class {
            PotionClass::Any => true,
            class => class == character,
        }
    }
}

// ============================================================================
// Potion Library
// ============================================================================

/// A library of potion definitions loaded from JSON.
#[derive(Debug, Clone)]
pub struct PotionLibrary {
    potions: HashMap<String, PotionDefinition>,
}

impl PotionLibrary {
    /// Load potion definitions from a JSON file.
    ///
    /// # Arguments
    /// * `path` - Path to the potions.json file
    ///
    /// # Returns
    /// * `Ok(PotionLibrary)` - Successfully loaded library
    /// * `Err(PotionError)` - Error during loading
    ///
    /// # Example
    /// ```no_run
    /// use sts_sim::potions::PotionLibrary;
    ///
    /// let library = PotionLibrary::load("data/potions.json")?;
    /// let fire_potion = library.get("FirePotion")?;
    /// # Ok::<(), sts_sim::potions::PotionError>(())
    /// ```
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PotionError> {
        let path_str = path.as_ref().display().to_string();

        // Read file contents
        let contents = fs::read_to_string(&path).map_err(|e| PotionError::FileRead {
            path: path_str.clone(),
            source: e,
        })?;

        // Parse JSON array of potion definitions
        let potions_vec: Vec<PotionDefinition> =
            serde_json::from_str(&contents).map_err(|e| PotionError::JsonParse {
                path: path_str.clone(),
                source: e,
            })?;

        if potions_vec.is_empty() {
            return Err(PotionError::EmptyLibrary(path_str));
        }

        // Build HashMap for O(1) lookup by ID
        let potions: HashMap<String, PotionDefinition> = potions_vec
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();

        game_log!(
            "[Loader] Loaded {} potion definitions from '{}'",
            potions.len(),
            path_str
        );

        Ok(Self { potions })
    }

    /// Get a potion definition by ID.
    pub fn get(&self, id: &str) -> Result<&PotionDefinition, PotionError> {
        self.potions
            .get(id)
            .ok_or_else(|| PotionError::PotionNotFound(id.to_string()))
    }

    /// Get a potion definition by ID, returning None if not found.
    pub fn get_opt(&self, id: &str) -> Option<&PotionDefinition> {
        self.potions.get(id)
    }

    /// Check if a potion exists in the library.
    pub fn contains(&self, id: &str) -> bool {
        self.potions.contains_key(id)
    }

    /// Get the total number of potions in the library.
    pub fn len(&self) -> usize {
        self.potions.len()
    }

    /// Check if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.potions.is_empty()
    }

    /// Iterate over all potion definitions.
    pub fn iter(&self) -> impl Iterator<Item = &PotionDefinition> {
        self.potions.values()
    }

    /// Get all potion IDs.
    pub fn potion_ids(&self) -> impl Iterator<Item = &String> {
        self.potions.keys()
    }

    /// Filter potions by rarity.
    pub fn potions_of_rarity(&self, rarity: PotionRarity) -> Vec<&PotionDefinition> {
        self.potions
            .values()
            .filter(|p| p.rarity == rarity)
            .collect()
    }

    /// Filter potions by class (including Any).
    pub fn potions_for_class(&self, class: PotionClass) -> Vec<&PotionDefinition> {
        self.potions
            .values()
            .filter(|p| p.potion_class == PotionClass::Any || p.potion_class == class)
            .collect()
    }

    /// Get a random potion for the given class and rarity.
    pub fn random_potion(
        &self,
        class: PotionClass,
        rarity: PotionRarity,
        rng: &mut impl rand::Rng,
    ) -> Option<&PotionDefinition> {
        use rand::prelude::IndexedRandom;

        let candidates: Vec<_> = self
            .potions
            .values()
            .filter(|p| {
                p.rarity == rarity
                    && (p.potion_class == PotionClass::Any || p.potion_class == class)
            })
            .collect();

        candidates.choose(rng).copied()
    }
}

/// Load the default potion library from `data/potions.json`.
pub fn load_default() -> Result<PotionLibrary, PotionError> {
    PotionLibrary::load("data/potions.json")
}

// ============================================================================
// Potion Slots (Runtime State)
// ============================================================================

/// Maximum number of potion slots (3 base + 2 from Potion Belt).
pub const MAX_POTION_SLOTS: usize = 5;

/// Manages the player's potion slots during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotionSlots {
    /// Potion slots (None = empty slot)
    slots: Vec<Option<String>>,
}

impl Default for PotionSlots {
    fn default() -> Self {
        Self::new(3) // Default: 3 slots
    }
}

impl PotionSlots {
    /// Create potion slots with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
        }
    }

    /// Get the number of slots.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Get the number of filled slots.
    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Check if all slots are full.
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// Check if all slots are empty.
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Get the potion ID at the given slot index.
    pub fn get(&self, index: usize) -> Result<Option<&String>, PotionError> {
        self.slots
            .get(index)
            .map(|s| s.as_ref())
            .ok_or(PotionError::InvalidSlot(index))
    }

    /// Get a reference to all slots.
    pub fn slots(&self) -> &[Option<String>] {
        &self.slots
    }

    /// Add a potion to the first empty slot.
    ///
    /// Returns the slot index where the potion was added.
    pub fn add(&mut self, potion_id: String) -> Result<usize, PotionError> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(potion_id);
                return Ok(i);
            }
        }
        Err(PotionError::SlotsFull)
    }

    /// Remove and return the potion at the given slot.
    pub fn remove(&mut self, index: usize) -> Result<String, PotionError> {
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(PotionError::InvalidSlot(index))?;

        slot.take().ok_or(PotionError::EmptySlot(index))
    }

    /// Discard (remove without returning) the potion at the given slot.
    pub fn discard(&mut self, index: usize) -> Result<(), PotionError> {
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(PotionError::InvalidSlot(index))?;

        if slot.is_none() {
            return Err(PotionError::EmptySlot(index));
        }

        *slot = None;
        Ok(())
    }

    /// Set a potion at a specific slot (replaces any existing potion).
    pub fn set(&mut self, index: usize, potion_id: Option<String>) -> Result<(), PotionError> {
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(PotionError::InvalidSlot(index))?;

        *slot = potion_id;
        Ok(())
    }

    /// Find the slot index of a potion by ID (first occurrence).
    pub fn find(&self, potion_id: &str) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_ref().map_or(false, |id| id == potion_id))
    }

    /// Check if the player has a specific potion.
    pub fn has(&self, potion_id: &str) -> bool {
        self.find(potion_id).is_some()
    }

    /// Get indices of all empty slots.
    pub fn empty_slots(&self) -> Vec<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.is_none() { Some(i) } else { None })
            .collect()
    }

    /// Get indices of all filled slots.
    pub fn filled_slots(&self) -> Vec<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.is_some() { Some(i) } else { None })
            .collect()
    }

    /// Add empty potion slots (e.g., from Potion Belt relic).
    ///
    /// Respects the maximum cap of [`MAX_POTION_SLOTS`] (5).
    /// Returns the number of slots actually added.
    pub fn add_slots(&mut self, count: usize) -> usize {
        let current = self.slots.len();
        let new_capacity = (current + count).min(MAX_POTION_SLOTS);
        let added = new_capacity - current;
        for _ in 0..added {
            self.slots.push(None);
        }
        added
    }

    /// Clear all potion slots.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_potion_slots_basic() {
        let mut slots = PotionSlots::new(3);

        assert_eq!(slots.capacity(), 3);
        assert_eq!(slots.count(), 0);
        assert!(slots.is_empty());
        assert!(!slots.is_full());

        // Add potions
        assert_eq!(slots.add("FirePotion".to_string()).unwrap(), 0);
        assert_eq!(slots.add("BlockPotion".to_string()).unwrap(), 1);

        assert_eq!(slots.count(), 2);
        assert!(!slots.is_empty());
        assert!(!slots.is_full());

        // Check contents
        assert_eq!(slots.get(0).unwrap(), Some(&"FirePotion".to_string()));
        assert_eq!(slots.get(1).unwrap(), Some(&"BlockPotion".to_string()));
        assert_eq!(slots.get(2).unwrap(), None);

        // Remove potion
        assert_eq!(slots.remove(0).unwrap(), "FirePotion");
        assert_eq!(slots.get(0).unwrap(), None);
        assert_eq!(slots.count(), 1);
    }

    #[test]
    fn test_potion_slots_full() {
        let mut slots = PotionSlots::new(2);

        slots.add("P1".to_string()).unwrap();
        slots.add("P2".to_string()).unwrap();

        assert!(slots.is_full());
        assert!(matches!(
            slots.add("P3".to_string()),
            Err(PotionError::SlotsFull)
        ));
    }

    #[test]
    fn test_potion_slots_find() {
        let mut slots = PotionSlots::new(3);

        slots.add("FirePotion".to_string()).unwrap();
        slots.add("BlockPotion".to_string()).unwrap();

        assert_eq!(slots.find("FirePotion"), Some(0));
        assert_eq!(slots.find("BlockPotion"), Some(1));
        assert_eq!(slots.find("Unknown"), None);

        assert!(slots.has("FirePotion"));
        assert!(!slots.has("Unknown"));
    }

    #[test]
    fn test_potion_definition_helpers() {
        let potion = PotionDefinition {
            id: "FirePotion".to_string(),
            name: "Fire Potion".to_string(),
            rarity: PotionRarity::Common,
            potion_class: PotionClass::Any,
            target: PotionTarget::Enemy,
            description: "Deal 20 damage.".to_string(),
            command_hint: PotionCommand::DealDamage,
            potency: Some(20),
            potency_upgraded: Some(40),
            potency_percent: None,
            potency_percent_upgraded: None,
        };

        // Test potency calculation
        assert_eq!(potion.get_potency(false), 20);
        assert_eq!(potion.get_potency(true), 40);

        // Test target requirement
        assert!(potion.requires_target());

        // Test class restriction
        assert!(potion.can_use(PotionClass::Ironclad));
        assert!(potion.can_use(PotionClass::Silent));
    }

    #[test]
    fn test_class_restricted_potion() {
        let potion = PotionDefinition {
            id: "FocusPotion".to_string(),
            name: "Focus Potion".to_string(),
            rarity: PotionRarity::Common,
            potion_class: PotionClass::Defect,
            target: PotionTarget::Player,
            description: "Gain 2 Focus.".to_string(),
            command_hint: PotionCommand::GainFocus,
            potency: Some(2),
            potency_upgraded: Some(4),
            potency_percent: None,
            potency_percent_upgraded: None,
        };

        assert!(!potion.requires_target());
        assert!(potion.can_use(PotionClass::Defect));
        assert!(!potion.can_use(PotionClass::Ironclad));
        assert!(!potion.can_use(PotionClass::Silent));
    }

    #[test]
    fn test_load_potions_json() {
        // This test verifies that data/potions.json loads correctly
        let library = PotionLibrary::load("data/potions.json").expect("Failed to load potions.json");

        // Check expected count
        assert_eq!(library.len(), 42, "Expected 42 potions in library");

        // Verify some specific potions
        let fire = library.get("FirePotion").expect("FirePotion not found");
        assert_eq!(fire.name, "Fire Potion");
        assert_eq!(fire.rarity, PotionRarity::Common);
        assert_eq!(fire.target, PotionTarget::Enemy);
        assert_eq!(fire.command_hint, PotionCommand::DealDamage);
        assert_eq!(fire.potency, Some(20));
        assert_eq!(fire.potency_upgraded, Some(40));

        let focus = library.get("FocusPotion").expect("FocusPotion not found");
        assert_eq!(focus.name, "Focus Potion");
        assert_eq!(focus.potion_class, PotionClass::Defect);
        assert_eq!(focus.command_hint, PotionCommand::GainFocus);

        let fairy = library.get("FairyinaBottle").expect("FairyinaBottle not found");
        assert_eq!(fairy.rarity, PotionRarity::Rare);
        assert_eq!(fairy.command_hint, PotionCommand::FairyRevive);
        assert_eq!(fairy.potency_percent, Some(30));

        // Test filtering by rarity
        let common_potions = library.potions_of_rarity(PotionRarity::Common);
        let uncommon_potions = library.potions_of_rarity(PotionRarity::Uncommon);
        let rare_potions = library.potions_of_rarity(PotionRarity::Rare);

        assert!(!common_potions.is_empty(), "Should have common potions");
        assert!(!uncommon_potions.is_empty(), "Should have uncommon potions");
        assert!(!rare_potions.is_empty(), "Should have rare potions");

        // Test filtering by class
        let defect_potions = library.potions_for_class(PotionClass::Defect);
        assert!(
            defect_potions.iter().any(|p| p.id == "FocusPotion"),
            "Defect potions should include FocusPotion"
        );
        assert!(
            defect_potions.iter().any(|p| p.id == "FirePotion"),
            "Defect potions should include common potions like FirePotion"
        );
    }
}
