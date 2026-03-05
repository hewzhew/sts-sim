#![allow(unused_imports, unused_variables, dead_code, unused_mut, unused_assignments)]
//! # Slay the Spire Simulator
//!
//! A high-performance, headless simulator for Slay the Spire,
//! designed for Reinforcement Learning training.
//!
//! ## Features
//!
//! - **Data-driven**: Card and enemy logic loaded from JSON, not hardcoded
//! - **Deterministic**: Seeded RNG for reproducible simulations
//! - **High-performance**: SmallVec optimization, Rayon parallelism
//! - **Python bindings**: PyO3 FFI for Gym-style RL training

// ============================================================================
// Global verbose flag for log suppression during RL training
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to control game log output.
/// Set to `false` during RL training to suppress verbose println output.
/// Default: `true` (all output enabled, backward compatible).
pub static VERBOSE: AtomicBool = AtomicBool::new(true);

/// Set the global verbose flag from anywhere.
pub fn set_verbose(v: bool) {
    VERBOSE.store(v, Ordering::Relaxed);
}

/// Check if verbose output is enabled.
#[inline]
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Game log macro — like println! but only prints when VERBOSE is true.
/// Use this instead of println! for all gameplay/combat/loader output.
#[macro_export]
macro_rules! game_log {
    ($($arg:tt)*) => {
        if $crate::VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}

// ============================================================================
// Directory modules
// ============================================================================

pub mod core;            // schema, state, loader
pub mod powers_mod;      // power_set (PowerSet/PowerDefinition), hooks (PowerId/dispatch)
pub mod monsters;        // enemy, encounters (dungeon.rs), act_config
pub mod rooms;           // shop, campfire, events
pub mod dungeon_mod;     // map, rewards
pub mod items;           // relics, potions
pub mod ai;              // encoding, features, card_features, mcts
pub mod engine;          // commands, combat, navigation, events, potions_use
pub mod interop;         // PyO3 bindings

#[cfg(test)]
mod card_tests;

pub mod testing;

// ============================================================================
// Convenience aliases — preserve old crate::xxx paths via pub use
// ============================================================================

// These allow existing code to keep using `crate::schema`, `crate::state`, etc.
pub use core::schema;
pub use core::state;
pub use core::loader;
pub use powers_mod::power_set as powers;
pub use powers_mod::hooks as power_hooks;
pub use monsters::enemy;
pub use monsters::encounters as dungeon;
pub use monsters::act_config;
pub use rooms::shop;
pub use rooms::campfire;
pub use rooms::events;
pub use dungeon_mod::map;
pub use dungeon_mod::rewards;

// ============================================================================
// Re-export common types
// ============================================================================

pub use engine::{apply_command, play_card, play_card_from_hand, CommandResult};
pub use engine::{on_battle_start, on_turn_start, on_turn_start_post_draw, on_turn_end, on_battle_end};
pub use engine::{all_enemies_dead, player_dead};
pub use engine::{
    NodeResult, proceed_to_node, get_valid_moves,
    finish_rewards, leave_shop, leave_rest, finish_event,
    on_combat_victory, on_player_death,
};
pub use ai::encoding::{encode_state, OBS_DIM as ENCODING_OBS_DIM};
pub use ai::features::{encode_observation, get_action_mask, OBS_DIM};
pub use core::loader::{CardLibrary, MonsterLibrary, LoaderError};
pub use ai::mcts::{MctsNode, MctsSearcher};

pub use items::potions::{
    PotionDefinition, PotionLibrary, PotionError, PotionSlots,
    PotionRarity, PotionTarget, PotionClass, PotionCommand,
    MAX_POTION_SLOTS,
};

pub use core::schema::{CardCommand, CardDefinition, CardInstance, CardType};
pub use core::state::{Enemy, GameState, GamePhase, Player, ShopState, ShopCard, ShopRelic, ShopPotion, CampfireRelicState};

// Re-export enemy AI types
pub use monsters::enemy::{
    Intent, V5Value, V5HpRange, V5Effect, V5Card, V5Move, V5AscOverride,
    MonsterDefinition, MonsterState,
};

// Re-export relic types
pub use items::relics::{
    GameEvent, RelicDefinition, RelicInstance, RelicLibrary, RelicTier,
    trigger_relics, apply_relic_results, create_starter_relic,
};

// Re-export power system types
pub use powers::{
    PowerDefinition, PowerLibrary, PowerSet, PowerType, StackType, PowerTrigger,
    power_ids,
};

// Re-export shop types (logic functions)
pub use rooms::shop::{
    ShopResult, generate_shop, CardPricing, RelicPricing, PotionPricing, PurgePricing,
};

// Re-export campfire types
pub use rooms::campfire::{
    CampfireOption, CampfireResult, RestHealing,
    get_available_options, execute_option as execute_campfire_option,
};

// Re-export dungeon/encounter types
pub use monsters::encounters::{
    EncounterResult, get_random_encounter, get_encounter_pool,
    ActConfig, WeightedEncounter, get_upgraded_card_chance,
    MonsterSpawn, spawn_encounter,
    ACT1_WEAK_POOL, ACT1_STRONG_POOL, ACT1_ELITE_POOL, ACT1_BOSS_POOL,
    ACT2_WEAK_POOL, ACT2_STRONG_POOL, ACT2_ELITE_POOL, ACT2_BOSS_POOL,
    ACT3_WEAK_POOL, ACT3_STRONG_POOL, ACT3_ELITE_POOL, ACT3_BOSS_POOL,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
