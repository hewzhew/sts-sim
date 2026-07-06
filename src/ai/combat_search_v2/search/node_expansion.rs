use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) struct PreparedNodeExpansion {
    pub(super) position: CombatPosition,
    pub(super) pending_choice: Option<PendingChoiceProfile>,
    pub(super) action_prior_state_hash: Option<String>,
    pub(super) ordered_choices: Vec<IndexedActionChoice>,
    pub(super) turn_branching: TurnBranchingStateObservation,
    pub(super) turn_local_dominance: TurnLocalDominanceStateObservation,
}

pub(super) fn prepare_node_expansion(
    loop_state: &mut SearchLoopState,
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
) -> Option<PreparedNodeExpansion> {
    loop_state.stats.nodes_expanded = loop_state.stats.nodes_expanded.saturating_add(1);
    let expansion_started = Instant::now();
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    let legal = filtered_legal_actions(
        stepper.legal_action_choices(&position),
        config.potion_policy,
        &node.combat,
    );
    let pending_choice = summarize_pending_choice(&node.engine);
    loop_state
        .diagnostics
        .observe_pending_choice(pending_choice.as_ref());
    let expansion = summarize_action_expansion(&node.engine, &node.combat, &legal);
    loop_state.diagnostics.observe_legal_actions(&expansion);
    let turn_prefix = summarize_turn_prefix(&node.turn_prefix, legal.len());
    loop_state.diagnostics.observe_turn_prefix(&turn_prefix);
    let turn_sequence = summarize_turn_sequence(node, legal.len());
    loop_state
        .diagnostics
        .observe_turn_sequence(&turn_sequence, node);
    let card_identity = summarize_card_identity(&node.combat);
    loop_state.diagnostics.observe_card_identity(&card_identity);
    let target_fanout = summarize_target_fanout(&node.combat, &legal);
    loop_state.diagnostics.observe_target_fanout(&target_fanout);
    if legal.is_empty() {
        loop_state.unresolved_leaf_count = loop_state.unresolved_leaf_count.saturating_add(1);
        record_expansion_elapsed(loop_state, expansion_started);
        return None;
    }

    let equivalence = compress_equivalent_actions(&node.engine, &node.combat, legal);
    loop_state
        .diagnostics
        .observe_action_equivalence(&equivalence.summary);
    let action_prior_state_hash = config
        .root_action_prior
        .as_ref()
        .filter(|prior| !prior.is_empty())
        .map(|_| combat_exact_state_hash_v1(&node.engine, &node.combat));
    let ordered = order_indexed_action_choices_with_prior(
        &node.engine,
        &node.combat,
        equivalence.choices,
        config.root_action_prior.as_ref(),
        config.phase_guard_policy,
        config.setup_bias_policy,
    );
    loop_state
        .diagnostics
        .observe_action_ordering(&ordered.summary);
    loop_state
        .diagnostics
        .observe_pending_choice_ordering(pending_choice.as_ref(), &ordered.summary);
    let turn_branching = TurnBranchingStateObservation::new(&node.combat, ordered.choices.len());
    let turn_local_dominance =
        TurnLocalDominanceStateObservation::new(&node.engine, &node.combat, ordered.choices.len());
    record_expansion_elapsed(loop_state, expansion_started);

    Some(PreparedNodeExpansion {
        position,
        pending_choice,
        action_prior_state_hash,
        ordered_choices: ordered.choices,
        turn_branching,
        turn_local_dominance,
    })
}

fn record_expansion_elapsed(loop_state: &mut SearchLoopState, started: Instant) {
    loop_state.performance.expansion_elapsed_us = loop_state
        .performance
        .expansion_elapsed_us
        .saturating_add(started.elapsed().as_micros());
}
