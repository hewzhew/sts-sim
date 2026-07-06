use std::time::Instant;

use super::super::super::frontier::{QueueEntry, SearchNode};
use super::SearchLoopState;

impl SearchLoopState {
    pub(in crate::ai::combat_search_v2::search) fn push_frontier(&mut self, node: SearchNode) {
        self.frontier.push_node(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn pop_frontier(&mut self) -> Option<QueueEntry> {
        let started = Instant::now();
        let entry = self.frontier.pop();
        self.performance.frontier_pop_calls = self.performance.frontier_pop_calls.saturating_add(1);
        self.performance.frontier_pop_elapsed_us = self
            .performance
            .frontier_pop_elapsed_us
            .saturating_add(started.elapsed().as_micros());
        entry
    }
}
