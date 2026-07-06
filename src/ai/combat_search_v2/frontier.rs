use super::*;

mod node;
mod priority;
mod queue;
pub(in crate::ai::combat_search_v2) mod resources;

pub(super) use node::SearchNode;
pub(super) use priority::QueueEntry;
pub(super) use queue::FrontierQueue;
pub(super) use resources::{is_resource_covered, ResourceVector};

const MAX_REMEMBERED_WIN_CANDIDATES: usize = 128;

pub(super) fn remember_best_complete(best: &mut Option<SearchNode>, candidate: SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(&candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate);
    }
}

pub(super) fn remember_win_candidate(candidates: &mut Vec<SearchNode>, candidate: &SearchNode) {
    if candidates.len() < MAX_REMEMBERED_WIN_CANDIDATES {
        candidates.push(candidate.clone());
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
