use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) struct OrderedNodeActions {
    pub(super) action_prior_state_hash: Option<String>,
    pub(super) ordered_choices: Vec<IndexedActionChoice>,
}

pub(super) fn order_node_actions(
    loop_state: &mut SearchLoopState,
    node: &SearchNode,
    legal: Vec<CombatActionChoice>,
    pending_choice: Option<&PendingChoiceProfile>,
    config: &CombatSearchV2Config,
) -> OrderedNodeActions {
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
        .observe_pending_choice_ordering(pending_choice, &ordered.summary);
    OrderedNodeActions {
        action_prior_state_hash,
        ordered_choices: ordered.choices,
    }
}
