mod cards;
mod engine;
mod meta;
mod monster;
mod pending_choice;
mod player;
mod potions;
mod powers;
mod queue;
mod rng;
mod runtime_hints;
mod turn;
mod zones;

use crate::runtime::combat::CombatState;
use crate::state::core::EngineState;

use super::types::{
    CombatDominanceKey, CombatDominancePlayerKey, CombatExactStateKey, CombatRuntimeKey,
};

/// Exact in-combat runtime key used by Combat Search V2 transposition pruning.
/// This is stricter than `stable_outcome_key`: player hp/block, card
/// instances, queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_exact_runtime_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatExactStateKey {
    CombatExactStateKey {
        common: combat_runtime_key(engine, combat),
        player: player::player_exact_key(combat),
    }
}

/// In-combat bucket used by Combat Search V2 resource dominance pruning. This
/// is not an exact transposition key: current HP/block are intentionally left
/// out because they are compared through `ResourceVector`, but card instances,
/// queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatDominanceKey {
    CombatDominanceKey {
        common: combat_runtime_key(engine, combat),
        player: CombatDominancePlayerKey {
            future_relevant: player::player_future_key(combat),
        },
    }
}

fn combat_runtime_key(engine: &EngineState, combat: &CombatState) -> CombatRuntimeKey {
    CombatRuntimeKey {
        engine: engine::engine_key(engine),
        turn: turn::turn_key(combat),
        meta: meta::meta_key(combat),
        zones: zones::zones_key(combat),
        monsters: monster::monsters_key(combat),
        powers: powers::powers_key(combat),
        potions: potions::potions_key(combat),
        queue: queue::queue_key(combat),
        runtime: runtime_hints::runtime_key(combat),
        rng: rng::rng_pool_key(&combat.rng.pool),
    }
}
