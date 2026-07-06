use std::time::Instant;

use super::super::frontier::is_resource_covered;
use super::super::rollout_scheduler::deferred_child_rollout_admission;
use super::super::*;
use super::loop_state::SearchLoopState;
use super::node_budget::{apply_node_budget_gate, NodeBudgetGateOutcome};
use super::rollout_timing::{
    observe_deferred_rollout_admission, timed_rollout_estimate, RolloutEstimateSource,
};
use super::turn_plan_seed_gate::should_seed_turn_plan_at_node;
use super::turn_plan_seeding::seed_turn_plan_frontier;

pub(super) enum NodePreflightOutcome {
    Expand(SearchNode),
    Continue,
    Stop,
}

pub(super) struct NodePreflightInput<'a, S: CombatStepper> {
    pub(super) node: SearchNode,
    pub(super) started: Instant,
    pub(super) stepper: &'a S,
    pub(super) config: &'a CombatSearchV2Config,
    pub(super) deadline: Option<Instant>,
}

pub(super) fn prepare_node_for_expansion<S: CombatStepper>(
    loop_state: &mut SearchLoopState,
    input: NodePreflightInput<'_, S>,
) -> NodePreflightOutcome {
    let mut node =
        match apply_node_budget_gate(loop_state, input.node, input.config, input.deadline) {
            NodeBudgetGateOutcome::Continue(node) => node,
            NodeBudgetGateOutcome::Stop => return NodePreflightOutcome::Stop,
        };
    let admission = deferred_child_rollout_admission(
        &node,
        input.config,
        &loop_state.stats,
        &loop_state.performance,
        input.started,
    );
    observe_deferred_rollout_admission(admission, &mut loop_state.performance);
    if admission.admitted() {
        node.rollout_estimate = timed_rollout_estimate(
            &mut loop_state.rollout_cache,
            &node,
            input.stepper,
            input.config,
            input.deadline,
            &mut loop_state.performance,
            RolloutEstimateSource::DeferredChild,
        );
        if node.rollout_estimate.is_evaluated() {
            loop_state.performance.deferred_child_rollout_requeues = loop_state
                .performance
                .deferred_child_rollout_requeues
                .saturating_add(1);
            loop_state.push_frontier(node);
            return NodePreflightOutcome::Continue;
        }
    }

    let pre_expand_started = Instant::now();
    loop_state.remember_best_frontier(&node);
    match terminal_label(&node.engine, &node.combat) {
        SearchTerminalLabel::Win => {
            if loop_state.remember_win(node, input.config) {
                record_pre_expand_elapsed(loop_state, pre_expand_started);
                loop_state.accepted_complete_candidate = true;
                return NodePreflightOutcome::Stop;
            }
            record_pre_expand_elapsed(loop_state, pre_expand_started);
            return NodePreflightOutcome::Continue;
        }
        SearchTerminalLabel::Loss => {
            loop_state.remember_loss(node);
            record_pre_expand_elapsed(loop_state, pre_expand_started);
            return NodePreflightOutcome::Continue;
        }
        SearchTerminalLabel::Unresolved => {}
    }

    if node.actions.len() >= input.config.max_actions_per_line {
        loop_state.max_actions_cut_count = loop_state.max_actions_cut_count.saturating_add(1);
        record_pre_expand_elapsed(loop_state, pre_expand_started);
        return NodePreflightOutcome::Continue;
    }

    let resource = node.resource_vector();
    let exact_key = combat_exact_state_key(&node.engine, &node.combat);
    if is_resource_covered(&mut loop_state.exact_transpositions, exact_key, resource) {
        loop_state.stats.transposition_prunes =
            loop_state.stats.transposition_prunes.saturating_add(1);
        record_pre_expand_elapsed(loop_state, pre_expand_started);
        return NodePreflightOutcome::Continue;
    }

    let dominance_key = combat_dominance_key(&node.engine, &node.combat);
    if is_resource_covered(&mut loop_state.dominance, dominance_key, resource) {
        loop_state.stats.dominance_prunes = loop_state.stats.dominance_prunes.saturating_add(1);
        record_pre_expand_elapsed(loop_state, pre_expand_started);
        return NodePreflightOutcome::Continue;
    }

    if should_seed_turn_plan_at_node(&node, input.config) {
        seed_turn_plan_frontier(
            loop_state,
            &node,
            input.stepper,
            input.config,
            input.deadline,
        );
    }
    record_pre_expand_elapsed(loop_state, pre_expand_started);
    NodePreflightOutcome::Expand(node)
}

fn record_pre_expand_elapsed(loop_state: &mut SearchLoopState, started: Instant) {
    loop_state.performance.pre_expand_elapsed_us = loop_state
        .performance
        .pre_expand_elapsed_us
        .saturating_add(started.elapsed().as_micros());
}
