use super::*;

mod node;
mod priority;
pub(in crate::ai::combat_search_v2) mod resources;

pub(super) use node::SearchNode;
use priority::priority_for_node;
pub(super) use priority::QueueEntry;
pub(super) use resources::{is_resource_covered, ResourceVector};

pub(super) fn push_frontier(
    frontier: &mut BinaryHeap<QueueEntry>,
    node: SearchNode,
    sequence_id: &mut u64,
) {
    let priority = priority_for_node(&node);
    frontier.push(QueueEntry {
        priority,
        sequence_id: *sequence_id,
        node,
    });
    *sequence_id = sequence_id.saturating_add(1);
}

pub(super) fn remember_best_complete(best: &mut Option<SearchNode>, candidate: SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(&candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate);
    }
}

pub(super) fn remember_best_frontier(best: &mut Option<SearchNode>, candidate: &SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate.clone());
    }
}

fn compare_nodes(left: &SearchNode, right: &SearchNode) -> Ordering {
    CombatOutcomeScore::from_node(left).cmp(&CombatOutcomeScore::from_node(right))
}

#[cfg(test)]
mod tests;
