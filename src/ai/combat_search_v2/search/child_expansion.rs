use std::time::Instant;

use super::super::*;
use super::child_node::{build_child_node, BuiltChildNode};
use super::child_preflight::{prepare_child_for_expansion, ChildPreflightOutcome};
use super::child_rollout::child_rollout_estimate;
use super::loop_state::SearchLoopState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChildExpansionOutcome {
    Advanced,
    DeadlineReached,
}

pub(super) struct ChildExpansionInput<'a, S: CombatStepper> {
    pub(super) parent: &'a SearchNode,
    pub(super) position: &'a CombatPosition,
    pub(super) ordered_choice: IndexedActionChoice,
    pub(super) action_prior_state_hash: Option<&'a str>,
    pub(super) pending_choice: Option<&'a PendingChoiceProfile>,
    pub(super) stepper: &'a S,
    pub(super) config: &'a CombatSearchV2Config,
    pub(super) deadline: Option<Instant>,
}

pub(super) fn expand_ordered_child<S: CombatStepper>(
    loop_state: &mut SearchLoopState,
    turn_branching: &mut TurnBranchingStateObservation,
    turn_local_dominance: &mut TurnLocalDominanceStateObservation,
    input: ChildExpansionInput<'_, S>,
) -> ChildExpansionOutcome {
    let potion_tactical_priority = match prepare_child_for_expansion(
        loop_state,
        input.parent,
        &input.ordered_choice,
        input.config,
        input.deadline,
    ) {
        ChildPreflightOutcome::Continue {
            potion_tactical_priority,
        } => potion_tactical_priority,
        ChildPreflightOutcome::Advanced => return ChildExpansionOutcome::Advanced,
        ChildPreflightOutcome::DeadlineReached => return ChildExpansionOutcome::DeadlineReached,
    };

    let step_started = Instant::now();
    let step = input.stepper.apply_to_stable(
        input.position,
        input.ordered_choice.choice.input.clone(),
        CombatStepLimits {
            max_engine_steps: input.config.max_engine_steps_per_action,
            deadline: input.deadline,
        },
    );
    loop_state.performance.engine_step_calls =
        loop_state.performance.engine_step_calls.saturating_add(1);
    loop_state.performance.engine_step_elapsed_us = loop_state
        .performance
        .engine_step_elapsed_us
        .saturating_add(step_started.elapsed().as_micros());
    if step.truncated && !step.timed_out {
        loop_state.engine_step_limit_count = loop_state.engine_step_limit_count.saturating_add(1);
    }
    if step.timed_out {
        loop_state.stats.deadline_hit = true;
        loop_state.exhausted = true;
    }

    let child_bookkeeping_started = Instant::now();
    let BuiltChildNode {
        node: mut child,
        turn_transition,
        truncated,
    } = build_child_node(
        input.parent,
        step,
        input.ordered_choice,
        input.action_prior_state_hash,
        potion_tactical_priority,
        input.config,
    );
    loop_state
        .diagnostics
        .observe_pending_choice_child_transition(input.pending_choice, truncated, &child.engine);
    turn_branching.observe_child(turn_transition);
    loop_state.stats.nodes_generated = loop_state.stats.nodes_generated.saturating_add(1);
    loop_state.performance.child_bookkeeping_elapsed_us = loop_state
        .performance
        .child_bookkeeping_elapsed_us
        .saturating_add(child_bookkeeping_started.elapsed().as_micros());

    let child_bookkeeping_started = Instant::now();
    if !truncated && turn_local_dominance.observe_child(&child) {
        loop_state.stats.turn_local_dominance_prunes = loop_state
            .stats
            .turn_local_dominance_prunes
            .saturating_add(1);
        loop_state.performance.turn_local_dominance_rollout_skips = loop_state
            .performance
            .turn_local_dominance_rollout_skips
            .saturating_add(1);
        loop_state.performance.child_bookkeeping_elapsed_us = loop_state
            .performance
            .child_bookkeeping_elapsed_us
            .saturating_add(child_bookkeeping_started.elapsed().as_micros());
        return ChildExpansionOutcome::Advanced;
    }

    child.rollout_estimate = child_rollout_estimate(
        loop_state,
        &child,
        input.stepper,
        input.config,
        input.deadline,
    );

    let child_bookkeeping_started = Instant::now();
    if loop_state.stats.nodes_to_first_win.is_none()
        && terminal_label(&child.engine, &child.combat) == SearchTerminalLabel::Win
    {
        loop_state.stats.nodes_to_first_win = Some(loop_state.stats.nodes_generated);
    }

    if !truncated {
        loop_state.push_frontier(child);
    } else {
        loop_state.unresolved_leaf_count = loop_state.unresolved_leaf_count.saturating_add(1);
        loop_state.remember_best_frontier(&child);
    }
    loop_state.performance.child_bookkeeping_elapsed_us = loop_state
        .performance
        .child_bookkeeping_elapsed_us
        .saturating_add(child_bookkeeping_started.elapsed().as_micros());
    ChildExpansionOutcome::Advanced
}
