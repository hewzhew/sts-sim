use super::priority::QueueEntry;
use std::collections::BinaryHeap;

#[derive(Default)]
pub(in crate::ai::combat_search_v2) struct FrontierQueue {
    heap: BinaryHeap<QueueEntry>,
}

impl FrontierQueue {
    pub(in crate::ai::combat_search_v2) fn new() -> Self {
        Self::default()
    }

    pub(in crate::ai::combat_search_v2) fn push(&mut self, entry: QueueEntry) {
        self.heap.push(entry);
    }

    pub(in crate::ai::combat_search_v2) fn pop(&mut self) -> Option<QueueEntry> {
        self.heap.pop()
    }

    pub(in crate::ai::combat_search_v2) fn len(&self) -> usize {
        self.heap.len()
    }

    pub(in crate::ai::combat_search_v2) fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub(in crate::ai::combat_search_v2) fn iter(&self) -> impl Iterator<Item = &QueueEntry> {
        self.heap.iter()
    }
}
