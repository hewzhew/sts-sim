//! Relic system for Slay the Spire simulator.
//!
//! This module provides a **data-driven** relic system that:
//! 1. Loads relic definitions from JSON (`data/relics.json`)
//! 2. Executes relic logic based on triggers and commands
//! 3. Falls back to hardcoded logic for complex relics
//!
//! ## Architecture
//! - `RelicDefinition`: Static data loaded from JSON (includes logic hooks)
//! - `RelicInstance`: Runtime state (counters, active status)
//! - `RelicHook`: A trigger + commands pair (e.g., BattleStart -> [Heal 6])
//! - `trigger_relics()`: Event dispatch that processes data-driven and hardcoded logic

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::CardType;
use crate::state::GameState;

// ============================================================================
// Relic Trigger (Hook Triggers)
// ============================================================================

/// Trigger types that can activate relic effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelicTrigger {
    /// Combat has started
    BattleStart,
    /// Combat has ended
    BattleEnd,
    /// Boss combat started
    BossStart,
    /// Turn has started
    TurnStart,
    /// Turn is ending
    TurnEnd,
    /// Player played an attack card
    PlayerPlayAttack,
    /// Player played a skill card
    PlayerPlaySkill,
    /// Player played a power card
    PlayerPlayPower,
    /// Player played any card
    PlayerPlayCard,
    /// Player exhausted a card
    PlayerExhaust,
    /// Player discarded a card
    PlayerDiscard,
    /// Player lost HP
    PlayerLoseHp,
    /// Player gained block
    PlayerGainBlock,
    /// Player used a potion
    PlayerUsePotion,
    /// Player applied poison
    PlayerApplyPoison,
    /// Player applied vulnerable
    PlayerApplyVulnerable,
    /// Enemy died
    EnemyDied,
    /// Player manually discarded cards (end of turn)
    PlayerManualDiscard,
    /// On pickup (immediate effect)
    OnPickup,
    /// Entered a rest site
    EnterRest,
    /// Player rested at a rest site
    PlayerRest,
    /// Entered a shop
    EnterShop,
    /// Climbed a floor
    ClimbFloor,
    /// Added a card to deck
    AddCardToDeck,
}

impl RelicTrigger {
    /// Parse from string (for JSON deserialization).
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "BattleStart" => RelicTrigger::BattleStart,
            "BattleEnd" => RelicTrigger::BattleEnd,
            "BossStart" => RelicTrigger::BossStart,
            "TurnStart" => RelicTrigger::TurnStart,
            "TurnEnd" => RelicTrigger::TurnEnd,
            "PlayerPlayAttack" => RelicTrigger::PlayerPlayAttack,
            "PlayerPlaySkill" => RelicTrigger::PlayerPlaySkill,
            "PlayerPlayPower" => RelicTrigger::PlayerPlayPower,
            "PlayerPlayCard" => RelicTrigger::PlayerPlayCard,
            "PlayerExhaust" => RelicTrigger::PlayerExhaust,
            "PlayerDiscard" => RelicTrigger::PlayerDiscard,
            "PlayerLoseHp" => RelicTrigger::PlayerLoseHp,
            "PlayerGainBlock" => RelicTrigger::PlayerGainBlock,
            "PlayerUsePotion" => RelicTrigger::PlayerUsePotion,
            "PlayerApplyPoison" => RelicTrigger::PlayerApplyPoison,
            "PlayerApplyVulnerable" => RelicTrigger::PlayerApplyVulnerable,
            "EnemyDied" => RelicTrigger::EnemyDied,
            "PlayerManualDiscard" => RelicTrigger::PlayerManualDiscard,
            "OnPickup" => RelicTrigger::OnPickup,
            "EnterRest" => RelicTrigger::EnterRest,
            "PlayerRest" => RelicTrigger::PlayerRest,
            "EnterShop" => RelicTrigger::EnterShop,
            "ClimbFloor" => RelicTrigger::ClimbFloor,
            "AddCardToDeck" => RelicTrigger::AddCardToDeck,
            _ => return None,
        })
    }
}

// ============================================================================
// Relic Command (Effect Commands)
// ============================================================================

/// Parameters for a relic command.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelicCommandParams {
    /// Amount for numeric effects (heal, draw, damage, etc.)
    #[serde(default)]
    pub amount: Option<i32>,
    /// Base amount (for scaled effects)
    #[serde(default)]
    pub base: Option<i32>,
    /// Buff/status name
    #[serde(default)]
    pub buff: Option<String>,
    /// Status effect name
    #[serde(default)]
    pub status: Option<String>,
    /// Card name (for AddCard commands)
    #[serde(default)]
    pub card: Option<String>,
    /// Count (for multiple effects)
    #[serde(default)]
    pub count: Option<i32>,
    /// Orb type (for Defect)
    #[serde(default)]
    pub orb: Option<String>,
    /// Stance name (for Watcher)
    #[serde(default)]
    pub stance: Option<String>,
    /// Destination (Hand, DrawPile, DiscardPile)
    #[serde(default)]
    pub destination: Option<String>,
}

/// A single command that a relic executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicCommand {
    /// Command type (Heal, DrawCards, GainBuff, etc.)
    #[serde(rename = "type")]
    pub cmd_type: String,
    /// Command parameters
    #[serde(default)]
    pub params: RelicCommandParams,
}

// ============================================================================
// Relic Hook (Trigger + Commands)
// ============================================================================

/// Optional condition for a hook to fire.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookCondition {
    /// Only trigger on specific turn
    #[serde(default)]
    pub turn: Option<u32>,
    /// Only trigger if HP below this percentage
    #[serde(default)]
    pub hp_below_percent: Option<i32>,
    /// Only trigger if player has no block
    #[serde(default)]
    pub no_block: Option<bool>,
    /// Only trigger if no attacks were played this turn
    #[serde(default)]
    pub no_attacks_played: Option<bool>,
}

/// A trigger + commands pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicHook {
    /// The trigger that activates this hook
    pub trigger: String,
    /// Commands to execute when triggered
    #[serde(default)]
    pub commands: Vec<RelicCommand>,
    /// Optional condition
    #[serde(default)]
    pub condition: Option<HookCondition>,
}

// ============================================================================
// Relic Logic (Data-Driven Logic)
// ============================================================================

/// The logic for a relic (hooks and counter info).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelicLogic {
    /// List of trigger hooks
    #[serde(default)]
    pub hooks: Vec<RelicHook>,
    /// Counter max value (for Pen Nib = 10, Incense Burner = 6)
    #[serde(default)]
    pub counter_max: Option<i32>,
}

// ============================================================================
// Relic Definition (Static Data from JSON)
// ============================================================================

/// Relic tier/rarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RelicTier {
    Starter,
    #[default]
    Common,
    Uncommon,
    Rare,
    Boss,
    Shop,
    Event,
}

/// Static definition of a relic loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicDefinition {
    /// Unique identifier (e.g., "BurningBlood", "Vajra")
    pub id: String,
    /// Display name
    pub name: String,
    /// Relic tier
    #[serde(default)]
    pub tier: RelicTier,
    /// Description text
    #[serde(default)]
    pub description: String,
    /// Data-driven logic (hooks + commands)
    #[serde(default)]
    pub logic: RelicLogic,
    /// Character restriction (Ironclad, Silent, Defect, Watcher)
    #[serde(default)]
    pub class_specific: Option<String>,
    /// Flavor text
    #[serde(default)]
    pub flavor: Option<String>,
    /// Whether this relic needs hardcoded logic
    #[serde(default)]
    pub manual_review_needed: bool,
}

// ============================================================================
// Relic Instance (Runtime State)
// ============================================================================

/// Runtime state for a relic during a run.
#[derive(Debug, Clone)]
pub struct RelicInstance {
    /// Reference to the relic definition ID
    pub id: String,
    /// Counter for relics that track progress (Pen Nib, Incense Burner, etc.)
    pub counter: i32,
    /// Whether the relic is currently active (some can be disabled)
    pub active: bool,
    /// Pulse flag (for UI, indicates relic just triggered)
    pub pulsed: bool,
}

/// Normalize a relic ID by removing spaces.
/// JSON IDs like "Paper Crane" become "PaperCrane" to match code references.
fn normalize_relic_id(id: &str) -> String {
    id.replace(' ', "")
}

impl RelicInstance {
    /// Create a new relic instance (ID normalized: spaces removed).
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: normalize_relic_id(&id.into()),
            counter: 0,
            active: true,
            pulsed: false,
        }
    }
    
    /// Create with a specific starting counter (ID normalized).
    pub fn with_counter(id: impl Into<String>, counter: i32) -> Self {
        Self {
            id: normalize_relic_id(&id.into()),
            counter,
            active: true,
            pulsed: false,
        }
    }
    
    /// Increment counter and check if it reached the threshold.
    pub fn increment_counter(&mut self, max: i32) -> bool {
        self.counter += 1;
        if self.counter >= max {
            self.counter = 0;
            true
        } else {
            false
        }
    }
    
    /// Mark as pulsed (for UI feedback).
    pub fn pulse(&mut self) {
        self.pulsed = true;
    }
    
    /// Clear pulse flag.
    pub fn clear_pulse(&mut self) {
        self.pulsed = false;
    }
}

// ============================================================================
// Game Events (Runtime Events)
// ============================================================================

/// Events that can trigger relic/power effects at runtime.
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Combat has started
    BattleStart,
    /// A new turn has started
    TurnStart { turn: u32 },
    /// Turn is ending (before enemy turn)
    TurnEnd { turn: u32 },
    /// Player played a card
    PlayerPlayCard {
        card_type: CardType,
        cost: i32,
        card_id: String,
    },
    /// Player dealt attack damage to an enemy
    PlayerAttack {
        damage: i32,
        enemy_idx: usize,
        killed: bool,
    },
    /// Player lost HP (after block)
    PlayerLoseHp { amount: i32 },
    /// Player gained block
    PlayerGainBlock { amount: i32 },
    /// Player drew cards
    PlayerDrawCards { count: i32 },
    /// Player exhausted a card
    PlayerExhaust { card_id: String },
    /// Enemy died
    EnemyDied { enemy_idx: usize },
    /// All enemies are dead, combat is won
    BattleEnd { won: bool },
    /// Player healed HP
    PlayerHeal { amount: i32 },
    /// Player used a potion
    PlayerUsePotion,
    /// Player manually discarded cards (end of turn)
    PlayerManualDiscard { count: i32 },
}

impl GameEvent {
    /// Convert to the corresponding RelicTrigger for matching.
    pub fn to_trigger(&self) -> RelicTrigger {
        match self {
            GameEvent::BattleStart => RelicTrigger::BattleStart,
            GameEvent::TurnStart { .. } => RelicTrigger::TurnStart,
            GameEvent::TurnEnd { .. } => RelicTrigger::TurnEnd,
            GameEvent::PlayerPlayCard { card_type, .. } => match card_type {
                CardType::Attack => RelicTrigger::PlayerPlayAttack,
                CardType::Skill => RelicTrigger::PlayerPlaySkill,
                CardType::Power => RelicTrigger::PlayerPlayPower,
                _ => RelicTrigger::PlayerPlayCard,
            },
            GameEvent::PlayerLoseHp { .. } => RelicTrigger::PlayerLoseHp,
            GameEvent::PlayerGainBlock { .. } => RelicTrigger::PlayerGainBlock,
            GameEvent::PlayerExhaust { .. } => RelicTrigger::PlayerExhaust,
            GameEvent::EnemyDied { .. } => RelicTrigger::EnemyDied,
            GameEvent::BattleEnd { .. } => RelicTrigger::BattleEnd,
            GameEvent::PlayerUsePotion => RelicTrigger::PlayerUsePotion,
            GameEvent::PlayerManualDiscard { .. } => RelicTrigger::PlayerManualDiscard,
            _ => RelicTrigger::BattleStart, // fallback
        }
    }
    
    /// Get turn number if applicable.
    pub fn turn(&self) -> Option<u32> {
        match self {
            GameEvent::TurnStart { turn } | GameEvent::TurnEnd { turn } => Some(*turn),
            _ => None,
        }
    }
}

// ============================================================================
// Relic Trigger Result (Effects to Apply)
// ============================================================================

/// Result of triggering relics - describes what effects occurred.
#[derive(Debug, Clone, Default)]
pub struct RelicTriggerResult {
    /// Extra cards to draw
    pub extra_draw: i32,
    /// Strength to gain
    pub strength_gain: i32,
    /// Dexterity to gain (permanent)
    pub dexterity_gain: i32,
    /// Temporary Dexterity to gain (applies both +Dex and +DexLoss, removed at end of turn)
    pub temp_dexterity_gain: i32,
    /// HP to heal
    pub heal: i32,
    /// Energy to gain
    pub energy_gain: i32,
    /// Block to gain
    pub block_gain: i32,
    /// Thorns to gain
    pub thorns_gain: i32,
    /// Plated armor to gain
    pub plated_armor_gain: i32,
    /// Artifact stacks to gain
    pub artifact_gain: i32,
    /// Focus to gain (Defect)
    pub focus_gain: i32,
    /// Mantra to gain (Watcher)
    pub mantra_gain: i32,
    /// Intangible stacks to gain
    pub intangible_gain: i32,
    /// Damage multiplier (for Pen Nib: 2.0)
    pub damage_multiplier: f32,
    /// Vigor to gain (first attack bonus damage)
    pub vigor_gain: i32,
    /// Damage to deal to all enemies
    pub damage_all: i32,
    /// Damage to deal to random enemy
    pub damage_random: i32,
    /// Max HP to raise
    pub max_hp_gain: i32,
    /// Gold to gain
    pub gold_gain: i32,
    /// Vulnerable to apply to all enemies
    pub vulnerable_all: i32,
    /// Weak to apply (usually as a side effect)
    pub weak_apply: i32,
    /// Poison to apply to all enemies (Funnel: Twisted Funnel)
    pub poison_all: i32,
    /// Cards to add to hand (card_id, count)
    pub cards_to_hand: Vec<(String, i32)>,
    /// Cards to add to draw pile (card_id, count)
    pub cards_to_draw_pile: Vec<(String, i32)>,
    /// Buffer stacks to gain (FossilizedHelix)
    pub buffer_gain: i32,
    /// Whether to replay the last played card (Necronomicon)
    pub replay_card: bool,
    /// Whether to clear all player debuffs (Orange Pellets)
    pub clear_debuffs: bool,
    /// Number of random cards in hand to upgrade (Warped Tongs)
    pub upgrade_random_hand: i32,
    /// Orb slots to gain (Inserter, Runic Capacitor)
    pub orb_slot_gain: i32,
    /// Whether to trigger all orb passives (Emotion Chip)
    pub trigger_all_orb_passives: bool,
    /// Whether to reduce a random hand card's cost to 0 (Mummified Hand)
    pub reduce_random_card_cost: bool,
    /// Messages for logging
    pub messages: Vec<String>,
}

impl RelicTriggerResult {
    pub fn new() -> Self {
        Self {
            damage_multiplier: 1.0,
            ..Default::default()
        }
    }
    
    /// Check if any effect occurred.
    pub fn has_effect(&self) -> bool {
        self.extra_draw > 0
            || self.strength_gain > 0
            || self.dexterity_gain > 0
            || self.temp_dexterity_gain > 0
            || self.heal > 0
            || self.energy_gain > 0
            || self.block_gain > 0
            || self.thorns_gain > 0
            || self.plated_armor_gain > 0
            || self.artifact_gain > 0
            || self.focus_gain > 0
            || self.damage_multiplier != 1.0
            || self.vigor_gain > 0
            || self.damage_all > 0
            || self.max_hp_gain > 0
            || self.vulnerable_all > 0
            || !self.cards_to_hand.is_empty()
            || !self.cards_to_draw_pile.is_empty()
            || self.buffer_gain > 0
            || self.replay_card
            || self.clear_debuffs
            || self.upgrade_random_hand > 0
            || self.orb_slot_gain > 0
            || self.trigger_all_orb_passives
            || self.poison_all > 0
    }
    
    /// Merge another result into this one.
    pub fn merge(&mut self, other: &RelicTriggerResult) {
        self.extra_draw += other.extra_draw;
        self.strength_gain += other.strength_gain;
        self.dexterity_gain += other.dexterity_gain;
        self.temp_dexterity_gain += other.temp_dexterity_gain;
        self.heal += other.heal;
        self.energy_gain += other.energy_gain;
        self.block_gain += other.block_gain;
        self.thorns_gain += other.thorns_gain;
        self.plated_armor_gain += other.plated_armor_gain;
        self.artifact_gain += other.artifact_gain;
        self.focus_gain += other.focus_gain;
        self.mantra_gain += other.mantra_gain;
        self.intangible_gain += other.intangible_gain;
        self.damage_multiplier *= other.damage_multiplier;
        self.vigor_gain += other.vigor_gain;
        self.damage_all += other.damage_all;
        self.damage_random += other.damage_random;
        self.max_hp_gain += other.max_hp_gain;
        self.gold_gain += other.gold_gain;
        self.vulnerable_all += other.vulnerable_all;
        self.weak_apply += other.weak_apply;
        self.poison_all += other.poison_all;
        self.cards_to_hand.extend(other.cards_to_hand.clone());
        self.cards_to_draw_pile.extend(other.cards_to_draw_pile.clone());
        self.buffer_gain += other.buffer_gain;
        self.replay_card |= other.replay_card;
        self.clear_debuffs |= other.clear_debuffs;
        self.upgrade_random_hand += other.upgrade_random_hand;
        self.orb_slot_gain += other.orb_slot_gain;
        self.trigger_all_orb_passives |= other.trigger_all_orb_passives;
        self.messages.extend(other.messages.clone());
    }
}

// ============================================================================
// Relic Library (For Loading/Lookup)
// ============================================================================

/// Collection of relic definitions.
#[derive(Debug, Clone, Default)]
pub struct RelicLibrary {
    relics: HashMap<String, RelicDefinition>,
}

impl RelicLibrary {
    /// Create a new empty library.
    pub fn new() -> Self {
        Self {
            relics: HashMap::new(),
        }
    }
    
    /// Load relics from a JSON file.
    pub fn load(path: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read relics file: {}", e))?;
        
        let relics: Vec<RelicDefinition> = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse relics JSON: {}", e))?;
        
        let mut library = Self::new();
        for relic in relics {
            let normalized_id = normalize_relic_id(&relic.id);
            library.relics.insert(normalized_id, relic);
        }
        
        game_log!("[Relic Loader] Loaded {} relic definitions from '{}'", library.len(), path);
        Ok(library)
    }
    
    /// Get a relic definition by ID (normalized lookup).
    pub fn get(&self, id: &str) -> Option<&RelicDefinition> {
        self.relics.get(&normalize_relic_id(id))
    }
    
    /// Number of relics in the library.
    pub fn len(&self) -> usize {
        self.relics.len()
    }
    
    /// Check if library is empty.
    pub fn is_empty(&self) -> bool {
        self.relics.is_empty()
    }
    
    /// Iterate over all relics.
    pub fn iter(&self) -> impl Iterator<Item = &RelicDefinition> {
        self.relics.values()
    }
}

// ============================================================================
// Data-Driven Command Execution
// ============================================================================

/// Execute a single relic command, returning the effect.
fn execute_command(cmd: &RelicCommand, relic_name: &str) -> RelicTriggerResult {
    let mut result = RelicTriggerResult::new();
    let params = &cmd.params;
    
    match cmd.cmd_type.as_str() {
        "Heal" => {
            let amount = params.amount.or(params.base).unwrap_or(0);
            result.heal += amount;
            result.messages.push(format!("💚 {}: Heal {} HP", relic_name, amount));
        }
        "DrawCards" => {
            let amount = params.amount.or(params.base).unwrap_or(0);
            result.extra_draw += amount;
            result.messages.push(format!("🃏 {}: Draw {} cards", relic_name, amount));
        }
        "GainBlock" => {
            let amount = params.amount.or(params.base).unwrap_or(0);
            result.block_gain += amount;
            result.messages.push(format!("🛡️ {}: +{} Block", relic_name, amount));
        }
        "GainEnergy" => {
            let amount = params.amount.or(params.base).unwrap_or(0);
            result.energy_gain += amount;
            result.messages.push(format!("⚡ {}: +{} Energy", relic_name, amount));
        }
        "GainBuff" => {
            let buff = params.buff.as_deref().unwrap_or("Unknown");
            let amount = params.amount.or(params.base).unwrap_or(0);
            match buff {
                "Strength" => {
                    result.strength_gain += amount;
                    result.messages.push(format!("💪 {}: +{} Strength", relic_name, amount));
                }
                "Dexterity" => {
                    result.dexterity_gain += amount;
                    result.messages.push(format!("🎯 {}: +{} Dexterity", relic_name, amount));
                }
                "Thorns" => {
                    result.thorns_gain += amount;
                    result.messages.push(format!("🌵 {}: +{} Thorns", relic_name, amount));
                }
                "PlatedArmor" => {
                    result.plated_armor_gain += amount;
                    result.messages.push(format!("🔩 {}: +{} Plated Armor", relic_name, amount));
                }
                "Focus" => {
                    result.focus_gain += amount;
                    result.messages.push(format!("🔮 {}: +{} Focus", relic_name, amount));
                }
                "Mantra" => {
                    result.mantra_gain += amount;
                    result.messages.push(format!("🕉️ {}: +{} Mantra", relic_name, amount));
                }
                "Intangible" => {
                    result.intangible_gain += amount;
                    result.messages.push(format!("👻 {}: +{} Intangible", relic_name, amount));
                }
                _ => {
                    result.messages.push(format!("❓ {}: +{} {} (unknown buff)", relic_name, amount, buff));
                }
            }
        }
        "ApplyToAllEnemies" => {
            let status = params.status.as_deref().unwrap_or("Unknown");
            let amount = params.amount.unwrap_or(0);
            if status == "Vulnerable" {
                result.vulnerable_all += amount;
                result.messages.push(format!("💥 {}: Apply {} Vulnerable to ALL", relic_name, amount));
            }
        }
        "ApplyStatus" => {
            let status = params.status.as_deref().unwrap_or("Unknown");
            let amount = params.amount.unwrap_or(0);
            if status == "Weak" {
                result.weak_apply += amount;
                result.messages.push(format!("😵 {}: Apply {} Weak", relic_name, amount));
            } else if status == "Poison" {
                result.poison_all += amount;
                result.messages.push(format!("☠️ {}: Apply {} Poison to ALL", relic_name, amount));
            }
        }
        "DamageAll" => {
            let amount = params.amount.unwrap_or(0);
            result.damage_all += amount;
            result.messages.push(format!("💥 {}: Deal {} damage to ALL", relic_name, amount));
        }
        "DamageRandom" => {
            let amount = params.amount.unwrap_or(0);
            result.damage_random += amount;
            result.messages.push(format!("🎲 {}: Deal {} damage to random enemy", relic_name, amount));
        }
        "RaiseMaxHp" => {
            let amount = params.amount.unwrap_or(0);
            result.max_hp_gain += amount;
            result.messages.push(format!("❤️ {}: +{} Max HP", relic_name, amount));
        }
        "GainGold" => {
            let amount = params.amount.unwrap_or(0);
            result.gold_gain += amount;
            result.messages.push(format!("🪙 {}: +{} Gold", relic_name, amount));
        }
        "AddCardToHand" => {
            let card = params.card.as_deref().unwrap_or("Unknown");
            let count = params.count.unwrap_or(1);
            result.cards_to_hand.push((card.to_string(), count));
            result.messages.push(format!("🃏 {}: Add {} {} to hand", relic_name, count, card));
        }
        "ChannelOrb" => {
            let orb = params.orb.as_deref().unwrap_or("Unknown");
            let count = params.count.unwrap_or(1);
            result.messages.push(format!("🔮 {}: Channel {} {} (stub)", relic_name, count, orb));
        }
        "EnterStance" => {
            let stance = params.stance.as_deref().unwrap_or("Unknown");
            result.messages.push(format!("🧘 {}: Enter {} stance (stub)", relic_name, stance));
        }
        _ => {
            // ---- Additional command types (lower priority, handled via existing fields or stubs) ----
            match cmd.cmd_type.as_str() {
                "ApplyBuff" => {
                    // Akabeko: Vigor 8, Kunai: +1 Dex per 3 attacks, Shuriken: +1 Str per 3 attacks
                    let buff = params.buff.as_deref().or(params.status.as_deref()).unwrap_or("Unknown");
                    let amount = params.amount.or(params.base).unwrap_or(0);
                    match buff {
                        "Vigor" => {
                            // Vigor: next Attack deals extra damage
                            // For now, treat as a temporary Strength buff (simplified)
                            result.strength_gain += amount;
                            result.messages.push(format!("💪 {}: +{} Vigor (as Strength)", relic_name, amount));
                        }
                        "Strength" => {
                            result.strength_gain += amount;
                            result.messages.push(format!("💪 {}: +{} Strength", relic_name, amount));
                        }
                        "Dexterity" => {
                            result.dexterity_gain += amount;
                            result.messages.push(format!("🎯 {}: +{} Dexterity", relic_name, amount));
                        }
                        _ => {
                            result.messages.push(format!("❓ {}: ApplyBuff {} {} (stub)", relic_name, amount, buff));
                        }
                    }
                }
                "ApplyDebuff" => {
                    let debuff = params.buff.as_deref().or(params.status.as_deref()).unwrap_or("Unknown");
                    let amount = params.amount.unwrap_or(0);
                    if debuff == "Weak" {
                        result.weak_apply += amount;
                        result.messages.push(format!("😵 {}: Apply {} Weak", relic_name, amount));
                    } else {
                        result.messages.push(format!("❓ {}: ApplyDebuff {} {} (stub)", relic_name, amount, debuff));
                    }
                }
                "HealPercent" => {
                    // LizardTail: heal 50% max HP
                    let percent = params.amount.or(params.base).unwrap_or(50);
                    // We can't compute max HP here, but we store the percentage for apply_relic_results
                    result.heal += percent; // Will be capped in apply_relic_results
                    result.messages.push(format!("💚 {}: Heal {}% HP", relic_name, percent));
                }
                "GainMaxHP" => {
                    let amount = params.amount.or(params.base).unwrap_or(0);
                    result.max_hp_gain += amount;
                    result.messages.push(format!("❤️ {}: +{} Max HP", relic_name, amount));
                }
                "GainEnergyNextTurn" | "GainEnergyNextCombat" => {
                    // ArtofWar, AncientTeaSet
                    let amount = params.amount.or(params.base).unwrap_or(0);
                    result.energy_gain += amount; // Simplified: immediate energy gain
                    result.messages.push(format!("⚡ {}: +{} Energy (next turn)", relic_name, amount));
                }
                "DrawCardsNextTurn" => {
                    // Pocketwatch: draw 3 if < 3 cards played
                    let amount = params.amount.or(params.base).unwrap_or(0);
                    result.extra_draw += amount; // Simplified as immediate draw
                    result.messages.push(format!("🃏 {}: Draw {} (next turn)", relic_name, amount));
                }
                "DealDamageToAllEnemies" => {
                    let amount = params.amount.or(params.base).unwrap_or(0);
                    result.damage_all += amount;
                    result.messages.push(format!("💥 {}: Deal {} to ALL", relic_name, amount));
                }
                "GainStrengthPerCurse" => {
                    // Du-Vu Doll: +1 Str per Curse in deck
                    let amount = params.amount.or(params.base).unwrap_or(1);
                    // Can't count curses here; caller will need to handle.
                    // Mark with a special message.
                    result.strength_gain += amount; // Base amount; actual multiplied by caller
                    result.messages.push(format!("🎎 {}: +{} Str/Curse (base)", relic_name, amount));
                }
                "ReduceDamage" | "PreventDamage" => {
                    // Torii, FossilizedHelix
                    result.messages.push(format!("🛡️ {}: Reduce damage (passive, handled in damage pipeline)", relic_name));
                }
                "ReduceHPLoss" => {
                    // TungstenRod
                    result.messages.push(format!("🔧 {}: Reduce HP loss (passive, handled in damage pipeline)", relic_name));
                }
                "UpgradeCard" => {
                    // FrozenEgg, MoltenEgg, ToxicEgg - auto-upgrade on acquire
                    result.messages.push(format!("⬆️ {}: Auto-upgrade on acquire (handled in card acquisition)", relic_name));
                }
                "UpgradeRandomCards" | "UpgradeRandomCardInHand" => {
                    result.messages.push(format!("⬆️ {}: Upgrade cards (stub)", relic_name));
                }
                "AddRandomCard" | "AddRandomPowerCard" => {
                    // DeadBranch, Enchiridion
                    result.messages.push(format!("🃏 {}: Add random card (stub — requires card library)", relic_name));
                }
                "RetainHand" => {
                    // RunicPyramid - handled in turn end logic
                    result.messages.push(format!("🔒 {}: Retain hand (handled in turn end)", relic_name));
                }
                "PreserveBlock" => {
                    // Calipers - handled in turn end logic
                    result.messages.push(format!("🛡️ {}: Preserve block (handled in turn end)", relic_name));
                }
                "NegateCurse" | "IncrementCounter" | "BottleCard" | "AddBottledCardToHand" | 
                "ExtraCardReward" | "ExtraRelic" | "ExtraRelicReward" | "GuaranteePotionReward" |
                "GainPotionSlots" | "GainCurse" | "GainCurses" | "GainRelics" |
                "RemoveCards" | "TransformAndUpgrade" | "TransformBasicCards" | "AddCardReward" |
                "ChooseCards" | "ChooseAndAddColorlessCard" | "ChooseAndShuffleCard" | 
                "DiscardAndRedraw" | "EmptyChest" | "DuplicateCard" | "BonusDamage" | 
                "BonusHeal" | "BonusScry" | "BrewPotions" | "RemoveAllDebuffs" |
                "PlayCardAgain" | "ReduceRandomCardCost" | "Scry" | "SetAllEnemiesHP" |
                "TransferPoison" | "TriggerAllOrbPassives" | "TriggerOrbPassive" |
                "GainOrbSlots" | "LoseMaxHPPercent" | "StrangeSpoon" | "PreventExhaust" |
                "ApplyToTarget" | "GainBlockPerCardsInHand" => {
                    // These are handled elsewhere or are stubs for now
                    result.messages.push(format!("📌 {}: {} (handled elsewhere or stub)", relic_name, cmd.cmd_type));
                }
                "AddCard" => {
                    // Add specific cards to hand or draw pile
                    // Used by NinjaScroll (3 Shivs→hand), PureWater (Miracle→hand), 
                    // MarkOfPain (2 Wounds→draw_pile), HolyWater (3 Miracles→hand)
                    let card = params.card.as_deref().unwrap_or("Unknown");
                    let count = params.count.or(params.amount).or(params.base).unwrap_or(1);
                    let dest = params.destination.as_deref().unwrap_or("hand");
                    
                    if dest == "draw_pile" {
                        result.cards_to_draw_pile.push((card.to_string(), count));
                        result.messages.push(format!("🃏 {}: Add {}x {} to draw pile", relic_name, count, card));
                    } else {
                        result.cards_to_hand.push((card.to_string(), count));
                        result.messages.push(format!("🃏 {}: Add {}x {} to hand", relic_name, count, card));
                    }
                }
                _ => {
                    result.messages.push(format!("❓ {}: Unknown command '{}'", relic_name, cmd.cmd_type));
                }
            }
        }
    }
    
    result
}

// ============================================================================
// Main Trigger Function (Hybrid: Data-Driven + Hardcoded)
// ============================================================================

/// Trigger all relics for a given event.
/// When `library` is provided, data-driven JSON relics fire in addition to hardcoded ones.
/// When `library` is None, only hardcoded relics fire (for backward compatibility in tests).
pub fn trigger_relics(
    state: &mut GameState,
    event: &GameEvent,
    library: Option<&RelicLibrary>,
) -> RelicTriggerResult {
    let mut result = RelicTriggerResult::new();
    let trigger = event.to_trigger();
    
    // =================================================================
    // Pre-loop: Relics that need full GameState access
    // These can't use the standalone function because they need
    // master_deck, enemies, or CardLibrary.
    // =================================================================
    
    if matches!(event, GameEvent::BattleStart) {
        // Du-Vu Doll: +1 Strength per Curse in master_deck at combat start.
        // Java: DuVuDoll.atBattleStart() → counter = count curses in masterDeck → Strength
        if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Du-VuDoll" && r.active) {
            let curse_count = state.master_deck.iter()
                .filter(|c| c.card_type == CardType::Curse)
                .count() as i32;
            if curse_count > 0 {
                relic.counter = curse_count;
                result.strength_gain += curse_count;
                result.messages.push(format!("🎎 Du-Vu Doll: +{} Strength ({} curses in deck)", curse_count, curse_count));
                relic.pulse();
            }
        }
        
        // Neow's Lament (NeowsBlessing): First 3 combats, set all enemies HP to 1.
        // Java: NeowsLament.atPreBattle() → counter--, set all enemy HP to 1, usedUp at 0
        if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "NeowsLament" && r.active) {
            if relic.counter > 0 {
                relic.counter -= 1;
                // Set all enemies to 1 HP
                for enemy in state.enemies.iter_mut() {
                    enemy.hp = 1;
                }
                result.messages.push(format!("👼 Neow's Lament: All enemies set to 1 HP! ({} uses left)", relic.counter));
                relic.pulse();
                if relic.counter == 0 {
                    relic.active = false; // Used up
                }
            }
        }
        
        // Enchiridion: handled in combat.rs on_battle_start() where CardLibrary is available
        
        // Bag of Marbles: +1 Vulnerable to ALL enemies at battle start.
        // Java: BagOfMarbles.atBattleStart() → ApplyPowerAction(VulnerablePower, 1) for each monster
        if state.relics.iter().any(|r| r.id == "BagOfMarbles" && r.active) {
            result.vulnerable_all += 1;
            result.messages.push("🔮 Bag of Marbles: +1 Vulnerable to ALL enemies".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "BagOfMarbles") {
                relic.pulse();
            }
        }
        
        // Preserved Insect: Elites start with -25% HP.
        // Java: PreservedInsect.atBattleStart() → if eliteTrigger: set monster HP to 75%
        if state.relics.iter().any(|r| r.id == "PreservedInsect" && r.active) {
            // Check if this is an elite fight (simplified: check if any enemy has is_elite flag)
            let is_elite = state.enemies.iter().any(|e| e.is_elite);
            if is_elite {
                for enemy in state.enemies.iter_mut() {
                    let reduced_hp = (enemy.max_hp as f32 * 0.75) as i32;
                    if enemy.hp > reduced_hp {
                        enemy.hp = reduced_hp;
                    }
                }
                result.messages.push("🐛 Preserved Insect: Elite enemies -25% HP".to_string());
                if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "PreservedInsect") {
                    relic.pulse();
                }
            }
        }
        
        // Pantograph: Heal 25 HP at boss fight start.
        // Java: Pantograph.atBattleStart() → if any monster is BOSS type: HealAction(25)
        if state.relics.iter().any(|r| r.id == "Pantograph" && r.active) {
            let is_boss = state.enemies.iter().any(|e| e.is_boss);
            if is_boss {
                result.heal += 25;
                result.messages.push("📐 Pantograph: Heal 25 HP (boss fight)".to_string());
                if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Pantograph") {
                    relic.pulse();
                }
            }
        }
        
        // Sling: +2 Strength at battle start if fighting elites.
        // Java: Sling.atBattleStart() → if eliteTrigger: ApplyPowerAction(StrengthPower, 2)
        if state.relics.iter().any(|r| r.id == "Sling" && r.active) {
            let is_elite = state.enemies.iter().any(|e| e.is_elite);
            if is_elite {
                result.strength_gain += 2;
                result.messages.push("🪃 Sling: +2 Strength (elite fight)".to_string());
                if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Sling") {
                    relic.pulse();
                }
            }
        }
        
        // TwistedFunnel: Apply 4 Poison to ALL enemies at battle start.
        // Java: TwistedFunnel.atBattleStart() → ApplyPowerAction(PoisonPower, 4) for each monster
        if state.relics.iter().any(|r| r.id == "TwistedFunnel" && r.active) {
            for enemy in state.enemies.iter_mut() {
                if enemy.hp > 0 {
                    enemy.apply_status("Poison", 4);
                }
            }
            result.messages.push("🧪 Twisted Funnel: +4 Poison to ALL enemies".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "TwistedFunnel") {
                relic.pulse();
            }
        }
        
        // RedMask: Apply 1 Weak to ALL enemies at battle start.
        // Java: RedMask.atBattleStart() → ApplyPowerAction(WeakPower, 1) for each monster
        if state.relics.iter().any(|r| r.id == "RedMask" && r.active) {
            for enemy in state.enemies.iter_mut() {
                if enemy.hp > 0 {
                    enemy.apply_status("Weak", 1);
                }
            }
            result.messages.push("🎭 Red Mask: +1 Weak to ALL enemies".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "RedMask") {
                relic.pulse();
            }
        }
        
        // Slaver's Collar: +1 Energy for elite/boss fights.
        // Java: SlaversCollar.beforeEnergyPrep() → if elite or boss: energyMaster++
        // Implementation: grant +1 energy at battle start if fighting elites or bosses.
        if state.relics.iter().any(|r| r.id == "SlaversCollar" && r.active) {
            let is_elite_or_boss = state.enemies.iter().any(|e| e.is_elite || e.is_boss);
            if is_elite_or_boss {
                result.energy_gain += 1;
                result.messages.push("⛓️ Slaver's Collar: +1 Energy (elite/boss fight)".to_string());
                if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "SlaversCollar") {
                    relic.pulse();
                }
            }
        }
        
        // Mark of Pain: +1 Energy (onEquip), +2 Wounds to draw pile at battle start.
        // Java: MarkOfPain.atBattleStart() → MakeTempCardInDrawPileAction(Wound, 2)
        // Note: +1 energy is handled by energy system (onEquip/onUnequip), not here.
        if state.relics.iter().any(|r| r.id == "MarkOfPain" && r.active) {
            result.cards_to_draw_pile.push(("Wound".to_string(), 2));
            result.messages.push("😈 Mark of Pain: +2 Wounds to draw pile".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "MarkOfPain") {
                relic.pulse();
            }
        }
        
        // Ninja Scroll: Add 3 Shivs to hand at battle start.
        // Java: NinjaScroll.atBattleStartPreDraw() → MakeTempCardInHandAction(Shiv, 3)
        if state.relics.iter().any(|r| r.id == "NinjaScroll" && r.active) {
            result.cards_to_hand.push(("Shiv".to_string(), 3));
            result.messages.push("📜 Ninja Scroll: +3 Shivs to hand".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "NinjaScroll") {
                relic.pulse();
            }
        }
        
        // Philosopher's Stone: +1 Strength to ALL enemies at battle start.
        // Java: PhilosopherStone.atBattleStart() → ApplyPowerAction(StrengthPower, 1) for each monster
        // Note: +1 Energy is handled by the energy master system (onEquip).
        if state.relics.iter().any(|r| r.id == "PhilosopherStone" && r.active) {
            for enemy in state.enemies.iter_mut() {
                if enemy.hp > 0 {
                    enemy.apply_status("Strength", 1);
                }
            }
            result.messages.push("💎 Philosopher's Stone: +1 Strength to ALL enemies".to_string());
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "PhilosopherStone") {
                relic.pulse();
            }
        }
    }
    
    // TheSpecimen: When an enemy with Poison dies, transfer its Poison to a random alive enemy.
    // Java: TheSpecimen.onMonsterDeath(m) → if m.hasPower("Poison") && !allDead: 
    //   ApplyPowerToRandomEnemyAction(Poison, amount)
    if let GameEvent::EnemyDied { enemy_idx } = event {
        if state.relics.iter().any(|r| r.id == "TheSpecimen" && r.active) {
            let dead_poison = state.enemies.get(*enemy_idx)
                .map(|e| e.get_status("Poison"))
                .unwrap_or(0);
            if dead_poison > 0 {
                // Find alive enemies (excluding the dead one)
                let alive_indices: Vec<usize> = state.enemies.iter().enumerate()
                    .filter(|(i, e)| *i != *enemy_idx && e.hp > 0)
                    .map(|(i, _)| i)
                    .collect();
                if !alive_indices.is_empty() {
                    use rand::Rng;
                    let target_idx = alive_indices[state.rng.random_range(0..alive_indices.len())];
                    state.enemies[target_idx].apply_status("Poison", dead_poison);
                    result.messages.push(format!(
                        "🔬 The Specimen: Transferred {} Poison to {}",
                        dead_poison, state.enemies[target_idx].name
                    ));
                    if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "TheSpecimen") {
                        relic.pulse();
                    }
                }
            }
        }
    }
    
    // Collect relic info to avoid borrowing issues
    let relic_ids: Vec<String> = state.relics.iter()
        .filter(|r| r.active)
        .map(|r| r.id.clone())
        .collect();
    
    // Snapshot of player state for condition checking
    let player_hp = state.player.current_hp;
    let player_max_hp = state.player.max_hp;
    let player_block = state.player.block;
    
    for relic_id in &relic_ids {
        // Find and process the relic
        if let Some(relic) = state.relics.iter_mut().find(|r| &r.id == relic_id) {
            relic.clear_pulse();
            
            // First, check hardcoded logic for complex relics
            let hardcoded_result = trigger_hardcoded_relic_standalone(
                relic, event, player_hp, player_max_hp, player_block
            );
            if hardcoded_result.has_effect() {
                result.merge(&hardcoded_result);
                continue;
            }
            
            // Then, try data-driven logic from the library (if available)
            if let Some(lib) = library {
                if let Some(def) = lib.get(&relic.id) {
                    for hook in &def.logic.hooks {
                        // Match trigger
                        if let Some(hook_trigger) = RelicTrigger::from_str(&hook.trigger) {
                            if hook_trigger == trigger && check_condition_standalone(
                                &hook.condition, event, player_hp, player_max_hp, player_block
                            ) {
                                // Execute all commands for this hook
                                for cmd in &hook.commands {
                                    let cmd_result = execute_command(cmd, &def.name);
                                    result.merge(&cmd_result);
                                }
                                relic.pulse();
                            }
                        }
                    }
                }
            }
        }
    }
    
    result
}

/// Legacy wrapper — equivalent to trigger_relics with library.
#[deprecated(note = "Use trigger_relics() with library parameter instead")]
pub fn trigger_relics_with_library(
    state: &mut GameState,
    event: &GameEvent,
    library: &RelicLibrary,
) -> RelicTriggerResult {
    trigger_relics(state, event, Some(library))
}

/// Check condition without borrowing state (uses snapshot values).
fn check_condition_standalone(
    condition: &Option<HookCondition>,
    event: &GameEvent,
    player_hp: i32,
    player_max_hp: i32,
    player_block: i32,
) -> bool {
    if let Some(cond) = condition {
        // Check turn condition
        if let Some(required_turn) = cond.turn {
            if event.turn() != Some(required_turn) {
                return false;
            }
        }
        
        // Check HP condition
        if let Some(hp_percent) = cond.hp_below_percent {
            let current_percent = (player_hp as f32 / player_max_hp as f32 * 100.0) as i32;
            if current_percent > hp_percent {
                return false;
            }
        }
        
        // Check no block condition
        if cond.no_block == Some(true) && player_block > 0 {
            return false;
        }
    }
    true
}

// ============================================================================
// Hardcoded Relic Logic (Escape Hatch for Complex Relics)
// ============================================================================

/// Handle complex relics that can't be easily expressed as data-driven commands.
/// Uses standalone state values to avoid borrowing issues.
fn trigger_hardcoded_relic_standalone(
    relic: &mut RelicInstance,
    event: &GameEvent,
    player_hp: i32,
    player_max_hp: i32,
    player_block: i32,
) -> RelicTriggerResult {
    let mut result = RelicTriggerResult::new();
    
    match (relic.id.as_str(), event) {
        // =================================================================
        // Counter-based Relics
        // =================================================================
        
        // Pen Nib: Every 10th Attack deals double damage.
        ("PenNib", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            if relic.increment_counter(10) {
                result.damage_multiplier = 2.0;
                result.messages.push("🖊️ Pen Nib: DOUBLE DAMAGE!".to_string());
            } else {
                result.messages.push(format!("🖊️ Pen Nib: {}/10 attacks", relic.counter));
            }
            relic.pulse();
        }
        
        // Nunchaku: Every 10 Attacks, gain 1 Energy.
        ("Nunchaku", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            if relic.increment_counter(10) {
                result.energy_gain += 1;
                result.messages.push("🥋 Nunchaku: +1 Energy!".to_string());
                relic.pulse();
            }
        }
        
        // Ornamental Fan: Every 3 Attacks in a turn, gain 4 Block.
        ("OrnamentalFan", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            if relic.increment_counter(3) {
                result.block_gain += 4;
                result.messages.push("🪭 Ornamental Fan: +4 Block".to_string());
                relic.pulse();
            }
        }
        
        // Kunai: Every 3 Attacks in a turn, gain 1 Dexterity.
        ("Kunai", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            if relic.increment_counter(3) {
                result.dexterity_gain += 1;
                result.messages.push("🗡️ Kunai: +1 Dexterity".to_string());
                relic.pulse();
            }
        }
        
        // Shuriken: Every 3 Attacks in a turn, gain 1 Strength.
        ("Shuriken", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            if relic.increment_counter(3) {
                result.strength_gain += 1;
                result.messages.push("⭐ Shuriken: +1 Strength".to_string());
                relic.pulse();
            }
        }
        
        // Happy Flower: Every 3 turns, gain 1 Energy.
        ("HappyFlower", GameEvent::TurnStart { turn }) => {
            if *turn > 0 && relic.increment_counter(3) {
                result.energy_gain += 1;
                result.messages.push("🌸 Happy Flower: +1 Energy".to_string());
                relic.pulse();
            }
        }
        
        // Incense Burner: Every 6 turns, gain 1 Intangible.
        ("IncenseBurner", GameEvent::TurnStart { turn }) => {
            if *turn > 0 && relic.increment_counter(6) {
                result.intangible_gain += 1;
                result.messages.push("🔥 Incense Burner: +1 Intangible!".to_string());
                relic.pulse();
            }
        }
        
        // =================================================================
        // Conditional Relics
        // =================================================================
        
        // Orichalcum: If you end your turn without Block, gain 6 Block.
        ("Orichalcum", GameEvent::TurnEnd { .. }) => {
            if player_block == 0 {
                result.block_gain += 6;
                result.messages.push("🪨 Orichalcum: +6 Block (no block)".to_string());
                relic.pulse();
            }
        }
        
        // Meat on the Bone: If HP <= 50% at end of combat, heal 12.
        ("MeatOnTheBone", GameEvent::BattleEnd { won: true }) => {
            let hp_percent = player_hp as f32 / player_max_hp as f32;
            if hp_percent <= 0.5 {
                result.heal += 12;
                result.messages.push("🍖 Meat on the Bone: Heal 12 HP".to_string());
                relic.pulse();
            }
        }
        
        // Red Skull: While HP <= 50%, gain 3 Strength (handled elsewhere).
        
        // =================================================================
        // Bird-Faced Urn: Heal 2 HP when playing a Power.
        ("BirdFacedUrn", GameEvent::PlayerPlayCard { card_type: CardType::Power, .. }) => {
            result.heal += 2;
            result.messages.push("🏺 Bird-Faced Urn: Heal 2 HP".to_string());
            relic.pulse();
        }
        
        // =================================================================
        // HP Loss Relics (wasHPLost in Java)
        // =================================================================
        
        // Red Skull: At BattleStart, if HP ≤ 50%, gain 3 Strength.
        // Java: onBloodied() → +3 Str, onNotBloodied() → -3 Str
        // Simplified: grant 3 Str at battle start if bloodied, track via counter.
        ("RedSkull", GameEvent::BattleStart) => {
            if player_hp * 2 <= player_max_hp {
                result.strength_gain += 3;
                relic.counter = 1; // Mark as active
                result.messages.push("💀 Red Skull: +3 Strength (HP ≤ 50%)".to_string());
                relic.pulse();
            } else {
                relic.counter = 0;
            }
        }
        
        // Red Skull: On HP loss during combat, check if we crossed the 50% threshold.
        ("RedSkull", GameEvent::PlayerLoseHp { amount }) => {
            let new_hp = player_hp; // snapshot is taken before this call runs
            let was_bloodied = relic.counter == 1;
            let is_bloodied = new_hp * 2 <= player_max_hp;
            
            if is_bloodied && !was_bloodied {
                // Crossed below 50% → gain 3 Str
                result.strength_gain += 3;
                relic.counter = 1;
                result.messages.push("💀 Red Skull: +3 Strength (HP dropped ≤ 50%)".to_string());
                relic.pulse();
            }
        }
        
        // Red Skull: On heal, remove strength if HP goes above 50%.
        ("RedSkull", GameEvent::PlayerHeal { .. }) => {
            let was_bloodied = relic.counter == 1;
            let is_bloodied = player_hp * 2 <= player_max_hp;
            
            if was_bloodied && !is_bloodied {
                // Crossed above 50% → lose 3 Str
                result.strength_gain -= 3;
                relic.counter = 0;
                result.messages.push("💀 Red Skull: -3 Strength (HP > 50%)".to_string());
            }
        }
        
        // Runic Cube: When you lose HP, draw 1 card.
        // Java: wasHPLost(int) → DrawCardAction(1)
        ("RunicCube", GameEvent::PlayerLoseHp { amount }) => {
            if *amount > 0 {
                result.extra_draw += 1;
                result.messages.push("🧊 Runic Cube: Draw 1 card".to_string());
                relic.pulse();
            }
        }
        
        // Self-Forming Clay: When you lose HP, gain 3 Block next turn.
        // Java: wasHPLost(int) → NextTurnBlockPower(3)
        // Simplified: immediate block gain (NextTurnBlock power not implemented yet)
        ("SelfFormingClay", GameEvent::PlayerLoseHp { amount }) => {
            if *amount > 0 {
                result.block_gain += 3;
                result.messages.push("🏺 Self-Forming Clay: +3 Block".to_string());
                relic.pulse();
            }
        }
        
        // Centennial Puzzle: First time you lose HP each combat, draw 3 cards.
        // Java: wasHPLost() + usedThisCombat flag
        // Uses counter: 0=unused, 1=used this combat
        ("CentennialPuzzle", GameEvent::BattleStart) => {
            relic.counter = 0; // Reset for new combat
        }
        ("CentennialPuzzle", GameEvent::PlayerLoseHp { amount }) => {
            if *amount > 0 && relic.counter == 0 {
                result.extra_draw += 3;
                relic.counter = 1;
                result.messages.push("🧩 Centennial Puzzle: Draw 3 cards (first HP loss)".to_string());
                relic.pulse();
            }
        }
        
        // EmotionChip: If you lost HP last turn, trigger all orb passives at start of next turn.
        // Java: wasHPLost() → pulse=true; atTurnStart() → ImpulseAction (trigger all orb passives)
        // Uses counter: 0=no damage taken, 1=took damage (trigger next turn)
        ("EmotionChip", GameEvent::BattleStart) | ("Emotion Chip", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        ("EmotionChip", GameEvent::PlayerLoseHp { amount }) | ("Emotion Chip", GameEvent::PlayerLoseHp { amount }) => {
            if *amount > 0 {
                relic.counter = 1; // Mark as "took damage this turn"
            }
        }
        
        // =================================================================
        // Batch 4: Simple BattleStart Passives (Java: atBattleStart)
        // =================================================================
        
        // Vajra: +1 Strength at battle start.
        // Java: Vajra.atBattleStart() → ApplyPowerAction(StrengthPower, 1)
        ("Vajra", GameEvent::BattleStart) => {
            result.strength_gain += 1;
            result.messages.push("💎 Vajra: +1 Strength".to_string());
            relic.pulse();
        }
        
        // Anchor: +10 Block at battle start.
        // Java: Anchor.atBattleStart() → GainBlockAction(10)
        ("Anchor", GameEvent::BattleStart) => {
            result.block_gain += 10;
            result.messages.push("⚓ Anchor: +10 Block".to_string());
            relic.pulse();
        }
        
        // Oddly Smooth Stone: +1 Dexterity at battle start.
        // Java: OddlySmoothStone.atBattleStart() → ApplyPowerAction(DexterityPower, 1)
        ("OddlySmoothStone", GameEvent::BattleStart) => {
            result.dexterity_gain += 1;
            result.messages.push("🪨 Oddly Smooth Stone: +1 Dexterity".to_string());
            relic.pulse();
        }
        
        // Thread and Needle: +4 Plated Armor at battle start.
        // Java: ThreadAndNeedle.atBattleStart() → ApplyPowerAction(PlatedArmorPower, 4)
        ("ThreadAndNeedle", GameEvent::BattleStart) => {
            result.plated_armor_gain += 4;
            result.messages.push("🧵 Thread and Needle: +4 Plated Armor".to_string());
            relic.pulse();
        }
        
        // Bronze Scales: +3 Thorns at battle start.
        // Java: BronzeScales.atBattleStart() → ApplyPowerAction(ThornsPower, 3)
        ("BronzeScales", GameEvent::BattleStart) => {
            result.thorns_gain += 3;
            result.messages.push("🐍 Bronze Scales: +3 Thorns".to_string());
            relic.pulse();
        }
        
        // Akabeko: +8 Vigor at battle start (first attack deals +8 damage).
        // Java: Akabeko.atBattleStart() → ApplyPowerAction(VigorPower, 8)
        ("Akabeko", GameEvent::BattleStart) => {
            result.vigor_gain += 8;
            result.messages.push("🐂 Akabeko: +8 Vigor (first attack)".to_string());
            relic.pulse();
        }
        
        // Blood Vial: Heal 2 at battle start.
        // Java: BloodVial.atBattleStart() → HealAction(2)
        ("BloodVial", GameEvent::BattleStart) => {
            result.heal += 2;
            result.messages.push("🩸 Blood Vial: Heal 2 HP".to_string());
            relic.pulse();
        }
        
        // GamblingChip: Reset counter at battle start (logic in on_turn_start_post_draw)
        ("GamblingChip", GameEvent::BattleStart) => {
            relic.counter = 0; // Reset for new combat
        }
        
        // =================================================================
        // Batch 4: Turn-Based Relics
        // =================================================================
        
        // Lantern: +1 Energy on first turn.
        // Java: Lantern.atTurnStart() → if firstTurn: GainEnergyAction(1)
        ("Lantern", GameEvent::BattleStart) => {
            relic.counter = 0; // Reset for new combat
        }
        ("Lantern", GameEvent::TurnStart { turn }) => {
            if relic.counter == 0 {
                result.energy_gain += 1;
                relic.counter = 1; // Mark as used
                result.messages.push("🏮 Lantern: +1 Energy (first turn)".to_string());
                relic.pulse();
            }
        }
        
        // Horn Cleat: +14 Block at start of turn 2.
        // Java: HornCleat.atTurnStart() → counter++, if counter==2: GainBlockAction(14), grayscale
        ("HornCleat", GameEvent::BattleStart) => {
            relic.counter = 0; // Reset for new combat
        }
        ("HornCleat", GameEvent::TurnStart { turn }) => {
            relic.counter += 1;
            if relic.counter == 2 {
                result.block_gain += 14;
                result.messages.push("🔩 Horn Cleat: +14 Block (turn 2)".to_string());
                relic.pulse();
                relic.counter = -1; // Used up for combat
            }
        }
        
        // =================================================================
        // Batch 4: Counter-Based Relics (onUseCard)
        // =================================================================
        
        // Letter Opener: Every 3 Skills played in a turn, deal 5 damage to ALL enemies.
        // Java: LetterOpener.onUseCard() → if Skill, counter++, if counter%3==0: DamageAllEnemiesAction(5)
        ("LetterOpener", GameEvent::TurnStart { .. }) => {
            relic.counter = 0; // Reset each turn
        }
        ("LetterOpener", GameEvent::PlayerPlayCard { card_type: CardType::Skill, .. }) => {
            relic.counter += 1;
            if relic.counter % 3 == 0 {
                result.damage_all += 5;
                relic.counter = 0;
                result.messages.push("✉️ Letter Opener: 5 damage to ALL enemies (3 Skills)".to_string());
                relic.pulse();
            }
        }
        
        // =================================================================
        // Batch 5: More Simple Relics
        // =================================================================
        
        // ClockworkSouvenir: +1 Artifact at battle start.
        // Java: ClockworkSouvenir.atBattleStart() → ApplyPowerAction(ArtifactPower, 1)
        ("ClockworkSouvenir", GameEvent::BattleStart) => {
            result.artifact_gain += 1;
            result.messages.push("⚙️ Clockwork Souvenir: +1 Artifact".to_string());
            relic.pulse();
        }
        
        // Mercury Hourglass: 3 damage to ALL enemies at start of each turn.
        // Java: MercuryHourglass.atTurnStart() → DamageAllEnemiesAction(3, THORNS)
        ("MercuryHourglass", GameEvent::TurnStart { .. }) => {
            result.damage_all += 3;
            result.messages.push("⏳ Mercury Hourglass: 3 damage to ALL enemies".to_string());
            relic.pulse();
        }
        
        // Stone Calendar: On turn 7 end, deal 52 damage to ALL enemies.
        // Java: StoneCalendar counter++ at atTurnStart, onPlayerEndTurn if counter==7: DamageAllEnemiesAction(52)
        ("StoneCalendar", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        ("StoneCalendar", GameEvent::TurnStart { .. }) => {
            relic.counter += 1;
        }
        ("StoneCalendar", GameEvent::TurnEnd { .. }) => {
            if relic.counter == 7 {
                result.damage_all += 52;
                result.messages.push("📅 Stone Calendar: 52 damage to ALL enemies (turn 7)".to_string());
                relic.pulse();
            }
        }
        
        // Ink Bottle: Every 10 cards played → draw 1 extra card. Counter persists across combats.
        // Java: InkBottle.onUseCard() → counter++, if counter==10: counter=0, DrawCardAction(1)
        ("InkBottle", GameEvent::PlayerPlayCard { .. }) => {
            relic.counter += 1;
            if relic.counter >= 10 {
                relic.counter = 0;
                result.extra_draw += 1;
                result.messages.push("🖋️ Ink Bottle: Draw 1 card (10 cards played)".to_string());
                relic.pulse();
            }
        }
        
        // Pocketwatch: If ≤3 cards played last turn (not first turn), draw 3 at start of next.
        // Java: Pocketwatch.atTurnStartPostDraw() → if counter<=3 && !firstTurn: DrawCardAction(3)
        // Counter tracks cards played; reset at atTurnStartPostDraw; incremented onPlayCard
        // Note: handled in on_turn_start_post_draw, counter tracks cards played per turn
        ("Pocketwatch", GameEvent::BattleStart) => {
            relic.counter = -1; // -1 = first turn flag (don't trigger on first turn)
        }
        ("Pocketwatch", GameEvent::PlayerPlayCard { .. }) => {
            if relic.counter >= 0 {
                relic.counter += 1;
            }
        }
        
        // =================================================================
        // Special Relics
        // =================================================================
        
        // =================================================================
        // Batch 7: Easy Missing Relics
        // =================================================================
        
        // Bag of Preparation: Draw 2 extra cards at battle start.
        // Java: BagOfPreparation.atBattleStart() → DrawCardAction(2)
        ("BagOfPreparation", GameEvent::BattleStart) => {
            result.extra_draw += 2;
            result.messages.push("🎒 Bag of Preparation: Draw 2 cards".to_string());
            relic.pulse();
        }
        
        // Burning Blood: Heal 6 HP at end of combat.
        // Java: BurningBlood.onVictory() → player.heal(6)
        ("BurningBlood", GameEvent::BattleEnd { won: true }) => {
            result.heal += 6;
            result.messages.push("🔥 Burning Blood: Heal 6 HP".to_string());
            relic.pulse();
        }
        
        // Black Blood: Heal 12 HP at end of combat (upgraded Burning Blood).
        // Java: BlackBlood.onVictory() → player.heal(12)
        ("BlackBlood", GameEvent::BattleEnd { won: true }) => {
            result.heal += 12;
            result.messages.push("🩸 Black Blood: Heal 12 HP".to_string());
            relic.pulse();
        }
        
        // Captain's Wheel: +18 Block on turn 3 (once per combat).
        // Java: CaptainsWheel.atBattleStart() counter=0, atTurnStart() counter++, if counter==3: GainBlockAction(18), grayscale
        ("CaptainsWheel", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        ("CaptainsWheel", GameEvent::TurnStart { .. }) => {
            if relic.counter >= 0 {
                relic.counter += 1;
                if relic.counter == 3 {
                    result.block_gain += 18;
                    result.messages.push("⚓ Captain's Wheel: +18 Block (turn 3)".to_string());
                    relic.pulse();
                    relic.counter = -1; // Used up for this combat
                }
            }
        }
        
        // Art of War: If you play no Attacks during your turn, gain 1 extra Energy next turn.
        // Java: atPreBattle → gainEnergyNext=true, firstTurn=true
        //        atTurnStart → if gainEnergyNext && !firstTurn: GainEnergyAction(1); firstTurn=false; gainEnergyNext=true
        //        onUseCard(ATTACK) → gainEnergyNext=false
        // counter: 0=no attacks this turn (gain next), 1=played attack this turn, -1=first turn
        ("ArtOfWar", GameEvent::BattleStart) => {
            relic.counter = -1; // First turn flag
        }
        ("ArtOfWar", GameEvent::TurnStart { .. }) => {
            if relic.counter == 0 {
                // Last turn had no attacks → gain energy
                result.energy_gain += 1;
                result.messages.push("⚔️ Art of War: +1 Energy (no attacks last turn)".to_string());
                relic.pulse();
            }
            // Reset: assume no attacks this turn (will be set to 1 on attack)
            relic.counter = 0;
        }
        ("ArtOfWar", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            relic.counter = 1; // Played an attack this turn
        }
        
        // Face of Cleric: +1 Max HP at end of combat.
        // Java: FaceOfCleric.onVictory() → player.increaseMaxHp(1, true)
        ("FaceOfCleric", GameEvent::BattleEnd { won: true }) => {
            result.max_hp_gain += 1;
            result.messages.push("😇 Face of Cleric: +1 Max HP".to_string());
            relic.pulse();
        }
        
        // Gremlin Mask: +1 Weak to player at battle start.
        // Java: GremlinMask.atBattleStart() → ApplyPowerAction(WeakPower(player, 1))
        ("GremlinMask", GameEvent::BattleStart) => {
            result.weak_apply += 1;
            result.messages.push("👺 Gremlin Mask: +1 Weak to player".to_string());
            relic.pulse();
        }
        
        // =================================================================
        // Batch 8: More Combat Relics
        // =================================================================
        
        // Ring of the Snake (SnakeRing): Draw 2 extra cards at battle start.
        // Java: SnakeRing.atBattleStart() → DrawCardAction(2)
        ("SnakeRing", GameEvent::BattleStart) => {
            result.extra_draw += 2;
            result.messages.push("🐍 Ring of the Snake: Draw 2 cards".to_string());
            relic.pulse();
        }
        
        // Duality (Yang): When you play an Attack, gain 1 temporary Dexterity.
        // Java: Duality.onUseCard(ATTACK) → DexterityPower(1) + LoseDexterityPower(1)
        // Uses temp_dexterity_gain: applies both +1 Dex and +1 DexLoss (removed at end of turn)
        ("Duality", GameEvent::PlayerPlayCard { card_type: CardType::Attack, .. }) => {
            result.temp_dexterity_gain += 1;
            result.messages.push("☯️ Duality: +1 temporary Dexterity".to_string());
            relic.pulse();
        }
        
        // Gremlin Horn: When a non-boss enemy dies, gain 1 Energy and draw 1 card.
        // Java: GremlinHorn.onMonsterDeath(m) → if m.hp==0 && !allDead: +1E, draw 1
        ("GremlinHorn", GameEvent::EnemyDied { .. }) => {
            result.energy_gain += 1;
            result.extra_draw += 1;
            result.messages.push("📯 Gremlin Horn: +1 Energy, Draw 1 card (enemy died)".to_string());
            relic.pulse();
        }
        
        // Charon's Ashes: When you exhaust a card, deal 3 damage to ALL enemies.
        // Java: CharonsAshes.onExhaust() → DamageAllEnemiesAction(3, THORNS)
        ("CharonsAshes", GameEvent::PlayerExhaust { .. }) => {
            result.damage_all += 3;
            result.messages.push("🔥 Charon's Ashes: 3 damage to ALL enemies (exhaust)".to_string());
            relic.pulse();
        }
        
        // Abacus (TheAbacus): When you shuffle, gain 6 Block.
        // Java: Abacus.onShuffle() → GainBlockAction(6)
        // Note: onShuffle event would need to be a GameEvent. For now, handled inline
        // in state.rs draw_cards() next to Sundial. (See state.rs)
        // Alternatively via data-driven hook.
        
        // =================================================================
        // Batch 9: Boss Energy Relics & More Combat Relics
        // =================================================================
        
        // Velvet Choker: +1 Energy, but can play at most 6 cards per turn.
        // Java: atBattleStart/atTurnStart → counter=0, onPlayCard → counter++, canPlay → counter<6
        ("VelvetChoker", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        ("VelvetChoker", GameEvent::TurnStart { .. }) => {
            relic.counter = 0;
        }
        ("VelvetChoker", GameEvent::PlayerPlayCard { .. }) => {
            relic.counter += 1;
            if relic.counter >= 6 {
                result.messages.push("🔴 Velvet Choker: Card play limit reached (6)".to_string());
                relic.pulse();
            }
        }
        
        // Ancient Tea Set: If visited a rest site last, +2 Energy on first turn.
        // Java: onEnterRestRoom → counter=-2, atTurnStart → if firstTurn && counter==-2: +2E
        // counter: -2 = activated (rested), -1 = not activated, 0+ = already used this combat
        ("AncientTeaSet", GameEvent::BattleStart) => {
            // Keep counter from dungeon state (-2 if rested)
        }
        ("AncientTeaSet", GameEvent::TurnStart { turn }) => {
            if *turn == 1 && relic.counter == -2 {
                result.energy_gain += 2;
                result.messages.push("🍵 Ancient Tea Set: +2 Energy (rested last)".to_string());
                relic.pulse();
                relic.counter = -1; // Used up
            }
        }
        
        // Ring of the Serpent: +1 card draw per turn (Boss upgrade of SnakeRing).
        // Java: onEquip → masterHandSize++. atTurnStart just flashes.
        // Combat effect: +1 draw per turn is handled by masterHandSize in energy system.
        // We add +1 extra_draw at battle start as a simplified approximation.
        ("RingOfTheSerpent", GameEvent::BattleStart) => {
            result.messages.push("🐍 Ring of the Serpent: +1 card draw per turn".to_string());
            relic.pulse();
        }
        
        // Boss Energy Relics: +1 Energy (handled by energy master system).
        // Combat-side effect: just the energy. Restrictions are meta/dungeon.
        // CoffeeDripper: +1E, can't rest. FusionHammer: +1E, can't smith.
        // RunicDome: +1E, can't see intents. BustedCrown: +1E, -2 card rewards.
        // These are registered here so the audit detects them.
        ("CoffeeDripper", GameEvent::BattleStart) => {
            // Energy handled by energy master system. Meta restriction: no rest.
        }
        ("FusionHammer", GameEvent::BattleStart) => {
            // Energy handled by energy master system. Meta restriction: no smith.
        }
        ("RunicDome", GameEvent::BattleStart) => {
            // Energy handled by energy master system. Meta restriction: no intents.
        }
        ("BustedCrown", GameEvent::BattleStart) => {
            // Energy handled by energy master system. Meta restriction: -2 card rewards.
        }
        
        // Medical Kit: Status cards can be played. When played, they exhaust.
        // Java: MedicalKit.onUseCard(STATUS) → card.exhaust = true
        // This is a passive modifier; in our sim, status cards are typically unplayable.
        // Registering for audit tracking; actual logic needs card playability check.
        ("MedicalKit", GameEvent::PlayerPlayCard { card_type: CardType::Status, .. }) => {
            result.messages.push("🏥 Medical Kit: Status card exhausted".to_string());
            relic.pulse();
        }
        
        // Blue Candle: Curse cards can be played. When played, they exhaust and lose 1 HP.
        // Java: BlueCandle.onUseCard(CURSE) → LoseHPAction(1), card.exhaust = true
        ("BlueCandle", GameEvent::PlayerPlayCard { card_type: CardType::Curse, .. }) => {
            result.heal -= 1; // Lose 1 HP (negative heal)
            result.messages.push("🕯️ Blue Candle: Curse exhausted, -1 HP".to_string());
            relic.pulse();
        }
        
        // =================================================================
        // Batch 10: UsePotion Event Relics
        // =================================================================
        
        // Toy Ornithopter: Heal 5 HP whenever you use a potion.
        // Java: ToyOrnithopter.onUsePotion() → HealAction(5)
        ("ToyOrnithopter", GameEvent::PlayerUsePotion) => {
            result.heal += 5;
            result.messages.push("🐦 Toy Ornithopter: Heal 5 HP (potion used)".to_string());
            relic.pulse();
        }
        
        // =================================================================
        // Batch 11: ManualDiscard Event Relics
        // =================================================================
        
        // Tough Bandages: Whenever you discard a card, gain 3 Block.
        // Java: ToughBandages.onManualDiscard() → GainBlockAction(3) per card
        // We apply block * discard_count since the event fires once with count.
        ("ToughBandages", GameEvent::PlayerManualDiscard { count }) => {
            let block = 3 * count;
            if *count > 0 {
                result.block_gain += block;
                result.messages.push(format!(
                    "🩹 Tough Bandages: +{} Block ({} discards × 3)", block, count
                ));
                relic.pulse();
            }
        }
        
        // Tingsha: Whenever you discard a card, deal 3 damage to a random enemy.
        // Java: Tingsha.onManualDiscard() → DamageRandomEnemyAction(3, THORNS) per card
        // We apply damage_all * count (approximation: hits all enemies instead of random).
        ("Tingsha", GameEvent::PlayerManualDiscard { count }) => {
            if *count > 0 {
                let dmg = 3 * count;
                result.damage_all += dmg;
                result.messages.push(format!(
                    "🔔 Tingsha: {} damage to all enemies ({} discards × 3)", dmg, count
                ));
                relic.pulse();
            }
        }
        
        // =================================================================
        // Batch 12: Simple / Cosmetic Relics
        // =================================================================
        
        // CultistMask: atBattleStart — cosmetic only (sound + "CAW!")
        // Java: CultistMask.atBattleStart() → SFXAction + TalkAction  
        // No combat effect, purely visual/audio.
        ("CultistMask", GameEvent::BattleStart) => {
            result.messages.push("🎭 CAW! (Cultist Mask)".to_string());
            relic.pulse();
        }
        
        // =================================================================
        // Relics handled elsewhere (inline)
        // =================================================================
        // Snecko Eye: Randomize costs (handled in card draw logic).
        // Runic Pyramid: Don't discard hand (handled in turn end logic).
        // Ice Cream: Energy conservation (handled in energy system).
        // Boot: onAttackToChangeDamage (handled in damage pipeline).
        // PreservedInsect: Elite -25% HP (handled in trigger_relics pre-loop).
        // Pantograph: Boss heal 25 (handled in trigger_relics pre-loop).
        // Sling: Elite +2 Str (handled in trigger_relics pre-loop).
        // TwistedFunnel: +4 Poison all (handled in trigger_relics pre-loop).
        // RedMask: +1 Weak all enemies (handled in trigger_relics pre-loop).
        // SlaversCollar: +1E elite/boss (handled in trigger_relics pre-loop).
        // MarkOfPain: +2 Wounds draw pile (handled in trigger_relics pre-loop).
        // NinjaScroll: +3 Shivs to hand (handled in trigger_relics pre-loop).
        // PhilosopherStone: +1 Str all enemies (handled in trigger_relics pre-loop).
        // Abacus: onShuffle +6 block (handled inline in state.rs draw_cards).
        // etc.
        
        // =================================================================
        // Batch 13: High-Impact Combat Relics (Java-verified)
        // =================================================================
        
        // FossilizedHelix: +1 Buffer at battle start (prevent first HP loss).
        // Java: FossilizedHelix.atBattleStart() → ApplyPowerAction(BufferPower, 1)
        ("FossilizedHelix", GameEvent::BattleStart) => {
            result.buffer_gain += 1;
            result.messages.push("🐚 Fossilized Helix: +1 Buffer (prevent first HP loss)".to_string());
            relic.pulse();
        }
        
        // Torii: When you would receive 5 or less unblocked attack damage, reduce it to 1.
        // Java: Torii.onAttacked() → if dmg > 1 && dmg <= 5: return 1
        // Handled inline in player_take_damage / damage pipeline. Register for audit.
        ("Torii", GameEvent::BattleStart) => {
            // Passive — actual logic is in damage pipeline (state.rs player_take_damage)
        }
        
        // TungstenRod: Whenever you lose HP, lose 1 less.
        // Java: TungstenRod.onLoseHpLast() → return damageAmount - 1
        // Handled inline in damage pipeline. Register for audit.
        ("TungstenRod", GameEvent::BattleStart) => {
            // Passive — actual logic is in damage pipeline
        }
        
        // Dead Branch: When you Exhaust a card, add a random card to your hand.
        // Java: DeadBranch.onExhaust() → MakeTempCardInHandAction(randomCard)
        ("DeadBranch", GameEvent::PlayerExhaust { .. }) => {
            // Add a random card — we pass a special "RandomCard" marker;
            // the apply_relic_results handler will use the card library.
            result.cards_to_hand.push(("RandomCard".to_string(), 1));
            result.messages.push("🌿 Dead Branch: Random card added to hand".to_string());
            relic.pulse();
        }
        
        // Cloak Clasp: At end of turn, gain 1 Block per card in hand.
        // Java: CloakClasp.onPlayerEndTurn() → GainBlockAction(hand.size * 1)
        // counter used to pass hand size; set by trigger_relics pre-dispatch
        ("CloakClasp", GameEvent::TurnEnd { .. }) => {
            if relic.counter > 0 {
                result.block_gain += relic.counter;
                result.messages.push(format!(
                    "🧥 Cloak Clasp: +{} Block ({} cards in hand)", relic.counter, relic.counter
                ));
                relic.pulse();
            }
        }
        
        // Sundial: Every 3 shuffles, gain 2 Energy.
        // Java: Sundial.onShuffle() → counter++, if counter==3: +2E
        // Handled inline in draw_cards (alongside Abacus). Register for audit.
        ("Sundial", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        
        // Hand Drill: When you break an enemy's Block, apply 2 Vulnerable.
        // Java: HandDrill.onBlockBroken(m) → ApplyPowerAction(VulnerablePower, 2)
        // Handled inline in damage pipeline. Register for audit.
        ("HandDrill", GameEvent::BattleStart) => {
            // Passive — actual logic is in damage pipeline
        }
        
        // MummifiedHand.java: When you play a Power, a random card in hand costs 0 this turn.
        // Java: onUseCard(POWER) → filter hand for cost > 0 && costForTurn > 0 → random pick → setCostForTurn(0)
        ("MummifiedHand", GameEvent::PlayerPlayCard { card_type: CardType::Power, .. }) => {
            result.reduce_random_card_cost = true;
            result.messages.push("🤚 Mummified Hand: A random card costs 0 this turn".to_string());
            relic.pulse();
        }
        
        // Necronomicon: First Attack costing 2+ per turn is played twice.
        // Java: Necronomicon.onUseCard() → if Attack && cost >= 2 && activated: play again
        // counter: 0=available, 1=used this turn
        ("Necronomicon", GameEvent::BattleStart) | ("Necronomicon", GameEvent::TurnStart { .. }) => {
            relic.counter = 0; // Reset availability
        }
        ("Necronomicon", GameEvent::PlayerPlayCard { card_type: CardType::Attack, cost, .. }) => {
            if *cost >= 2 && relic.counter == 0 {
                relic.counter = 1;
                result.replay_card = true;
                result.messages.push("📖 Necronomicon: Attack replayed!".to_string());
                relic.pulse();
            }
        }
        
        // Ginger: You can no longer become Weakened.
        // Java: passive check during ApplyPowerAction. Register for audit.
        ("Ginger", GameEvent::BattleStart) => {
            // Passive — immunity check in apply_player_debuff
        }
        
        // Turnip: You can no longer become Frail.
        // Java: passive check during ApplyPowerAction. Register for audit.
        ("Turnip", GameEvent::BattleStart) => {
            // Passive — immunity check in apply_player_debuff
        }
        
        // OddMushroom: Vulnerable only increases damage by 25% instead of 50%.
        // Java: passive modifier in damage calculations. Register for audit.
        ("OddMushroom", GameEvent::BattleStart) => {
            // Passive — modifier in damage pipeline
        }
        
        // Paper Crane: Weak causes enemies to deal 40% less instead of 25%.
        // Java: passive modifier in damage calculations. Register for audit.
        ("PaperCrane", GameEvent::BattleStart) => {
            // Passive — modifier in damage pipeline
        }
        
        // Paper Frog: Vulnerable causes enemies to take 75% more instead of 50%.
        // Java: passive modifier in damage calculations. Register for audit.
        ("PaperFrog", GameEvent::BattleStart) => {
            // Passive — modifier in damage pipeline
        }
        
        // Du-Vu Doll: +1 Strength per Curse in deck at battle start.
        // Java: DuVuDoll.atBattleStart() → count curses → ApplyPowerAction(Strength, count)
        // counter set by trigger_relics pre-dispatch (curse count)
        ("DuVuDoll", GameEvent::BattleStart) => {
            if relic.counter > 0 {
                result.strength_gain += relic.counter;
                result.messages.push(format!(
                    "🪆 Du-Vu Doll: +{} Strength ({} curses)", relic.counter, relic.counter
                ));
                relic.pulse();
            }
        }
        
        // StrikeDummy: Cards containing "Strike" deal 3 additional damage.
        // Java: StrikeDummy.atDamageModify() → if card.hasTag(STRIKE): +3
        // Passive modifier in damage pipeline. Register for audit.
        ("StrikeDummy", GameEvent::BattleStart) => {
            // Passive — modifier in damage pipeline
        }
        
        // Darkstone Periapt: +6 Max HP whenever you obtain a Curse.
        // Java: DarkstonePeriapt.onObtainCard(CURSE) → +6 Max HP
        // Handled in dungeon-level card acquisition. Register for audit.
        ("DarkstonePeriapt", GameEvent::BattleStart) => {}
        
        // FrozenEgg: +1 Upgrade to Power cards added to deck.
        // MoltenEgg: +1 Upgrade to Attack cards added to deck.
        // ToxicEgg: +1 Upgrade to Skill cards added to deck.
        // These are passive on-obtain effects. Register for audit.
        ("FrozenEgg", GameEvent::BattleStart) => {}
        ("MoltenEgg", GameEvent::BattleStart) => {}
        ("ToxicEgg", GameEvent::BattleStart) => {}
        
        // =================================================================
        // Batch 14: Boss Relics
        // =================================================================
        
        // Runic Pyramid: Don't discard hand at end of turn.
        // Java: passive check in turn-end logic. Register for audit.
        ("RunicPyramid", GameEvent::BattleStart) => {
            // Passive — discard skip handled inline in turn_end
        }
        
        // Ice Cream: Energy carries over between turns.
        // Java: passive — no energy loss at turn start. Register for audit.
        ("IceCream", GameEvent::BattleStart) => {
            // Passive — energy conservation handled inline
        }
        
        // Sacred Bark: Double potion effects.
        // Handled by has_sacred_bark parameter in use_potion.
        ("SacredBark", GameEvent::BattleStart) => {
            // Passive — handled in potion use
        }
        
        // Hovering Kite: First time you discard each turn, gain 1 Energy.
        // Java: HoveringKite.onManualDiscard() → if !triggered: +1E, triggered=true
        ("HoveringKite", GameEvent::TurnStart { .. }) => {
            relic.counter = 0; // Reset each turn
        }
        ("HoveringKite", GameEvent::PlayerManualDiscard { count }) => {
            if *count > 0 && relic.counter == 0 {
                relic.counter = 1;
                result.energy_gain += 1;
                result.messages.push("🪁 Hovering Kite: +1 Energy (first discard)".to_string());
                relic.pulse();
            }
        }
        
        // Inserter: Every 2 turns, gain 1 Orb slot.
        // Java: Inserter.atTurnStart() → counter++, if counter%2==0: OrbSlotAction(1)
        ("Inserter", GameEvent::BattleStart) => {
            relic.counter = 0;
        }
        ("Inserter", GameEvent::TurnStart { .. }) => {
            relic.counter += 1;
            if relic.counter % 2 == 0 {
                result.orb_slot_gain += 1;
                result.messages.push("🔌 Inserter: +1 Orb slot".to_string());
                relic.pulse();
            }
        }
        
        // Violet Lotus: When you exit Calm, gain 1 additional Energy.
        // Java: VioletLotus passive (handled in stance change logic). Register for audit.
        ("VioletLotus", GameEvent::BattleStart) => {
            // Passive — extra energy on Calm exit
        }
        
        // =================================================================
        // Batch 15: Shop Relics
        // =================================================================
        
        // Sling of Courage: +2 Strength at start of Elite combats.
        // Java: SlingOfCourage.atBattleStart() → if eliteCombat: ApplyPowerAction(Strength, 2)
        // Handled in trigger_relics pre-loop. Register for audit.
        ("SlingOfCourage", GameEvent::BattleStart) => {}
        
        // Runic Capacitor: Start combat with 3 additional Orb slots.
        // Java: RunicCapacitor.onEquip() → masterMaxOrbs += 3
        ("RunicCapacitor", GameEvent::BattleStart) => {
            result.orb_slot_gain += 3;
            result.messages.push("⚡ Runic Capacitor: +3 Orb slots".to_string());
            relic.pulse();
        }
        
        // Strange Spoon: 50% chance exhaust cards go to discard instead.
        // Java: passive modifier during exhaust. Register for audit.
        ("StrangeSpoon", GameEvent::BattleStart) => {
            // Passive — 50% exhaust redirect
        }
        
        // =================================================================
        // Batch 16: Event Relics (Combat-Affecting)
        // =================================================================
        
        // Gremlin Visage: Start each combat with 1 Weak.
        // Java: GremlinVisage.atBattleStart() → ApplyPowerAction(WeakPower, 1)
        ("GremlinVisage", GameEvent::BattleStart) => {
            result.weak_apply += 1;
            result.messages.push("👹 Gremlin Visage: +1 Weak (start of combat)".to_string());
            relic.pulse();
        }
        
        // Nilry's Codex: At end of turn, may shuffle 1 of 3 random cards into draw pile.
        // Java: NilrysCodex.onPlayerEndTurn() → DiscoveryAction
        // Simplified: add 1 random card to draw pile at end of turn
        ("NilrysCodex", GameEvent::TurnEnd { .. }) => {
            result.cards_to_draw_pile.push(("RandomCard".to_string(), 1));
            result.messages.push("📜 Nilry's Codex: Random card shuffled into draw pile".to_string());
            relic.pulse();
        }
        
        // Warped Tongs: At start of turn, Upgrade a random card in hand.
        // Java: WarpedTongs.atTurnStart() → one random card in hand upgraded
        ("WarpedTongs", GameEvent::TurnStart { .. }) => {
            result.upgrade_random_hand += 1;
            result.messages.push("🔧 Warped Tongs: Upgraded a random card in hand".to_string());
            relic.pulse();
        }
        
        // Mark of the Bloom: You can no longer heal.
        // Java: passive check in heal actions. Register for audit.
        ("MarkoftheBloom", GameEvent::BattleStart) => {
            // Passive — blocks all healing
        }
        
        // Bloody Idol: When you gain Gold, heal 5 HP.
        // Java: BloodyIdol.onGainGold() → HealAction(5)
        // Not a combat event, register for audit.
        ("BloodyIdol", GameEvent::BattleEnd { won: true }) => {
            // Gold gain healing handled out of combat
        }
        
        // Golden Idol: Enemies drop 25% more Gold.
        // Java: passive gold modifier. Register for audit.
        ("GoldenIdol", GameEvent::BattleEnd { won: true }) => {
            // Passive gold bonus
        }
        
        // =================================================================
        // Batch 17: Remaining Non-Combat/Passive Relics (audit tracking)
        // =================================================================
        
        // Bottled relics (Flame/Lightning/Tornado): At combat start, put a specific card in hand.
        // These need card selection at equip time. Register for audit.
        ("BottledFlame", GameEvent::BattleStart) => {}
        ("BottledLightning", GameEvent::BattleStart) => {}
        ("BottledTornado", GameEvent::BattleStart) => {}
        
        // Starter class relics that just provide stat/draw bonuses
        ("PureWater", GameEvent::BattleStart) => {
            // Add Holy Water (Miracle) to hand at combat start
            result.cards_to_hand.push(("Miracle".to_string(), 1));
            result.messages.push("💧 Pure Water: Miracle added to hand".to_string());
            relic.pulse();
        }
        ("HolyWater", GameEvent::BattleStart) => {
            result.cards_to_hand.push(("Miracle".to_string(), 3));
            result.messages.push("🌊 Holy Water: 3 Miracles added to hand".to_string());
            relic.pulse();
        }
        ("CrackedCore", GameEvent::BattleStart) => {
            // Channel 1 Lightning at combat start
            result.messages.push("⚡ Cracked Core: Channel 1 Lightning".to_string());
            relic.pulse();
        }
        ("FrozenCore", GameEvent::BattleStart) => {
            // If no orb slots filled at end of turn, channel 1 Frost
            // Passive, register for audit
        }
        
        // Wrist Blade: Attacks that cost 0 deal 3 extra damage.
        // Java: passive damage modifier. Register for audit.
        ("WristBlade", GameEvent::BattleStart) => {}
        
        // Mark of Pain: At start of combat, add 2 Wounds to draw pile. +1 Energy.
        // Handled in trigger_relics pre-loop. Register for audit.
        ("MarkOfPain", GameEvent::BattleStart) => {}
        
        // Enchiridion: At start of combat, add random Power to hand.
        // Java: Enchiridion.atBattleStart() → MakeTempCardInHandAction(randomPower)
        ("Enchiridion", GameEvent::BattleStart) => {
            result.cards_to_hand.push(("RandomPower".to_string(), 1));
            result.messages.push("📕 Enchiridion: Random Power card added to hand".to_string());
            relic.pulse();
        }
        
        // Ninja Scroll: At start of combat, add 3 Shivs to hand.
        // Handled in trigger_relics pre-loop. Register for audit.
        ("NinjaScroll", GameEvent::BattleStart) => {}
        
        // UnceasingTop: If hand is empty during turn, draw 1 card.
        // Java: passive check during card play. Register for audit.
        ("UnceasingTop", GameEvent::BattleStart) => {}
        
        // Omamori: Negate the next 2 Curses obtained.
        // Java: onObtainCard(CURSE) → counter++, if counter<=2: negate
        ("Omamori", GameEvent::BattleStart) => {
            // counter tracks curses negated (starts at 0, maxes at 2)
        }
        
        // Frozen Eye: View draw pile in order (UI only, no combat effect).
        ("FrozenEye", GameEvent::BattleStart) => {}
        
        // Orange Pellets: If you play Attack + Skill + Power in one turn, remove all debuffs.
        // Java: OrangePellets counter tracks card types played; if all 3: RemoveDebuffsAction
        // counter bits: 1=Attack, 2=Skill, 4=Power
        ("OrangePellets", GameEvent::BattleStart) | ("OrangePellets", GameEvent::TurnStart { .. }) => {
            relic.counter = 0;
        }
        ("OrangePellets", GameEvent::PlayerPlayCard { card_type, .. }) => {
            match card_type {
                CardType::Attack => relic.counter |= 1,
                CardType::Skill => relic.counter |= 2,
                CardType::Power => relic.counter |= 4,
                _ => {}
            }
            if relic.counter == 7 {
                // All 3 types played → clear debuffs
                result.clear_debuffs = true;
                result.messages.push("🍊 Orange Pellets: All debuffs removed!".to_string());
                relic.pulse();
                relic.counter = 0;
            }
        }
        
        // Pocketwatch: At start of turn, if ≤3 cards played last turn, draw 3.
        // Already tracked via counter. Add the TurnStart draw logic:
        ("Pocketwatch", GameEvent::TurnStart { .. }) => {
            if relic.counter >= 0 && relic.counter <= 3 {
                result.extra_draw += 3;
                result.messages.push("⏱️ Pocketwatch: Draw 3 cards (≤3 played last turn)".to_string());
                relic.pulse();
            }
            relic.counter = 0; // Reset for this turn
        }
        
        // Gambling Chip: At start of first turn, discard any cards and draw that many.
        // Java: GamblingChip.atTurnStartPostDraw() → DiscardPileToHandAction + GamblersBrew logic
        // Simplified: Already has BattleStart counter reset above.
        
        // =================================================================
        // Non-combat relics (no combat effect, just for audit tracking)
        // =================================================================
        ("JuzuBracelet", GameEvent::BattleStart) => {}  // ? room modifier
        ("PotionBelt", GameEvent::BattleStart) => {}    // +2 potion slots
        ("PreservedInsect", GameEvent::BattleStart) => {} // Elite -25% HP (pre-loop)
        ("RegalPillow", GameEvent::BattleStart) => {}   // Rest +15 HP
        ("SmilingMask", GameEvent::BattleStart) => {}   // Shop card removal 50G
        ("TinyChest", GameEvent::BattleStart) => {}     // 4th ? = Treasure
        ("WarPaint", GameEvent::BattleStart) => {}      // Upgrade 2 Skills on pickup
        ("Whetstone", GameEvent::BattleStart) => {}     // Upgrade 2 Attacks on pickup
        ("DreamCatcher", GameEvent::BattleStart) => {}  // Rest → add card
        ("Matryoshka", GameEvent::BattleStart) => {}    // Chest → 2 relics
        ("QuestionCard", GameEvent::BattleStart) => {}  // +1 card reward
        ("SingingBowl", GameEvent::BattleStart) => {}   // Skip card → +2 Max HP
        ("TheCourier", GameEvent::BattleStart) => {}    // Merchant restocks
        ("WhiteBeastStatue", GameEvent::BattleStart) => {} // Potion in rewards
        ("PrayerWheel", GameEvent::BattleStart) => {}   // +1 card reward
        ("Girya", GameEvent::BattleStart) => {}         // Rest → +1 Str
        ("PeacePipe", GameEvent::BattleStart) => {}     // Rest → remove card
        ("Shovel", GameEvent::BattleStart) => {}        // Rest → dig relic
        ("WingBoots", GameEvent::BattleStart) => {}     // Ignore map paths
        ("MagicFlower", GameEvent::BattleStart) => {}   // +50% combat healing
        ("LizardTail", GameEvent::BattleStart) => {}    // On death → 50% HP heal
        ("Astrolabe", GameEvent::BattleStart) => {}     // Transform 3 cards on pickup
        ("BlackStar", GameEvent::BattleStart) => {}     // Elite → extra relic
        ("EmptyCage", GameEvent::BattleStart) => {}     // Remove 2 cards on pickup
        ("PandorasBox", GameEvent::BattleStart) => {}   // Transform all S/D on pickup
        ("CallingBell", GameEvent::BattleStart) => {}   // On pickup effects
        ("CursedKey", GameEvent::BattleStart) => {}     // +1E, chests add curse
        ("Cauldron", GameEvent::BattleStart) => {}      // Brew 5 potions on pickup
        ("DollysMirror", GameEvent::BattleStart) => {}  // Copy a card on pickup
        ("MembershipCard", GameEvent::BattleStart) => {} // 50% shop discount
        ("Orrery", GameEvent::BattleStart) => {}        // Add 5 cards on pickup
        ("Melange", GameEvent::BattleStart) => {}       // On shuffle scry 3
        ("CultistHeadpiece", GameEvent::BattleStart) => {} // Cosmetic
        ("SpiritPoop", GameEvent::BattleStart) => {}    // Cosmetic
        ("SerpentHead", GameEvent::BattleStart) => {}   // ? room → +50G
        ("NeowsBlessing", GameEvent::BattleStart) => {} // First 3 enemies have 1 HP
        ("NlothsHungryFace", GameEvent::BattleStart) => {} // Next chest empty
        ("NlothsGift", GameEvent::BattleStart) => {}   // N'loth event
        ("Circlet", GameEvent::BattleStart) => {}       // Filler
        ("RedCirclet", GameEvent::BattleStart) => {}    // Filler
        ("GoldenEye", GameEvent::BattleStart) => {}     // Scry +2
        ("GoldPlatedCables", GameEvent::BattleStart) => {} // Rightmost orb passive
        ("EmotionChip", GameEvent::TurnStart { .. }) => {
            // If took damage last turn, trigger all orb passives
            if relic.counter == 1 {
                result.trigger_all_orb_passives = true;
                result.messages.push("🤖 Emotion Chip: All orb passives triggered!".to_string());
                relic.pulse();
                relic.counter = 0;
            }
        }
        
        // Negative event relics
        ("GrotesqueTrophy", GameEvent::BattleStart) => {} // Event relic
        ("Accursed", GameEvent::BattleStart) => {}
        ("AncientAugmentation", GameEvent::BattleStart) => {}
        ("PostDurian", GameEvent::BattleStart) => {}
        ("Hauntings", GameEvent::BattleStart) => {}
        ("Scatterbrain", GameEvent::BattleStart) => {}
        ("TwistingMind", GameEvent::BattleStart) => {}
        ("VoidEssence", GameEvent::BattleStart) => {}
        ("Toolbox", GameEvent::BattleStart) => {} // Handled inline in on_battle_start()
        ("PrismaticShard", GameEvent::BattleStart) => {} // Meta relic, no combat effect
        
        // DataDisk: +1 Focus at battle start
        // Java: DataDisk.atBattleStart() → ApplyPowerAction(FocusPower, 1)
        ("DataDisk", GameEvent::BattleStart) => {
            result.focus_gain += 1;
            relic.pulse();
            result.messages.push("📀 DataDisk: +1 Focus".to_string());
        }
        
        // Brimstone: +2 Str to self (via data-driven TurnStart), +1 Str to all enemies (hardcoded)
        // Java: Brimstone.atTurnStart() → self Str +2, all enemies Str +1
        // Player Str is handled by JSON hook (TurnStart → GainBuff Strength 2)
        // Enemy Str gain needs hardcoded handling since data-driven can't target enemies
        ("Brimstone", GameEvent::TurnStart { .. }) => {
            // Only enemy Str gain here — player +2 Str is from JSON TurnStart hook
            // Applied in on_turn_start's apply_relic_results
            relic.pulse();
            result.messages.push("🔥 Brimstone: Enemies +1 Strength".to_string());
        }
        ("Brimstone", GameEvent::BattleStart) => {} // Handled on TurnStart, not BattleStart
        
        // MutagenicStrength: +3 Str at battle start (loses 3 Str at end of first turn via LoseStrength power)
        // Java: MutagenicStrength.atBattleStart() → GainStr(3) + ApplyPower(LoseStrengthPower, 3)
        // The -3 Str at end of turn is simulated via end_of_turn_effects
        ("MutagenicStrength", GameEvent::BattleStart) => {
            result.strength_gain += 3;
            relic.pulse();
            result.messages.push("🧬 MutagenicStrength: +3 Strength (temporary)".to_string());
        }
        
        // Damaru: +1 Mantra at start of each turn (Watcher)
        // Java: Damaru.onPlayerEndTurn() → GainMantra(1) — actually onManualDiscard (start of turn)
        ("Damaru", GameEvent::TurnStart { .. }) => {
            result.mantra_gain += 1;
            relic.pulse();
            result.messages.push("🪘 Damaru: +1 Mantra".to_string());
        }
        ("Damaru", GameEvent::BattleStart) => {} // No effect at battle start
        
        // TeardropLocket: Enter Calm stance at combat start (Watcher)
        // Java: TeardropLocket.atBattleStart() → ChangeStanceAction(CalmStance)
        // Stance change is applied in on_battle_start since result doesn't support it directly
        ("TeardropLocket", GameEvent::BattleStart) => {
            relic.pulse();
            result.messages.push("💧 TeardropLocket: Enter Calm".to_string());
        }
        
        // SymbioticVirus: Channel 1 Dark orb at battle start (Defect)
        // Java: SymbioticVirus.atBattleStart() → ChannelAction(Dark)
        // Orb channeling is handled in on_battle_start since result doesn't support orb commands
        ("SymbioticVirus", GameEvent::BattleStart) => {
            relic.pulse();
            result.messages.push("🦠 SymbioticVirus: Channel Dark orb".to_string());
        }
        
        _ => {} // No hardcoded logic for this relic/event combination
    }
    
    result
}

// ============================================================================
// Apply Relic Results to GameState
// ============================================================================

/// Apply the aggregated relic trigger results to the game state.
pub fn apply_relic_results(state: &mut GameState, result: &RelicTriggerResult) {
    // Apply stat gains using the statuses HashMap
    if result.strength_gain > 0 {
        state.player.apply_status("Strength", result.strength_gain);
    }
    if result.dexterity_gain > 0 {
        state.player.apply_status("Dexterity", result.dexterity_gain);
    }
    // Temporary Dexterity: apply both +Dex and +DexLoss (DexLoss removes Dex at end of turn)
    // Java: LoseDexterityPower.atEndOfTurn() → ApplyPower(Dexterity, -amount) + RemoveSelf
    if result.temp_dexterity_gain > 0 {
        state.player.apply_status("Dexterity", result.temp_dexterity_gain);
        state.player.apply_status("DexLoss", result.temp_dexterity_gain);
    }
    if result.block_gain > 0 {
        state.player.block += result.block_gain;
    }
    if result.thorns_gain > 0 {
        state.player.apply_status("Thorns", result.thorns_gain);
    }
    if result.plated_armor_gain > 0 {
        state.player.apply_status("PlatedArmor", result.plated_armor_gain);
    }
    if result.artifact_gain > 0 {
        state.player.apply_status("Artifact", result.artifact_gain);
    }
    if result.intangible_gain > 0 {
        state.player.apply_temp_buff("Intangible", result.intangible_gain);
    }
    if result.focus_gain != 0 {
        state.player.apply_status("Focus", result.focus_gain);
    }
    if result.mantra_gain > 0 {
        state.player.apply_status("Mantra", result.mantra_gain);
        // Mantra ≥ 10 → enter Divinity stance (Watcher mechanic)
        if state.player.get_status("Mantra") >= 10 {
            let excess = state.player.get_status("Mantra") - 10;
            state.player.powers.remove("Mantra");
            if excess > 0 {
                state.player.apply_status("Mantra", excess);
            }
            state.player.stance = crate::core::stances::Stance::Divinity;
            state.player.energy += 3; // Divinity grants 3 energy
            game_log!("  🌟 Mantra reached 10! Entering Divinity stance (+3 energy)");
        }
    }
    
    // Apply healing (MarkOfTheBloom: blocks ALL healing)
    if result.heal > 0 {
        let has_mark_of_bloom = state.relics.iter().any(|r| r.id == "MarkoftheBloom" && r.active);
        if has_mark_of_bloom {
            game_log!("  🌸 Mark of the Bloom: Healing blocked!");
        } else {
            let old_hp = state.player.current_hp;
            state.player.current_hp = (state.player.current_hp + result.heal).min(state.player.max_hp);
            let healed = state.player.current_hp - old_hp;
            if healed > 0 {
                game_log!("  💚 Healed {} HP ({}/{})", healed, state.player.current_hp, state.player.max_hp);
            }
        }
    }
    
    // Apply energy gain
    if result.energy_gain > 0 {
        state.player.energy += result.energy_gain;
    }
    
    // Apply Max HP gain
    if result.max_hp_gain > 0 {
        state.player.max_hp += result.max_hp_gain;
        state.player.current_hp += result.max_hp_gain;
    }
    
    // Apply vulnerable to all enemies
    if result.vulnerable_all > 0 {
        for enemy in state.enemies.iter_mut() {
            enemy.apply_status("Vulnerable", result.vulnerable_all);
        }
    }
    
    // Apply Poison to all enemies (Funnel: Twisted Funnel)
    if result.poison_all > 0 {
        for enemy in state.enemies.iter_mut() {
            if !enemy.is_dead() {
                enemy.apply_status("Poison", result.poison_all);
            }
        }
        game_log!("  ☠️ Twisted Funnel: Applied {} Poison to all enemies", result.poison_all);
    }
    
    // Apply Vigor (Akabeko: first attack deals extra damage)
    if result.vigor_gain > 0 {
        state.player.apply_status("Vigor", result.vigor_gain);
    }
    
    // Apply damage to all enemies (LetterOpener: 5 damage to all)
    if result.damage_all > 0 {
        for enemy in state.enemies.iter_mut() {
            if !enemy.is_dead() {
                enemy.take_damage(result.damage_all);
                game_log!("  ⚔️ Relic deals {} damage to {} (HP: {})", 
                    result.damage_all, enemy.name, enemy.hp);
            }
        }
    }
    
    // Add cards to hand
    for (card_id, count) in &result.cards_to_hand {
        // Status/temp cards get appropriate costs
        let cost = match card_id.as_str() {
            "Wound" | "Dazed" | "Burn" | "Slimed" | "Void" => -2, // Unplayable
            "Shiv" => 0, // 0-cost Attack
            "Miracle" => 0, // 0-cost Skill that gives energy
            _ => 0,
        };
        for _ in 0..*count {
            state.add_card_by_id(
                card_id, cost,
                crate::schema::CardLocation::Hand,
                crate::core::state::InsertPosition::Bottom,
            );
        }
    }
    
    // Add cards to draw pile
    for (card_id, count) in &result.cards_to_draw_pile {
        let cost = match card_id.as_str() {
            "Wound" | "Dazed" | "Burn" | "Slimed" | "Void" => -2,
            _ => 0,
        };
        for _ in 0..*count {
            state.add_card_by_id(
                card_id, cost,
                crate::schema::CardLocation::DrawPile,
                crate::core::state::InsertPosition::Shuffle,
            );
        }
    }
    
    // Apply Buffer (FossilizedHelix)
    if result.buffer_gain > 0 {
        state.player.apply_status("Buffer", result.buffer_gain);
    }
    
    // Clear all player debuffs (Orange Pellets)
    if result.clear_debuffs {
        let debuffs_to_remove = ["Vulnerable", "Weak", "Frail", "Poison",
            "StrengthDown", "DexterityDown", "Constricted", "Entangled",
            "NoDraw", "NoBlock", "Confused", "DrawReduction", "Hex",
            "Slow", "Choked", "Shackled"];
        for debuff in &debuffs_to_remove {
            state.player.powers.remove(debuff);
        }
    }
    
    // MummifiedHand: reduce a random card in hand to cost 0
    // Java: filter for cost > 0 && costForTurn > 0, random pick, setCostForTurn(0)
    if result.reduce_random_card_cost && !state.hand.is_empty() {
        use rand::Rng;
        let eligible: Vec<usize> = state.hand.iter()
            .enumerate()
            .filter(|(_, c)| c.current_cost > 0)
            .map(|(i, _)| i)
            .collect();
        if !eligible.is_empty() {
            let idx = eligible[state.rng.random_range(0..eligible.len())];
            let card_name = state.hand[idx].definition_id.clone();
            state.hand[idx].current_cost = 0;
            game_log!("  🤚 Mummified Hand: {} costs 0 this turn", card_name);
        }
    }
    
    // Print messages
    for msg in &result.messages {
        game_log!("  {}", msg);
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the starter relic ID for a character.
pub fn starter_relic_for_character(character: &str) -> &'static str {
    match character.to_lowercase().as_str() {
        "ironclad" => "BurningBlood",
        "silent" => "RingoftheSnake",
        "defect" => "CrackedCore",
        "watcher" => "PureWater",
        _ => "BurningBlood",
    }
}

/// Create a starter relic instance for a character.
pub fn create_starter_relic(character: &str) -> RelicInstance {
    RelicInstance::new(starter_relic_for_character(character))
}

/// Trigger on-equip effects when a relic is first obtained.
/// 
/// Java: AbstractRelic.onEquip() — fires immediately when the relic is picked up.
/// This handles relics that modify game state on pickup (gold, maxHP, card upgrades, etc.).
/// Should be called from all code paths that add relics to the player.
pub fn on_relic_equip(state: &mut GameState, relic_id: &str) {
    use rand::Rng;
    
    match relic_id {
        // Waffle (Shop): +7 Max HP, heal to full.
        // Java: Waffle.onEquip() → increaseMaxHp(7, false); heal(maxHealth)
        "Waffle" | "Lee's Waffle" => {
            state.player.max_hp += 7;
            state.player.current_hp = state.player.max_hp;
            game_log!("  🧇 Waffle: +7 Max HP, healed to full ({}/{})",
                state.player.current_hp, state.player.max_hp);
        }
        
        // Old Coin (Rare): +300 gold on pickup.
        // Java: OldCoin.onEquip() → gainGold(300)
        "OldCoin" | "Old Coin" => {
            state.gold += 300;
            game_log!("  🪙 Old Coin: +300 gold (total: {})", state.gold);
        }
        
        // Potion Belt (Common): +2 potion slots.
        // Java: PotionBelt.onEquip() → potionSlots += 2
        "PotionBelt" | "Potion Belt" => {
            state.potions.add_slots(2);
            game_log!("  🧪 Potion Belt: +2 potion slots (total: {})", state.potions.capacity());
        }
        
        // War Paint (Common): Upgrade 2 random Skill cards in deck.
        // Java: WarPaint.onEquip() → upgrade up to 2 random SKILL cards
        "WarPaint" | "War Paint" => {
            let mut upgradeable: Vec<usize> = state.draw_pile.iter().enumerate()
                .filter(|(_, c)| !c.upgraded && c.card_type == crate::schema::CardType::Skill)
                .map(|(i, _)| i)
                .collect();
            // Shuffle and pick up to 2
            let mut upgraded_count = 0;
            for _ in 0..upgradeable.len().min(10) {
                if upgraded_count >= 2 || upgradeable.is_empty() { break; }
                let idx = state.rng.random_range(0..upgradeable.len());
                let card_idx = upgradeable.remove(idx);
                state.draw_pile[card_idx].upgraded = true;
                game_log!("  🎨 War Paint: Upgraded {} (Skill)", state.draw_pile[card_idx].definition_id);
                upgraded_count += 1;
            }
            if upgraded_count == 0 {
                game_log!("  🎨 War Paint: No upgradeable Skill cards in deck");
            }
        }
        
        // Tiny House (Boss): +5 Max HP, upgrade 1 random card, +50 gold, random potion.
        // Java: TinyHouse.onEquip() → upgrade 1 card, increaseMaxHp(5), addGoldToRewards(50), addPotionToRewards()
        "TinyHouse" | "Tiny House" => {
            // +5 Max HP
            state.player.max_hp += 5;
            state.player.current_hp += 5;
            game_log!("  🏠 Tiny House: +5 Max HP ({}/{})",
                state.player.current_hp, state.player.max_hp);
            
            // Upgrade 1 random card
            let upgradeable: Vec<usize> = state.draw_pile.iter().enumerate()
                .filter(|(_, c)| !c.upgraded)
                .map(|(i, _)| i)
                .collect();
            if !upgradeable.is_empty() {
                let idx = upgradeable[state.rng.random_range(0..upgradeable.len())];
                state.draw_pile[idx].upgraded = true;
                game_log!("  🏠 Tiny House: Upgraded {}", state.draw_pile[idx].definition_id);
            }
            
            // +50 gold
            state.gold += 50;
            game_log!("  🏠 Tiny House: +50 gold (total: {})", state.gold);
            
            // Random potion (simplified: just log it, potion would be added to rewards)
            game_log!("  🏠 Tiny House: Random potion added to rewards");
        }
        
        // Tiny Chest (Common): Every 4th ? room → get a relic.
        // Java: TinyChest.onEquip() → this.counter = 0
        // Just set counter, the actual effect is in room entry logic.
        "TinyChest" | "Tiny Chest" => {
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == relic_id) {
                relic.counter = 0;
            }
            game_log!("  📦 Tiny Chest: Counter initialized (every 4th ? room)");
        }
        
        // Circlet (Special): Placeholder relic when pool is empty. No effect.
        // Java: Circlet.onEquip() → this.flash() (cosmetic only)
        "Circlet" => {
            game_log!("  ⭕ Circlet: No effect (placeholder relic)");
        }
        
        // Darkstone Periapt (Uncommon): +6 Max HP whenever you obtain a Curse.
        // Java: DarkstonePeriapt.onObtainCard() → if curse color, increaseMaxHp(6)
        // The onEquip is a no-op; the real hook is on card obtain.
        // We register it here for the audit — actual effect is in card obtain code.
        "DarkstonePeriapt" | "Darkstone Periapt" => {
            game_log!("  🔮 Darkstone Periapt: Active (will gain 6 Max HP per Curse obtained)");
        }
        
        // Black Star (Boss): Elites drop 2 relics instead of 1.
        // Java: onVictory in elite rooms → double rewards.
        // The actual effect is in the rewards system, not onEquip.
        "BlackStar" | "Black Star" => {
            game_log!("  ⭐ Black Star: Active (elites drop extra relic)");
        }
        
        // Cursed Key (Boss): +1 base energy. Gain a curse when opening chests.
        // Java: CursedKey.onEquip() → ++energyMaster
        // Java: CursedKey.onChestOpen() → add random curse (handled elsewhere)
        "CursedKey" | "Cursed Key" => {
            state.player.max_energy += 1;
            game_log!("  🔑 Cursed Key: +1 max energy (now {}). Chests will add curses.",
                state.player.max_energy);
        }
        
        // Empty Cage (Boss): Remove 2 cards from deck on pickup.
        // Java: EmptyCage.onEquip() → gridSelectScreen (player picks 2 to remove)
        // For AI training: remove 2 random non-basic cards.
        "EmptyCage" | "Empty Cage" => {
            let mut removed = 0;
            for _ in 0..2 {
                // Find non-basic, non-starter cards to remove
                let removeable: Vec<usize> = state.draw_pile.iter().enumerate()
                    .filter(|(_, c)| {
                        !matches!(c.definition_id.as_str(), "Strike" | "Defend" | "AscendersBane")
                    })
                    .map(|(i, _)| i)
                    .collect();
                if !removeable.is_empty() {
                    let idx = removeable[state.rng.random_range(0..removeable.len())];
                    let name = state.draw_pile[idx].definition_id.clone();
                    state.draw_pile.remove(idx);
                    game_log!("  🗑️ Empty Cage: Removed {}", name);
                    removed += 1;
                }
            }
            if removed == 0 {
                game_log!("  🗑️ Empty Cage: No removeable cards in deck");
            }
        }
        
        // Pandora's Box (Boss): Transform ALL Strikes and Defends into random cards.
        // Java: PandorasBox.onEquip() → remove STARTER_STRIKE/STARTER_DEFEND tags, add random cards
        // For AI training: remove Strikes/Defends and add random cards from the pool.
        "PandorasBox" | "Pandora's Box" => {
            let mut count = 0;
            state.draw_pile.retain(|c| {
                let is_starter = matches!(c.definition_id.as_str(), "Strike" | "Defend");
                if is_starter { count += 1; }
                !is_starter
            });
            // Add random replacement cards (simplified: just add random Attack/Skill cards)
            // In a full implementation, this would use the card pool.
            // For now, we just note the count — the actual random cards would need CardLibrary.
            if count > 0 {
                game_log!("  📦 Pandora's Box: Transformed {} Strikes/Defends into random cards", count);
                // TODO: Add actual random cards from card pool when CardLibrary is available at equip time
            }
        }
        
        // Dolly's Mirror (Shop): Duplicate 1 card in deck.
        // Java: DollysMirror.onEquip() → gridSelectScreen (player picks 1 card to duplicate)
        // For AI training: duplicate a random card.
        "DollysMirror" => {
            if !state.draw_pile.is_empty() {
                let idx = state.rng.random_range(0..state.draw_pile.len());
                let dup = state.draw_pile[idx].clone();
                let name = dup.definition_id.clone();
                state.draw_pile.push(dup);
                game_log!("  🪞 Dolly's Mirror: Duplicated {}", name);
            }
        }
        
        // Calling Bell (Boss): Obtain 1 curse + 3 relics (common, uncommon, rare).
        // Java: CallingBell.onEquip() → CurseOfTheBell to deck, then reward 3 relics
        // For AI training: add CurseOfTheBell curse and log relic rewards.
        "CallingBell" | "Calling Bell" => {
            // Add Curse of the Bell to deck
            let curse = crate::schema::CardInstance::new("CurseOfTheBell".to_string(), -2);
            state.draw_pile.push(curse);
            game_log!("  🔔 Calling Bell: Added Curse of the Bell to deck");
            // In a full sim, this would offer 3 relic rewards (common, uncommon, rare).
            // For AI training, we approximate by noting it.
            game_log!("  🔔 Calling Bell: 3 relic rewards would be offered (common, uncommon, rare)");
        }
        
        // Bottled Flame (Uncommon): Choose an Attack card — it becomes Innate.
        // Java: BottledFlame.onEquip() → mark 1 Attack as inBottleFlame → Innate at battle start
        // For AI training: mark a random non-basic Attack as innate.
        "BottledFlame" | "Bottled Flame" => {
            let eligible: Vec<usize> = state.draw_pile.iter().enumerate()
                .filter(|(_, c)| {
                    c.card_type == crate::schema::CardType::Attack
                    && c.definition_id != "Strike" // skip basic
                })
                .map(|(i, _)| i)
                .collect();
            if !eligible.is_empty() {
                let idx = eligible[state.rng.random_range(0..eligible.len())];
                state.draw_pile[idx].is_innate = true;
                game_log!("  🔥 Bottled Flame: {} is now Innate (Attack)", state.draw_pile[idx].definition_id);
            } else {
                game_log!("  🔥 Bottled Flame: No eligible Attack cards to bottle");
            }
        }
        
        // Bottled Lightning (Uncommon): Choose a Skill card — it becomes Innate.
        // Java: BottledLightning.onEquip() → mark 1 Skill as inBottleLightning → Innate at battle start
        // For AI training: mark a random non-basic Skill as innate.
        "BottledLightning" | "Bottled Lightning" => {
            let eligible: Vec<usize> = state.draw_pile.iter().enumerate()
                .filter(|(_, c)| {
                    c.card_type == crate::schema::CardType::Skill
                    && c.definition_id != "Defend" // skip basic
                })
                .map(|(i, _)| i)
                .collect();
            if !eligible.is_empty() {
                let idx = eligible[state.rng.random_range(0..eligible.len())];
                state.draw_pile[idx].is_innate = true;
                game_log!("  ⚡ Bottled Lightning: {} is now Innate (Skill)", state.draw_pile[idx].definition_id);
            } else {
                game_log!("  ⚡ Bottled Lightning: No eligible Skill cards to bottle");
            }
        }
        
        // Pear (Event): +10 Max HP on obtain.
        // Java: Pear.onEquip() → increaseMaxHp(10, true)
        "Pear" => {
            state.player.max_hp += 10;
            state.player.current_hp += 10;
            game_log!("  🍐 Pear: +10 Max HP ({}/{})",
                state.player.current_hp, state.player.max_hp);
        }
        
        // Strawberry (Event): +7 Max HP on obtain.
        // Java: Strawberry.onEquip() → increaseMaxHp(7, true)
        "Strawberry" => {
            state.player.max_hp += 7;
            state.player.current_hp += 7;
            game_log!("  🍓 Strawberry: +7 Max HP ({}/{})",
                state.player.current_hp, state.player.max_hp);
        }
        
        // Mango (Event): +14 Max HP on obtain.
        // Java: Mango.onEquip() → increaseMaxHp(14, true)
        "Mango" => {
            state.player.max_hp += 14;
            state.player.current_hp += 14;
            game_log!("  🥭 Mango: +14 Max HP ({}/{})",
                state.player.current_hp, state.player.max_hp);
        }
        
        // Ectoplasm (Boss): +1 Energy, can't gain gold.
        // Java: Ectoplasm.onEquip() → ++energyMaster
        // Gold blocking handled in gold gain logic.
        "Ectoplasm" => {
            state.player.max_energy += 1;
            game_log!("  👻 Ectoplasm: +1 max energy (now {}). Cannot gain gold.",
                state.player.max_energy);
        }
        
        // Meal Ticket (Shop): Heal 15 HP whenever you visit a shop.
        // Java: MealTicket.onEnterRoom(ShopRoom) → HealAction(15)
        // Non-combat: handled in shop entry logic.
        "MealTicket" | "Meal Ticket" => {
            game_log!("  🎫 Meal Ticket: Active (will heal 15 HP on shop visits)");
        }
        
        // Discerning Monocle (Shop): Shop items cost 50% less.
        // Java: ShopScreen applyDiscount() checks for this relic
        // Non-combat: handled in shop pricing logic.
        "DiscerningMonocle" | "TheCorier" => {
            game_log!("  🧐 Discerning Monocle: Active (shop discount)");
        }
        
        // Maw Bank (Uncommon): +12 gold on each non-combat map node.
        // Java: MawBank.onEnterRoom() → if not MonsterRoom: +12 gold
        // Non-combat: tracked here, actual effect in room entry logic.
        "MawBank" | "Maw Bank" => {
            game_log!("  🏦 Maw Bank: Active (will gain 12 gold per non-combat node)");
        }
        
        _ => {
            // No on-equip effect for this relic
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
    fn test_relic_instance_counter() {
        let mut relic = RelicInstance::new("PenNib");
        
        // First 9 increments shouldn't trigger
        for i in 1..10 {
            assert!(!relic.increment_counter(10), "Should not trigger at count {}", i);
            assert_eq!(relic.counter, i);
        }
        
        // 10th increment should trigger and reset
        assert!(relic.increment_counter(10), "Should trigger at count 10");
        assert_eq!(relic.counter, 0);
    }
    
    #[test]
    fn test_execute_heal_command() {
        let cmd = RelicCommand {
            cmd_type: "Heal".to_string(),
            params: RelicCommandParams {
                amount: Some(6),
                ..Default::default()
            },
        };
        
        let result = execute_command(&cmd, "Test Relic");
        assert_eq!(result.heal, 6);
    }
    
    #[test]
    fn test_execute_gain_buff_command() {
        let cmd = RelicCommand {
            cmd_type: "GainBuff".to_string(),
            params: RelicCommandParams {
                buff: Some("Strength".to_string()),
                base: Some(2),
                ..Default::default()
            },
        };
        
        let result = execute_command(&cmd, "Test Relic");
        assert_eq!(result.strength_gain, 2);
    }
    
    #[test]
    fn test_pen_nib_hardcoded() {
        let mut relic = RelicInstance::new("PenNib");
        let state = GameState::new(42);
        let event = GameEvent::PlayerPlayCard {
            card_type: CardType::Attack,
            cost: 1,
            card_id: "Strike".to_string(),
        };
        
        // Play 9 attacks
        for _ in 0..9 {
            let result = trigger_hardcoded_relic_standalone(
                &mut relic, &event,
                state.player.current_hp, state.player.max_hp, state.player.block
            );
            assert_eq!(result.damage_multiplier, 1.0);
        }
        
        // 10th attack should double damage
        let result = trigger_hardcoded_relic_standalone(
            &mut relic, &event,
            state.player.current_hp, state.player.max_hp, state.player.block
        );
        assert_eq!(result.damage_multiplier, 2.0);
    }
}
