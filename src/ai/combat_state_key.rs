//! State-key boundaries for combat search.
//!
//! Exact keys are for duplicate runtime states; dominance keys intentionally
//! remove player hp/block and compare them through a resource vector; stable
//! outcome keys are only for stable frontiers and may ignore runtime noise.
//! Do not use one key family in place of another.

mod combat;
mod monster;
mod pending_choice;
mod postcombat;
mod stable;
#[cfg(test)]
mod tests;
mod types;

use crate::engine::core::is_smoke_escape_stable_boundary;
use crate::runtime::combat::CombatState;
use crate::state::EngineState;

use combat::{combat_dominance_bucket_key, combat_exact_runtime_key};
use stable::build_stable_outcome_key;
pub(crate) use types::{CombatDominanceKey, CombatExactStateKey, StableOutcomeKey};

/// Exact in-combat key for Combat Search V2 transposition. It keeps player
/// hp/block and runtime details that affect future combat transitions.
pub(crate) fn combat_exact_state_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatExactStateKey {
    combat_exact_runtime_key(engine, combat)
}

/// In-combat bucket for Combat Search V2 resource dominance. It keeps runtime
/// details that affect future combat transitions, while leaving current
/// hp/block to the searched resource vector.
pub(crate) fn combat_dominance_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatDominanceKey {
    combat_dominance_bucket_key(engine, combat)
}

/// Stable frontier key for comparing outcomes after the engine reaches a
/// player decision boundary. This intentionally abstracts display/runtime noise
/// that should not affect future decisions from that boundary.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn stable_outcome_key(engine: &EngineState, combat: &CombatState) -> StableOutcomeKey {
    debug_assert_ne!(
        stable_frontier_scope(engine, combat),
        StableFrontierScope::Unstable,
        "stable_outcome_key should only be requested for stable frontiers"
    );
    diagnostic_outcome_key(engine, combat)
}

/// Stable dominance bucket only exists at stable frontiers. Unstable engine
/// processing states must not be merged under this abstraction.
pub(crate) fn stable_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> Option<StableOutcomeKey> {
    match stable_frontier_scope(engine, combat) {
        StableFrontierScope::Unstable => None,
        _ => Some(diagnostic_outcome_key(engine, combat)),
    }
}

fn diagnostic_outcome_key(engine: &EngineState, combat: &CombatState) -> StableOutcomeKey {
    build_stable_outcome_key(engine, combat)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StableFrontierScope {
    CombatReady,
    PendingChoice,
    Unstable,
    PostCombat,
    GameOver,
}

fn stable_frontier_scope(engine: &EngineState, combat: &CombatState) -> StableFrontierScope {
    match engine {
        EngineState::CombatPlayerTurn => StableFrontierScope::CombatReady,
        EngineState::PendingChoice(_) => StableFrontierScope::PendingChoice,
        EngineState::CombatProcessing if is_smoke_escape_stable_boundary(engine, combat) => {
            StableFrontierScope::PostCombat
        }
        EngineState::CombatProcessing => StableFrontierScope::Unstable,
        EngineState::RewardScreen(_)
        | EngineState::TreasureRoom(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::EventCombat(_)
        | EngineState::BossRelicSelect(_) => StableFrontierScope::PostCombat,
        EngineState::GameOver(_) => StableFrontierScope::GameOver,
    }
}
