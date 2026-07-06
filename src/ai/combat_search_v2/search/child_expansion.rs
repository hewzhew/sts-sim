use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;
use super::rollout_timing::{timed_rollout_estimate, RolloutEstimateSource};

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
    let action_id = input.ordered_choice.original_action_id;
    let choice = input.ordered_choice.choice;
    let potion_tactical_priority =
        potions::semantic_potion_tactical_priority(&input.parent.combat, &choice.input);
    if input
        .config
        .max_potions_used
        .is_some_and(|max| input.parent.potions_used >= max && is_use_potion_input(&choice.input))
    {
        loop_state.potion_budget_cut_count = loop_state.potion_budget_cut_count.saturating_add(1);
        return ChildExpansionOutcome::Advanced;
    }
    if input.deadline.is_some_and(|limit| Instant::now() >= limit) {
        loop_state.stats.deadline_hit = true;
        loop_state.exhausted = true;
        return ChildExpansionOutcome::DeadlineReached;
    }

    let step_started = Instant::now();
    let step = input.stepper.apply_to_stable(
        input.position,
        choice.input.clone(),
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
    let mut child = input
        .parent
        .clone_for_child(step.position.engine, step.position.combat);
    loop_state
        .diagnostics
        .observe_pending_choice_child_transition(
            input.pending_choice,
            step.truncated,
            &child.engine,
        );
    let turn_transition = classify_turn_branch_transition(
        &input.parent.engine,
        &input.parent.combat,
        &choice.input,
        &child.engine,
        &child.combat,
    );
    child.note_turn_prefix(&input.parent.combat, &choice.input, turn_transition);
    child.note_input(&choice.input);
    child.note_action_prior_score(input.action_prior_state_hash.and_then(|state_hash| {
        input
            .config
            .root_action_prior
            .as_ref()
            .and_then(|prior| prior.score(state_hash, &choice.action_key))
    }));
    child.note_potion_tactical_priority(potion_tactical_priority);
    child.note_turn_branch_priority(turn_transition.frontier_priority_hint());
    turn_branching.observe_child(turn_transition);
    child.actions.push(CombatSearchV2ActionTrace {
        step_index: input.parent.actions.len(),
        action_id,
        action_key: choice.action_key,
        action_debug: choice.action_debug,
        input: choice.input,
    });
    loop_state.stats.nodes_generated = loop_state.stats.nodes_generated.saturating_add(1);
    loop_state.performance.child_bookkeeping_elapsed_us = loop_state
        .performance
        .child_bookkeeping_elapsed_us
        .saturating_add(child_bookkeeping_started.elapsed().as_micros());

    let child_bookkeeping_started = Instant::now();
    if !step.truncated && turn_local_dominance.observe_child(&child) {
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

    if !step.truncated {
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

fn child_rollout_estimate(
    loop_state: &mut SearchLoopState,
    child: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    if terminal_label(&child.engine, &child.combat) != SearchTerminalLabel::Unresolved {
        loop_state.performance.terminal_child_rollout_skips = loop_state
            .performance
            .terminal_child_rollout_skips
            .saturating_add(1);
        return RolloutNodeEstimate::from_node(
            child,
            0,
            RolloutStopReason::TerminalState,
            Some("terminal_child_no_rollout"),
            super::super::rollout_pending_choice::RolloutPendingChoiceProgress::default(),
        );
    }
    if config.child_rollout_policy == CombatSearchV2ChildRolloutPolicy::LazyOnPop
        && config.rollout_policy != CombatSearchV2RolloutPolicy::Disabled
    {
        loop_state.performance.deferred_child_rollout_nodes = loop_state
            .performance
            .deferred_child_rollout_nodes
            .saturating_add(1);
        return RolloutNodeEstimate::unevaluated();
    }
    timed_rollout_estimate(
        &mut loop_state.rollout_cache,
        child,
        stepper,
        config,
        deadline,
        &mut loop_state.performance,
        RolloutEstimateSource::Child,
    )
}
