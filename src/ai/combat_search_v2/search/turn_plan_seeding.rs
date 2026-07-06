use std::collections::HashSet;
use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;
use super::rollout_timing::{timed_rollout_estimate, RolloutEstimateSource};

#[derive(Default)]
pub(super) struct TurnPlanSeedTracker {
    seeded_sources: HashSet<CombatExactStateKey>,
}

impl TurnPlanSeedTracker {
    fn claim_source(&mut self, source: &SearchNode) -> bool {
        self.seeded_sources
            .insert(combat_exact_state_key(&source.engine, &source.combat))
    }
}

pub(super) fn seed_turn_plan_frontier(
    loop_state: &mut SearchLoopState,
    source: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) {
    if !loop_state.turn_plan_seed_tracker.claim_source(source) {
        return;
    }

    let seed_started = Instant::now();
    let mut seeded_nodes = turn_plan_frontier_seed(source, stepper, config, deadline);
    loop_state.performance.turn_plan_frontier_seed_calls = loop_state
        .performance
        .turn_plan_frontier_seed_calls
        .saturating_add(1);
    loop_state.performance.turn_plan_frontier_seed_elapsed_us = loop_state
        .performance
        .turn_plan_frontier_seed_elapsed_us
        .saturating_add(seed_started.elapsed().as_micros());
    loop_state
        .diagnostics
        .observe_turn_plan_frontier_seeded_nodes(seeded_nodes.nodes.len());
    loop_state
        .diagnostics
        .observe_turn_plan_prior_scored_plans(seeded_nodes.turn_plan_prior_scored_plans);
    for mut seed in seeded_nodes.nodes.drain(..) {
        seed.rollout_estimate =
            turn_plan_seed_rollout_estimate(loop_state, &seed, stepper, config, deadline);
        loop_state.record_node_generated();
        loop_state.record_first_generated_win_if_needed(&seed);
        loop_state.push_frontier(seed);
    }
}

fn turn_plan_seed_rollout_estimate(
    loop_state: &mut SearchLoopState,
    seed: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    if terminal_label(&seed.engine, &seed.combat) == SearchTerminalLabel::Unresolved {
        timed_rollout_estimate(
            &mut loop_state.rollout_cache,
            seed,
            stepper,
            config,
            deadline,
            &mut loop_state.performance,
            RolloutEstimateSource::TurnPlanSeed,
        )
    } else {
        loop_state.performance.terminal_turn_plan_seed_rollout_skips = loop_state
            .performance
            .terminal_turn_plan_seed_rollout_skips
            .saturating_add(1);
        RolloutNodeEstimate::from_node(
            seed,
            0,
            RolloutStopReason::TerminalState,
            Some("terminal_turn_plan_seed_no_rollout"),
            super::super::rollout_pending_choice::RolloutPendingChoiceProgress::default(),
        )
    }
}
