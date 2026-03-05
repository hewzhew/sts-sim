//! Game state management for the Slay the Spire simulator.
//!
//! This module defines the complete game state that gets mutated during combat.

use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;

use crate::schema::{CardColor, CardInstance, CardLocation, CardType};
use crate::loader::CardLibrary;
use crate::enemy::MonsterState;
use crate::items::relics::RelicInstance;
use crate::map::SimpleMap;
use crate::events::{ActiveEventState, CardSelectAction, CardFilter as EventCardFilter};
use crate::items::potions::PotionSlots;

// ============================================================================
// Game Phase (Screen State for RL Agent)
// ============================================================================

/// The current game screen/phase. Determines what actions are valid.
/// An RL agent checks this to know which action API to call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GamePhase {
    /// Viewing the map, choosing the next node to travel to.
    #[default]
    Map,
    /// In an active combat encounter.
    Combat,
    /// Combat victory - choosing rewards (cards, gold, potions, relics).
    Reward,
    /// Inside a shop - can buy/sell items or purge cards.
    Shop,
    /// At a rest site (campfire) - can rest, smith, or use relic abilities.
    Rest,
    /// In a random event - making choices.
    Event,
    /// Card selection screen (Remove, Transform, Upgrade, etc.)
    CardSelect,
    /// The run has ended (won or lost).
    GameOver,
}

impl GamePhase {
    /// Returns true if this phase allows ending the turn (only Combat).
    pub fn can_end_turn(&self) -> bool {
        matches!(self, GamePhase::Combat)
    }
    
    /// Returns true if this phase requires choosing a map node.
    pub fn is_map_selection(&self) -> bool {
        matches!(self, GamePhase::Map)
    }
    
    /// Returns true if the game has ended.
    pub fn is_game_over(&self) -> bool {
        matches!(self, GamePhase::GameOver)
    }
}

// ============================================================================
// Shop State (defined here to avoid circular imports)
// ============================================================================

/// A card available for purchase in the shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopCard {
    /// The card instance being sold.
    pub card: CardInstance,
    /// The price in gold.
    pub price: i32,
    /// Whether this card is on sale (50% off).
    pub on_sale: bool,
}

/// A relic available for purchase in the shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopRelic {
    /// The relic ID.
    pub relic_id: String,
    /// The price in gold.
    pub price: i32,
}

/// A potion available for purchase in the shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopPotion {
    /// The potion ID.
    pub potion_id: String,
    /// The price in gold.
    pub price: i32,
}

/// The state of a shop, including inventory and prices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopState {
    /// Cards available for purchase.
    pub cards: Vec<ShopCard>,
    /// Relics available for purchase.
    pub relics: Vec<ShopRelic>,
    /// Potions available for purchase.
    pub potions: Vec<ShopPotion>,
}

impl ShopState {
    /// Create an empty shop state.
    pub fn new() -> Self {
        Self {
            cards: Vec::new(),
            relics: Vec::new(),
            potions: Vec::new(),
        }
    }
}

impl Default for ShopState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Campfire State (defined here to avoid circular imports)
// ============================================================================

/// Minimal campfire relic state tracking.
/// The full logic is in the campfire module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampfireRelicState {
    /// Number of times Girya (Lift) has been used (max 3).
    pub girya_uses: u8,
    /// Whether Shovel (Dig) has been used this run.
    pub shovel_used: bool,
}

/// Maximum expected hand size (for SmallVec optimization).
const MAX_HAND_SIZE: usize = 10;
/// Typical draw pile size.
const TYPICAL_DECK_SIZE: usize = 32;

// ============================================================================
// Card Selection & Filtering
// ============================================================================

/// How to select cards for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectMode {
    /// Randomly select cards
    #[default]
    Random,
    /// Let player/agent choose (for RL, defaults to random in Phase 2)
    Choose,
    /// Select all matching cards
    All,
    /// Select the top card(s)
    Top,
    /// Select the bottom card(s)
    Bottom,
}

impl SelectMode {
    /// Parse from JSON string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "random" => SelectMode::Random,
            "choose" | "player" => SelectMode::Choose,
            "all" => SelectMode::All,
            "top" => SelectMode::Top,
            "bottom" => SelectMode::Bottom,
            _ => SelectMode::Random,
        }
    }
}

/// Where to insert a card in a pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertPosition {
    /// Insert at top (index 0 for hand, last for draw pile since we pop from end)
    Top,
    /// Insert at bottom
    Bottom,
    /// Insert at random position
    Random,
    /// Shuffle the entire pile after inserting
    #[default]
    Shuffle,
}

impl InsertPosition {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "top" => InsertPosition::Top,
            "bottom" => InsertPosition::Bottom,
            "random" => InsertPosition::Random,
            "shuffle" => InsertPosition::Shuffle,
            _ => InsertPosition::Shuffle,
        }
    }
}

/// Filter criteria for selecting cards.
#[derive(Debug, Clone, Default)]
pub struct CardFilter {
    /// Filter by card type
    pub card_type: Option<CardType>,
    /// Filter by name containing substring
    pub name_contains: Option<String>,
    /// Filter by exact cost
    pub cost: Option<i32>,
    /// Filter by max cost
    pub max_cost: Option<i32>,
    /// Filter by upgraded status
    pub upgraded: Option<bool>,
}

impl CardFilter {
    /// Check if a card matches this filter
    pub fn matches(&self, card: &CardInstance, library: Option<&CardLibrary>) -> bool {
        // Check card type if specified
        if let Some(required_type) = self.card_type {
            if let Some(lib) = library {
                if let Ok(def) = lib.get(&card.definition_id) {
                    if def.card_type != required_type {
                        return false;
                    }
                }
            }
        }
        
        // Check name contains
        if let Some(ref substr) = self.name_contains {
            if !card.definition_id.to_lowercase().contains(&substr.to_lowercase()) {
                return false;
            }
        }
        
        // Check exact cost
        if let Some(required_cost) = self.cost {
            if card.current_cost != required_cost {
                return false;
            }
        }
        
        // Check max cost
        if let Some(max) = self.max_cost {
            if card.current_cost > max {
                return false;
            }
        }
        
        // Check upgraded status
        if let Some(required_upgraded) = self.upgraded {
            if card.upgraded != required_upgraded {
                return false;
            }
        }
        
        true
    }
    
    /// Parse a CardFilter from JSON value
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut filter = CardFilter::default();
        
        if let Some(obj) = value.as_object() {
            if let Some(t) = obj.get("type").and_then(|v| v.as_str()) {
                filter.card_type = match t.to_lowercase().as_str() {
                    "attack" => Some(CardType::Attack),
                    "skill" => Some(CardType::Skill),
                    "power" => Some(CardType::Power),
                    "status" => Some(CardType::Status),
                    "curse" => Some(CardType::Curse),
                    _ => None,
                };
            }
            
            if let Some(name) = obj.get("name_contains").and_then(|v| v.as_str()) {
                filter.name_contains = Some(name.to_string());
            }
            
            if let Some(c) = obj.get("cost").and_then(|v| v.as_i64()) {
                filter.cost = Some(c as i32);
            }
            
            if let Some(mc) = obj.get("max_cost").and_then(|v| v.as_i64()) {
                filter.max_cost = Some(mc as i32);
            }
            
            if let Some(u) = obj.get("upgraded").and_then(|v| v.as_bool()) {
                filter.upgraded = Some(u);
            }
        } else if let Some(s) = value.as_str() {
            // Simple string filter like "Attack" or "Skill"
            filter.card_type = match s.to_lowercase().as_str() {
                "attack" | "attacks" => Some(CardType::Attack),
                "skill" | "skills" => Some(CardType::Skill),
                "power" | "powers" => Some(CardType::Power),
                _ => None,
            };
        }
        
        filter
    }
}

// ============================================================================
// Power System Import
// ============================================================================

use crate::powers::{PowerSet, power_ids};

/// Card play modifiers that affect the NEXT card played
#[derive(Debug, Clone, Default)]
pub struct CardPlayModifiers {
    /// If > 0, next Attack card is played twice (DoubleTap, Necronomicon)
    pub duplicate_next_attack: i32,
    /// If > 0, next Skill card is played twice (Burst)
    pub duplicate_next_skill: i32,
    /// If true, next card costs 0 energy
    pub next_card_free: bool,
    /// If true, next card played goes on top of draw pile instead of discard
    pub next_card_to_top: bool,
    /// Strength multiplier for next damage calculation (Heavy Blade: 3/5)
    pub strength_multiplier: i32,
}

impl CardPlayModifiers {
    pub fn clear(&mut self) {
        self.duplicate_next_attack = 0;
        self.duplicate_next_skill = 0;
        self.next_card_free = false;
        self.next_card_to_top = false;
        // Note: strength_multiplier is reset after each damage, not on turn end
    }
}

/// Backwards compatibility: Enemy is now an alias for MonsterState.
/// Use MonsterState directly for new code.
pub type Enemy = MonsterState;

/// The player's state during combat.
#[derive(Debug, Clone)]
pub struct Player {
    pub max_hp: i32,
    pub current_hp: i32,
    pub block: i32,
    pub energy: i32,
    pub max_energy: i32,
    /// All powers/buffs/debuffs on the player (Strength, Vigor, Vulnerable, etc.)
    pub powers: PowerSet,
    /// Current stance (Watcher mechanic)
    pub stance: crate::core::stances::Stance,
    /// Player's gold
    pub gold: i32,
}

impl Player {
    pub fn new(max_hp: i32, max_energy: i32) -> Self {
        Self {
            max_hp,
            current_hp: max_hp,
            block: 0,
            energy: max_energy,
            max_energy,
            powers: PowerSet::new(),
            stance: crate::core::stances::Stance::Neutral,
            gold: 99,
        }
    }
    
    // ========================================================================
    // Basic Attribute Accessors (Compatibility Layer)
    // ========================================================================
    
    /// Get current strength (for damage calculation).
    pub fn strength(&self) -> i32 {
        self.powers.get(power_ids::STRENGTH)
    }
    
    /// Get current dexterity (for block calculation).
    pub fn dexterity(&self) -> i32 {
        self.powers.get(power_ids::DEXTERITY)
    }
    
    /// Get current focus (for orb damage).
    pub fn focus(&self) -> i32 {
        self.powers.get(power_ids::FOCUS)
    }
    
    // ========================================================================
    // Status Management (Compatibility Layer)
    // ========================================================================
    
    /// Check if player has a status/power.
    pub fn has_status(&self, id: &str) -> bool {
        self.powers.has(id)
    }
    
    /// Get stacks of a status/power.
    pub fn get_status(&self, id: &str) -> i32 {
        self.powers.get(id)
    }
    
    /// Get strength for damage calculation (legacy alias).
    pub fn get_strength(&self) -> i32 {
        self.strength()
    }
    
    /// Check if weak.
    pub fn is_weak(&self) -> bool {
        self.powers.has(power_ids::WEAK)
    }
    
    // ========================================================================
    // Temporary Buff Accessors (Vigor, DoubleTap, Burst, etc.)
    // ========================================================================
    
    /// Get current vigor stacks.
    pub fn vigor(&self) -> i32 {
        self.powers.get(power_ids::VIGOR)
    }
    
    /// Consume vigor and return the amount (for attack damage).
    pub fn consume_vigor(&mut self) -> i32 {
        let v = self.powers.get(power_ids::VIGOR);
        if v > 0 {
            self.powers.remove(power_ids::VIGOR);
        }
        v
    }
    
    /// Get current double tap stacks.
    pub fn double_tap(&self) -> i32 {
        self.powers.get(power_ids::DOUBLE_TAP)
    }
    
    /// Consume one DoubleTap stack, returns true if attack should be doubled.
    pub fn consume_double_tap(&mut self) -> bool {
        if self.powers.get(power_ids::DOUBLE_TAP) > 0 {
            self.powers.remove_stacks(power_ids::DOUBLE_TAP, 1);
            true
        } else {
            false
        }
    }
    
    /// Check if next attack should be doubled (DoubleTap) - consumes a stack.
    pub fn should_double_attack(&mut self) -> bool {
        self.consume_double_tap()
    }
    
    /// Get current burst stacks.
    pub fn burst(&self) -> i32 {
        self.powers.get(power_ids::BURST)
    }
    
    /// Consume one Burst stack, returns true if skill should be doubled.
    pub fn consume_burst(&mut self) -> bool {
        if self.powers.get(power_ids::BURST) > 0 {
            self.powers.remove_stacks(power_ids::BURST, 1);
            true
        } else {
            false
        }
    }
    
    /// Check if next skill should be doubled (Burst) - consumes a stack.
    pub fn should_double_skill(&mut self) -> bool {
        self.consume_burst()
    }
    
    /// Get intangible stacks.
    pub fn intangible(&self) -> i32 {
        self.powers.get(power_ids::INTANGIBLE)
    }
    
    /// Get blur stacks.
    pub fn blur(&self) -> i32 {
        self.powers.get(power_ids::BLUR)
    }
    
    /// Check if block should be retained (Blur active).
    pub fn should_retain_block(&self) -> bool {
        self.powers.has(power_ids::BLUR) || self.powers.has(power_ids::BARRICADE)
    }
    
    /// Check if player has NoDraw.
    pub fn has_no_draw(&self) -> bool {
        self.powers.has(power_ids::NO_DRAW)
    }
    
    /// Get duplication stacks.
    pub fn duplication(&self) -> i32 {
        self.powers.get(power_ids::DUPLICATION)
    }
    
    /// Get free attack stacks.
    pub fn free_attack(&self) -> i32 {
        self.powers.get(power_ids::FREE_ATTACK)
    }
    
    /// Get rebound stacks.
    pub fn rebound(&self) -> i32 {
        self.powers.get(power_ids::REBOUND)
    }
    
    /// Apply intangible reduction: all damage becomes 1.
    pub fn apply_intangible(&self, damage: i32) -> i32 {
        if self.intangible() > 0 && damage > 0 {
            1
        } else {
            damage
        }
    }
    
    /// End of turn: decrement turn-based buffs.
    pub fn end_turn_decrement(&mut self) {
        // Decrement Blur
        if self.powers.has(power_ids::BLUR) {
            self.powers.remove_stacks(power_ids::BLUR, 1);
        }
        // Decrement Intangible
        if self.powers.has(power_ids::INTANGIBLE) {
            self.powers.remove_stacks(power_ids::INTANGIBLE, 1);
        }
        // Clear NoDraw
        self.powers.remove(power_ids::NO_DRAW);
        // Clear Rebound
        self.powers.remove(power_ids::REBOUND);
    }
    
    // ========================================================================
    // Status Application (Compatibility Layer)
    // ========================================================================
    
    /// Apply a permanent status/buff (Strength, Dexterity, Vulnerable, etc.).
    ///
    /// For negative amounts (e.g., No Draw = -1, Barricade = -1), use set()
    /// since these are Java-style flag powers where -1 means "present".
    pub fn apply_status(&mut self, status: &str, stacks: i32) {
        if stacks < 0 {
            // Flag powers: Java uses amount=-1 for non-stackable powers
            self.powers.set(status, stacks);
        } else {
            self.powers.apply(status, stacks, None);
        }
    }
    
    /// Apply a temporary buff (Vigor, DoubleTap, Blur, etc.).
    /// Delegates to apply_status for consistent handling of negative amounts.
    pub fn apply_temp_buff(&mut self, buff: &str, amount: i32) {
        self.apply_status(buff, amount);
    }
    
    /// Gain block.
    pub fn gain_block(&mut self, amount: i32) {
        // Route through power hooks for Dexterity, Frail, NoBlock, etc.
        let modified = crate::power_hooks::calculate_block_hooked(amount, &self.powers);
        self.block += modified;
    }
    
    /// Take damage, accounting for block and intangible.
    pub fn take_damage(&mut self, mut damage: i32) -> i32 {
        // Apply intangible: all damage reduced to 1
        damage = self.apply_intangible(damage);
        
        let blocked = damage.min(self.block);
        self.block -= blocked;
        let actual_damage = damage - blocked;
        
        self.current_hp = (self.current_hp - actual_damage).max(0);
        actual_damage
    }
    
    /// Remove stacks of a buff. If `all` is true, remove all stacks.
    /// Returns the number of stacks actually removed.
    pub fn remove_buff(&mut self, buff: &str, amount: i32, all: bool) -> i32 {
        if all {
            self.powers.remove(buff)
        } else {
            self.powers.remove_stacks(buff, amount) as i32
        }
    }
}

// ============================================================================
// Card Trigger System
// ============================================================================



/// An effect to execute at end of turn (e.g., Flex's "lose Strength at end of turn").
#[derive(Debug, Clone)]
pub enum EndOfTurnEffect {
    /// Remove stacks of a buff at end of turn
    LoseBuff { buff: String, amount: i32, all: bool },
}

/// The complete game state for a combat encounter.
#[derive(Debug, Clone)]
pub struct GameState {
    // === Persistent Deck (survives across combats) ===
    /// The player's permanent card collection. Modified by rewards, events, shops.
    /// Java: `AbstractPlayer.masterDeck`. At combat start, this is copied into draw_pile.
    /// Temporary combat cards (Shiv, Wound, Dead Branch cards) never enter this deck.
    pub master_deck: Vec<CardInstance>,
    
    // === Combat Card Piles (temporary, rebuilt each combat from master_deck) ===
    /// Cards currently in hand.
    pub hand: SmallVec<[CardInstance; MAX_HAND_SIZE]>,
    /// Cards in draw pile.
    pub draw_pile: SmallVec<[CardInstance; TYPICAL_DECK_SIZE]>,
    /// Cards in discard pile.
    pub discard_pile: SmallVec<[CardInstance; TYPICAL_DECK_SIZE]>,
    /// Exhausted cards (removed for this combat).
    pub exhaust_pile: SmallVec<[CardInstance; 16]>,
    
    // === Combatants ===
    pub player: Player,
    pub enemies: SmallVec<[Enemy; 4]>,
    
    // === Combat tracking ===
    pub turn: u32,
    pub cards_played_this_turn: u32,
    
    // === Card play modifiers (for DoubleTap, Burst, etc.) ===
    pub card_modifiers: CardPlayModifiers,
    
    // === Last action tracking (for conditionals) ===
    /// Did the last DealDamage kill the target?
    pub last_attack_killed: bool,
    /// Damage dealt by last attack (before block)
    pub last_attack_damage: i32,
    /// Unblocked damage from last attack
    pub last_unblocked_damage: i32,
    
    // === Turn control ===
    /// If true, the turn ends immediately (used by Vault)
    pub end_turn_requested: bool,
    
    // === Card context ===
    /// ID of the card currently being played (for AddCard "this card")
    pub last_played_card_id: Option<String>,
    /// Cost of the card currently being played (for WristBlade relic)
    pub last_played_card_cost: i32,
    /// Index of the enemy being targeted by the current card (None = first alive)
    pub target_enemy_idx: Option<usize>,
    
    /// End-of-turn effects to execute (e.g., Flex's LoseBuff)
    pub end_of_turn_effects: Vec<EndOfTurnEffect>,
    
    /// X-cost value for the current card being played (Whirlwind, etc.).
    /// Set to the player's current energy when an X-cost card (cost == -1) is played.
    /// Reset to 0 after card execution. Commands like DealDamageAll use this as hit count.
    pub x_cost_value: i32,
    
    // === Relics ===
    /// Player's relics (passive items that trigger on events)
    pub relics: Vec<RelicInstance>,
    
    // === Orbs (Defect) ===
    /// Current orb slots (channeled orbs). Index 0 = leftmost (evoked first).
    pub orb_slots: Vec<crate::core::orbs::OrbSlot>,
    /// Maximum number of orb slots (default 3 for Defect).
    pub max_orbs: usize,
    /// Count of Frost orbs channeled this combat (for Blizzard card).
    /// Java: AbstractDungeon.actionManager.orbsChanneledThisCombat (filtered to Frost).
    pub frost_channeled_this_combat: i32,
    
    // === Map & Run Progression ===
    /// Current act map (procedurally generated)
    pub map: Option<SimpleMap>,
    /// Current position on the map (node index)
    pub current_map_node: Option<usize>,
    /// Current act (1, 2, 3, or 4 for Heart route)
    pub act: u8,
    /// Current floor within the act (0-14 per act, continues across acts: 1-50+)
    pub floor: u8,
    /// Number of combats completed in the current act (for encounter pool selection).
    /// Resets to 0 when entering a new act.
    pub combat_count: u8,
    /// Ascension level (0-20, affects difficulty modifiers)
    pub ascension_level: u8,
    /// Run seed for deterministic map generation
    pub run_seed: u64,
    /// Whether the boss of the current act has been defeated (awaiting act transition)
    pub boss_defeated: bool,
    
    // === Meta-Scaling RNG (Pseudo-RNG / Pity Timers) ===
    /// Potion drop chance (starts at 40%, +/-10% per combat). Resets to 40 at Act start.
    pub potion_drop_chance: i32,
    /// Rare card offset (starts at -5, +1 per common, resets to -5 on rare). Resets at Act start.
    pub rare_card_offset: i32,
    /// Floor number (1-indexed within the run)
    pub floor_num: i32,
    /// Gold held by the player
    pub gold: i32,
    /// Flag indicating combat rewards are pending (agent should call get_rewards)
    pub rewards_pending: bool,
    /// Current rewards available for selection (populated after combat)
    pub current_rewards: Vec<crate::rewards::RewardType>,
    
    // === Shop & Campfire ===
    /// Current shop state (if in a shop node)
    pub shop_state: Option<ShopState>,
    /// Number of times the player has purged cards (affects purge cost)
    pub purge_count: i32,
    /// Campfire relic state tracking (Girya uses, Shovel used, etc.)
    pub campfire_state: CampfireRelicState,
    
    // === Event System ===
    /// Active event state (when in Event screen)
    pub event_state: Option<ActiveEventState>,
    /// Events already seen this run (for one-time events)
    pub seen_events: Vec<String>,
    
    // === Potions ===
    /// Player's potion slots (default: 3 slots, 2 at A11+)
    pub potions: PotionSlots,
    
    // === Card Selection (for event/shop card selections) ===
    /// Current card selection action (Remove, Transform, Upgrade, etc.)
    pub card_select_action: Option<CardSelectAction>,
    /// Number of cards to pick in current selection
    pub card_select_count: i32,
    /// Card pool for selection (indexes into draw_pile)
    pub card_select_pool: Vec<usize>,
    /// Filter for card selection
    pub card_select_filter: Option<EventCardFilter>,
    
    // === Game Phase / Screen ===
    /// Current game screen (Map, Combat, Reward, Shop, Rest, Event, GameOver).
    /// Determines what actions are valid for the RL agent.
    pub screen: GamePhase,
    
    // === RNG ===
    /// Main RNG for combat, card draws, etc.
    pub rng: Xoshiro256StarStar,
    /// Separate RNG for encounter selection (spawning monsters based on floor).
    pub encounter_rng: Xoshiro256StarStar,
    
    // === Card Library (for runtime card generation) ===
    /// Player's class color (determines which cards appear in rewards/generation).
    /// Matches Java's `AbstractPlayer.getCardColor()` — Ironclad=Red, Silent=Green, etc.
    pub player_class: CardColor,
    
    /// Optional reference to CardLibrary for CreateRandomCardInHand (Magnetism/HelloWorld/CreativeAI).
    /// None = random card generation falls back to no-op.
    #[allow(dead_code)]
    pub card_library: Option<std::sync::Arc<crate::loader::CardLibrary>>,
}

impl GameState {
    /// Create a new game state with a seed for deterministic RNG.
    pub fn new(seed: u64) -> Self {
        Self {
            master_deck: Vec::new(),
            
            hand: SmallVec::new(),
            draw_pile: SmallVec::new(),
            discard_pile: SmallVec::new(),
            exhaust_pile: SmallVec::new(),
            
            player: Player::new(80, 3), // Default Ironclad stats
            enemies: SmallVec::new(),
            
            turn: 0,
            cards_played_this_turn: 0,
            
            card_modifiers: CardPlayModifiers::default(),
            
            last_attack_killed: false,
            last_attack_damage: 0,
            last_unblocked_damage: 0,
            
            end_turn_requested: false,
            last_played_card_id: None,
            last_played_card_cost: 0,
            target_enemy_idx: None,
            end_of_turn_effects: Vec::new(),
            x_cost_value: 0,
            
            relics: Vec::new(),
            
            orb_slots: Vec::new(),
            max_orbs: 3, // Defect default
            frost_channeled_this_combat: 0,
            
            map: None,
            current_map_node: None,
            act: 1,
            floor: 0,
            combat_count: 0,  // Resets each Act
            ascension_level: 0,
            run_seed: seed,
            boss_defeated: false,
            
            // Meta-scaling RNG fields (pity timers)
            potion_drop_chance: 40,  // 40% base, resets at Act start
            rare_card_offset: -5,    // Starts at -5, +1 per common, -5 on rare
            floor_num: 0,
            gold: 99,                // Starting gold
            rewards_pending: false,
            current_rewards: Vec::new(),
            
            // Shop & Campfire
            shop_state: None,
            purge_count: 0,
            campfire_state: CampfireRelicState::default(),
            
            // Event system
            event_state: None,
            seen_events: Vec::new(),
            
            // Potions (3 slots normally, 2 at A11+)
            potions: PotionSlots::new(3),
            
            // Card selection
            card_select_action: None,
            card_select_count: 0,
            card_select_pool: Vec::new(),
            card_select_filter: None,
            
            // Game phase - start on Map
            screen: GamePhase::Map,
            
            // Initialize both RNGs from the same seed (offset for encounter)
            rng: Xoshiro256StarStar::seed_from_u64(seed),
            encounter_rng: Xoshiro256StarStar::seed_from_u64(seed.wrapping_add(0x1234_5678)),
            
            // Player class (default Ironclad)
            player_class: CardColor::Red,
            
            // Card library (set externally when available)
            card_library: None,
        }
    }
    
    /// Create a test state with a mock enemy.
    pub fn new_test(seed: u64) -> Self {
        let mut state = Self::new(seed);
        state.enemies.push(MonsterState::new_simple("Test Dummy", 50));
        state
    }
    
    /// Set the ascension level and apply ascension-specific adjustments.
    ///
    /// Key ascension effects:
    /// - A11+: Only 2 potion slots instead of 3
    /// - A6+: Heal less between combats (handled elsewhere)
    /// - A15+: Enemies hit harder (handled in monster logic)
    pub fn set_ascension(&mut self, level: u8) {
        self.ascension_level = level;
        
        // A11+: Reduce potion slots from 3 to 2
        if level >= 11 {
            self.potions = PotionSlots::new(2);
        }
    }
    
    /// Record the result of an attack for conditional checks.
    pub fn record_attack_result(&mut self, damage: i32, unblocked: i32, killed: bool) {
        self.last_attack_damage = damage;
        self.last_unblocked_damage = unblocked;
        self.last_attack_killed = killed;
    }
    
    /// Check if the last attack was fatal.
    pub fn was_last_attack_fatal(&self) -> bool {
        self.last_attack_killed
    }
    
    /// Obtain a new card into the master deck (rewards, events, shops).
    /// Java: ShowCardAndObtainEffect → masterDeck.addToTop(card)
    /// 
    /// This adds to the PERSISTENT master_deck, not the combat draw_pile.
    /// Egg relics trigger on acquisition (Java: onObtainCard).
    pub fn obtain_card(&mut self, mut card: CardInstance) {
        // Omamori: Negate curse gains (counter starts at 2)
        // Java: Omamori.onObtainCard → if curse and counter > 0, decrement and cancel
        if card.card_type == crate::core::schema::CardType::Curse {
            if let Some(omamori) = self.relics.iter_mut().find(|r| r.id == "Omamori" && r.active) {
                if omamori.counter > 0 {
                    omamori.counter -= 1;
                    game_log!("  🎋 Omamori: Negated curse {}! ({} uses left)", card.definition_id, omamori.counter);
                    if omamori.counter == 0 {
                        omamori.active = false; // Used up
                    }
                    return; // Card NOT added to deck
                }
            }
        }
        
        // Egg relics: auto-upgrade cards on acquisition (Java: onObtainCard)
        if !card.upgraded {
            let should_upgrade = match card.card_type {
                crate::core::schema::CardType::Power => 
                    self.relics.iter().any(|r| r.id == "FrozenEgg" || r.id == "Frozen Egg 2"),
                crate::core::schema::CardType::Attack =>
                    self.relics.iter().any(|r| r.id == "MoltenEgg" || r.id == "Molten Egg 2"),
                crate::core::schema::CardType::Skill =>
                    self.relics.iter().any(|r| r.id == "ToxicEgg" || r.id == "Toxic Egg 2"),
                _ => false,
            };
            if should_upgrade {
                card.upgraded = true;
                game_log!("  🥚 Egg: auto-upgraded {}", card.definition_id);
            }
        }
        let obtained_card_type = card.card_type;
        self.master_deck.push(card);
        
        // CeramicFish: +9 gold whenever a card is obtained.
        // Java: CeramicFish.onObtainCard() → player.gainGold(9)
        // TODO(future): generalize onObtainCard hook if more relics need it
        if self.relics.iter().any(|r| r.id == "CeramicFish" && r.active) {
            self.gold += 9;
            game_log!("  🐟 Ceramic Fish: +9 gold");
        }
        
        // DarkstonePeriapt: +6 Max HP whenever a Curse is obtained.
        // Java: DarkstonePeriapt.onObtainCard() → if card.color == CURSE, increaseMaxHp(6)
        if obtained_card_type == crate::core::schema::CardType::Curse {
            if self.relics.iter().any(|r| (r.id == "DarkstonePeriapt" || r.id == "Darkstone Periapt") && r.active) {
                self.player.max_hp += 6;
                self.player.current_hp += 6;
                game_log!("  🔮 Darkstone Periapt: +6 Max HP (curse obtained)");
            }
        }
    }
    
    /// Remove a card from the master deck by index (shops, events, Peace Pipe).
    /// Java: masterDeck.removeCard(card)
    pub fn remove_card_from_deck(&mut self, index: usize) -> Option<CardInstance> {
        if index < self.master_deck.len() {
            Some(self.master_deck.remove(index))
        } else {
            None
        }
    }
    
    /// Initialize combat card piles from master_deck.
    /// Java: drawPile.initializeDeck(masterDeck) — CardGroup.java:911-938
    /// 
    /// 1. Clears all combat piles (hand, draw, discard, exhaust)
    /// 2. Copies master_deck into draw_pile (cards are cloned, not moved)
    /// 3. Resets all card costs to base_cost (undo any combat-only modifications)
    /// 4. Shuffles draw pile
    /// 5. Moves Innate cards to the top of draw pile (drawn first)
    pub fn initialize_combat_deck(&mut self) {
        // 1. Clear all combat piles
        self.hand.clear();
        self.draw_pile.clear();
        self.discard_pile.clear();
        self.exhaust_pile.clear();
        
        // 2. Copy master_deck into draw_pile with cost reset
        for card in &self.master_deck {
            let mut combat_card = card.clone();
            combat_card.reset_cost_for_turn(); // Ensure clean cost state
            self.draw_pile.push(combat_card);
        }
        
        // 3. Shuffle
        self.shuffle_draw_pile();
        
        // 4. Move Innate cards to top of draw pile (Java: placeOnTop)
        // Innate cards are drawn first. In Java, they go on top AFTER shuffling.
        let mut innate_cards = Vec::new();
        self.draw_pile.retain(|c| {
            if c.is_innate {
                innate_cards.push(c.clone());
                false
            } else {
                true
            }
        });
        // Append innate cards to the end (= top of draw pile, since we draw from end)
        for card in innate_cards {
            self.draw_pile.push(card);
        }
        
        // 5. Reset combat state
        self.turn = 0;
        self.cards_played_this_turn = 0;
        self.end_of_turn_effects.clear();
        self.card_modifiers = CardPlayModifiers::default();
        self.last_attack_killed = false;
        self.last_attack_damage = 0;
        self.last_unblocked_damage = 0;
        self.end_turn_requested = false;
        self.last_played_card_id = None;
        self.target_enemy_idx = None;
        self.x_cost_value = 0;
        
        game_log!("  📚 Initialized combat deck: {} cards from master deck", self.draw_pile.len());
    }
    
    /// Legacy: Add a card directly to the draw pile during combat.
    /// Use this for TEMPORARY combat cards (Shiv, Wound, Dead Branch, etc.)
    /// that should NOT persist beyond the current combat.
    #[deprecated(note = "Use obtain_card() for persistent deck additions, or add_temp_card_to_draw_pile() for combat-only cards")]
    pub fn add_to_deck(&mut self, card: CardInstance) {
        self.draw_pile.push(card);
    }
    
    /// Add a temporary card to the draw pile during combat.
    /// These cards will NOT persist to the next combat (they never enter master_deck).
    pub fn add_temp_card_to_draw_pile(&mut self, card: CardInstance) {
        self.draw_pile.push(card);
    }
    
    /// Shuffle the draw pile using the seeded RNG.
    pub fn shuffle_draw_pile(&mut self) {
        use rand::seq::SliceRandom;
        self.draw_pile.shuffle(&mut self.rng);
    }
    
    // ========================================================================
    // Pile Manipulation Helpers (Phase 2.6)
    // ========================================================================
    
    /// Get the size of a pile by location.
    pub fn pile_len(&self, location: CardLocation) -> usize {
        match location {
            CardLocation::Hand => self.hand.len(),
            CardLocation::DrawPile => self.draw_pile.len(),
            CardLocation::DiscardPile => self.discard_pile.len(),
            CardLocation::ExhaustPile => self.exhaust_pile.len(),
            CardLocation::Deck => self.hand.len() + self.draw_pile.len() + self.discard_pile.len(),
        }
    }
    
    /// Reshuffle discard pile into draw pile.
    pub fn reshuffle_discard_into_draw(&mut self) {
        while let Some(card) = self.discard_pile.pop() {
            self.draw_pile.push(card);
        }
        self.shuffle_draw_pile();
    }
    
    /// Remove a card from a pile by index, returning it.
    pub fn remove_from_pile(&mut self, location: CardLocation, index: usize) -> Option<CardInstance> {
        match location {
            CardLocation::Hand => {
                if index < self.hand.len() {
                    Some(self.hand.remove(index))
                } else {
                    None
                }
            }
            CardLocation::DrawPile => {
                if index < self.draw_pile.len() {
                    Some(self.draw_pile.remove(index))
                } else {
                    None
                }
            }
            CardLocation::DiscardPile => {
                if index < self.discard_pile.len() {
                    Some(self.discard_pile.remove(index))
                } else {
                    None
                }
            }
            CardLocation::ExhaustPile => {
                if index < self.exhaust_pile.len() {
                    Some(self.exhaust_pile.remove(index))
                } else {
                    None
                }
            }
            CardLocation::Deck => {
                panic!("Cannot remove from Deck directly; use specific pile")
            }
        }
    }
    
    /// Add a card to a pile at a specific position.
    pub fn add_to_pile(&mut self, card: CardInstance, location: CardLocation, position: InsertPosition) {
        match location {
            CardLocation::Hand => {
                match position {
                    InsertPosition::Top => self.hand.insert(0, card),
                    InsertPosition::Bottom | InsertPosition::Shuffle => self.hand.push(card),
                    InsertPosition::Random => {
                        use rand::Rng;
                        let idx = if self.hand.is_empty() { 0 } else { self.rng.random_range(0..=self.hand.len()) };
                        self.hand.insert(idx, card);
                    }
                }
            }
            CardLocation::DrawPile => {
                match position {
                    InsertPosition::Top => self.draw_pile.push(card), // Top = end (we pop from end)
                    InsertPosition::Bottom => self.draw_pile.insert(0, card),
                    InsertPosition::Random => {
                        use rand::Rng;
                        let idx = if self.draw_pile.is_empty() { 0 } else { self.rng.random_range(0..=self.draw_pile.len()) };
                        self.draw_pile.insert(idx, card);
                    }
                    InsertPosition::Shuffle => {
                        self.draw_pile.push(card);
                        self.shuffle_draw_pile();
                    }
                }
            }
            CardLocation::DiscardPile => {
                self.discard_pile.push(card);
            }
            CardLocation::ExhaustPile => {
                self.exhaust_pile.push(card);
            }
            CardLocation::Deck => {
                // Deck insert not meaningful; default to discard pile
                self.discard_pile.push(card);
            }
        }
    }
    
    /// Move a card from one pile to another.
    /// Returns true if successful.
    pub fn move_card(&mut self, from: CardLocation, to: CardLocation, index: usize, position: InsertPosition) -> bool {
        if let Some(card) = self.remove_from_pile(from, index) {
            self.add_to_pile(card, to, position);
            true
        } else {
            false
        }
    }
    
    /// Get indices of cards in a pile that match a filter.
    pub fn filter_pile(&self, location: CardLocation, filter: &CardFilter, library: Option<&CardLibrary>) -> Vec<usize> {
        let pile: &[CardInstance] = match location {
            CardLocation::Hand => &self.hand,
            CardLocation::DrawPile => &self.draw_pile,
            CardLocation::DiscardPile => &self.discard_pile,
            CardLocation::ExhaustPile => &self.exhaust_pile,
            CardLocation::Deck => &self.hand, // Deck filtering defaults to hand
        };
        
        pile.iter()
            .enumerate()
            .filter(|(_, card)| filter.matches(card, library))
            .map(|(idx, _)| idx)
            .collect()
    }
    
    /// Select N random indices from a pile (optionally filtered).
    pub fn select_random_indices(&mut self, location: CardLocation, count: usize, filter: Option<&CardFilter>, library: Option<&CardLibrary>) -> Vec<usize> {
        let candidates: Vec<usize> = if let Some(f) = filter {
            self.filter_pile(location, f, library)
        } else {
            (0..self.pile_len(location)).collect()
        };
        
        if candidates.is_empty() {
            return Vec::new();
        }
        
        let mut selected = candidates;
        selected.shuffle(&mut self.rng);
        selected.truncate(count);
        // Sort descending so we can remove from end first without index shifting
        selected.sort_by(|a, b| b.cmp(a));
        selected
    }
    
    /// Discard cards from hand.
    /// Returns the number of cards discarded.
    pub fn discard_cards(&mut self, count: i32, mode: SelectMode, filter: Option<&CardFilter>, library: Option<&CardLibrary>) -> i32 {
        let target_count = if count < 0 { self.hand.len() } else { count as usize };
        
        let indices_to_discard = match mode {
            SelectMode::All => {
                // Discard all matching cards
                if let Some(f) = filter {
                    self.filter_pile(CardLocation::Hand, f, library)
                } else {
                    (0..self.hand.len()).collect()
                }
            }
            SelectMode::Random | SelectMode::Choose => {
                // For Choose, default to Random in Phase 2 (will be agent decision later)
                if mode == SelectMode::Choose {
                    game_log!("    [TODO: Agent Input - defaulting to random discard]");
                }
                self.select_random_indices(CardLocation::Hand, target_count, filter, library)
            }
            SelectMode::Top => {
                // Top N from hand
                let candidates: Vec<usize> = if let Some(f) = filter {
                    self.filter_pile(CardLocation::Hand, f, library)
                } else {
                    (0..self.hand.len()).collect()
                };
                candidates.into_iter().take(target_count).collect()
            }
            SelectMode::Bottom => {
                // Bottom N from hand
                let candidates: Vec<usize> = if let Some(f) = filter {
                    self.filter_pile(CardLocation::Hand, f, library)
                } else {
                    (0..self.hand.len()).collect()
                };
                let len = candidates.len();
                candidates.into_iter().skip(len.saturating_sub(target_count)).collect()
            }
        };
        
        // Sort descending to remove from end first
        let mut indices: Vec<usize> = indices_to_discard;
        indices.sort_by(|a, b| b.cmp(a));
        
        let mut discarded = 0;
        for idx in indices {
            if let Some(card) = self.remove_from_pile(CardLocation::Hand, idx) {
                self.discard_pile.push(card);
                discarded += 1;
            }
        }
        
        discarded
    }
    
    /// Move cards between piles with filtering.
    /// Returns the number of cards moved.
    pub fn move_cards(
        &mut self,
        from: CardLocation,
        to: CardLocation,
        count: i32,
        mode: SelectMode,
        position: InsertPosition,
        filter: Option<&CardFilter>,
        library: Option<&CardLibrary>,
    ) -> i32 {
        let target_count = if count < 0 { self.pile_len(from) } else { count as usize };
        
        let indices_to_move = match mode {
            SelectMode::All => {
                if let Some(f) = filter {
                    self.filter_pile(from, f, library)
                } else {
                    (0..self.pile_len(from)).collect()
                }
            }
            SelectMode::Random | SelectMode::Choose => {
                if mode == SelectMode::Choose {
                    game_log!("    [TODO: Agent Input - defaulting to random selection]");
                }
                self.select_random_indices(from, target_count, filter, library)
            }
            SelectMode::Top => {
                let pile_len = self.pile_len(from);
                let candidates: Vec<usize> = if let Some(f) = filter {
                    self.filter_pile(from, f, library)
                } else {
                    (0..pile_len).collect()
                };
                // "Top" for draw pile means the end (where we pop from)
                candidates.into_iter().rev().take(target_count).collect()
            }
            SelectMode::Bottom => {
                let candidates: Vec<usize> = if let Some(f) = filter {
                    self.filter_pile(from, f, library)
                } else {
                    (0..self.pile_len(from)).collect()
                };
                candidates.into_iter().take(target_count).collect()
            }
        };
        
        // Sort descending to remove from end first
        let mut indices: Vec<usize> = indices_to_move;
        indices.sort_by(|a, b| b.cmp(a));
        
        let mut moved = 0;
        for idx in indices {
            if let Some(card) = self.remove_from_pile(from, idx) {
                self.add_to_pile(card, to, position);
                moved += 1;
            }
        }
        
        moved
    }
    
    /// Add a new card (by ID) to a pile.
    /// Returns true if successful.
    pub fn add_card_by_id(&mut self, card_id: &str, cost: i32, to: CardLocation, position: InsertPosition) -> bool {
        let card = CardInstance::new(card_id.to_string(), cost);
        self.add_to_pile(card, to, position);
        true
    }
    
    /// Draw cards from draw pile to hand.
    pub fn draw_cards(&mut self, count: i32) -> i32 {
        // P0.4: No Draw power prevents all card drawing
        // Java: DrawCardAction.update() → if hasPower("No Draw"), flash and return
        if self.player.powers.has("NoDraw") {
            game_log!("  🚫 No Draw power active — cannot draw cards");
            return 0;
        }
        
        let mut drawn = 0;
        let mut extra_draws = 0; // Queue extra draws from hooks (Evolve)
        
        for _ in 0..count {
            // Hand limit: Java caps at 10 cards (DrawCardAction checks hand.size() == 10)
            if self.hand.len() >= 10 {
                game_log!("  ✋ Hand is full (10 cards), cannot draw more");
                break;
            }
            // If draw pile is empty, shuffle discard into draw
            if self.draw_pile.is_empty() {
                if self.discard_pile.is_empty() {
                    break; // No cards left to draw
                }
                std::mem::swap(&mut self.draw_pile, &mut self.discard_pile);
                self.shuffle_draw_pile();
                
                // Pipeline hook: Sundial + Abacus on shuffle
                self.relic_on_shuffle();
            }
            
            if let Some(mut card) = self.draw_pile.pop() {
                // Fire on_card_draw power hooks for ALL powers
                let card_type_str = match card.card_type {
                    crate::schema::CardType::Attack => "Attack",
                    crate::schema::CardType::Skill => "Skill",
                    crate::schema::CardType::Power => "Power",
                    crate::schema::CardType::Status => "Status",
                    crate::schema::CardType::Curse => "Curse",
                };
                
                // Check NoDraw before firing hooks (Evolve checks this)
                let has_no_draw = self.player.powers.has("NoDraw");
                
                let effects = crate::power_hooks::collect_on_card_draw_effects(
                    card_type_str, &self.player.powers
                );
                
                for effect in &effects {
                    match effect {
                        crate::power_hooks::HookEffect::SetSkillCostZero => {
                            // Corruption: Skill cards cost 0 for the turn
                            card.set_cost_for_turn(0);
                            game_log!("    💀 {} cost set to 0 (Corruption)", card.definition_id);
                        }
                        crate::power_hooks::HookEffect::DrawCards(n) => {
                            // Evolve: draw extra cards when Status drawn
                            if !has_no_draw {
                                extra_draws += n;
                                game_log!("    🧬 Evolve triggered — will draw {} extra", n);
                            }
                        }
                        crate::power_hooks::HookEffect::DamageAllEnemies(amount) => {
                            // FireBreathing: deal damage to all enemies
                            for enemy in self.enemies.iter_mut() {
                                if !enemy.is_dead() {
                                    let actual = enemy.take_damage(*amount);
                                    game_log!("    🔥 {} takes {} damage (Fire Breathing)", enemy.name, actual);
                                }
                            }
                        }
                        _ => {} // Other effects handled elsewhere
                    }
                }
                
                // Pipeline hook: SneckoEye — randomize card cost to 0-3 on draw
                if card.current_cost >= 0 && self.has_active_relic("SneckoEye") {
                    use rand::Rng;
                    card.current_cost = self.rng.random_range(0..4);
                }
                
                self.hand.push(card);
                drawn += 1;
            }
        }
        
        // Process queued extra draws (from Evolve)
        if extra_draws > 0 {
            drawn += self.draw_cards(extra_draws);
        }
        
        drawn
    }
    
    /// Resolve card_type for all cards in all zones from the CardLibrary.
    /// Call this at combat start to ensure all card type checks work correctly.
    pub fn resolve_all_card_types(&mut self, library: &crate::loader::CardLibrary) {
        for card in self.hand.iter_mut()
            .chain(self.draw_pile.iter_mut())
            .chain(self.discard_pile.iter_mut())
            .chain(self.exhaust_pile.iter_mut())
        {
            card.resolve_type(library);
        }
    }
    
    /// Discard a card from hand by index.
    pub fn discard_from_hand(&mut self, index: usize) -> Option<CardInstance> {
        if index < self.hand.len() {
            let card = self.hand.remove(index);
            self.discard_pile.push(card.clone());
            Some(card)
        } else {
            None
        }
    }
    
    /// Exhaust a card (remove from combat).
    pub fn exhaust_card(&mut self, card: CardInstance) {
        self.exhaust_pile.push(card);
    }
    
    /// Start a new turn.
    pub fn start_turn(&mut self) {
        self.turn += 1;
        self.cards_played_this_turn = 0;
        
        // Pipeline hook: IceCream energy carry-over
        self.player.energy = self.relic_modify_energy_reset(
            self.player.energy, self.player.max_energy
        );
        
        // Reset attack tracking
        self.last_attack_killed = false;
        self.last_attack_damage = 0;
        self.last_unblocked_damage = 0;
        
        // Pipeline hook: Calipers block decay
        if !self.player.should_retain_block() {
            self.player.block = self.relic_modify_block_decay(self.player.block);
        }
        
        // Reset temporary cost modifications (Java: resetAttributes)
        // This undoes setCostForTurn changes (e.g., Corruption, Enlightenment)
        // while preserving modifyCostForCombat changes (e.g., Madness)
        for card in self.hand.iter_mut()
            .chain(self.draw_pile.iter_mut())
            .chain(self.discard_pile.iter_mut())
        {
            card.reset_cost_for_turn();
        }
        
        // Pipeline hook: SneckoEye +2 draw
        let draw_count = self.relic_modify_draw_count(5);
        self.draw_cards(draw_count);
        
        // GamblingChip: discard any number of cards and draw that many (simplified AI version)
        // Java: atTurnStartPostDraw → GamblingChipAction (player chooses cards to discard)
        // AI heuristic: discard Strikes and other low-value cards, draw replacements
        if self.turn == 1 { // Only fires on first turn of combat (Java: atTurnStartPostDraw with activated flag)
            if let Some(chip) = self.relics.iter_mut().find(|r| r.id == "Gambling Chip" || r.id == "GamblingChip") {
                if chip.counter == 0 {
                    chip.counter = 1; // Mark as used this combat
                    // Java: GamblingChipAction opens card select (up to 99 cards, can skip).
                    // Player CHOOSES which cards to discard, then draws that many.
                    // AI heuristic: discard bad cards (Status, Curse, Wound, Dazed, Burn, etc.)
                    let mut discard_count = 0;
                    let mut i = 0;
                    while i < self.hand.len() {
                        let card = &self.hand[i];
                        let should_discard = match card.card_type {
                            crate::core::schema::CardType::Status | crate::core::schema::CardType::Curse => true,
                            _ => {
                                // Also discard unplayable status-like cards
                                let id = &card.definition_id;
                                id == "Wound" || id == "Dazed" || id == "Burn" || id == "Slimed" || id == "Void"
                                    || id == "AscendersBane"
                            }
                        };
                        if should_discard {
                            let card = self.hand.remove(i);
                            self.discard_pile.push(card);
                            discard_count += 1;
                        } else {
                            i += 1;
                        }
                    }
                    if discard_count > 0 {
                        self.draw_cards(discard_count);
                        game_log!("  🎲 GamblingChip: discarded {} bad cards, drew {} replacements", discard_count, discard_count);
                    }
                }
            }
        }
    }
    
    /// End the current turn.
    /// Returns the number of cards manually discarded (for ToughBandages, Tingsha relics).
    pub fn end_turn(&mut self) -> i32 {
        let mut discarded_count: i32 = 0;
        
        // Determine how many cards to retain
        let retain_count = {
            // Runic Pyramid: retain all cards
            if self.relics.iter().any(|r| r.id == "Runic Pyramid") {
                self.hand.len()
            // Equilibrium power: retain all cards (like Runic Pyramid)
            } else if self.player.powers.has("Equilibrium") {
                self.hand.len()
            } else {
                // WellLaidPlans: retain up to this.amount cards
                let wlp_stacks = self.player.powers.get("WellLaidPlans");
                if wlp_stacks > 0 { wlp_stacks as usize } else { 0 }
            }
        };
        
        // Discard hand — Ethereal cards are exhausted, retained cards stay
        if retain_count > 0 && retain_count < self.hand.len() {
            // AI heuristic: retain the highest-cost cards (they're most valuable)
            // Sort indices by cost descending, keep the top N
            let mut indices: Vec<usize> = (0..self.hand.len()).collect();
            indices.sort_by(|&a, &b| {
                let cost_a = self.hand[a].current_cost;
                let cost_b = self.hand[b].current_cost;
                cost_b.cmp(&cost_a) // descending
            });
            
            let retain_indices: std::collections::HashSet<usize> = 
                indices.into_iter().take(retain_count).collect();
            
            let mut retained = Vec::new();
            let hand_cards: Vec<_> = self.hand.drain(..).collect();
            
            for (i, card) in hand_cards.into_iter().enumerate() {
                if (retain_indices.contains(&i) || card.self_retain) && !card.is_ethereal {
                    game_log!("  🔒 Retained: {}", card.definition_id);
                    retained.push(card);
                } else if card.is_ethereal {
                    game_log!("  ✦ Ethereal '{}' exhausted (removed from game)", card.definition_id);
                    self.exhaust_pile.push(card);
                } else {
                    // Manual discard (end of turn)
                    discarded_count += 1;
                    self.discard_pile.push(card);
                }
            }
            
            // Put retained cards back in hand
            for card in retained {
                self.hand.push(card);
            }
        } else if retain_count >= self.hand.len() {
            // Retain all cards (Runic Pyramid or enough WLP stacks)
            // Still exhaust Ethereal cards
            let hand_cards: Vec<_> = self.hand.drain(..).collect();
            for card in hand_cards {
                if card.is_ethereal {
                    game_log!("  ✦ Ethereal '{}' exhausted (removed from game)", card.definition_id);
                    self.exhaust_pile.push(card);
                } else {
                    self.hand.push(card);
                }
            }
            // No manual discards when retaining
        } else {
            // No retain — discard everything (except selfRetain cards)
            let hand_cards: Vec<_> = self.hand.drain(..).collect();
            for card in hand_cards {
                if card.is_ethereal {
                    game_log!("  ✦ Ethereal '{}' exhausted (removed from game)", card.definition_id);
                    self.exhaust_pile.push(card);
                } else if card.self_retain {
                    game_log!("  🔒 Self-retained: {}", card.definition_id);
                    self.hand.push(card);
                } else {
                    discarded_count += 1;
                    self.discard_pile.push(card);
                }
            }
        }
        
        // Clear card play modifiers
        self.card_modifiers.clear();
        
        // Decrement temporary buffs
        self.player.end_turn_decrement();
        
        discarded_count
    }
    
    /// Get the first living enemy (for single-target attacks).
    pub fn get_target_enemy(&mut self) -> Option<&mut MonsterState> {
        if let Some(idx) = self.target_enemy_idx {
            self.enemies.get_mut(idx).filter(|e| !e.is_dead())
        } else {
            self.enemies.iter_mut().find(|e| !e.is_dead())
        }
    }
    
    /// Check if combat is won.
    pub fn combat_won(&self) -> bool {
        self.enemies.iter().all(|e| e.is_dead())
    }
    
    /// Check if combat is lost.
    pub fn combat_lost(&self) -> bool {
        self.player.current_hp <= 0
    }
    
    /// Take damage to the player, applying passive relic modifiers.
    /// Uses `relic_modify_damage_taken()` pipeline hook (Torii, TungstenRod).
    pub fn player_take_damage(&mut self, damage: i32) -> i32 {
        // Apply intangible first
        let damage = self.player.apply_intangible(damage);
        
        // Calculate block absorption
        let blocked = damage.min(self.player.block);
        self.player.block -= blocked;
        let unblocked = damage - blocked;
        
        // Pipeline hook: Torii + TungstenRod
        let unblocked = self.relic_modify_damage_taken(unblocked);
        
        self.player.current_hp = (self.player.current_hp - unblocked).max(0);
        unblocked
    }
    
    /// Apply a debuff to the player with relic immunity checks.
    /// Uses `relic_blocks_debuff()` pipeline hook (Ginger, Turnip).
    /// Artifact blocking is handled inside PowerSet.
    pub fn apply_player_debuff(&mut self, debuff: &str, stacks: i32) -> bool {
        // Pipeline hook: Ginger blocks Weak, Turnip blocks Frail
        if let Some(msg) = self.relic_blocks_debuff(debuff) {
            game_log!("  {}", msg);
            return false;
        }
        
        // Artifact: blocks the next debuff application
        // Java: AbstractCreature.addPower() → if Artifact > 0, flash Artifact, reduce by 1, skip
        let artifact_stacks = self.player.powers.get("Artifact");
        if artifact_stacks > 0 {
            self.player.powers.apply("Artifact", -1, None);
            game_log!("  🛡️ Artifact blocked {}! ({} remaining)", debuff, artifact_stacks - 1);
            if artifact_stacks - 1 <= 0 {
                self.player.powers.remove("Artifact");
            }
            return false;
        }
        
        // Apply normally
        self.player.apply_status(debuff, stacks);
        true
    }
    
    // ========================================================================
    // Static Pipeline Hooks — Relic Modifiers
    // ========================================================================
    //
    // Rust equivalent of Java's AbstractRelic virtual methods (onAttacked,
    // onLoseHpLast, onCardDraw, etc.), implemented as direct static lookups
    // for maximum MCTS performance (zero heap alloc, zero virtual dispatch).
    //
    // Each method corresponds to a Java pipeline interception point.
    // ========================================================================
    
    /// Check if the player has a specific relic (active or not).
    /// Fast O(N) linear scan — N is typically ≤ 15.
    #[inline]
    pub fn has_relic(&self, id: &str) -> bool {
        self.relics.iter().any(|r| r.id == id)
    }
    
    /// Check if the player has a specific ACTIVE relic.
    #[inline]
    pub fn has_active_relic(&self, id: &str) -> bool {
        self.relics.iter().any(|r| r.id == id && r.active)
    }
    
    /// Java: AbstractPlayer.gameHandSize (default 5, +2 for Snecko Eye, etc.)
    /// Called from start_turn() to determine how many cards to draw.
    pub fn relic_modify_draw_count(&self, base: i32) -> i32 {
        let mut count = base;
        if self.has_active_relic("SneckoEye") {
            count += 2;
        }
        // Future: BagOfMarbles, etc. if they affect draw count
        count
    }
    
    /// Java: Sundial.onShuffle() + Abacus.onShuffle()
    /// Called when discard pile is shuffled into draw pile.
    pub fn relic_on_shuffle(&mut self) {
        // Sundial: Every 3 shuffles → +2 energy
        if let Some(relic) = self.relics.iter_mut().find(|r| r.id == "Sundial" && r.active) {
            relic.counter += 1;
            if relic.counter >= 3 {
                relic.counter = 0;
                self.player.energy += 2;
                relic.pulse();
                game_log!("  ☀️ Sundial: +2 Energy (3rd shuffle)");
            }
        }
        // Abacus (TheAbacus): +6 Block on shuffle
        if self.has_active_relic("TheAbacus") {
            self.player.block += 6;
            if let Some(relic) = self.relics.iter_mut().find(|r| r.id == "TheAbacus") {
                relic.pulse();
            }
            game_log!("  🧮 Abacus: +6 Block (shuffle)");
        }
    }
    
    /// Java: IceCream.onEnergyRecharge() — preserve leftover energy.
    /// Returns the new energy value at turn start.
    pub fn relic_modify_energy_reset(&self, current_energy: i32, max_energy: i32) -> i32 {
        if self.has_relic("IceCream") || self.has_relic("Ice Cream") {
            // IceCream: carry over leftover + add base energy
            current_energy + max_energy
        } else {
            max_energy
        }
    }
    
    /// Java: Calipers — lose only 15 block instead of all.
    /// Barricade/Blur handled by player.should_retain_block().
    /// Returns the new block value after decay.
    pub fn relic_modify_block_decay(&self, current_block: i32) -> i32 {
        if self.has_relic("Calipers") && current_block > 0 {
            let preserved = (current_block - 15).max(0);
            if preserved > 0 {
                game_log!("  🔧 Calipers: preserved {} block", preserved);
            }
            preserved
        } else {
            0
        }
    }
    
    /// Java: Ginger blocks Weak, Turnip blocks Frail.
    /// Checked BEFORE Artifact in Java's ApplyPowerAction.
    /// Returns true if the debuff is blocked by a relic.
    pub fn relic_blocks_debuff(&self, debuff: &str) -> Option<&'static str> {
        if (debuff == "Weak" || debuff == "Weakened") 
            && self.has_active_relic("Ginger") 
        {
            Some("🫚 Ginger: Blocked Weak!")
        } else if debuff == "Frail" 
            && self.has_active_relic("Turnip") 
        {
            Some("🥕 Turnip: Blocked Frail!")
        } else {
            None
        }
    }
    
    /// Java: Torii.onAttacked() + TungstenRod.onLoseHpLast()
    /// Modifies unblocked damage after block absorption.
    /// Called from player_take_damage().
    pub fn relic_modify_damage_taken(&self, mut unblocked: i32) -> i32 {
        // Torii: unblocked damage 2-5 → 1
        if self.has_relic("Torii") && unblocked > 1 && unblocked <= 5 {
            game_log!("  ⛩️ Torii: reduced {} damage to 1", unblocked);
            unblocked = 1;
        }
        // TungstenRod: reduce HP loss by 1
        if self.has_relic("TungstenRod") && unblocked > 0 {
            unblocked = (unblocked - 1).max(0);
            game_log!("  🔧 TungstenRod: reduced damage by 1 (now {})", unblocked);
        }
        unblocked
    }
}

impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Game State (Turn {}) ===", self.turn)?;
        writeln!(f, "Player: {}/{} HP, {} Block, {} Energy", 
            self.player.current_hp, self.player.max_hp, 
            self.player.block, self.player.energy)?;
        
        if !self.player.powers.is_empty() {
            write!(f, "  Statuses: ")?;
            for (status, stacks) in self.player.powers.iter() {
                write!(f, "{}({}) ", status, stacks)?;
            }
            writeln!(f)?;
        }
        
        writeln!(f, "Hand: {} cards, Draw: {}, Discard: {}, Exhaust: {}",
            self.hand.len(), self.draw_pile.len(), 
            self.discard_pile.len(), self.exhaust_pile.len())?;
        
        for (i, enemy) in self.enemies.iter().enumerate() {
            writeln!(f, "Enemy {}: {} - {}/{} HP, {} Block",
                i, enemy.name, enemy.hp, enemy.max_hp, enemy.block)?;
            if !enemy.powers.is_empty() {
                write!(f, "  Statuses: ")?;
                for (status, stacks) in enemy.powers.iter() {
                    write!(f, "{}({}) ", status, stacks)?;
                }
                writeln!(f)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deterministic_rng() {
        let mut state1 = GameState::new(12345);
        let mut state2 = GameState::new(12345);
        
        // Add same cards
        for i in 0..10 {
            state1.add_to_deck(CardInstance::new(format!("card_{}", i), 1));
            state2.add_to_deck(CardInstance::new(format!("card_{}", i), 1));
        }
        
        state1.shuffle_draw_pile();
        state2.shuffle_draw_pile();
        
        // Should be identical after shuffle with same seed
        for (c1, c2) in state1.draw_pile.iter().zip(state2.draw_pile.iter()) {
            assert_eq!(c1.definition_id, c2.definition_id);
        }
    }
    
    #[test]
    fn test_vulnerability() {
        let mut enemy = MonsterState::new_simple("Test", 100);
        enemy.apply_status("Vulnerable", 2);
        
        // take_damage does NOT apply Vulnerable — that's in engine::calculate_card_damage()
        // (Java Phase A, not Phase B). take_damage only handles block + intangible.
        let damage = enemy.take_damage(10);
        assert_eq!(damage, 10); // raw damage, no Vulnerable multiplier here
        assert_eq!(enemy.hp, 90);
    }
}
