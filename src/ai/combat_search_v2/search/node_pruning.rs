use super::super::frontier::is_resource_covered;
use super::super::*;
use super::loop_state::SearchLoopState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NodePruneOutcome {
    Continue,
    Pruned,
}

pub(super) fn apply_node_prune_gates(
    loop_state: &mut SearchLoopState,
    node: &SearchNode,
    config: &CombatSearchV2Config,
) -> NodePruneOutcome {
    if node.actions.len() >= config.max_actions_per_line {
        loop_state.max_actions_cut_count = loop_state.max_actions_cut_count.saturating_add(1);
        return NodePruneOutcome::Pruned;
    }

    let resource = node.resource_vector();
    let exact_key = combat_exact_state_key(&node.engine, &node.combat);
    if is_resource_covered(&mut loop_state.exact_transpositions, exact_key, resource) {
        loop_state.stats.transposition_prunes =
            loop_state.stats.transposition_prunes.saturating_add(1);
        return NodePruneOutcome::Pruned;
    }

    let dominance_key = combat_dominance_key(&node.engine, &node.combat);
    if is_resource_covered(&mut loop_state.dominance, dominance_key, resource) {
        loop_state.stats.dominance_prunes = loop_state.stats.dominance_prunes.saturating_add(1);
        return NodePruneOutcome::Pruned;
    }

    NodePruneOutcome::Continue
}
