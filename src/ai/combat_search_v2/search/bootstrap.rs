use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;
use super::rollout_timing::{timed_rollout_estimate, RolloutEstimateSource};
use super::turn_plan_seeding::seed_turn_plan_frontier;

pub(super) fn initialize_root_frontier(
    loop_state: &mut SearchLoopState,
    engine: &EngineState,
    combat: &CombatState,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> SearchNode {
    let mut root = SearchNode::root(engine.clone(), combat.clone());
    root.rollout_estimate = timed_rollout_estimate(
        &mut loop_state.rollout_cache,
        &root,
        stepper,
        config,
        deadline,
        &mut loop_state.performance,
        RolloutEstimateSource::Root,
    );
    if terminal_label(&root.engine, &root.combat) == SearchTerminalLabel::Win {
        loop_state.stats.nodes_to_first_win = Some(0);
    }
    let root_for_turn_plan_diagnostics = root.clone();
    loop_state.push_frontier(root);
    if config.turn_plan_policy.seeds_root_frontier() {
        seed_turn_plan_frontier(
            loop_state,
            &root_for_turn_plan_diagnostics,
            stepper,
            config,
            deadline,
        );
    }
    root_for_turn_plan_diagnostics
}
