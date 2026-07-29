use super::*;
use serde::Serialize;

/// Read-only census of retained container ownership after one search slice.
///
/// Counts and capacities are observational only. Search scheduling, stopping,
/// and exact-state identity never read this structure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalTurnGraphStorageSnapshot {
    pub exact_nodes: usize,
    pub finished_generators: usize,
    pub exhausted_nodes: usize,
    pub live_generator_work_items: usize,
    pub generator_work_slots: usize,
    pub generator_work_capacity: usize,
    pub generator_seen_states: usize,
    pub generator_seen_capacity: usize,
    pub generator_anchor_entries: usize,
    pub generator_anchor_capacity: usize,
    pub generator_guide_frontiers: usize,
    pub generator_guide_entries: usize,
    pub generator_guide_capacity: usize,
    pub generator_scheduled_round_entries: usize,
    pub generator_scheduled_round_capacity: usize,
    pub generator_completed_options: usize,
    pub generator_completed_capacity: usize,
    pub generator_gaps: usize,
    pub generator_gaps_capacity: usize,
    pub finished_generator_work_capacity: usize,
    pub finished_generator_seen_capacity: usize,
    pub finished_generator_anchor_capacity: usize,
    pub finished_generator_guide_capacity: usize,
    pub finished_generator_scheduled_round_capacity: usize,
    pub finished_generator_completed_capacity: usize,
    pub finished_generator_gaps_capacity: usize,
    pub graph_edges: usize,
    pub graph_edge_capacity: usize,
}

impl LocalTurnGraphWitnessSession {
    pub fn storage_snapshot(&self) -> LocalTurnGraphStorageSnapshot {
        let mut snapshot = LocalTurnGraphStorageSnapshot {
            exact_nodes: self.nodes.len(),
            exhausted_nodes: self.nodes.iter().filter(|node| node.exhausted).count(),
            ..LocalTurnGraphStorageSnapshot::default()
        };
        for node in &self.nodes {
            let generator = node.generator.storage_snapshot();
            snapshot.finished_generators = snapshot
                .finished_generators
                .saturating_add(usize::from(generator.finished));
            snapshot.live_generator_work_items = snapshot
                .live_generator_work_items
                .saturating_add(generator.live_work_items);
            snapshot.generator_work_slots = snapshot
                .generator_work_slots
                .saturating_add(generator.work_slots);
            snapshot.generator_work_capacity = snapshot
                .generator_work_capacity
                .saturating_add(generator.work_capacity);
            snapshot.generator_seen_states = snapshot
                .generator_seen_states
                .saturating_add(generator.seen_states);
            snapshot.generator_seen_capacity = snapshot
                .generator_seen_capacity
                .saturating_add(generator.seen_capacity);
            snapshot.generator_anchor_entries = snapshot
                .generator_anchor_entries
                .saturating_add(generator.anchor_entries);
            snapshot.generator_anchor_capacity = snapshot
                .generator_anchor_capacity
                .saturating_add(generator.anchor_capacity);
            snapshot.generator_guide_frontiers = snapshot
                .generator_guide_frontiers
                .saturating_add(generator.guide_frontiers);
            snapshot.generator_guide_entries = snapshot
                .generator_guide_entries
                .saturating_add(generator.guide_entries);
            snapshot.generator_guide_capacity = snapshot
                .generator_guide_capacity
                .saturating_add(generator.guide_capacity);
            snapshot.generator_scheduled_round_entries = snapshot
                .generator_scheduled_round_entries
                .saturating_add(generator.scheduled_round_entries);
            snapshot.generator_scheduled_round_capacity = snapshot
                .generator_scheduled_round_capacity
                .saturating_add(generator.scheduled_round_capacity);
            snapshot.generator_completed_options = snapshot
                .generator_completed_options
                .saturating_add(generator.completed_options);
            snapshot.generator_completed_capacity = snapshot
                .generator_completed_capacity
                .saturating_add(generator.completed_capacity);
            snapshot.generator_gaps = snapshot.generator_gaps.saturating_add(generator.gaps);
            snapshot.generator_gaps_capacity = snapshot
                .generator_gaps_capacity
                .saturating_add(generator.gaps_capacity);
            if generator.finished {
                snapshot.finished_generator_work_capacity = snapshot
                    .finished_generator_work_capacity
                    .saturating_add(generator.work_capacity);
                snapshot.finished_generator_seen_capacity = snapshot
                    .finished_generator_seen_capacity
                    .saturating_add(generator.seen_capacity);
                snapshot.finished_generator_anchor_capacity = snapshot
                    .finished_generator_anchor_capacity
                    .saturating_add(generator.anchor_capacity);
                snapshot.finished_generator_guide_capacity = snapshot
                    .finished_generator_guide_capacity
                    .saturating_add(generator.guide_capacity);
                snapshot.finished_generator_scheduled_round_capacity = snapshot
                    .finished_generator_scheduled_round_capacity
                    .saturating_add(generator.scheduled_round_capacity);
                snapshot.finished_generator_completed_capacity = snapshot
                    .finished_generator_completed_capacity
                    .saturating_add(generator.completed_capacity);
                snapshot.finished_generator_gaps_capacity = snapshot
                    .finished_generator_gaps_capacity
                    .saturating_add(generator.gaps_capacity);
            }
            snapshot.graph_edges = snapshot.graph_edges.saturating_add(node.children.len());
            snapshot.graph_edge_capacity = snapshot
                .graph_edge_capacity
                .saturating_add(node.children.capacity());
        }
        snapshot
    }
}
