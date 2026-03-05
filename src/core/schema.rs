//! Card schema definitions for data-driven card loading.
//! 
//! These structs mirror the JSON format produced by our Python ETL pipeline.

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Represents the color/class of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardColor {
    Red,       // Ironclad
    Green,     // Silent
    Blue,      // Defect
    Purple,    // Watcher
    Colorless,
    Curse,
}

/// Represents the type of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

/// Represents the rarity of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardRarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Curse,
}

/// Represents the targeting type for a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TargetType {
    #[default]
    #[serde(rename = "Self")]
    TargetSelf,
    Enemy,
    #[serde(rename = "AllEnemies")]
    AllEnemies,
    #[serde(rename = "RandomEnemy")]
    RandomEnemy,
}

/// Numeric value that can differ between base and upgraded versions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScalingValue {
    pub base: i32,
    pub upgrade: i32,
}

impl ScalingValue {
    /// Get the appropriate value based on upgrade state.
    #[inline]
    pub fn get(&self, upgraded: bool) -> i32 {
        if upgraded { self.upgrade } else { self.base }
    }
}

/// Amount that can be a fixed number or "ALL" (represented as -1 internally).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexibleAmount {
    #[serde(default, alias = "amount_base")]
    pub base: AmountValue,
    #[serde(default, alias = "amount_upgrade")]
    pub upgrade: AmountValue,
}

impl FlexibleAmount {
    /// Get the amount, where -1 represents "ALL".
    pub fn get(&self, upgraded: bool) -> i32 {
        let val = if upgraded { &self.upgrade } else { &self.base };
        val.as_i32()
    }
    
    /// Check if this amount means "all" cards.
    pub fn is_all(&self, upgraded: bool) -> bool {
        let val = if upgraded { &self.upgrade } else { &self.base };
        matches!(val, AmountValue::All(_))
    }
}

/// Represents an amount that can be numeric or "ALL".
/// For untagged enums, serde tries variants in order, so put String variant first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmountValue {
    /// String "ALL" means upgrade all cards
    All(AllMarker),
    /// Numeric amount
    Number(i32),
}

/// Helper struct to match the exact string "ALL"
#[derive(Debug, Clone)]
pub struct AllMarker;

impl<'de> serde::de::Deserialize<'de> for AllMarker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "ALL" {
            Ok(AllMarker)
        } else {
            Err(serde::de::Error::custom(format!("expected 'ALL', got '{}'", s)))
        }
    }
}

impl serde::Serialize for AllMarker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("ALL")
    }
}

impl Default for AmountValue {
    fn default() -> Self {
        AmountValue::Number(1)
    }
}

impl AmountValue {
    pub fn as_i32(&self) -> i32 {
        match self {
            AmountValue::Number(n) => *n,
            AmountValue::All(_) => -1, // Convention: -1 means "all"
        }
    }
}

/// Card selection parameters for commands that need to select cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSelection {
    #[serde(default)]
    pub mode: Option<String>,  // "Choose", "Random", "All"
    #[serde(default)]
    pub count: Option<i32>,
}

/// Wrapper that can handle both known and unknown commands gracefully.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedCommand(pub Result<CardCommand, RawCommand>);

/// Raw command data for commands we don't recognize yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCommand {
    #[serde(rename = "type")]
    pub command_type: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl<'de> serde::Deserialize<'de> for ParsedCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        
        // Strategy: Try multiple parsing approaches in order of likelihood
        // 1. Try as-is first (handles most commands with params)
        // 2. Try with empty params removed (handles unit variants like Unplayable)
        
        // First attempt: parse as-is
        if let Ok(cmd) = serde_json::from_value::<CardCommand>(value.clone()) {
            return Ok(ParsedCommand(Ok(cmd)));
        }
        
        // Second attempt: remove empty "params": {} for unit variants
        // This handles JSON like {"type": "Unplayable", "params": {}} which fails
        // because serde's adjacently tagged format expects no "params" for unit variants
        let has_empty_params = value.as_object()
            .and_then(|obj| obj.get("params"))
            .and_then(|p| p.as_object())
            .map_or(false, |o| o.is_empty());
        
        if has_empty_params {
            if let Some(obj) = value.as_object_mut() {
                obj.remove("params");
            }
            if let Ok(cmd) = serde_json::from_value::<CardCommand>(value.clone()) {
                return Ok(ParsedCommand(Ok(cmd)));
            }
        }
        
        // Fall back to raw command for unknown types
        match serde_json::from_value::<RawCommand>(value) {
            Ok(raw) => Ok(ParsedCommand(Err(raw))),
            Err(e) => Err(serde::de::Error::custom(format!("Failed to parse command: {}", e))),
        }
    }
}

impl ParsedCommand {
    /// Get the command if it was successfully parsed.
    pub fn as_known(&self) -> Option<&CardCommand> {
        self.0.as_ref().ok()
    }
    
    /// Get the raw command if it wasn't recognized.
    pub fn as_raw(&self) -> Option<&RawCommand> {
        self.0.as_ref().err()
    }
    
    /// Check if this is a known command type.
    pub fn is_known(&self) -> bool {
        self.0.is_ok()
    }
}

/// The bytecode command enum - the heart of our data-driven engine.
/// Each variant represents an atomic game action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum CardCommand {
    /// Deal damage to target(s).
    DealDamage {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        times: Option<i32>,
        /// Number of hits when upgraded (e.g., Pummel 4→5)
        #[serde(default)]
        times_upgrade: Option<i32>,
        /// Special scaling mode (e.g., "Block" for Body Slam)
        #[serde(default)]
        scaling: Option<String>,
    },
    
    /// Deal damage to ALL enemies.
    DealDamageAll {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        times: Option<i32>,
    },
    
    /// Modifier for strength scaling on damage (e.g., Heavy Blade).
    /// This modifies how strength applies to the previous DealDamage command.
    StrengthMultiplier {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Gain block for the player.
    GainBlock {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Apply a status effect to target(s).
    ApplyStatus {
        status: String,
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Apply status to ALL enemies.
    ApplyStatusAll {
        status: String,
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Draw cards from draw pile.
    DrawCards {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Gain energy this turn.
    GainEnergy {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Upgrade card(s) in a location.
    UpgradeCards {
        #[serde(default)]
        amount_base: AmountValue,
        #[serde(default)]
        amount_upgrade: AmountValue,
        #[serde(default)]
        target: CardLocation,
    },
    
    /// Exhaust this card after playing.
    ExhaustSelf {
        #[serde(default)]
        base_only: bool,
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Add a card to a destination pile.
    AddCard {
        card: String,
        destination: String,
        #[serde(default = "default_one")]
        count: i32,
    },
    
    /// Discard cards from hand.
    DiscardCards {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        random: bool,
    },
    
    /// Gain strength (or other buff).
    GainBuff {
        buff: String,
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Double a buff (e.g., "Double your Strength").
    DoubleBuff {
        buff: String,
        #[serde(default)]
        base_only: bool,
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Lose HP (self-damage).
    #[serde(alias = "LoseHP")]
    LoseHp {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Gain HP (heal).
    #[serde(alias = "GainHP")]
    GainHp {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Channel an orb (Defect).
    ChannelOrb {
        orb: String,
        #[serde(default = "default_one")]
        count: i32,
    },
    
    /// Evoke orb(s) (Defect).
    EvokeOrb {
        #[serde(default = "default_one")]
        count: i32,
        #[serde(default)]
        all: bool,
    },
    
    /// Gain focus (Defect).
    GainFocus {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Enter a stance (Watcher).
    EnterStance {
        stance: String,
    },
    
    /// Exit current stance (Watcher).
    ExitStance,
    
    /// Scry X cards (Watcher).
    Scry {
        base: i32,
        upgrade: i32,
    },
    
    /// Gain Mantra (Watcher).
    GainMantra {
        base: i32,
        upgrade: i32,
    },
    
    /// Retain this card.
    RetainSelf {
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Make this card Innate.
    InnateSelf {
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Apply a power/buff to self (e.g., Demon Form).
    ApplyPower {
        #[serde(default)]
        power: String,
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        amount: Option<i32>,
        #[serde(default)]
        upgrade_amount: Option<i32>,
    },
    
    /// Conditional effect (complex trigger).
    Conditional {
        #[serde(default)]
        condition: Option<serde_json::Value>,
        #[serde(default)]
        then_do: Option<Vec<serde_json::Value>>,
        #[serde(default)]
        else_do: Option<Vec<serde_json::Value>>,
    },
    
    /// Multi-hit attack with different damage per hit.
    MultiHit {
        #[serde(default)]
        damage_per_hit: i32,
        #[serde(default)]
        hits_base: i32,
        #[serde(default)]
        hits_upgrade: i32,
    },
    
    /// Unplayable card marker.
    Unplayable,
    
    // ========================================================================
    // Phase 1: Core Commands (新增)
    // ========================================================================
    
    /// Apply a temporary buff (Double Tap, Vigor, etc.)
    ApplyBuff {
        buff: String,
        #[serde(default)]
        amount: Option<serde_json::Value>,  // Can be ValueSource
        #[serde(default)]
        upgrade_amount: Option<i32>,
        #[serde(default)]
        target: Option<String>,
    },
    
    /// Apply a debuff to self
    ApplyDebuff {
        debuff: String,
        #[serde(default)]
        amount: i32,
        #[serde(default)]
        target: Option<String>,
    },
    
    /// Discard cards from hand
    Discard {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        select_mode: Option<String>,  // "random", "choose"
        #[serde(default)]
        filter: Option<serde_json::Value>,
    },
    
    /// Move card between piles
    MoveCard {
        #[serde(alias = "from")]
        from_pile: Option<String>,
        #[serde(alias = "to")]
        to_pile: Option<String>,
        #[serde(default)]
        select_mode: Option<String>,  // "choose", "random", "all"
        #[serde(default)]
        select: Option<CardSelection>,  // New: nested select object
        #[serde(default)]
        count: Option<i32>,
        #[serde(default)]
        upgrade_count: Option<i32>,
        #[serde(default)]
        insert_at: Option<String>,  // "top", "bottom", "random"
        #[serde(default)]
        filter: Option<serde_json::Value>,
        #[serde(default)]
        retain: Option<bool>,
    },
    
    /// Shuffle cards into draw pile
    ShuffleInto {
        card: Option<String>,
        #[serde(default)]
        destination: Option<String>,
        #[serde(default)]
        count: Option<i32>,
    },
    
    /// Put card on top of draw pile
    PutOnTop {
        #[serde(default)]
        source: Option<String>,  // "hand", "discard"
        #[serde(default)]
        select_mode: Option<String>,
        #[serde(default)]
        count: Option<i32>,
    },
    
    // ========================================================================
    // Phase 2: Extended Commands (新增)
    // ========================================================================
    
    /// End the current turn
    EndTurn,
    
    /// Heal HP
    Heal {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        amount: Option<serde_json::Value>,  // Can be ValueSource
    },
    
    LoseBuff {
        buff: String,
        #[serde(default)]
        amount: i32,
        /// Amount to lose when upgraded (if different from base)
        #[serde(default)]
        amount_upgrade: Option<i32>,
        #[serde(default)]
        all: bool,
        /// If true, this LoseBuff executes at end of turn instead of immediately
        #[serde(default)]
        end_of_turn: bool,
    },
    
    /// Remove enemy buff (e.g., Artifact)
    RemoveEnemyBuff {
        buff: String,
        #[serde(default)]
        amount: i32,
        /// Amount to remove (base value, used when `amount` is 0)
        #[serde(default)]
        base: i32,
        /// Amount to remove when upgraded
        #[serde(default)]
        upgrade: i32,
    },
    
    /// Exhaust a specific card from a pile
    ExhaustCard {
        #[serde(default)]
        pile: Option<String>,
        #[serde(default)]
        select_mode: Option<String>,
        #[serde(default)]
        count: Option<i32>,
        #[serde(default)]
        upgrade_count: Option<i32>,
    },
    
    /// Exhaust multiple cards
    ExhaustCards {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        pile: Option<String>,
        #[serde(default)]
        select_mode: Option<String>,
    },
    
    /// Increase this card's damage (Rampage)
    IncreaseDamage {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
    },
    
    DealDamageRandom {
        #[serde(default)]
        base: i32,
        #[serde(default)]
        upgrade: i32,
        #[serde(default)]
        times: Option<i32>,
        #[serde(default)]
        times_upgrade: Option<i32>,
    },
    
    // ========================================================================
    // Phase 3: Special Effects (新增)
    // ========================================================================
    
    /// Double current block
    DoubleBlock,
    
    /// Double current energy
    DoubleEnergy,
    
    /// Play top card from draw pile
    PlayTopCard {
        #[serde(default)]
        count: Option<i32>,
        #[serde(default)]
        exhaust: bool,
    },
    
    /// Discover (choose from random cards)
    Discover {
        #[serde(default, rename = "from")]
        from_count: Option<i32>,
        #[serde(default)]
        choose: Option<i32>,
    },
    
    /// Draw until hand has X cards
    DrawUntil {
        target: i32,
        #[serde(default)]
        upgrade: Option<i32>,
    },
    
    /// Draw until hand is full
    DrawUntilFull,
    
    /// Draw cards (alternate format)
    Draw {
        #[serde(default)]
        amount: i32,
        #[serde(default)]
        upgrade_amount: Option<i32>,
    },
    
    /// Execute enemy if HP below threshold (Judgment)
    Execute {
        threshold: i32,
        #[serde(default)]
        upgrade_threshold: Option<i32>,
    },
    
    /// Gain gold
    GainGold {
        #[serde(default)]
        amount: i32,
        #[serde(default)]
        upgrade: Option<i32>,
    },
    
    /// Gain max HP
    GainMaxHP {
        #[serde(default)]
        amount: i32,
        #[serde(default)]
        upgrade: Option<i32>,
    },
    
    /// Obtain a random potion
    ObtainPotion {
        #[serde(default)]
        source: Option<String>,
    },
    
    /// Remove all block from target
    RemoveBlock {
        #[serde(default)]
        target: Option<String>,
    },
    
    /// Set cost of all cards in pile
    SetCostAll {
        #[serde(default)]
        pile: Option<String>,
        #[serde(default)]
        cost: i32,
        #[serde(default)]
        permanent: bool,
    },
    
    /// Set cost of random card
    SetCostRandom {
        #[serde(default)]
        pile: Option<String>,
        #[serde(default)]
        cost: i32,
        #[serde(default)]
        permanent: bool,
    },
    
    /// Upgrade a single card
    UpgradeCard {
        #[serde(default)]
        select_mode: Option<String>,
        #[serde(default)]
        pile: Option<String>,
    },
    
    /// Take an extra turn (Vault)
    ExtraTurn,
    
    /// Multiply a status effect (Catalyst)
    MultiplyStatus {
        status: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        multiplier: i32,
        #[serde(default)]
        upgrade_multiplier: Option<i32>,
    },
    
    /// Double a status effect (simpler version)
    DoubleStatus {
        status: String,
        #[serde(default)]
        target: Option<String>,
    },
    
    // ========================================================================
    // Marker Commands (属性标记)
    // ========================================================================
    
    /// Mark card as Ethereal
    Ethereal {
        #[serde(default)]
        base_only: bool,
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Mark card as Innate
    Innate {
        #[serde(default)]
        base_only: bool,
        #[serde(default)]
        upgrade_only: bool,
    },
    
    /// Mark card as Retain
    Retain {
        #[serde(default)]
        base_only: bool,
        #[serde(default)]
        upgrade_only: bool,
    },
}

fn default_one() -> i32 { 1 }

/// Location for card operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CardLocation {
    #[default]
    Hand,
    DrawPile,
    DiscardPile,
    ExhaustPile,
    /// Deck = Hand + DrawPile + DiscardPile (used by Apotheosis "upgrade ALL")
    Deck,
}

/// The logic block containing target type and command sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLogic {
    #[serde(default)]
    pub target_type: TargetType,
    
    #[serde(default)]
    pub commands: SmallVec<[ParsedCommand; 4]>,
    
    /// Condition strings from JSON (e.g., "TurnTrigger: At the start of your turn").
    /// Used by the engine to determine if commands are triggered vs immediate.
    #[serde(default)]
    pub conditions: Vec<String>,
    
    #[serde(default)]
    pub keywords_used: SmallVec<[String; 4]>,
}

impl Default for CardLogic {
    fn default() -> Self {
        Self {
            target_type: TargetType::default(),
            commands: SmallVec::new(),
            conditions: Vec::new(),
            keywords_used: SmallVec::new(),
        }
    }
}

/// The complete card definition as loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDefinition {
    pub id: String,
    
    #[serde(default)]
    pub name: String,
    
    #[serde(rename = "type")]
    pub card_type: CardType,
    
    #[serde(default)]
    pub cost: i32,
    
    #[serde(default)]
    pub color: Option<CardColor>,
    
    #[serde(default)]
    pub rarity: Option<CardRarity>,
    
    #[serde(default)]
    pub logic: CardLogic,
    
    #[serde(default)]
    pub original_text: Option<String>,
}

/// A runtime card instance in the player's deck/hand/etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInstance {
    /// Reference to the card definition ID.
    pub definition_id: String,
    /// Whether this instance is upgraded.
    pub upgraded: bool,
    /// Base/permanent cost — Java's `cost`. Modified by `modifyCostForCombat`.
    /// Reset does NOT touch this; it persists across turns.
    pub base_cost: i32,
    /// Current turn cost — Java's `costForTurn`. This is the actual play cost.
    /// Reset to `base_cost` at start of each turn via `reset_cost_for_turn()`.
    pub current_cost: i32,
    /// Whether this card has the Ethereal keyword (exhaust at end of turn if still in hand).
    pub is_ethereal: bool,
    /// Whether this card has the Innate keyword (drawn at start of combat).
    /// Java: `card.isInnate`. Innate cards are placed on top of draw pile in initializeDeck().
    #[serde(default)]
    pub is_innate: bool,
    /// Whether this card has the selfRetain flag (auto-retains at end of turn).
    /// Java: card.selfRetain. Cards with this flag stay in hand without Runic Pyramid.
    #[serde(default)]
    pub self_retain: bool,
    /// Card type (Attack/Skill/Power/Status/Curse).
    /// Defaults to Attack if not resolved from CardLibrary.
    #[serde(default = "CardInstance::default_card_type")]
    pub card_type: CardType,
}

impl CardInstance {
    /// Default card type for serde deserialization.
    fn default_card_type() -> CardType {
        CardType::Attack
    }

    pub fn new(definition_id: String, cost: i32) -> Self {
        let is_ethereal = Self::check_ethereal(&definition_id);
        let is_innate = Self::check_innate(&definition_id);
        let self_retain = Self::check_self_retain(&definition_id);
        Self {
            definition_id,
            upgraded: false,
            base_cost: cost,
            current_cost: cost,
            is_ethereal,
            is_innate,
            self_retain,
            card_type: CardType::Attack,
        }
    }
    
    /// Create a new card instance with a &str ID (convenience method).
    pub fn new_basic(definition_id: &str, cost: i32) -> Self {
        let is_ethereal = Self::check_ethereal(definition_id);
        let is_innate = Self::check_innate(definition_id);
        let self_retain = Self::check_self_retain(definition_id);
        Self {
            definition_id: definition_id.to_string(),
            upgraded: false,
            base_cost: cost,
            current_cost: cost,
            is_ethereal,
            is_innate,
            self_retain,
            card_type: CardType::Attack,
        }
    }
    
    pub fn new_upgraded(definition_id: String, cost: i32) -> Self {
        let is_ethereal = Self::check_ethereal(&definition_id);
        let is_innate = Self::check_innate(&definition_id);
        let self_retain = Self::check_self_retain(&definition_id);
        Self {
            definition_id,
            upgraded: true,
            base_cost: cost,
            current_cost: cost,
            is_ethereal,
            is_innate,
            self_retain,
            card_type: CardType::Attack,
        }
    }

    /// Create a CardInstance from a CardLibrary lookup.
    /// Automatically resolves card_type, cost, and ethereal from the definition.
    pub fn from_library(definition_id: &str, library: &crate::loader::CardLibrary) -> Self {
        if let Ok(def) = library.get(definition_id) {
            Self {
                definition_id: definition_id.to_string(),
                upgraded: false,
                base_cost: def.cost,
                current_cost: def.cost,
                is_ethereal: Self::check_ethereal(definition_id),
                is_innate: Self::check_innate(definition_id),
                self_retain: Self::check_self_retain(definition_id),
                card_type: def.card_type,
            }
        } else {
            // Fallback for unknown cards
            Self::new(definition_id.to_string(), 1)
        }
    }

    /// Set card type (builder pattern).
    pub fn with_type(mut self, card_type: CardType) -> Self {
        self.card_type = card_type;
        self
    }

    /// Resolve card_type from a CardLibrary.
    /// Call this to fix up instances created without library access.
    pub fn resolve_type(&mut self, library: &crate::loader::CardLibrary) {
        if let Ok(def) = library.get(&self.definition_id) {
            self.card_type = def.card_type;
        }
    }


    /// Check if a card ID is known to be Ethereal.
    /// This is a static lookup based on card definitions that have the Ethereal command.
    pub(crate) fn check_ethereal(definition_id: &str) -> bool {
        matches!(definition_id,
            "Dazed" | "Apparition" | "GhostlyArmor" | "Ghostly" 
            | "Reprieve" | "Safety" | "Insight"
        )
    }

    /// Check if a card ID has the Innate keyword.
    /// Java: card.isInnate — these cards are placed on top of draw pile at combat start.
    /// Note: Some cards gain Innate when upgraded (e.g. Battle Trance+).
    pub(crate) fn check_innate(definition_id: &str) -> bool {
        matches!(definition_id,
            "Dramatic Entrance" | "Bandage Up" | "Panache" | "Sadistic Nature"
            | "HandOfGreed" | "Swift Strike"
        )
    }

    /// Check if a card ID has the selfRetain flag.
    /// Java: card.selfRetain — these cards stay in hand at end of turn automatically.
    pub(crate) fn check_self_retain(definition_id: &str) -> bool {
        matches!(definition_id,
            // Watcher cards with selfRetain
            "Blasphemy" | "Scrawl" | "Windmill Strike" | "Sands of Time"
            | "Battle Hymn" | "Fasting"
            // Colorless
            | "Finesse" | "Flash of Steel"
        )
    }

    // ========================================================================
    // Cost Modification API (mirrors Java's setCostForTurn / modifyCostForCombat)
    // ========================================================================

    /// Temporary cost set for this turn only (Java: setCostForTurn).
    /// Resets to base_cost at start of next turn.
    /// Used by: Corruption (Skills cost 0), Enlightenment, Snecko Eye randomization.
    pub fn set_cost_for_turn(&mut self, cost: i32) {
        if self.current_cost >= 0 {
            self.current_cost = cost.max(0);
        }
    }

    /// Permanent cost modification for the rest of combat (Java: modifyCostForCombat).
    /// Modifies BOTH base_cost and current_cost.
    /// Used by: Madness (random card permanently costs 0).
    pub fn modify_cost_for_combat(&mut self, delta: i32) {
        if self.current_cost > 0 {
            self.current_cost = (self.current_cost + delta).max(0);
            self.base_cost = self.current_cost;
        } else if self.base_cost >= 0 {
            self.base_cost = (self.base_cost + delta).max(0);
            self.current_cost = 0;
        }
    }

    /// Reset current_cost to base_cost (Java: resetAttributes).
    /// Called at the start of each turn to undo temporary cost changes.
    pub fn reset_cost_for_turn(&mut self) {
        self.current_cost = self.base_cost;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scaling_value() {
        let sv = ScalingValue { base: 8, upgrade: 10 };
        assert_eq!(sv.get(false), 8);
        assert_eq!(sv.get(true), 10);
    }
    
    #[test]
    fn test_amount_value_parsing() {
        let num: AmountValue = serde_json::from_str("5").unwrap();
        assert_eq!(num.as_i32(), 5);
        
        let all: AmountValue = serde_json::from_str("\"ALL\"").unwrap();
        assert_eq!(all.as_i32(), -1);
    }
}
