use super::super::frontier::FrontierQueue;
use super::super::*;

pub(super) fn frontier_sample_states(frontier: &FrontierQueue) -> Vec<CombatSearchV2StateSummary> {
    let mut seen = std::collections::HashSet::new();
    frontier
        .iter()
        .filter(|entry| {
            seen.insert(combat_exact_state_key(
                &entry.node.engine,
                &entry.node.combat,
            ))
        })
        .take(FRONTIER_SAMPLE_LIMIT)
        .map(|entry| summarize_state(&entry.node.engine, &entry.node.combat))
        .collect()
}
