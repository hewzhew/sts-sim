//! Act Configuration System for Slay the Spire.
//!
//! This module provides a unified API for all Act-related configuration:
//! - Monster encounter pools (weak, strong, elite, boss)
//! - Event pools (regular and shrine) - delegates to events.rs
//! - Act-specific mechanics (upgraded card chances, healing, etc.)
//!
//! # Architecture
//! - Monster data is stored here as static constants
//! - Event data is stored in events_final_master.json and accessed via events.rs
//! - This module provides the unified `ActConfig` API

use crate::events::{ActId, EventSelector, PoolType};
use rand::Rng;
use rand::distr::{Distribution, weighted::WeightedIndex};

// ============================================================================
// ACT CONFIGURATION
// ============================================================================

/// Complete configuration for an act.
#[derive(Debug, Clone)]
pub struct ActConfig {
    /// Act number (1, 2, 3, or 4).
    pub act_number: u8,
    /// Act identifier for event system.
    pub act_id: ActId,
    /// Number of initial "weak" encounters before switching to strong pool.
    pub weak_encounter_count: u8,
    /// Chance (0-100) that a card reward is upgraded.
    pub upgraded_card_chance: u8,
    /// HP healed on entering this act (percentage of max HP).
    pub heal_percent_on_enter: u8,
}

impl ActConfig {
    /// Get configuration for Act 1 - The Exordium
    pub fn act1() -> Self {
        Self {
            act_number: 1,
            act_id: ActId::Act1,
            weak_encounter_count: 3,
            upgraded_card_chance: 0,
            heal_percent_on_enter: 0,
        }
    }
    
    /// Get configuration for Act 2 - The City
    pub fn act2() -> Self {
        Self {
            act_number: 2,
            act_id: ActId::Act2,
            weak_encounter_count: 2,
            upgraded_card_chance: 25,
            heal_percent_on_enter: 75,
        }
    }
    
    /// Get configuration for Act 3 - The Beyond
    pub fn act3() -> Self {
        Self {
            act_number: 3,
            act_id: ActId::Act3,
            weak_encounter_count: 2,
            upgraded_card_chance: 50,
            heal_percent_on_enter: 75,
        }
    }
    
    /// Get configuration for Act 4 - The Ending (Heart route)
    pub fn act4() -> Self {
        Self {
            act_number: 4,
            act_id: ActId::Act3, // Act 4 uses Act 3's event system
            weak_encounter_count: 0,
            upgraded_card_chance: 50,
            heal_percent_on_enter: 0,
        }
    }
    
    /// Get configuration for a specific act number.
    pub fn for_act(act: u8) -> Self {
        match act {
            1 => Self::act1(),
            2 => Self::act2(),
            3 => Self::act3(),
            4 => Self::act4(),
            _ => Self::act1(),
        }
    }
    
    /// Get the weak encounter pool for this act.
    pub fn weak_pool(&self) -> &'static [WeightedEncounter] {
        match self.act_number {
            1 => ACT1_WEAK_POOL,
            2 => ACT2_WEAK_POOL,
            3 => ACT3_WEAK_POOL,
            _ => ACT1_WEAK_POOL,
        }
    }
    
    /// Get the strong encounter pool for this act.
    pub fn strong_pool(&self) -> &'static [WeightedEncounter] {
        match self.act_number {
            1 => ACT1_STRONG_POOL,
            2 => ACT2_STRONG_POOL,
            3 => ACT3_STRONG_POOL,
            _ => ACT1_STRONG_POOL,
        }
    }
    
    /// Get the elite encounter pool for this act.
    pub fn elite_pool(&self) -> &'static [WeightedEncounter] {
        match self.act_number {
            1 => ACT1_ELITE_POOL,
            2 => ACT2_ELITE_POOL,
            3 => ACT3_ELITE_POOL,
            4 => ACT4_ELITE_POOL,
            _ => ACT1_ELITE_POOL,
        }
    }
    
    /// Get the boss encounter pool for this act.
    pub fn boss_pool(&self) -> &'static [WeightedEncounter] {
        match self.act_number {
            1 => ACT1_BOSS_POOL,
            2 => ACT2_BOSS_POOL,
            3 => ACT3_BOSS_POOL,
            4 => ACT4_BOSS_POOL,
            _ => ACT1_BOSS_POOL,
        }
    }
    
    /// Get available regular events for this act (delegates to events.rs).
    pub fn regular_events(&self) -> Vec<&'static str> {
        let (regular, _) = EventSelector::get_events_for_act(self.act_id);
        regular
    }
    
    /// Get available shrine events for this act (delegates to events.rs).
    pub fn shrine_events(&self) -> Vec<&'static str> {
        let (_, shrine) = EventSelector::get_events_for_act(self.act_id);
        shrine
    }
}

// ============================================================================
// WEIGHTED ENCOUNTER
// ============================================================================

/// A weighted encounter entry for probability-based selection.
#[derive(Debug, Clone, Copy)]
pub struct WeightedEncounter {
    /// Encounter identifier (matches monster definition ID).
    pub id: &'static str,
    /// Weight for selection (will be normalized against pool total).
    pub weight: f32,
}

impl WeightedEncounter {
    pub const fn new(id: &'static str, weight: f32) -> Self {
        Self { id, weight }
    }
}

/// Select a random encounter from a weighted pool.
pub fn select_from_weighted_pool<R: Rng>(
    rng: &mut R,
    pool: &[WeightedEncounter],
) -> Option<&'static str> {
    if pool.is_empty() {
        return None;
    }
    
    let weights: Vec<f32> = pool.iter().map(|e| e.weight).collect();
    
    match WeightedIndex::new(&weights) {
        Ok(dist) => {
            let idx = dist.sample(rng);
            Some(pool[idx].id)
        }
        Err(_) => {
            // Fallback to uniform if weights are invalid
            let idx = rng.random_range(0..pool.len());
            Some(pool[idx].id)
        }
    }
}

// ============================================================================
// ACT 1 - THE EXORDIUM
// ============================================================================

/// Act 1 weak encounters (first 3 combats).
/// Source: act_1_exo.txt - "First Three Combat Encounters"
pub const ACT1_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Cultist", 2.0),
    WeightedEncounter::new("Jaw Worm", 2.0),
    WeightedEncounter::new("2 Louse", 2.0),
    WeightedEncounter::new("Small Slimes", 2.0),
];

/// Act 1 strong encounters (combats 4+).
/// Source: act_1_exo.txt - "Remaining Combat Encounters"
pub const ACT1_STRONG_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Gremlin Gang", 1.0),
    WeightedEncounter::new("Large Slime", 2.0),
    WeightedEncounter::new("Lots of Slimes", 1.0),
    WeightedEncounter::new("Blue Slaver", 2.0),
    WeightedEncounter::new("Red Slaver", 1.0),
    WeightedEncounter::new("3 Louse", 2.0),
    WeightedEncounter::new("2 Fungi Beasts", 2.0),
    WeightedEncounter::new("Exordium Thugs", 1.5),
    WeightedEncounter::new("Exordium Wildlife", 1.5),
    WeightedEncounter::new("Looter", 2.0),
];

/// Act 1 elite encounters.
pub const ACT1_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Gremlin Nob", 1.0),
    WeightedEncounter::new("Lagavulin", 1.0),
    WeightedEncounter::new("3 Sentries", 1.0),
];

/// Act 1 boss encounters.
pub const ACT1_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("The Guardian", 1.0),
    WeightedEncounter::new("Hexaghost", 1.0),
    WeightedEncounter::new("Slime Boss", 1.0),
];

// ============================================================================
// ACT 2 - THE CITY
// ============================================================================

/// Act 2 weak encounters (first 2 combats).
/// Source: act_2_city.txt - "First Two Combat Encounters"
pub const ACT2_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Spheric Guardian", 2.0),
    WeightedEncounter::new("Chosen", 2.0),
    WeightedEncounter::new("Shell Parasite", 2.0),
    WeightedEncounter::new("3 Byrds", 2.0),
    WeightedEncounter::new("2 Thieves", 2.0),
];

/// Act 2 strong encounters (combats 3+).
/// Source: act_2_city.txt - "Remaining Combat Encounters"
pub const ACT2_STRONG_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Chosen and Byrds", 2.0),
    WeightedEncounter::new("Cultist and Chosen", 3.0),
    WeightedEncounter::new("Sentry and Sphere", 2.0),
    WeightedEncounter::new("Snake Plant", 6.0),
    WeightedEncounter::new("Snecko", 4.0),
    WeightedEncounter::new("Centurion and Healer", 6.0),
    WeightedEncounter::new("3 Cultists", 3.0),
    WeightedEncounter::new("Shelled Parasite and Fungi", 3.0),
];

/// Act 2 elite encounters.
pub const ACT2_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Gremlin Leader", 1.0),
    WeightedEncounter::new("Slavers", 1.0),
    WeightedEncounter::new("Book of Stabbing", 1.0),
];

/// Act 2 boss encounters.
pub const ACT2_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("The Champ", 1.0),
    WeightedEncounter::new("Bronze Automaton", 1.0),
    WeightedEncounter::new("The Collector", 1.0),
];

// ============================================================================
// ACT 3 - THE BEYOND
// ============================================================================

/// Act 3 weak encounters (first 2 combats).
/// Source: act_3_beyond.txt - "First Two Combat Encounters"
pub const ACT3_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("3 Darklings", 2.0),
    WeightedEncounter::new("Orb Walker", 2.0),
    WeightedEncounter::new("3 Shapes", 2.0),
];

/// Act 3 strong encounters (combats 3+).
/// Source: act_3_beyond.txt - "Remaining Combat Encounters"
pub const ACT3_STRONG_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("4 Shapes", 1.0),
    WeightedEncounter::new("Maw", 1.0),
    WeightedEncounter::new("Spheric Guardian and 2 Shapes", 1.0),
    WeightedEncounter::new("3 Darklings", 1.0),
    WeightedEncounter::new("Spire Growth", 1.0),
    WeightedEncounter::new("Transient", 1.0),
    WeightedEncounter::new("Jaw Worm Horde", 1.0),
    WeightedEncounter::new("Writhing Mass", 1.0),
];

/// Act 3 elite encounters.
pub const ACT3_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Giant Head", 2.0),
    WeightedEncounter::new("Nemesis", 2.0),
    WeightedEncounter::new("Reptomancer", 2.0),
];

/// Act 3 boss encounters.
pub const ACT3_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Awakened One", 1.0),
    WeightedEncounter::new("Donu and Deca", 1.0),
    WeightedEncounter::new("Time Eater", 1.0),
];

// ============================================================================
// ACT 4 - THE ENDING (HEART ROUTE)
// ============================================================================

/// Act 4 elite - Spire Shields/Spears before Heart.
pub const ACT4_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Spire Shield and Spire Spear", 1.0),
];

/// Act 4 boss - The Corrupt Heart.
pub const ACT4_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Corrupt Heart", 1.0),
];

// ============================================================================
// ENCOUNTER SELECTION API
// ============================================================================

/// Type of encounter room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterType {
    /// Normal monster fight (weak or strong pool based on combat count).
    Normal,
    /// Elite monster fight.
    Elite,
    /// Boss fight.
    Boss,
}

/// Result of encounter selection.
#[derive(Debug, Clone)]
pub struct EncounterResult {
    /// The encounter ID (e.g., "Cultist", "Gremlin Nob").
    pub encounter_id: String,
    /// Type of encounter.
    pub encounter_type: EncounterType,
}

impl EncounterResult {
    pub fn normal(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            encounter_type: EncounterType::Normal,
        }
    }
    
    pub fn elite(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            encounter_type: EncounterType::Elite,
        }
    }
    
    pub fn boss(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            encounter_type: EncounterType::Boss,
        }
    }
    
    pub fn is_elite(&self) -> bool {
        self.encounter_type == EncounterType::Elite
    }
    
    pub fn is_boss(&self) -> bool {
        self.encounter_type == EncounterType::Boss
    }
}

/// Select a random encounter for a given act and encounter type.
/// 
/// # Arguments
/// * `rng` - Random number generator
/// * `act` - Act number (1-4)
/// * `encounter_type` - Type of encounter (Normal, Elite, Boss)
/// * `combat_count` - Number of combats completed in this act (for Normal only)
pub fn select_encounter<R: Rng>(
    rng: &mut R,
    act: u8,
    encounter_type: EncounterType,
    combat_count: u8,
) -> Option<EncounterResult> {
    let config = ActConfig::for_act(act);
    
    match encounter_type {
        EncounterType::Normal => {
            let pool = if combat_count < config.weak_encounter_count {
                config.weak_pool()
            } else {
                config.strong_pool()
            };
            select_from_weighted_pool(rng, pool).map(EncounterResult::normal)
        }
        EncounterType::Elite => {
            select_from_weighted_pool(rng, config.elite_pool()).map(EncounterResult::elite)
        }
        EncounterType::Boss => {
            select_from_weighted_pool(rng, config.boss_pool()).map(EncounterResult::boss)
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256StarStar;

    #[test]
    fn test_act_configs() {
        let act1 = ActConfig::act1();
        assert_eq!(act1.weak_encounter_count, 3);
        assert_eq!(act1.upgraded_card_chance, 0);
        
        let act2 = ActConfig::act2();
        assert_eq!(act2.weak_encounter_count, 2);
        assert_eq!(act2.upgraded_card_chance, 25);
        
        let act3 = ActConfig::act3();
        assert_eq!(act3.weak_encounter_count, 2);
        assert_eq!(act3.upgraded_card_chance, 50);
    }
    
    #[test]
    fn test_pool_sizes() {
        // Verify pool sizes match txt file counts
        assert_eq!(ACT1_WEAK_POOL.len(), 4, "Act 1 weak pool should have 4 encounters");
        assert_eq!(ACT1_STRONG_POOL.len(), 10, "Act 1 strong pool should have 10 encounters");
        assert_eq!(ACT1_ELITE_POOL.len(), 3, "Act 1 elite pool should have 3 encounters");
        assert_eq!(ACT1_BOSS_POOL.len(), 3, "Act 1 boss pool should have 3 bosses");
        
        assert_eq!(ACT2_WEAK_POOL.len(), 5, "Act 2 weak pool should have 5 encounters");
        assert_eq!(ACT2_STRONG_POOL.len(), 8, "Act 2 strong pool should have 8 encounters");
        assert_eq!(ACT2_ELITE_POOL.len(), 3, "Act 2 elite pool should have 3 encounters");
        assert_eq!(ACT2_BOSS_POOL.len(), 3, "Act 2 boss pool should have 3 bosses");
        
        assert_eq!(ACT3_WEAK_POOL.len(), 3, "Act 3 weak pool should have 3 encounters");
        assert_eq!(ACT3_STRONG_POOL.len(), 8, "Act 3 strong pool should have 8 encounters");
        assert_eq!(ACT3_ELITE_POOL.len(), 3, "Act 3 elite pool should have 3 encounters");
        assert_eq!(ACT3_BOSS_POOL.len(), 3, "Act 3 boss pool should have 3 bosses");
    }
    
    #[test]
    fn test_weak_vs_strong_selection() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        // Act 1: combats 0-2 should use weak pool
        for combat in 0..3 {
            let result = select_encounter(&mut rng, 1, EncounterType::Normal, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            
            let weak_ids: Vec<&str> = ACT1_WEAK_POOL.iter().map(|e| e.id).collect();
            assert!(weak_ids.contains(&enc.encounter_id.as_str()),
                "Combat {} should use weak pool, got {}", combat, enc.encounter_id);
        }
        
        // Act 1: combats 3+ should use strong pool
        for combat in 3..8 {
            let result = select_encounter(&mut rng, 1, EncounterType::Normal, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            
            let strong_ids: Vec<&str> = ACT1_STRONG_POOL.iter().map(|e| e.id).collect();
            assert!(strong_ids.contains(&enc.encounter_id.as_str()),
                "Combat {} should use strong pool, got {}", combat, enc.encounter_id);
        }
    }
    
    #[test]
    fn test_act2_weak_count() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(123);
        
        // Act 2: combats 0-1 should use weak pool
        for combat in 0..2 {
            let result = select_encounter(&mut rng, 2, EncounterType::Normal, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            
            let weak_ids: Vec<&str> = ACT2_WEAK_POOL.iter().map(|e| e.id).collect();
            assert!(weak_ids.contains(&enc.encounter_id.as_str()),
                "Act 2 combat {} should use weak pool, got {}", combat, enc.encounter_id);
        }
        
        // Act 2: combat 2+ should use strong pool
        let result = select_encounter(&mut rng, 2, EncounterType::Normal, 2);
        let enc = result.unwrap();
        let strong_ids: Vec<&str> = ACT2_STRONG_POOL.iter().map(|e| e.id).collect();
        assert!(strong_ids.contains(&enc.encounter_id.as_str()),
            "Act 2 combat 2 should use strong pool, got {}", enc.encounter_id);
    }
    
    #[test]
    fn test_elite_selection() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(456);
        
        for act in 1..=3 {
            let result = select_encounter(&mut rng, act, EncounterType::Elite, 0);
            assert!(result.is_some(), "Act {} should have elites", act);
            assert!(result.as_ref().unwrap().is_elite());
        }
    }
    
    #[test]
    fn test_boss_selection() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(789);
        
        for act in 1..=4 {
            let result = select_encounter(&mut rng, act, EncounterType::Boss, 0);
            assert!(result.is_some(), "Act {} should have a boss", act);
            assert!(result.as_ref().unwrap().is_boss());
        }
    }
    
    #[test]
    fn test_event_integration() {
        let act1 = ActConfig::act1();
        let regular = act1.regular_events();
        let shrine = act1.shrine_events();
        
        assert!(!regular.is_empty(), "Act 1 should have regular events");
        assert!(!shrine.is_empty(), "Act 1 should have shrine events");
        
        // Verify specific events exist
        assert!(regular.contains(&"big_fish"), "Act 1 should have big_fish event");
        assert!(shrine.contains(&"bonfire_spirits"), "Act 1 should have bonfire_spirits shrine");
    }
}
