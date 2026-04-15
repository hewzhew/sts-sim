//! Verification Structs for Java-Rust Differential Testing
//!
//! These types define the "contract" between the Java recorder mod and the
//! Rust diff_driver. Java outputs JSONL matching these structs exactly;
//! Rust deserializes with serde_json::from_str().
//!
//! Design: Rust structs are the single source of truth for the schema.
//! Java outputs card.cardID as-is (e.g., "Strike_R"); mapping to Rust
//! CardId happens via CardDefinition.java_id.

use serde::{Deserialize, Serialize};

// ============================================================================
// Top-Level Replay Events
// ============================================================================

/// One line in the JSONL replay file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ReplayEvent {
    /// Run metadata (seed, class, ascension, starting deck/relics).
    #[serde(rename = "init")]
    Init(RunInit),

    /// Combat begins — initial state snapshot before any player action.
    #[serde(rename = "combat_start")]
    CombatStart(CombatStartEvent),

    /// A player action (play card, end turn, use potion) with before/after.
    #[serde(rename = "action")]
    Action(CombatAction),

    /// Combat ends.
    #[serde(rename = "combat_end")]
    CombatEnd(CombatEndEvent),
}

// ============================================================================
// Run Init
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInit {
    pub seed: i64,
    #[serde(rename = "class")]
    pub player_class: String,
    pub ascension: u32,
    /// Java card IDs of starting deck
    pub deck: Vec<String>,
    /// Java relic IDs
    pub relics: Vec<String>,
}

// ============================================================================
// Combat Events
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatStartEvent {
    pub floor: u32,
    /// Java monster IDs (e.g., "JawWorm", "Cultist")
    pub monster_ids: Vec<String>,
    /// Full state snapshot at combat start (before any player action)
    pub snapshot: CombatSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatAction {
    pub floor: u32,
    pub turn: u32,
    pub command: ActionCommand,
    /// State BEFORE the action executes
    pub before: CombatSnapshot,
    /// State AFTER the action fully resolves
    pub after: CombatSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEndEvent {
    pub floor: u32,
}

// ============================================================================
// Action Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ActionCommand {
    /// Play a card from hand.
    #[serde(rename = "play")]
    Play {
        /// 0-indexed card position in hand (already converted from Java's 1-indexed)
        card_index: usize,
        /// Java cardID of the card being played (e.g., "Strike_R")
        card_id: String,
        /// Target monster index (0-indexed), None for non-targeted cards
        target: Option<usize>,
    },

    /// End the player's turn.
    #[serde(rename = "end_turn")]
    EndTurn,

    /// Use or discard a potion.
    #[serde(rename = "potion")]
    Potion {
        /// "use" or "discard"
        use_type: String,
        /// Potion slot index (0-indexed)
        slot: usize,
        /// Java potion ID (e.g., "Block Potion")
        potion_id: String,
        /// Target monster index for targeted potions
        target: Option<usize>,
    },
}

// ============================================================================
// Combat Snapshot — the core state representation
// ============================================================================

/// Complete combat state at a point in time.
/// This is what both Java and Rust serialize for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatSnapshot {
    pub player: PlayerSnap,
    pub monsters: Vec<MonsterSnap>,

    /// Cards currently in hand
    pub hand: Vec<CardSnap>,

    /// Draw pile — only IDs, order may not match
    pub draw_pile: Vec<String>,
    /// Discard pile — only IDs
    pub discard_pile: Vec<String>,
    /// Exhaust pile — only IDs
    pub exhaust_pile: Vec<String>,

    pub turn: u32,
}

// ============================================================================
// Player / Monster / Card Snapshots
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnap {
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub energy: u8,
    pub powers: Vec<PowerSnap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterSnap {
    /// Java monster ID (e.g., "JawWorm")
    pub id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub is_gone: bool,
    pub powers: Vec<PowerSnap>,
    /// Monster intent (e.g., "ATTACK", "DEFEND", "BUFF")
    #[serde(default)]
    pub intent: String,
    /// Move ID byte
    #[serde(default)]
    pub move_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSnap {
    /// Java cardID as-is (e.g., "Strike_R", "Twin Strike", "Bash")
    pub id: String,
    pub cost: i32,
    pub upgrades: i32,
    /// Card UUID from Java (for tracking specific cards across piles)
    #[serde(default)]
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSnap {
    /// Java power ID (e.g., "Strength", "Vulnerable")
    pub id: String,
    pub amount: i32,
}
