use std::time::Instant;

use super::super::frontier::is_resource_covered;
use super::super::*;
use super::loop_state::SearchLoopState;
use super::node_budget::{apply_node_budget_gate, NodeBudgetGateOutcome};
use super::node_deferred_rollout::{apply_deferred_child_rollout, DeferredRolloutOutcome};
use super::node_terminal::{apply_node_terminal_gate, NodeTerminalOutcome};
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
    let node = match apply_node_budget_gate(loop_state, input.node, input.config, input.deadline) {
        NodeBudgetGateOutcome::Continue(node) => node,
        NodeBudgetGateOutcome::Stop => return NodePreflightOutcome::Stop,
    };

    let node = match apply_deferred_child_rollout(
        loop_state,
        node,
        input.started,
        input.stepper,
        input.config,
        input.deadline,
    ) {
        DeferredRolloutOutcome::Continue(node) => node,
        DeferredRolloutOutcome::Requeued => return NodePreflightOutcome::Continue,
    };

    let pre_expand_started = Instant::now();
    let node = match apply_node_terminal_gate(loop_state, node, input.config) {
        NodeTerminalOutcome::Continue(node) => node,
        NodeTerminalOutcome::Skip => {
            record_pre_expand_elapsed(loop_state, pre_expand_started);
            return NodePreflightOutcome::Continue;
        }
        NodeTerminalOutcome::StopAcceptedWin => {
            loop_state.accepted_complete_candidate = true;
            record_pre_expand_elapsed(loop_state, pre_expand_started);
            return NodePreflightOutcome::Stop;
        }
    };

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
