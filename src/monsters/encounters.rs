//! Dungeon encounter system for Slay the Spire.
//!
//! This module handles monster encounter selection based on act, floor, and room type.
//! Implements the official spawn logic with weighted probabilities per act.
//!
//! # Encounter Rules
//! 
//! ## Act 1 (The Exordium)
//! - First 3 combats: Weak pool (Cultist, Jaw Worm, 2 Louse, Small Slimes)
//! - Remaining combats: Strong pool (weighted selection)
//! - Upgraded card chance: 0%
//!
//! ## Act 2 (The City)  
//! - First 2 combats: Weak pool
//! - Remaining combats: Strong pool (weighted selection)
//! - Upgraded card chance: 25%
//!
//! ## Act 3 (The Beyond)
//! - First 2 combats: Weak pool
//! - Remaining combats: Strong pool (weighted selection)
//! - Upgraded card chance: 50%

use rand::Rng;
use rand::distr::{Distribution, weighted::WeightedIndex};
use rand_xoshiro::Xoshiro256StarStar;

use crate::map::RoomType;

// ============================================================================
// Act Configuration
// ============================================================================

/// Configuration for each act's mechanics.
#[derive(Debug, Clone, Copy)]
pub struct ActConfig {
    /// Number of initial "weak" encounters before switching to strong pool.
    pub weak_encounter_count: u8,
    /// Chance (0-100) that a card reward is upgraded.
    pub upgraded_card_chance: u8,
    /// HP healed on entering this act (percentage of max HP, affected by Ascension).
    pub heal_percent_on_enter: u8,
}

impl ActConfig {
    /// Act 1 - The Exordium
    pub const ACT1: Self = Self {
        weak_encounter_count: 3,
        upgraded_card_chance: 0,
        heal_percent_on_enter: 0, // No heal entering Act 1 (start of run)
    };
    
    /// Act 2 - The City
    pub const ACT2: Self = Self {
        weak_encounter_count: 2,
        upgraded_card_chance: 25,
        heal_percent_on_enter: 75, // 75% heal at Act 2 (Ascension affects this)
    };
    
    /// Act 3 - The Beyond
    pub const ACT3: Self = Self {
        weak_encounter_count: 2,
        upgraded_card_chance: 50,
        heal_percent_on_enter: 75,
    };
    
    /// Get config for a specific act.
    pub fn for_act(act: u8) -> Self {
        match act {
            1 => Self::ACT1,
            2 => Self::ACT2,
            3 => Self::ACT3,
            _ => Self::ACT1,
        }
    }
}

// ============================================================================
// Weighted Encounter Entry
// ============================================================================

/// A weighted encounter entry for probability-based selection.
#[derive(Debug, Clone, Copy)]
pub struct WeightedEncounter {
    pub id: &'static str,
    /// Weight for selection (can be fractional, will be normalized).
    pub weight: f32,
}

impl WeightedEncounter {
    pub const fn new(id: &'static str, weight: f32) -> Self {
        Self { id, weight }
    }
}

// ============================================================================
// Act 1 - The Exordium
// ============================================================================

/// Act 1 weak encounters (first 3 combats) - equal weights.
pub const ACT1_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Cultist", 2.0),
    WeightedEncounter::new("Jaw Worm", 2.0),
    WeightedEncounter::new("2 Louse", 2.0),
    WeightedEncounter::new("Small Slimes", 2.0),
];

/// Act 1 strong encounters (combats 4+) - official weighted probabilities.
pub const ACT1_STRONG_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Gremlin Gang", 1.0),
    WeightedEncounter::new("Large Slime", 2.0),
    WeightedEncounter::new("Lots of Slimes", 1.0),
    WeightedEncounter::new("Blue Slaver", 2.0),
    WeightedEncounter::new("Red Slaver", 1.0),
    WeightedEncounter::new("3 Louse", 2.0),
    WeightedEncounter::new("2 Fungi Beasts", 2.0),
    WeightedEncounter::new("Looter", 2.0),
    WeightedEncounter::new("Exordium Thugs", 1.5),
    WeightedEncounter::new("Exordium Wildlife", 1.5),
];

/// Act 1 elite encounters - equal weights.
pub const ACT1_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Gremlin Nob", 1.0),
    WeightedEncounter::new("Lagavulin", 1.0),
    WeightedEncounter::new("3 Sentries", 1.0),
];

/// Act 1 boss encounters - one is selected at random for the act.
pub const ACT1_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("The Guardian", 1.0),
    WeightedEncounter::new("Hexaghost", 1.0),
    WeightedEncounter::new("Slime Boss", 1.0),
];

// ============================================================================
// Act 2 - The City
// ============================================================================

/// Act 2 weak encounters (first 2 combats).
pub const ACT2_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Spheric Guardian", 2.0),
    WeightedEncounter::new("Chosen", 2.0),
    WeightedEncounter::new("Shell Parasite", 2.0),
    WeightedEncounter::new("3 Byrds", 2.0),
    WeightedEncounter::new("2 Thieves", 2.0),
];

/// Act 2 strong encounters (combats 3+).
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
// Act 3 - The Beyond
// ============================================================================

/// Act 3 weak encounters (first 2 combats).
pub const ACT3_WEAK_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("3 Darklings", 2.0),
    WeightedEncounter::new("Orb Walker", 2.0),
    WeightedEncounter::new("3 Shapes", 2.0),
];

/// Act 3 strong encounters (combats 3+).
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
// Act 4 - The Ending (Heart Route)
// ============================================================================

/// Act 4 elite encounters (The Spire Elites before Heart).
pub const ACT4_ELITE_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Spire Shield and Spire Spear", 1.0),
];

/// Act 4 boss - The Corrupt Heart.
pub const ACT4_BOSS_POOL: &[WeightedEncounter] = &[
    WeightedEncounter::new("Corrupt Heart", 1.0),
];

// ============================================================================
// Event Pools - DEPRECATED
// ============================================================================
// Event pool logic has been moved to src/events.rs with full condition checking.
// Use events::EventSelector for event selection.
// These constants are kept for backwards compatibility but should not be used.

#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT1_EVENTS: &[&str] = &[];
#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT1_SHRINES: &[&str] = &[];
#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT2_EVENTS: &[&str] = &[];
#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT2_SHRINES: &[&str] = &[];
#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT3_EVENTS: &[&str] = &[];
#[deprecated(note = "Use events::EventSelector instead")]
pub const ACT3_SHRINES: &[&str] = &[];

// ============================================================================
// Encounter Result
// ============================================================================

/// Result of encounter selection.
#[derive(Debug, Clone)]
pub struct EncounterResult {
    /// The encounter ID (e.g., "Cultist", "Gremlin Nob").
    pub encounter_id: String,
    /// Whether this is an elite fight.
    pub is_elite: bool,
    /// Whether this is a boss fight.
    pub is_boss: bool,
}

impl EncounterResult {
    /// Create a normal monster encounter.
    pub fn normal(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            is_elite: false,
            is_boss: false,
        }
    }
    
    /// Create an elite encounter.
    pub fn elite(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            is_elite: true,
            is_boss: false,
        }
    }
    
    /// Create a boss encounter.
    pub fn boss(id: &str) -> Self {
        Self {
            encounter_id: id.to_string(),
            is_elite: false,
            is_boss: true,
        }
    }
}

// ============================================================================
// Pool Accessors
// ============================================================================

/// Get the weak encounter pool for an act.
fn get_weak_pool(act: u8) -> &'static [WeightedEncounter] {
    match act {
        1 => ACT1_WEAK_POOL,
        2 => ACT2_WEAK_POOL,
        3 => ACT3_WEAK_POOL,
        _ => ACT1_WEAK_POOL,
    }
}

/// Get the strong encounter pool for an act.
fn get_strong_pool(act: u8) -> &'static [WeightedEncounter] {
    match act {
        1 => ACT1_STRONG_POOL,
        2 => ACT2_STRONG_POOL,
        3 => ACT3_STRONG_POOL,
        _ => ACT1_STRONG_POOL,
    }
}

/// Get the elite encounter pool for an act.
fn get_elite_pool(act: u8) -> &'static [WeightedEncounter] {
    match act {
        1 => ACT1_ELITE_POOL,
        2 => ACT2_ELITE_POOL,
        3 => ACT3_ELITE_POOL,
        4 => ACT4_ELITE_POOL,
        _ => ACT1_ELITE_POOL,
    }
}

/// Get the boss encounter pool for an act.
fn get_boss_pool(act: u8) -> &'static [WeightedEncounter] {
    match act {
        1 => ACT1_BOSS_POOL,
        2 => ACT2_BOSS_POOL,
        3 => ACT3_BOSS_POOL,
        4 => ACT4_BOSS_POOL,
        _ => ACT1_BOSS_POOL,
    }
}

// ============================================================================
// Weighted Selection
// ============================================================================

/// Select a random encounter from a weighted pool.
fn select_from_weighted_pool(
    rng: &mut Xoshiro256StarStar,
    pool: &[WeightedEncounter],
) -> Option<&'static str> {
    if pool.is_empty() {
        return None;
    }
    
    // Extract weights
    let weights: Vec<f32> = pool.iter().map(|e| e.weight).collect();
    
    // Create weighted distribution
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
// Main Selection API
// ============================================================================

/// Select a random encounter based on act, room type, and combat count.
/// 
/// # Arguments
/// * `rng` - The seeded RNG for deterministic selection.
/// * `act` - Current act (1, 2, 3, or 4).
/// * `room_type` - The type of room (Monster, MonsterElite, Boss, etc.).
/// * `combat_count` - Number of combats completed in this act (0-indexed).
///
/// # Returns
/// An `EncounterResult` with the selected encounter ID and type flags.
/// 
/// # Combat Count Logic
/// - Act 1: combats 0-2 use weak pool, combats 3+ use strong pool
/// - Act 2: combats 0-1 use weak pool, combats 2+ use strong pool
/// - Act 3: combats 0-1 use weak pool, combats 2+ use strong pool
pub fn get_random_encounter(
    rng: &mut Xoshiro256StarStar,
    act: u8,
    room_type: RoomType,
    combat_count: u8,
) -> Option<EncounterResult> {
    match room_type {
        RoomType::Monster => {
            let config = ActConfig::for_act(act);
            
            // Choose weak or strong based on combat count
            let pool = if combat_count < config.weak_encounter_count {
                get_weak_pool(act)
            } else {
                get_strong_pool(act)
            };
            
            select_from_weighted_pool(rng, pool)
                .map(EncounterResult::normal)
        }
        
        RoomType::MonsterElite => {
            let pool = get_elite_pool(act);
            select_from_weighted_pool(rng, pool)
                .map(EncounterResult::elite)
        }
        
        RoomType::Boss => {
            let pool = get_boss_pool(act);
            select_from_weighted_pool(rng, pool)
                .map(EncounterResult::boss)
        }
        
        // Non-combat rooms
        RoomType::Rest | RoomType::Shop | RoomType::Treasure | RoomType::Event => None,
    }
}

/// Legacy API - uses floor number to estimate combat count.
/// Prefer `get_random_encounter` with explicit combat_count.
#[allow(dead_code)]
pub fn get_random_encounter_by_floor(
    rng: &mut Xoshiro256StarStar,
    act: u8,
    floor: u8,
    room_type: RoomType,
) -> Option<EncounterResult> {
    // Estimate combat count from floor (rough heuristic)
    // This is less accurate than tracking actual combats
    let estimated_combat_count = floor.saturating_sub(1);
    get_random_encounter(rng, act, room_type, estimated_combat_count)
}

/// Get all valid encounters for a given act and room type (for testing/enumeration).
#[allow(dead_code)]
pub fn get_encounter_pool(act: u8, room_type: RoomType, combat_count: u8) -> Vec<&'static str> {
    match room_type {
        RoomType::Monster => {
            let config = ActConfig::for_act(act);
            let pool = if combat_count < config.weak_encounter_count {
                get_weak_pool(act)
            } else {
                get_strong_pool(act)
            };
            pool.iter().map(|e| e.id).collect()
        }
        RoomType::MonsterElite => get_elite_pool(act).iter().map(|e| e.id).collect(),
        RoomType::Boss => get_boss_pool(act).iter().map(|e| e.id).collect(),
        _ => Vec::new(),
    }
}

/// Get the upgraded card chance for an act.
pub fn get_upgraded_card_chance(act: u8) -> u8 {
    ActConfig::for_act(act).upgraded_card_chance
}

/// Get the heal percentage when entering an act.
/// Note: This is affected by Ascension level in the actual game.
#[allow(dead_code)]
pub fn get_act_entry_heal_percent(act: u8, _ascension: u8) -> u8 {
    // TODO: Implement Ascension modifiers
    // Ascension 5+: 75% heal
    // Ascension 13+: Less healing (varies)
    ActConfig::for_act(act).heal_percent_on_enter
}

// ============================================================================
// Act Progression System
// ============================================================================

use crate::state::GameState;
use crate::map::{generate_map, SimpleMap};

/// Result of advancing to the next act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActTransitionResult {
    /// Successfully advanced to a new act.
    AdvancedToAct(u8),
    /// Player has completed all acts (victory!).
    Victory,
    /// Cannot advance (not at boss, or invalid state).
    CannotAdvance { reason: String },
}

/// Heal percentage by act and ascension level.
/// 
/// Standard game rules:
/// - Act 2/3: 75% heal (or less with ascension)
/// - Ascension 5+: Only 75% instead of full heal
/// - Ascension 15+: Boss heals fully
fn get_act_transition_heal_percent(act: u8, ascension: u8) -> u8 {
    if act == 1 {
        return 0; // No heal entering Act 1 (game start)
    }
    
    // Base heal at Act 2/3 entry
    if ascension >= 15 {
        // Ascension 15+: Reduced healing
        65
    } else if ascension >= 5 {
        // Ascension 5-14: Reduced healing  
        75
    } else {
        // Ascension 0-4: Full heal (100% in original, but we use 75% as baseline)
        75
    }
}

/// Advance the game to the next act.
/// 
/// This function should be called after the boss chest screen is dismissed.
/// It handles:
/// 1. Incrementing the act number
/// 2. Generating a new map for the new act
/// 3. Resetting combat count (for weak/strong pool selection)
/// 4. Healing the player (based on ascension rules)
/// 5. Resetting act-specific counters
/// 
/// # Returns
/// An `ActTransitionResult` indicating the outcome.
pub fn advance_act(state: &mut GameState) -> ActTransitionResult {
    // Validate: boss must be defeated
    if !state.boss_defeated {
        return ActTransitionResult::CannotAdvance {
            reason: "Boss has not been defeated".to_string(),
        };
    }
    
    // Increment act
    let next_act = state.act + 1;
    
    // Check for victory (completed Act 3, or Act 4 if Heart route)
    if next_act > 3 {
        // TODO: Add Act 4 (Heart) support later
        // For now, Act 3 boss = victory
        state.screen = crate::state::GamePhase::GameOver;
        return ActTransitionResult::Victory;
    }
    
    // Update act number
    state.act = next_act;
    
    // Generate new map for the new act
    let is_ascension_zero = state.ascension_level == 0;
    let full_map = generate_map(state.run_seed as i64, state.act, is_ascension_zero);
    state.map = Some(SimpleMap::from_map(&full_map));
    
    // Reset map position (start at map selection)
    state.current_map_node = None;
    
    // Reset combat count for new act (affects weak/strong pool selection)
    state.combat_count = 0;
    
    // Reset act-specific counters
    state.potion_drop_chance = 40;  // Reset potion chance
    state.rare_card_offset = -5;    // Reset rare card offset
    
    // Heal player based on ascension rules
    let heal_percent = get_act_transition_heal_percent(state.act, state.ascension_level);
    if heal_percent > 0 {
        let heal_amount = (state.player.max_hp as f32 * heal_percent as f32 / 100.0).floor() as i32;
        let new_hp = (state.player.current_hp + heal_amount).min(state.player.max_hp);
        state.player.current_hp = new_hp;
    }
    
    // Clear boss_defeated flag for the new act
    state.boss_defeated = false;
    
    // Transition to Map screen
    state.screen = crate::state::GamePhase::Map;
    
    ActTransitionResult::AdvancedToAct(state.act)
}

/// Initialize a new run, setting up Act 1.
/// 
/// This is called at the start of a new game to:
/// 1. Generate the Act 1 map
/// 2. Initialize all run state
/// 
/// # Arguments
/// * `state` - The game state to initialize
/// * `seed` - The run seed for deterministic generation
/// * `ascension` - Ascension level (0-20)
pub fn initialize_run(state: &mut GameState, seed: u64, ascension: u8) {
    // Set run parameters
    state.run_seed = seed;
    state.ascension_level = ascension;
    state.act = 1;
    state.floor = 0;
    state.floor_num = 0;
    state.combat_count = 0;
    state.boss_defeated = false;
    
    // Generate Act 1 map
    let is_ascension_zero = ascension == 0;
    let full_map = generate_map(seed as i64, 1, is_ascension_zero);
    state.map = Some(SimpleMap::from_map(&full_map));
    
    // Reset map position
    state.current_map_node = None;
    
    // Initialize meta-scaling RNG counters
    state.potion_drop_chance = 40;
    state.rare_card_offset = -5;
    
    // Set screen to Map (Neow event would be handled separately)
    state.screen = crate::state::GamePhase::Map;
}

/// Mark the current act's boss as defeated.
/// 
/// This should be called when combat victory is achieved against a boss.
/// The game will then show the Boss Chest screen before advancing.
pub fn mark_boss_defeated(state: &mut GameState) {
    state.boss_defeated = true;
}

/// Check if the game has reached victory condition.
pub fn is_victory(state: &GameState) -> bool {
    // Victory = Act 3 boss defeated (or Act 4 Heart for heart route)
    state.act >= 3 && state.boss_defeated
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_act1_weak_encounter_first_3() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        // Combat 0, 1, 2 should all use weak pool
        for combat in 0..3 {
            let result = get_random_encounter(&mut rng, 1, RoomType::Monster, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            assert!(!enc.is_elite);
            assert!(!enc.is_boss);
            
            // Verify it's from weak pool
            let weak_ids: Vec<&str> = ACT1_WEAK_POOL.iter().map(|e| e.id).collect();
            assert!(weak_ids.contains(&enc.encounter_id.as_str()), 
                "Combat {} gave {} which is not in weak pool", combat, enc.encounter_id);
        }
    }

    #[test]
    fn test_act1_strong_encounter_after_3() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        // Combat 3+ should use strong pool
        for combat in 3..8 {
            let result = get_random_encounter(&mut rng, 1, RoomType::Monster, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            
            // Verify it's from strong pool
            let strong_ids: Vec<&str> = ACT1_STRONG_POOL.iter().map(|e| e.id).collect();
            assert!(strong_ids.contains(&enc.encounter_id.as_str()),
                "Combat {} gave {} which is not in strong pool", combat, enc.encounter_id);
        }
    }

    #[test]
    fn test_act2_weak_only_first_2() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        // Combat 0, 1 should use weak pool
        for combat in 0..2 {
            let result = get_random_encounter(&mut rng, 2, RoomType::Monster, combat);
            assert!(result.is_some());
            let enc = result.unwrap();
            
            let weak_ids: Vec<&str> = ACT2_WEAK_POOL.iter().map(|e| e.id).collect();
            assert!(weak_ids.contains(&enc.encounter_id.as_str()),
                "Combat {} gave {} which is not in weak pool", combat, enc.encounter_id);
        }
        
        // Combat 2+ should use strong pool
        let result = get_random_encounter(&mut rng, 2, RoomType::Monster, 2);
        assert!(result.is_some());
        let enc = result.unwrap();
        let strong_ids: Vec<&str> = ACT2_STRONG_POOL.iter().map(|e| e.id).collect();
        assert!(strong_ids.contains(&enc.encounter_id.as_str()));
    }

    #[test]
    fn test_act1_elite_encounter() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        let result = get_random_encounter(&mut rng, 1, RoomType::MonsterElite, 5);
        assert!(result.is_some());
        let enc = result.unwrap();
        assert!(enc.is_elite);
        assert!(!enc.is_boss);
        
        let elite_ids: Vec<&str> = ACT1_ELITE_POOL.iter().map(|e| e.id).collect();
        assert!(elite_ids.contains(&enc.encounter_id.as_str()));
    }

    #[test]
    fn test_act1_boss_encounter() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        let result = get_random_encounter(&mut rng, 1, RoomType::Boss, 10);
        assert!(result.is_some());
        let enc = result.unwrap();
        assert!(!enc.is_elite);
        assert!(enc.is_boss);
        
        let boss_ids: Vec<&str> = ACT1_BOSS_POOL.iter().map(|e| e.id).collect();
        assert!(boss_ids.contains(&enc.encounter_id.as_str()));
    }

    #[test]
    fn test_non_combat_rooms_return_none() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        assert!(get_random_encounter(&mut rng, 1, RoomType::Rest, 5).is_none());
        assert!(get_random_encounter(&mut rng, 1, RoomType::Shop, 5).is_none());
        assert!(get_random_encounter(&mut rng, 1, RoomType::Treasure, 5).is_none());
        assert!(get_random_encounter(&mut rng, 1, RoomType::Event, 5).is_none());
    }

    #[test]
    fn test_weighted_distribution() {
        // Run many selections and verify rough distribution
        let mut rng = Xoshiro256StarStar::seed_from_u64(12345);
        let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        
        for _ in 0..10000 {
            let result = get_random_encounter(&mut rng, 1, RoomType::Monster, 5);
            if let Some(enc) = result {
                *counts.entry(enc.encounter_id).or_insert(0) += 1;
            }
        }
        
        // Large Slime (weight 2) should appear ~2x as often as Gremlin Gang (weight 1)
        let large_slime = *counts.get("Large Slime").unwrap_or(&0);
        let gremlin_gang = *counts.get("Gremlin Gang").unwrap_or(&0);
        
        let ratio = large_slime as f32 / gremlin_gang.max(1) as f32;
        assert!(ratio > 1.5 && ratio < 2.5, "Expected ~2:1 ratio, got {}", ratio);
    }

    #[test]
    fn test_act_config() {
        assert_eq!(ActConfig::ACT1.weak_encounter_count, 3);
        assert_eq!(ActConfig::ACT2.weak_encounter_count, 2);
        assert_eq!(ActConfig::ACT3.weak_encounter_count, 2);
        
        assert_eq!(ActConfig::ACT1.upgraded_card_chance, 0);
        assert_eq!(ActConfig::ACT2.upgraded_card_chance, 25);
        assert_eq!(ActConfig::ACT3.upgraded_card_chance, 50);
    }

    #[test]
    fn test_deterministic_encounter_selection() {
        let mut rng1 = Xoshiro256StarStar::seed_from_u64(12345);
        let mut rng2 = Xoshiro256StarStar::seed_from_u64(12345);
        
        let enc1 = get_random_encounter(&mut rng1, 1, RoomType::Monster, 5).unwrap();
        let enc2 = get_random_encounter(&mut rng2, 1, RoomType::Monster, 5).unwrap();
        
        assert_eq!(enc1.encounter_id, enc2.encounter_id);
    }
    
    #[test]
    fn test_get_upgraded_card_chance() {
        assert_eq!(get_upgraded_card_chance(1), 0);
        assert_eq!(get_upgraded_card_chance(2), 25);
        assert_eq!(get_upgraded_card_chance(3), 50);
    }
    
    #[test]
    fn test_spawn_encounter_cultist() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Cultist");
        assert_eq!(monsters.len(), 1);
        assert_eq!(monsters[0].monster_id, "Cultist");
    }
    
    #[test]
    fn test_spawn_encounter_2_louse() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "2 Louse");
        assert_eq!(monsters.len(), 2);
        
        // Each should be Red or Green Louse
        for m in &monsters {
            assert!(m.monster_id == "Red Louse" || m.monster_id == "Green Louse",
                "Expected Red or Green Louse, got {}", m.monster_id);
        }
    }
    
    #[test]
    fn test_spawn_encounter_gremlin_gang() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Gremlin Gang");
        assert_eq!(monsters.len(), 4);
        
        // All should be gremlins
        let valid_gremlins = ["Fat Gremlin", "Sneaky Gremlin", "Mad Gremlin", "Shield Gremlin", "Gremlin Wizard"];
        for m in &monsters {
            assert!(valid_gremlins.contains(&m.monster_id.as_str()),
                "Expected a gremlin, got {}", m.monster_id);
        }
    }
    
    #[test]
    fn test_spawn_encounter_3_sentries() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "3 Sentries");
        assert_eq!(monsters.len(), 3);
        
        for m in &monsters {
            assert_eq!(m.monster_id, "Sentry");
        }
    }
    
    #[test]
    fn test_spawn_encounter_lots_of_slimes() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Lots of Slimes");
        assert_eq!(monsters.len(), 5);
        
        // 3x Spike Slime (S) and 2x Acid Slime (S)
        let spike_count = monsters.iter().filter(|m| m.monster_id == "Spike Slime (S)").count();
        let acid_count = monsters.iter().filter(|m| m.monster_id == "Acid Slime (S)").count();
        assert_eq!(spike_count, 3);
        assert_eq!(acid_count, 2);
    }
    
    // ========================================================================
    // Act Progression Tests
    // ========================================================================
    
    #[test]
    fn test_initialize_run() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        assert_eq!(state.act, 1);
        assert_eq!(state.ascension_level, 0);
        assert_eq!(state.run_seed, 12345);
        assert_eq!(state.combat_count, 0);
        assert!(!state.boss_defeated);
        assert!(state.map.is_some());
        assert_eq!(state.screen, crate::state::GamePhase::Map);
    }
    
    #[test]
    fn test_initialize_run_with_ascension() {
        let mut state = GameState::new(42);
        initialize_run(&mut state, 42, 15);
        
        assert_eq!(state.ascension_level, 15);
        assert!(state.map.is_some());
    }
    
    #[test]
    fn test_advance_act_without_boss_defeat() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Try to advance without defeating boss - should fail
        let result = advance_act(&mut state);
        assert!(matches!(result, ActTransitionResult::CannotAdvance { .. }));
        assert_eq!(state.act, 1);
    }
    
    #[test]
    fn test_advance_act_1_to_2() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Simulate boss defeat
        state.boss_defeated = true;
        state.combat_count = 5; // Had some combats in Act 1
        state.player.current_hp = 50; // Took some damage
        
        // Advance to Act 2
        let result = advance_act(&mut state);
        assert_eq!(result, ActTransitionResult::AdvancedToAct(2));
        assert_eq!(state.act, 2);
        assert_eq!(state.combat_count, 0); // Reset for new act
        assert!(!state.boss_defeated); // Reset for new act
        assert!(state.map.is_some());
        
        // Verify new map is for Act 2
        let map = state.map.as_ref().unwrap();
        assert_eq!(map.act, 2);
        
        // Player should have healed (75% of max HP added)
        assert!(state.player.current_hp > 50);
    }
    
    #[test]
    fn test_advance_act_2_to_3() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Advance to Act 2 first
        state.boss_defeated = true;
        advance_act(&mut state);
        
        // Now advance to Act 3
        state.boss_defeated = true;
        let result = advance_act(&mut state);
        
        assert_eq!(result, ActTransitionResult::AdvancedToAct(3));
        assert_eq!(state.act, 3);
        
        // Verify map is for Act 3
        let map = state.map.as_ref().unwrap();
        assert_eq!(map.act, 3);
    }
    
    #[test]
    fn test_advance_act_3_to_victory() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Advance through all acts
        state.boss_defeated = true;
        advance_act(&mut state); // Act 1 -> 2
        
        state.boss_defeated = true;
        advance_act(&mut state); // Act 2 -> 3
        
        state.boss_defeated = true;
        let result = advance_act(&mut state); // Act 3 -> Victory
        
        assert_eq!(result, ActTransitionResult::Victory);
        assert_eq!(state.screen, crate::state::GamePhase::GameOver);
    }
    
    #[test]
    fn test_is_victory() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Not victory at Act 1
        assert!(!is_victory(&state));
        
        // Advance to Act 3
        state.boss_defeated = true;
        advance_act(&mut state);
        state.boss_defeated = true;
        advance_act(&mut state);
        
        // Not victory yet (boss not defeated)
        assert!(!is_victory(&state));
        
        // Now defeat Act 3 boss
        state.boss_defeated = true;
        assert!(is_victory(&state));
    }
    
    #[test]
    fn test_act_transition_heal() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Set player HP to low
        state.player.max_hp = 80;
        state.player.current_hp = 20;
        
        // Defeat boss and advance
        state.boss_defeated = true;
        advance_act(&mut state);
        
        // Should have healed (75% of 80 = 60, so 20 + 60 = 80, capped at max)
        // Actually: heal_amount = floor(80 * 0.75) = 60, new_hp = min(20+60, 80) = 80
        assert_eq!(state.player.current_hp, 80);
    }
    
    #[test]
    fn test_act_transition_heal_with_high_hp() {
        let mut state = GameState::new(12345);
        initialize_run(&mut state, 12345, 0);
        
        // Set player HP close to max
        state.player.max_hp = 80;
        state.player.current_hp = 70;
        
        // Defeat boss and advance
        state.boss_defeated = true;
        advance_act(&mut state);
        
        // heal_amount = 60, new_hp = min(70+60, 80) = 80 (capped)
        assert_eq!(state.player.current_hp, 80);
    }
    
    // ========================================================================
    // Act 2 Encounter Spawning Tests
    // ========================================================================
    
    #[test]
    fn test_spawn_encounter_3_byrds() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "3 Byrds");
        assert_eq!(monsters.len(), 3);
        for m in &monsters {
            assert_eq!(m.monster_id, "Byrd");
        }
    }
    
    #[test]
    fn test_spawn_encounter_chosen_and_byrds() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Chosen and Byrds");
        assert_eq!(monsters.len(), 3);
        assert_eq!(monsters[0].monster_id, "Chosen");
        assert_eq!(monsters[1].monster_id, "Byrd");
        assert_eq!(monsters[2].monster_id, "Byrd");
    }
    
    #[test]
    fn test_spawn_encounter_slavers() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Slavers");
        assert_eq!(monsters.len(), 3);
        assert_eq!(monsters[0].monster_id, "Taskmaster");
        assert_eq!(monsters[1].monster_id, "Blue Slaver");
        assert_eq!(monsters[2].monster_id, "Red Slaver");
    }
    
    #[test]
    fn test_spawn_encounter_bronze_automaton() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Bronze Automaton");
        assert_eq!(monsters.len(), 3);
        assert_eq!(monsters[0].monster_id, "Bronze Automaton");
        assert_eq!(monsters[1].monster_id, "Bronze Orb");
        assert_eq!(monsters[2].monster_id, "Bronze Orb");
    }
    
    // ========================================================================
    // Act 3 Encounter Spawning Tests
    // ========================================================================
    
    #[test]
    fn test_spawn_encounter_3_darklings() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "3 Darklings");
        assert_eq!(monsters.len(), 3);
        for m in &monsters {
            assert_eq!(m.monster_id, "Darkling");
        }
    }
    
    #[test]
    fn test_spawn_encounter_4_shapes() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "4 Shapes");
        assert_eq!(monsters.len(), 4);
        
        let valid_shapes = ["Exploder", "Repulsor", "Spiker"];
        for m in &monsters {
            assert!(valid_shapes.contains(&m.monster_id.as_str()),
                "Expected a shape, got {}", m.monster_id);
        }
    }
    
    #[test]
    fn test_spawn_encounter_donu_and_deca() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Donu and Deca");
        assert_eq!(monsters.len(), 2);
        assert_eq!(monsters[0].monster_id, "Donu");
        assert_eq!(monsters[1].monster_id, "Deca");
    }
    
    #[test]
    fn test_spawn_encounter_reptomancer() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Reptomancer");
        assert_eq!(monsters.len(), 3);
        assert_eq!(monsters[0].monster_id, "Reptomancer");
        assert_eq!(monsters[1].monster_id, "Dagger");
        assert_eq!(monsters[2].monster_id, "Dagger");
    }
    
    // ========================================================================
    // Act 4 Encounter Spawning Tests
    // ========================================================================
    
    #[test]
    fn test_spawn_encounter_spire_elites() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Spire Shield and Spire Spear");
        assert_eq!(monsters.len(), 2);
        assert_eq!(monsters[0].monster_id, "Spire Shield");
        assert_eq!(monsters[1].monster_id, "Spire Spear");
    }
    
    #[test]
    fn test_spawn_encounter_corrupt_heart() {
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        let monsters = spawn_encounter(&mut rng, "Corrupt Heart");
        assert_eq!(monsters.len(), 1);
        assert_eq!(monsters[0].monster_id, "Corrupt Heart");
    }
}

// ============================================================================
// Encounter Spawning - Convert encounter IDs to monster lists
// ============================================================================

/// A single monster to spawn with its configuration.
#[derive(Debug, Clone)]
pub struct MonsterSpawn {
    /// Monster ID in the monster library (e.g., "Red Louse", "Cultist").
    pub monster_id: String,
    /// Optional override for HP (None = use default from definition).
    pub hp_override: Option<i32>,
}

impl MonsterSpawn {
    pub fn new(monster_id: &str) -> Self {
        Self {
            monster_id: monster_id.to_string(),
            hp_override: None,
        }
    }
    
    pub fn with_hp(monster_id: &str, hp: i32) -> Self {
        Self {
            monster_id: monster_id.to_string(),
            hp_override: Some(hp),
        }
    }
}

/// Convert an encounter ID to a list of monsters to spawn.
/// 
/// This function handles the game's encounter logic, including:
/// - Random color selection for Louse (50% Red / 50% Green)
/// - Random slime type selection for "Small Slimes"
/// - Multi-monster encounters like "2 Louse", "Gremlin Gang", etc.
/// 
/// # Arguments
/// * `rng` - Random number generator for variant selection
/// * `encounter_id` - The encounter ID from `get_random_encounter`
/// 
/// # Returns
/// A vector of `MonsterSpawn` entries to create
pub fn spawn_encounter(
    rng: &mut Xoshiro256StarStar,
    encounter_id: &str,
) -> Vec<MonsterSpawn> {
    match encounter_id {
        // ====================================================================
        // Act 1 - Weak Pool Encounters
        // ====================================================================
        
        "Cultist" => vec![MonsterSpawn::new("Cultist")],
        
        "Jaw Worm" => vec![MonsterSpawn::new("Jaw Worm")],
        
        "2 Louse" => {
            // Each Louse independently has 50% chance of either color
            let louse1 = if rng.random_bool(0.5) { "Red Louse" } else { "Green Louse" };
            let louse2 = if rng.random_bool(0.5) { "Red Louse" } else { "Green Louse" };
            vec![MonsterSpawn::new(louse1), MonsterSpawn::new(louse2)]
        }
        
        "Small Slimes" => {
            // Either (Spike Slime(M) + Acid Slime (S)) or (Acid Slime (M) + Spike Slime (S))
            if rng.random_bool(0.5) {
                vec![
                    MonsterSpawn::new("Spike Slime (M)"),
                    MonsterSpawn::new("Acid Slime (S)"),
                ]
            } else {
                vec![
                    MonsterSpawn::new("Acid Slime (M)"),
                    MonsterSpawn::new("Spike Slime (S)"),
                ]
            }
        }
        
        // ====================================================================
        // Act 1 - Strong Pool Encounters
        // ====================================================================
        
        "Gremlin Gang" => {
            // 4 randomly chosen from: Fat Gremlin(x2), Sneaky Gremlin(x2), Mad Gremlin(x2), 
            // Shield Gremlin(x1), Gremlin Wizard(x1)
            let pool = [
                "Fat Gremlin", "Fat Gremlin",
                "Sneaky Gremlin", "Sneaky Gremlin", 
                "Mad Gremlin", "Mad Gremlin",
                "Shield Gremlin",
                "Gremlin Wizard",
            ];
            
            // Sample 4 without replacement
            let mut indices: Vec<usize> = (0..pool.len()).collect();
            let mut result = Vec::with_capacity(4);
            for _ in 0..4 {
                let idx = rng.random_range(0..indices.len());
                let pool_idx = indices.remove(idx);
                result.push(MonsterSpawn::new(pool[pool_idx]));
            }
            result
        }
        
        "Large Slime" => {
            // Spike Slime (L) or Acid Slime (L)
            if rng.random_bool(0.5) {
                vec![MonsterSpawn::new("Spike Slime (L)")]
            } else {
                vec![MonsterSpawn::new("Acid Slime (L)")]
            }
        }
        
        "Lots of Slimes" => {
            // 3x Spike Slime (S) and 2x Acid Slime (S)
            vec![
                MonsterSpawn::new("Spike Slime (S)"),
                MonsterSpawn::new("Spike Slime (S)"),
                MonsterSpawn::new("Spike Slime (S)"),
                MonsterSpawn::new("Acid Slime (S)"),
                MonsterSpawn::new("Acid Slime (S)"),
            ]
        }
        
        "Blue Slaver" => vec![MonsterSpawn::new("Blue Slaver")],
        
        "Red Slaver" => vec![MonsterSpawn::new("Red Slaver")],
        
        "3 Louse" => {
            // 3x Louse (each independently has 50% chance of either color)
            (0..3).map(|_| {
                if rng.random_bool(0.5) {
                    MonsterSpawn::new("Red Louse")
                } else {
                    MonsterSpawn::new("Green Louse")
                }
            }).collect()
        }
        
        "2 Fungi Beasts" => {
            vec![
                MonsterSpawn::new("Fungi Beast"),
                MonsterSpawn::new("Fungi Beast"),
            ]
        }
        
        "Exordium Thugs" => {
            // First enemy: Louse (any) or Medium Slime (any)
            // Second enemy: Slaver (any), Cultist, or Looter
            let first = match rng.random_range(0..4) {
                0 => "Red Louse",
                1 => "Green Louse",
                2 => "Spike Slime (M)",
                _ => "Acid Slime (M)",
            };
            let second = match rng.random_range(0..4) {
                0 => "Blue Slaver",
                1 => "Red Slaver",
                2 => "Cultist",
                _ => "Looter",
            };
            vec![MonsterSpawn::new(first), MonsterSpawn::new(second)]
        }
        
        "Exordium Wildlife" => {
            // First enemy: Fungi Beast or Jaw Worm
            // Second enemy: Louse (any) or Medium Slime (any)
            let first = if rng.random_bool(0.5) {
                "Fungi Beast"
            } else {
                "Jaw Worm"
            };
            let second = match rng.random_range(0..4) {
                0 => "Red Louse",
                1 => "Green Louse",
                2 => "Spike Slime (M)",
                _ => "Acid Slime (M)",
            };
            vec![MonsterSpawn::new(first), MonsterSpawn::new(second)]
        }
        
        "Looter" => vec![MonsterSpawn::new("Looter")],
        
        // ====================================================================
        // Act 1 - Elite Encounters
        // ====================================================================
        
        "Gremlin Nob" => vec![MonsterSpawn::new("Gremlin Nob")],
        
        "Lagavulin" => vec![MonsterSpawn::new("Lagavulin")],
        
        "3 Sentries" => {
            vec![
                MonsterSpawn::new("Sentry"),
                MonsterSpawn::new("Sentry"),
                MonsterSpawn::new("Sentry"),
            ]
        }
        
        // ====================================================================
        // Act 1 - Boss Encounters
        // ====================================================================
        
        "The Guardian" => vec![MonsterSpawn::new("The Guardian")],
        
        "Hexaghost" => vec![MonsterSpawn::new("Hexaghost")],
        
        "Slime Boss" => vec![MonsterSpawn::new("Slime Boss")],
        
        // ====================================================================
        // Act 2 - Weak Pool Encounters (first 2 combats)
        // ====================================================================
        
        "Spheric Guardian" => vec![MonsterSpawn::new("Spheric Guardian")],
        
        "Chosen" => vec![MonsterSpawn::new("Chosen")],
        
        "Shell Parasite" | "Shelled Parasite" => vec![MonsterSpawn::new("Shelled Parasite")],
        
        "3 Byrds" => vec![
            MonsterSpawn::new("Byrd"),
            MonsterSpawn::new("Byrd"),
            MonsterSpawn::new("Byrd"),
        ],
        
        "2 Thieves" => vec![
            MonsterSpawn::new("Mugger"),
            MonsterSpawn::new("Mugger"),
        ],
        
        // ====================================================================
        // Act 2 - Strong Pool Encounters (combats 3+)
        // ====================================================================
        
        "Chosen and Byrds" => vec![
            MonsterSpawn::new("Chosen"),
            MonsterSpawn::new("Byrd"),
            MonsterSpawn::new("Byrd"),
        ],
        
        "Cultist and Chosen" => vec![
            MonsterSpawn::new("Cultist"),
            MonsterSpawn::new("Chosen"),
        ],
        
        "Sentry and Sphere" => vec![
            MonsterSpawn::new("Sentry"),
            MonsterSpawn::new("Spheric Guardian"),
        ],
        
        "Snake Plant" => vec![MonsterSpawn::new("Snake Plant")],
        
        "Snecko" => vec![MonsterSpawn::new("Snecko")],
        
        "Centurion and Healer" | "Centurion and Mystic" => vec![
            MonsterSpawn::new("Centurion"),
            MonsterSpawn::new("Mystic"),
        ],
        
        "3 Cultists" => vec![
            MonsterSpawn::new("Cultist"),
            MonsterSpawn::new("Cultist"),
            MonsterSpawn::new("Cultist"),
        ],
        
        "Shelled Parasite and Fungi" => vec![
            MonsterSpawn::new("Shelled Parasite"),
            MonsterSpawn::new("Fungi Beast"),
        ],
        
        // ====================================================================
        // Act 2 - Elite Encounters
        // ====================================================================
        
        "Gremlin Leader" => {
            // Gremlin Leader starts with 2-3 random gremlins
            let gremlin_pool = ["Mad Gremlin", "Sneaky Gremlin", "Fat Gremlin", "Shield Gremlin", "Gremlin Wizard"];
            let count = rng.random_range(2..=3);
            let mut result = vec![MonsterSpawn::new("Gremlin Leader")];
            for _ in 0..count {
                let idx = rng.random_range(0..gremlin_pool.len());
                result.push(MonsterSpawn::new(gremlin_pool[idx]));
            }
            result
        }
        
        "Slavers" | "Slaver and Taskmaster" => vec![
            MonsterSpawn::new("Taskmaster"),
            MonsterSpawn::new("Blue Slaver"),
            MonsterSpawn::new("Red Slaver"),
        ],
        
        "Book of Stabbing" => vec![MonsterSpawn::new("Book of Stabbing")],
        
        // ====================================================================
        // Act 2 - Boss Encounters
        // ====================================================================
        
        "The Champ" => vec![MonsterSpawn::new("The Champ")],
        
        "Bronze Automaton" => vec![
            MonsterSpawn::new("Bronze Automaton"),
            MonsterSpawn::new("Bronze Orb"),
            MonsterSpawn::new("Bronze Orb"),
        ],
        
        "The Collector" => vec![
            MonsterSpawn::new("The Collector"),
            // Collector summons Torch Heads during battle, not at start
        ],
        
        // ====================================================================
        // Act 3 - Weak Pool Encounters (first 2 combats)
        // ====================================================================
        
        "3 Darklings" => vec![
            MonsterSpawn::new("Darkling"),
            MonsterSpawn::new("Darkling"),
            MonsterSpawn::new("Darkling"),
        ],
        
        "Orb Walker" => vec![MonsterSpawn::new("Orb Walker")],
        
        "3 Shapes" => {
            // 3 random shapes from: Exploder, Repulsor, Spiker
            let shape_pool = ["Exploder", "Repulsor", "Spiker"];
            (0..3).map(|_| {
                let idx = rng.random_range(0..shape_pool.len());
                MonsterSpawn::new(shape_pool[idx])
            }).collect()
        }
        
        // ====================================================================
        // Act 3 - Strong Pool Encounters (combats 3+)
        // ====================================================================
        
        "4 Shapes" => {
            // 4 random shapes from: Exploder, Repulsor, Spiker
            let shape_pool = ["Exploder", "Repulsor", "Spiker"];
            (0..4).map(|_| {
                let idx = rng.random_range(0..shape_pool.len());
                MonsterSpawn::new(shape_pool[idx])
            }).collect()
        }
        
        "Maw" | "The Maw" => vec![MonsterSpawn::new("The Maw")],
        
        "Spheric Guardian and 2 Shapes" => {
            let shape_pool = ["Exploder", "Repulsor", "Spiker"];
            let shape1 = shape_pool[rng.random_range(0..shape_pool.len())];
            let shape2 = shape_pool[rng.random_range(0..shape_pool.len())];
            vec![
                MonsterSpawn::new("Spheric Guardian"),
                MonsterSpawn::new(shape1),
                MonsterSpawn::new(shape2),
            ]
        }
        
        "Spire Growth" => vec![MonsterSpawn::new("Spire Growth")],
        
        "Transient" => vec![MonsterSpawn::new("Transient")],
        
        "Jaw Worm Horde" => vec![
            MonsterSpawn::new("Jaw Worm"),
            MonsterSpawn::new("Jaw Worm"),
            MonsterSpawn::new("Jaw Worm (Hard)"),
        ],
        
        "Writhing Mass" => vec![MonsterSpawn::new("Writhing Mass")],
        
        // ====================================================================
        // Act 3 - Elite Encounters
        // ====================================================================
        
        "Giant Head" => vec![MonsterSpawn::new("Giant Head")],
        
        "Nemesis" => vec![MonsterSpawn::new("Nemesis")],
        
        "Reptomancer" => vec![
            MonsterSpawn::new("Reptomancer"),
            MonsterSpawn::new("Dagger"),
            MonsterSpawn::new("Dagger"),
        ],
        
        // ====================================================================
        // Act 3 - Boss Encounters
        // ====================================================================
        
        "Awakened One" => vec![
            MonsterSpawn::new("Awakened One"),
            MonsterSpawn::new("Cultist"),
            MonsterSpawn::new("Cultist"),
        ],
        
        "Donu and Deca" => vec![
            MonsterSpawn::new("Donu"),
            MonsterSpawn::new("Deca"),
        ],
        
        "Time Eater" => vec![MonsterSpawn::new("Time Eater")],
        
        // ====================================================================
        // Act 4 - THE ENDING (Heart Route)
        // ====================================================================
        
        "Spire Shield and Spire Spear" | "Spire Elites" => vec![
            MonsterSpawn::new("Spire Shield"),
            MonsterSpawn::new("Spire Spear"),
        ],
        
        "Corrupt Heart" | "The Heart" => vec![MonsterSpawn::new("Corrupt Heart")],
        
        // ====================================================================
        // Special Event Fights
        // ====================================================================
        
        // Mind Bloom event - can trigger various fights
        "Mind Bloom Elite" | "2 Louses and Looter" => vec![
            MonsterSpawn::new("Red Louse"),
            MonsterSpawn::new("Green Louse"),
            MonsterSpawn::new("Looter"),
        ],
        
        // Masked Bandits (Colosseum event)
        "Masked Bandits" | "Romeo and Bear" => vec![
            MonsterSpawn::new("Romeo"),
            MonsterSpawn::new("Bear"),
        ],
        
        // Colosseum Nob & Slaver event
        "Colosseum Nob" | "Nob and Slaver" => vec![
            MonsterSpawn::new("Gremlin Nob"),
            MonsterSpawn::new("Blue Slaver"),
        ],
        
        // Mysterious Sphere event fight
        "Mysterious Sphere" | "2 Orb Walkers" => vec![
            MonsterSpawn::new("Orb Walker"),
            MonsterSpawn::new("Orb Walker"),
        ],
        
        // Pointy fight (from event)
        "Pointy" => vec![MonsterSpawn::new("Pointy")],
        
        // ====================================================================
        // Fallback: Single monster with same ID
        // ====================================================================
        _ => {
            // If no special handling, assume encounter_id is a single monster ID
            vec![MonsterSpawn::new(encounter_id)]
        }
    }
}
