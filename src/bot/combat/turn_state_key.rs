mod memo;
mod monster;
mod pending_choice;
mod postcombat;
mod stable;
#[cfg(test)]
mod tests;
mod types;

use crate::runtime::combat::CombatState;
use crate::state::EngineState;

use memo::render_state_key;
use stable::build_stable_outcome_key;
pub(super) use types::{StableOutcomeKey, TurnStateKey};

pub(super) fn turn_state_key(engine: &EngineState, combat: &CombatState) -> TurnStateKey {
    TurnStateKey(render_state_key(engine, combat, true, true, true, true))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn stable_outcome_key(engine: &EngineState, combat: &CombatState) -> StableOutcomeKey {
    debug_assert_ne!(
        stable_frontier_scope(engine),
        StableFrontierScope::Unstable,
        "stable_outcome_key should only be requested for stable frontiers"
    );
    diagnostic_outcome_key(engine, combat)
}

pub(super) fn stable_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> Option<StableOutcomeKey> {
    match stable_frontier_scope(engine) {
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

fn stable_frontier_scope(engine: &EngineState) -> StableFrontierScope {
    match engine {
        EngineState::CombatPlayerTurn => StableFrontierScope::CombatReady,
        EngineState::PendingChoice(_) => StableFrontierScope::PendingChoice,
        EngineState::CombatProcessing => StableFrontierScope::Unstable,
        EngineState::RewardScreen(_)
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
