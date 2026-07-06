use super::super::frontier::FrontierQueue;
use super::super::*;

pub(super) fn frontier_sample_states(frontier: &FrontierQueue) -> Vec<CombatSearchV2StateSummary> {
    frontier
        .iter()
        .take(FRONTIER_SAMPLE_LIMIT)
        .map(|entry| summarize_state(&entry.node.engine, &entry.node.combat))
        .collect()
}
