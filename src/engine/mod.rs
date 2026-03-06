//! Engine module — the heart of the data-driven Slay the Spire simulator.
//!
//! Split from a single engine.rs into submodules matching Java's architecture:
//! - `commands` — Card command interpreter (≈ Java actions/common/)
//! - `combat` — Combat flow, card play, enemy turns (≈ Java AbstractRoom + GameActionManager)
//! - `navigation` — Map traversal and room transitions (≈ Java AbstractDungeon)
//! - `events` — Event system integration (≈ Java events/)
//! - `potions_use` — Potion effect application (≈ Java potions/)

pub mod commands;
pub mod combat;
pub mod navigation;
pub mod events;
pub mod potions_use;
pub mod card_overrides;

#[cfg(test)]
mod tests;

// ============================================================================
// Shared types — used across multiple submodules
// ============================================================================

use crate::schema::{CardInstance, CardLocation};
use crate::state::GameState;

/// Parse a CardLocation from a string.
pub(crate) fn parse_location(s: &str) -> CardLocation {
    match s.to_lowercase().as_str() {
        "hand" => CardLocation::Hand,
        "draw" | "drawpile" | "draw_pile" | "draw pile" | "deck" => CardLocation::DrawPile,
        "discard" | "discardpile" | "discard_pile" | "discard pile" => CardLocation::DiscardPile,
        "exhaust" | "exhaustpile" | "exhaust_pile" | "exhaust pile" => CardLocation::ExhaustPile,
        _ => CardLocation::Hand, // Default
    }
}

/// Helper function to upgrade cards in a slice.
pub(crate) fn upgrade_cards_in_slice(cards: &mut [CardInstance], upgrade_all: bool, count: i32) -> i32 {
    let mut upgraded_count = 0;
    let limit = if upgrade_all { cards.len() } else { count as usize };
    
    for card in cards.iter_mut().take(limit) {
        if !card.upgraded {
            card.upgraded = true;
            upgraded_count += 1;
        }
    }
    upgraded_count
}

/// Result of applying a command, for logging/debugging.
#[derive(Debug)]
pub enum CommandResult {
    DamageDealt { target: String, amount: i32, killed: bool },
    BlockGained { amount: i32 },
    StatusApplied { target: String, status: String, stacks: i32 },
    CardsDrawn { count: i32 },
    EnergyGained { amount: i32 },
    CardsUpgraded { count: i32, location: CardLocation },
    CardExhausted,
    CardAdded { card: String, destination: String },
    CardsDiscarded { count: i32 },
    BuffGained { buff: String, amount: i32 },
    BuffDoubled { buff: String },
    HpLost { amount: i32 },
    HpGained { amount: i32 },
    ConditionalExecuted { condition_met: bool },
    Skipped { reason: String },
    TriggerRegistered,
    GoldGained { amount: i32 },
    CostModified { count: i32 },
    StatusMultiplied { status: String, old: i32, new: i32 },
    Unknown,
}

// ============================================================================
// Condition Evaluation
// ============================================================================

/// Supported condition types for Conditional commands.
#[derive(Debug)]
pub enum Condition {
    /// Last attack killed the target
    Fatal,
    /// Enemy HP is at or below threshold
    EnemyHpBelow { threshold: i32 },
    /// Player HP is at or below threshold  
    PlayerHpBelow { threshold: i32 },
    /// Hand is full (10 cards)
    HandFull,
    /// Hand is empty
    HandEmpty,
    /// Enemy has a specific status
    EnemyHasStatus { status: String, min_stacks: i32 },
    /// Player has a specific status/buff
    PlayerHasStatus { status: String, min_stacks: i32 },
    /// Player is in a specific stance (Wrath, Calm, Divinity)
    /// Java: AbstractDungeon.player.stance.ID.equals(stanceId)
    InStance { stance: String },
    /// Always true (for else branches that should always execute)
    Always,
    /// Unknown condition - defaults to false
    Unknown(String),
}

impl Condition {
    /// Parse a condition from JSON Value
    pub fn from_json(value: &serde_json::Value) -> Self {
        if let Some(obj) = value.as_object() {
            if let Some(cond_type) = obj.get("type").and_then(|v| v.as_str()) {
                match cond_type {
                    "Fatal" | "fatal" | "IfFatal" => Condition::Fatal,
                    "HandFull" | "hand_full" => Condition::HandFull,
                    "HandEmpty" | "hand_empty" => Condition::HandEmpty,
                    "InStance" | "in_stance" => {
                        let stance = obj.get("params")
                            .and_then(|p| p.get("stance"))
                            .and_then(|v| v.as_str())
                            .or_else(|| obj.get("stance").and_then(|v| v.as_str()))
                            .unwrap_or("")
                            .to_string();
                        Condition::InStance { stance }
                    }
                    "EnemyHpBelow" | "enemy_hp_below" => {
                        let threshold = obj.get("threshold")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        Condition::EnemyHpBelow { threshold }
                    }
                    "PlayerHpBelow" | "player_hp_below" => {
                        let threshold = obj.get("threshold")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        Condition::PlayerHpBelow { threshold }
                    }
                    "EnemyHasStatus" | "enemy_has_status" => {
                        let status = obj.get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let min_stacks = obj.get("min_stacks")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(1) as i32;
                        Condition::EnemyHasStatus { status, min_stacks }
                    }
                    "PlayerHasStatus" | "player_has_status" => {
                        let status = obj.get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let min_stacks = obj.get("min_stacks")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(1) as i32;
                        Condition::PlayerHasStatus { status, min_stacks }
                    }
                    other => Condition::Unknown(other.to_string()),
                }
            } else {
                Condition::Unknown("no type field".to_string())
            }
        } else if let Some(s) = value.as_str() {
            // Simple string conditions
            match s {
                "Fatal" | "fatal" | "IfFatal" => Condition::Fatal,
                "HandFull" | "hand_full" => Condition::HandFull,
                "HandEmpty" | "hand_empty" => Condition::HandEmpty,
                other => Condition::Unknown(other.to_string()),
            }
        } else {
            Condition::Unknown(format!("{:?}", value))
        }
    }
    
    /// Evaluate the condition against the current game state
    pub fn evaluate(&self, state: &GameState, target_idx: Option<usize>) -> bool {
        match self {
            Condition::Fatal => state.was_last_attack_fatal(),
            
            Condition::HandFull => state.hand.len() >= 10,
            
            Condition::HandEmpty => state.hand.is_empty(),
            
            Condition::EnemyHpBelow { threshold } => {
                if let Some(idx) = target_idx {
                    if let Some(enemy) = state.enemies.get(idx) {
                        return enemy.hp <= *threshold;
                    }
                }
                // Check first living enemy
                state.enemies.iter()
                    .find(|e| !e.is_dead())
                    .map_or(false, |e| e.hp <= *threshold)
            }
            
            Condition::PlayerHpBelow { threshold } => {
                state.player.current_hp <= *threshold
            }
            
            Condition::EnemyHasStatus { status, min_stacks } => {
                if let Some(idx) = target_idx {
                    if let Some(enemy) = state.enemies.get(idx) {
                        return enemy.get_buff(status) >= *min_stacks;
                    }
                }
                state.enemies.iter()
                    .find(|e| !e.is_dead())
                    .map_or(false, |e| e.get_buff(status) >= *min_stacks)
            }
            
            Condition::PlayerHasStatus { status, min_stacks } => {
                // Check player powers (unified with temp buffs)
                state.player.get_status(status) >= *min_stacks
            }
            
            Condition::InStance { stance } => {
                use crate::core::stances::Stance;
                let target_stance = Stance::from_str(stance);
                state.player.stance == target_stance
            }
            
            Condition::Always => true,
            
            Condition::Unknown(name) => {
                game_log!("  ⚠ Unknown condition: {}, defaulting to false", name);
                false
            }
        }
    }
}

// ============================================================================
// Re-exports — preserve existing public API
// ============================================================================

// From commands.rs
pub use commands::apply_command;
pub(crate) use commands::{apply_hook_effects, calculate_card_damage};

// From combat.rs
pub use combat::{
    play_card, play_card_from_hand, simulate_turn,
    execute_enemy_turn, plan_enemy_moves,
    on_battle_start, on_turn_start, on_turn_start_post_draw, on_turn_end, on_battle_end,
    all_enemies_dead, player_dead,
};

// From navigation.rs
pub use navigation::{
    NodeResult, ActTransitionResult,
    proceed_to_node, get_valid_moves,
    finish_rewards, finish_boss_rewards,
    leave_shop, leave_rest,
    on_combat_victory, on_player_death,
};

// From events.rs
pub use events::{
    EventProcessResult,
    build_event_pool_context, process_event_commands,
    execute_event_option, finish_event, complete_card_selection,
    get_available_event_options,
};

// From potions_use.rs
pub use potions_use::use_potion;
