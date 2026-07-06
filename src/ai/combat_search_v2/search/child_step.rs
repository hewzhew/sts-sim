use std::time::{Duration, Instant};

use crate::sim::combat::CombatStepResult;

use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) fn apply_child_step<S: CombatStepper>(
    loop_state: &mut SearchLoopState,
    position: &CombatPosition,
    input: &ClientInput,
    stepper: &S,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> CombatStepResult {
    let step_started = Instant::now();
    let step = stepper.apply_to_stable(
        position,
        input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline,
        },
    );
    observe_child_step(loop_state, &step, step_started.elapsed());
    step
}

fn observe_child_step(
    loop_state: &mut SearchLoopState,
    step: &CombatStepResult,
    elapsed: Duration,
) {
    loop_state.performance.engine_step_calls =
        loop_state.performance.engine_step_calls.saturating_add(1);
    loop_state.performance.engine_step_elapsed_us = loop_state
        .performance
        .engine_step_elapsed_us
        .saturating_add(elapsed.as_micros());
    if step.truncated && !step.timed_out {
        loop_state.record_engine_step_limit();
    }
    if step.timed_out {
        loop_state.mark_deadline_hit();
    }
}
