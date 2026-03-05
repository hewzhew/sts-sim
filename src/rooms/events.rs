//! Event System for Slay the Spire
//!
//! This module handles game events: loading definitions from JSON, tracking event state,
//! and executing event options with their costs, rewards, and commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// ============================================================================
// STATIC EVENT DATA
// ============================================================================

/// Global event definitions loaded from JSON
pub static EVENT_DEFINITIONS: Lazy<HashMap<String, EventDefinition>> = Lazy::new(|| {
    let json_str = include_str!("../../data/events_final_master.json");
    serde_json::from_str(json_str).expect("Failed to parse events JSON")
});

// ============================================================================
// CORE STRUCTS
// ============================================================================

/// Complete definition of a game event
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventDefinition {
    pub wiki_id: Option<String>,
    pub name: String,
    pub category: EventCategory,
    /// Pool type: "regular" or "shrine"
    #[serde(default)]
    pub pool_type: Option<PoolType>,
    /// Which acts this event can appear in
    #[serde(default)]
    pub act_pool: Vec<ActId>,
    /// Conditions required to enter the event pool
    #[serde(default)]
    pub pool_conditions: Vec<PoolCondition>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub loop_mechanic: Option<LoopMechanic>,
    #[serde(default)]
    pub initialization: Option<EventInitialization>,
    pub options: Vec<EventOption>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Pool type for event selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PoolType {
    Regular,
    Shrine,
}

/// Act identifier for event pools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActId {
    #[default]
    Act1,
    Act2,
    Act3,
}

/// Conditions for entering the event pool (checked before event selection)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PoolCondition {
    /// Player has at least this much gold
    HasGold { amount: i32 },
    /// Player has a specific relic
    HasRelic { relic_id: String },
    /// Player HP is at most X% of max
    HpPercent { max_percent: i32 },
    /// Player HP is above X
    HpAbove { amount: i32 },
    /// Player has at least one curse in deck
    HasCurse,
    /// Minimum floor number within the act
    FloorMin { floor: i32 },
    /// Only available above the guaranteed chest floor
    AboveChestFloor,
    /// Ascension level is below X
    AscensionBelow { level: i32 },
    /// Player has at least X relics
    RelicCount { min: i32 },
    /// At least X seconds have elapsed in the run
    TimeElapsed { seconds: i32 },
    /// Any of the sub-conditions is true
    Or { conditions: Vec<PoolCondition> },
}

/// Event category (which act or shrine) - legacy field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EventCategory {
    Act1,
    Act2,
    Act3,
    Shrines,
}

/// Loop mechanic for events like Dead Adventurer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoopMechanic {
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub state_tracking: Vec<String>,
}

/// Event initialization data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventInitialization {
    pub select_potion: Option<String>,
    pub lock_potion: Option<bool>,
}

/// A single option the player can choose in an event
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventOption {
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub costs: Option<EventCosts>,
    #[serde(default)]
    pub rewards: Option<EventRewards>,
    #[serde(default)]
    pub commands: Vec<EventCommand>,
    #[serde(default)]
    pub conditions: Vec<EventCondition>,
    #[serde(default)]
    pub random_outcomes: Vec<RandomOutcome>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub locked_if_fail: Option<bool>,
    #[serde(default)]
    pub locked_text: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub rewards_by_rarity: Option<HashMap<String, EventRewards>>,
}

/// Costs associated with an event option
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EventCosts {
    // Gold costs (integer only)
    pub gold: Option<i32>,
    pub gold_ascension: Option<i32>,
    // Gold all (lose all gold)
    pub gold_all: Option<bool>,
    // Gold random range
    pub gold_random: Option<GoldRange>,
    // HP costs (static)
    pub hp: Option<i32>,
    pub hp_ascension: Option<i32>,
    pub hp_percent: Option<f32>,
    pub hp_percent_ascension: Option<f32>,
    // HP costs (dynamic, structured)
    pub hp_dynamic: Option<HpDynamic>,
    // Max HP costs
    pub max_hp: Option<i32>,
    pub max_hp_percent: Option<f32>,
    pub max_hp_percent_ascension: Option<f32>,
    #[serde(default)]
    pub round: Option<String>,
    // Relic costs
    pub relic: Option<String>,
    // Potion costs
    pub potion: Option<String>,
    pub potion_count: Option<i32>,
}

/// Rewards from an event option
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EventRewards {
    // Gold rewards (integer only)
    pub gold: Option<i32>,
    pub gold_ascension: Option<i32>,
    // Gold random range
    pub gold_random: Option<GoldRange>,
    // Relics
    pub relic: Option<String>,
    // Cards (simple string identifier)
    pub card: Option<String>,
    pub curse: Option<String>,
    // Healing
    pub heal: Option<i32>,
    pub heal_percent: Option<f32>,
    pub heal_percent_ascension: Option<f32>,
    pub heal_max_hp: Option<bool>,
    #[serde(default)]
    pub round: Option<String>,
    // Max HP changes
    pub max_hp: Option<i32>,
    pub max_hp_percent: Option<f32>,
    // Potions (simple string identifier)
    pub potion: Option<String>,
    pub potion_count: Option<i32>,
    pub potion_count_ascension: Option<i32>,
    // Keys (for Act 3)
    pub key: Option<String>,
    // Loot table for complex rewards
    pub loot_table: Option<Vec<LootEntry>>,
    pub selection: Option<String>,
}

/// Gold range for random gold amounts
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoldRange {
    pub min: i32,
    pub max: i32,
}

/// Dynamic HP cost calculation (e.g., Knowing Skull)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HpDynamic {
    /// Base HP cost
    pub base: i32,
    /// Additional HP per selection count
    pub per_count: i32,
    /// Percent of max HP as floor (e.g., 0.10 = 10%)
    pub floor_percent: f32,
    /// Minimum HP cost
    pub min: i32,
    /// Counter name to track (e.g., "potion_count", "gold_count")
    pub counter: String,
}

/// Entry in a loot table
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LootEntry {
    #[serde(rename = "type")]
    pub loot_type: String,
    pub amount: Option<i32>,
    pub rarity: Option<String>,
    pub once_per_event: Option<bool>,
}

/// Random outcome with chance and results
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RandomOutcome {
    pub name: String,
    // Simple chance (fixed probability)
    pub chance: Option<f32>,
    pub weight: Option<i32>,
    // Complex chance logic (for scaling probabilities)
    pub chance_logic: Option<ChanceLogic>,
    // Results
    #[serde(default)]
    pub rewards: Option<EventRewards>,
    #[serde(default)]
    pub costs: Option<EventCosts>,
    #[serde(default)]
    pub commands: Vec<EventCommand>,
}

/// Complex chance calculation logic
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChanceLogic {
    pub base: f32,
    pub base_ascension: Option<f32>,
    pub increment_per_attempt: Option<f32>,
    pub decrement_per_attempt: Option<f32>,
}

// ============================================================================
// EVENT COMMANDS
// ============================================================================

/// Commands that trigger game actions
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum EventCommand {
    /// Start a combat encounter
    Combat {
        enemies: Vec<String>,
        #[serde(default)]
        enemy_selection: Option<String>,
        combat_type: Option<String>,
        #[serde(default)]
        boss_pool: Vec<String>,
        #[serde(default)]
        special: Option<String>,
    },
    /// Modify the player's deck
    DeckMod {
        op: DeckModOp,
        #[serde(default)]
        card_id: Option<String>,
        #[serde(default)]
        filter: Option<CardFilter>,
        #[serde(default)]
        count: Option<i32>,
        #[serde(default)]
        count_ascension: Option<i32>,
    },
    /// Interactive card selection (opens UI)
    CardSelect {
        action: CardSelectAction,
        pick: i32,
        pool: String,
        #[serde(default)]
        source_amount: Option<i32>,
        #[serde(default)]
        filter: Option<CardFilter>,
    },
    /// Set event state variable
    SetEventState {
        state: String,
        value: serde_json::Value,
    },
    /// Set event phase (for multi-phase events)
    /// Phase can be string ("trap") or integer (2)
    SetEventPhase {
        phase: serde_json::Value,
    },
    /// Teleport to a different location
    Teleport {
        destination: String,
    },
    /// Lose an item (potion, relic, etc.)
    LoseItem {
        item_type: String,
        selection: Option<String>,
    },
    /// Minigame (e.g., Memory Match in Match and Keep)
    Minigame {
        game: String,
        #[serde(default)]
        config: Option<serde_json::Value>,
    },
}

/// Deck modification operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum DeckModOp {
    Add,
    Remove,
    RemoveAll,     // Remove all cards matching criteria
    RemoveRandom,
    Transform,
    TransformRandom,
    Upgrade,
    UpgradeRandom,
    UpgradeAll,
    Duplicate,
}

/// Card selection actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSelectAction {
    Remove,
    Transform,
    Upgrade,
    Duplicate,
    Add,           // Add card to deck (e.g., The Library)
    OfferSpirits,  // Special action for Bonfire Spirits
}

/// Filter for card selection
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CardFilter {
    pub card_type: Option<String>,
    pub upgradeable: Option<bool>,
    pub rarity: Option<String>,
}

// ============================================================================
// EVENT CONDITIONS
// ============================================================================

/// Conditions that determine option availability
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum EventCondition {
    /// Player has enough gold
    HasGold {
        amount: i32,
        amount_ascension: Option<i32>,
    },
    /// Player has a specific relic
    HasRelic {
        relic_id: String,
    },
    /// Player does not have a specific relic
    NotHasRelic {
        relic_id: String,
    },
    /// Player has a card of specific type
    HasCardType {
        card_type: String,
    },
    /// Player has no cards of specified types
    NoCardOfType {
        types: Vec<String>,
    },
    /// Check deck size
    DeckSize {
        min: Option<i32>,
        max: Option<i32>,
    },
    /// Current floor is in range
    FloorRange {
        min: Option<i32>,
        max: Option<i32>,
    },
    /// Check event state variable
    EventState {
        state: String,
        value: serde_json::Value,
    },
    /// Player has a potion
    HasPotion {
        count: Option<i32>,
    },
    /// Player has a card with certain cost
    HasCardCost {
        min_cost: Option<i32>,
    },
}

// ============================================================================
// RUNTIME STATE
// ============================================================================

/// Runtime state for an active event
#[derive(Debug, Clone, Default)]
pub struct ActiveEventState {
    /// The event ID currently active
    pub event_id: String,
    /// Current phase/iteration for loop events
    pub iteration: u32,
    /// RNG seed for this event instance
    pub rng_seed: u64,
    /// Custom state variables (e.g., search_completed, rewards_found)
    pub variables: HashMap<String, serde_json::Value>,
    /// Pending command to execute after card selection
    pub pending_callback: Option<PendingCallback>,
}

/// Callback information for async operations like card selection
#[derive(Debug, Clone)]
pub struct PendingCallback {
    pub action: CardSelectAction,
    pub selected_cards: Vec<String>,
    /// For bonfire spirits - the rarity of the selected card
    pub rarity_rewards: Option<HashMap<String, EventRewards>>,
}

impl ActiveEventState {
    pub fn new(event_id: &str, rng_seed: u64) -> Self {
        Self {
            event_id: event_id.to_string(),
            iteration: 0,
            rng_seed,
            variables: HashMap::new(),
            pending_callback: None,
        }
    }

    pub fn set_state(&mut self, key: &str, value: serde_json::Value) {
        self.variables.insert(key.to_string(), value);
    }

    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.variables
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }
}

// ============================================================================
// EVENT EXECUTION
// ============================================================================

/// Result of executing an event option
#[derive(Debug, Clone)]
pub enum EventExecutionResult {
    /// Event completed, return to map
    Complete,
    /// Event continues (for loop events)
    Continue,
    /// Need to show card selection UI
    AwaitingCardSelect {
        action: CardSelectAction,
        pick_count: i32,
        pool: String,
        filter: Option<CardFilter>,
    },
    /// Start a combat encounter
    StartCombat {
        enemies: Vec<String>,
        combat_type: String,
    },
    /// Teleport to a new location
    Teleport {
        destination: String,
    },
}

impl EventDefinition {
    /// Get the definition for an event by ID
    pub fn get(event_id: &str) -> Option<&'static EventDefinition> {
        EVENT_DEFINITIONS.get(event_id)
    }

    /// Get all event IDs for a category
    pub fn get_by_category(category: EventCategory) -> Vec<&'static str> {
        EVENT_DEFINITIONS
            .iter()
            .filter(|(_, def)| def.category == category)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Check if an option is available given current game state
    pub fn is_option_available(
        &self,
        option_idx: usize,
        gold: i32,
        ascension: u8,
        event_state: &ActiveEventState,
        // Add more state params as needed
    ) -> bool {
        let option = match self.options.get(option_idx) {
            Some(o) => o,
            None => return false,
        };

        for condition in &option.conditions {
            match condition {
                EventCondition::HasGold { amount, amount_ascension } => {
                    let required = if ascension >= 15 {
                        amount_ascension.unwrap_or(*amount)
                    } else {
                        *amount
                    };
                    if gold < required {
                        return false;
                    }
                }
                EventCondition::EventState { state, value } => {
                    let current = event_state.get_state(state);
                    if current != Some(value) {
                        return false;
                    }
                }
                // Add other condition checks as needed
                _ => {}
            }
        }

        true
    }
}

impl EventOption {
    /// Calculate the actual gold cost based on ascension
    pub fn gold_cost(&self, ascension: u8) -> i32 {
        self.costs.as_ref().map_or(0, |c| {
            if ascension >= 15 {
                c.gold_ascension.unwrap_or(c.gold.unwrap_or(0))
            } else {
                c.gold.unwrap_or(0)
            }
        })
    }

    /// Calculate actual HP cost based on ascension
    pub fn hp_cost(&self, ascension: u8) -> i32 {
        self.costs.as_ref().map_or(0, |c| {
            if ascension >= 15 {
                c.hp_ascension.unwrap_or(c.hp.unwrap_or(0))
            } else {
                c.hp.unwrap_or(0)
            }
        })
    }

    /// Get HP percent cost if any
    pub fn hp_percent_cost(&self) -> Option<f32> {
        self.costs.as_ref().and_then(|c| c.hp_percent)
    }

    /// Roll random outcome based on chances
    pub fn roll_random_outcome(&self, rng: &mut impl rand::Rng, iteration: u32, ascension: u8) -> Option<&RandomOutcome> {
        if self.random_outcomes.is_empty() {
            return None;
        }

        // Calculate total weight/chance
        let outcomes_with_chance: Vec<(&RandomOutcome, f32)> = self
            .random_outcomes
            .iter()
            .map(|outcome| {
                let chance = if let Some(logic) = &outcome.chance_logic {
                    let base = if ascension >= 15 {
                        logic.base_ascension.unwrap_or(logic.base)
                    } else {
                        logic.base
                    };
                    let adjustment = iteration as f32
                        * (logic.increment_per_attempt.unwrap_or(0.0)
                            - logic.decrement_per_attempt.unwrap_or(0.0));
                    (base + adjustment).clamp(0.0, 1.0)
                } else if let Some(chance) = outcome.chance {
                    chance
                } else if let Some(weight) = outcome.weight {
                    weight as f32
                } else {
                    1.0
                };
                (outcome, chance)
            })
            .collect();

        let total: f32 = outcomes_with_chance.iter().map(|(_, c)| c).sum();
        let roll = rng.random::<f32>() * total;

        let mut cumulative = 0.0;
        for (outcome, chance) in outcomes_with_chance {
            cumulative += chance;
            if roll < cumulative {
                return Some(outcome);
            }
        }

        self.random_outcomes.last()
    }
}

// ============================================================================
// EVENT POOL SELECTION
// ============================================================================

/// Game state snapshot for pool condition checking
#[derive(Debug, Clone, Default)]
pub struct EventPoolContext {
    pub gold: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub ascension: u8,
    pub floor: i32,           // Floor within current act (1-indexed)
    pub act: ActId,
    pub relic_ids: Vec<String>,
    pub has_curse: bool,
    pub elapsed_seconds: i32,
    pub chest_floor: i32,     // Guaranteed chest floor for the act
    pub seen_events: Vec<String>, // Events already seen this run
}

impl EventPoolContext {
    /// Check if a pool condition is satisfied
    pub fn check_condition(&self, condition: &PoolCondition) -> bool {
        match condition {
            PoolCondition::HasGold { amount } => self.gold >= *amount,
            PoolCondition::HasRelic { relic_id } => self.relic_ids.contains(relic_id),
            PoolCondition::HpPercent { max_percent } => {
                let percent = (self.current_hp * 100) / self.max_hp.max(1);
                percent <= *max_percent
            }
            PoolCondition::HpAbove { amount } => self.current_hp > *amount,
            PoolCondition::HasCurse => self.has_curse,
            PoolCondition::FloorMin { floor } => self.floor >= *floor,
            PoolCondition::AboveChestFloor => self.floor > self.chest_floor,
            PoolCondition::AscensionBelow { level } => (self.ascension as i32) < *level,
            PoolCondition::RelicCount { min } => self.relic_ids.len() as i32 >= *min,
            PoolCondition::TimeElapsed { seconds } => self.elapsed_seconds >= *seconds,
            PoolCondition::Or { conditions } => {
                conditions.iter().any(|c| self.check_condition(c))
            }
        }
    }
    
    /// Check all pool conditions for an event
    pub fn check_all_conditions(&self, conditions: &[PoolCondition]) -> bool {
        conditions.iter().all(|c| self.check_condition(c))
    }
}

/// Event selector for choosing events based on act and game state
pub struct EventSelector;

impl EventSelector {
    /// Shrine event probability (approximately 25% when eligible)
    pub const SHRINE_PROBABILITY: f32 = 0.25;
    
    /// Get all events available in the pool for the current act and game state
    pub fn get_available_events(
        ctx: &EventPoolContext,
        pool_type: Option<PoolType>,
    ) -> Vec<&'static str> {
        EVENT_DEFINITIONS
            .iter()
            .filter(|(id, def)| {
                // Check act pool
                if !def.act_pool.contains(&ctx.act) {
                    return false;
                }
                
                // Check pool type if specified
                if let Some(pt) = pool_type {
                    if def.pool_type != Some(pt) {
                        return false;
                    }
                }
                
                // Check pool conditions
                if !ctx.check_all_conditions(&def.pool_conditions) {
                    return false;
                }
                
                // Check if already seen (for one-time events)
                // Note: Most events can be seen multiple times per run,
                // but some (like Note For Yourself) are one-time
                if ctx.seen_events.contains(&id.to_string()) {
                    // For now, allow re-encounter; specific exclusion can be added
                }
                
                true
            })
            .map(|(id, _)| id.as_str())
            .collect()
    }
    
    /// Select a random event from the available pool
    pub fn select_event(
        ctx: &EventPoolContext,
        rng: &mut impl rand::Rng,
    ) -> Option<&'static str> {
        // Determine pool type based on probability
        let use_shrine = rng.random::<f32>() < Self::SHRINE_PROBABILITY;
        
        // Try shrine pool first if rolled
        if use_shrine {
            let shrines = Self::get_available_events(ctx, Some(PoolType::Shrine));
            if !shrines.is_empty() {
                let idx = rng.random_range(0..shrines.len());
                return Some(shrines[idx]);
            }
        }
        
        // Fall back to regular events
        let regulars = Self::get_available_events(ctx, Some(PoolType::Regular));
        if !regulars.is_empty() {
            let idx = rng.random_range(0..regulars.len());
            return Some(regulars[idx]);
        }
        
        None
    }
    
    /// Get events by act (for debugging/display)
    pub fn get_events_for_act(act: ActId) -> (Vec<&'static str>, Vec<&'static str>) {
        let mut regular = Vec::new();
        let mut shrine = Vec::new();
        
        for (id, def) in EVENT_DEFINITIONS.iter() {
            if def.act_pool.contains(&act) {
                match def.pool_type {
                    Some(PoolType::Regular) => regular.push(id.as_str()),
                    Some(PoolType::Shrine) => shrine.push(id.as_str()),
                    None => {} // Skip events without pool_type
                }
            }
        }
        
        regular.sort();
        shrine.sort();
        (regular, shrine)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_events() {
        // This will panic if JSON is invalid
        let count = EVENT_DEFINITIONS.len();
        assert!(count > 0, "Should have loaded some events");
        game_log!("Loaded {} events", count);
    }

    #[test]
    fn test_event_categories() {
        let act1_events = EventDefinition::get_by_category(EventCategory::Act1);
        let shrines = EventDefinition::get_by_category(EventCategory::Shrines);

        assert!(!act1_events.is_empty(), "Should have Act 1 events");
        assert!(!shrines.is_empty(), "Should have shrine events");

        game_log!("Act 1 events: {:?}", act1_events);
        game_log!("Shrines: {:?}", shrines);
    }

    #[test]
    fn test_specific_event() {
        let big_fish = EventDefinition::get("big_fish");
        assert!(big_fish.is_some(), "Should find big_fish event");

        let event = big_fish.unwrap();
        assert_eq!(event.name, "Big Fish");
        assert_eq!(event.category, EventCategory::Act1);
        assert!(!event.options.is_empty());
    }

    #[test]
    fn test_dead_adventurer_no_inherits() {
        let event = EventDefinition::get("dead_adventurer").unwrap();
        
        // Find the [Continue] option
        let continue_opt = event.options.iter().find(|o| o.label == "[Continue]");
        assert!(continue_opt.is_some(), "Should have [Continue] option");
        
        let opt = continue_opt.unwrap();
        // Should have its own random_outcomes, not inherits
        assert!(!opt.random_outcomes.is_empty(), "[Continue] should have explicit random_outcomes");
    }

    #[test]
    fn test_card_select_commands() {
        let purifier = EventDefinition::get("purifier").unwrap();
        let pray_option = &purifier.options[0];
        
        assert!(!pray_option.commands.is_empty(), "Should have commands");
        
        match &pray_option.commands[0] {
            EventCommand::CardSelect { action, pick, pool, .. } => {
                assert_eq!(*action, CardSelectAction::Remove);
                assert_eq!(*pick, 1);
                assert_eq!(pool, "PlayerDeck");
            }
            _ => panic!("Expected CardSelect command"),
        }
    }

    #[test]
    fn test_bonfire_spirits_action() {
        let event = EventDefinition::get("bonfire_spirits").unwrap();
        let offer_option = &event.options[0];
        
        match &offer_option.commands[0] {
            EventCommand::CardSelect { action, .. } => {
                assert_eq!(*action, CardSelectAction::OfferSpirits);
            }
            _ => panic!("Expected CardSelect command"),
        }
        
        // Should have rewards_by_rarity
        assert!(offer_option.rewards_by_rarity.is_some());
    }

    #[test]
    fn test_gold_types_consistent() {
        // Verify all gold fields are integers, not dicts
        for (id, event) in EVENT_DEFINITIONS.iter() {
            for (idx, opt) in event.options.iter().enumerate() {
                if let Some(costs) = &opt.costs {
                    // gold should be i32, gold_random should be GoldRange
                    if costs.gold.is_some() {
                        // This will have been validated by serde
                        game_log!("{} option {} has costs.gold", id, idx);
                    }
                }
                if let Some(rewards) = &opt.rewards {
                    if rewards.gold.is_some() {
                        game_log!("{} option {} has rewards.gold", id, idx);
                    }
                }
            }
        }
    }
    
    #[test]
    fn test_event_pools_populated() {
        // Check that all events have pool info
        let mut missing_pool = Vec::new();
        
        for (id, event) in EVENT_DEFINITIONS.iter() {
            if event.pool_type.is_none() || event.act_pool.is_empty() {
                missing_pool.push(id.as_str());
            }
        }
        
        if !missing_pool.is_empty() {
            game_log!("Events missing pool info: {:?}", missing_pool);
        }
        
        // All events should have pool info
        assert!(missing_pool.is_empty(), "All events should have pool_type and act_pool");
    }
    
    #[test]
    fn test_act_event_distribution() {
        // Verify event distribution matches expected counts from txt files
        let (act1_regular, act1_shrine) = EventSelector::get_events_for_act(ActId::Act1);
        let (act2_regular, act2_shrine) = EventSelector::get_events_for_act(ActId::Act2);
        let (act3_regular, act3_shrine) = EventSelector::get_events_for_act(ActId::Act3);
        
        game_log!("Act 1: {} regular, {} shrine", act1_regular.len(), act1_shrine.len());
        game_log!("  Regular: {:?}", act1_regular);
        game_log!("  Shrine: {:?}", act1_shrine);
        
        game_log!("Act 2: {} regular, {} shrine", act2_regular.len(), act2_shrine.len());
        game_log!("  Regular: {:?}", act2_regular);
        game_log!("  Shrine: {:?}", act2_shrine);
        
        game_log!("Act 3: {} regular, {} shrine", act3_regular.len(), act3_shrine.len());
        game_log!("  Regular: {:?}", act3_regular);
        game_log!("  Shrine: {:?}", act3_shrine);
        
        // From txt files:
        // Act 1: 11 regular, 14 shrine
        // Act 2: 13 regular, 19 shrine  
        // Act 3: 7 regular, 16 shrine
        assert_eq!(act1_regular.len(), 11, "Act 1 should have 11 regular events");
        assert_eq!(act1_shrine.len(), 14, "Act 1 should have 14 shrine events");
        assert_eq!(act2_regular.len(), 13, "Act 2 should have 13 regular events");
        assert_eq!(act3_regular.len(), 7, "Act 3 should have 7 regular events");
    }
    
    #[test]
    fn test_pool_condition_checking() {
        let ctx = EventPoolContext {
            gold: 50,
            current_hp: 30,
            max_hp: 80,
            ascension: 10,
            floor: 5,
            act: ActId::Act1,
            relic_ids: vec!["Burning Blood".to_string()],
            has_curse: false,
            elapsed_seconds: 300,
            chest_floor: 8,
            seen_events: vec![],
        };
        
        // Test HasGold
        assert!(ctx.check_condition(&PoolCondition::HasGold { amount: 50 }));
        assert!(ctx.check_condition(&PoolCondition::HasGold { amount: 35 }));
        assert!(!ctx.check_condition(&PoolCondition::HasGold { amount: 75 }));
        
        // Test HpPercent (30/80 = 37.5%)
        assert!(ctx.check_condition(&PoolCondition::HpPercent { max_percent: 50 }));
        assert!(!ctx.check_condition(&PoolCondition::HpPercent { max_percent: 30 }));
        
        // Test FloorMin
        assert!(ctx.check_condition(&PoolCondition::FloorMin { floor: 5 }));
        assert!(!ctx.check_condition(&PoolCondition::FloorMin { floor: 7 }));
        
        // Test AscensionBelow
        assert!(ctx.check_condition(&PoolCondition::AscensionBelow { level: 15 }));
        assert!(!ctx.check_condition(&PoolCondition::AscensionBelow { level: 10 }));
        
        // Test Or condition
        let or_cond = PoolCondition::Or {
            conditions: vec![
                PoolCondition::HasGold { amount: 100 },  // false
                PoolCondition::FloorMin { floor: 3 },    // true
            ]
        };
        assert!(ctx.check_condition(&or_cond));
    }
    
    #[test]
    fn test_event_selection_filters() {
        let ctx = EventPoolContext {
            gold: 30,  // Not enough for The Cleric (35)
            current_hp: 70,
            max_hp: 80,
            ascension: 0,
            floor: 3,  // Below floor 7 for Dead Adventurer
            act: ActId::Act1,
            relic_ids: vec![],
            has_curse: false,
            elapsed_seconds: 100,
            chest_floor: 8,
            seen_events: vec![],
        };
        
        let available = EventSelector::get_available_events(&ctx, Some(PoolType::Regular));
        
        // Should NOT include the_cleric (needs 35 gold)
        assert!(!available.contains(&"the_cleric"), "The Cleric should be excluded (needs 35 gold)");
        
        // Should NOT include dead_adventurer (needs floor 7+)
        assert!(!available.contains(&"dead_adventurer"), "Dead Adventurer should be excluded (needs floor 7+)");
        
        // Should include big_fish (no conditions)
        assert!(available.contains(&"big_fish"), "Big Fish should be available");
    }
}
