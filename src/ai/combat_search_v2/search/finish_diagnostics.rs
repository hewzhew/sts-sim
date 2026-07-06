use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) fn finish_diagnostics_and_timing(
    loop_state: &mut SearchLoopState,
    started: Instant,
    root_for_turn_plan_diagnostics: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
) {
    let shadow_audit_started = Instant::now();
    loop_state
        .diagnostics
        .run_discard_order_exact_shadow_audit(stepper, config);
    loop_state.performance.shadow_audit_elapsed_us = shadow_audit_started.elapsed().as_micros();

    let root_turn_plan_diagnostics_started = Instant::now();
    loop_state
        .diagnostics
        .observe_root_turn_plan(root_for_turn_plan_diagnostics, stepper);
    loop_state.performance.root_turn_plan_diagnostics_elapsed_us =
        root_turn_plan_diagnostics_started.elapsed().as_micros();

    let total_elapsed = started.elapsed();
    loop_state.stats.elapsed_ms = total_elapsed.as_millis();
    loop_state.performance.total_elapsed_us = total_elapsed.as_micros();
    loop_state.performance.unattributed_elapsed_us =
        loop_state.performance.total_elapsed_us.saturating_sub(
            loop_state
                .performance
                .engine_step_elapsed_us
                .saturating_add(loop_state.performance.rollout_estimate_elapsed_us)
                .saturating_add(loop_state.performance.frontier_pop_elapsed_us)
                .saturating_add(loop_state.performance.pre_expand_elapsed_us)
                .saturating_add(loop_state.performance.expansion_elapsed_us)
                .saturating_add(loop_state.performance.child_bookkeeping_elapsed_us)
                .saturating_add(loop_state.performance.turn_plan_frontier_seed_elapsed_us)
                .saturating_add(loop_state.performance.shadow_audit_elapsed_us)
                .saturating_add(loop_state.performance.root_turn_plan_diagnostics_elapsed_us),
        );
}
