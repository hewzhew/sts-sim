use super::super::PendingChoiceActionWork;
use super::node::SearchNode;
use super::priority::{priority_for_node, QueueEntry};
use crate::ai::combat_state_key::combat_exact_state_key;
use std::collections::{BinaryHeap, HashSet};

pub(in crate::ai::combat_search_v2) struct FrontierQueue {
    heap: BinaryHeap<QueueEntry>,
    next_sequence_id: u64,
}

impl FrontierQueue {
    pub(in crate::ai::combat_search_v2) fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_sequence_id: 0,
        }
    }

    pub(in crate::ai::combat_search_v2) fn push_node(&mut self, node: SearchNode) {
        self.push_work_item(node, None);
    }

    pub(in crate::ai::combat_search_v2) fn push_pending_choice_work(
        &mut self,
        node: SearchNode,
        work: PendingChoiceActionWork,
    ) {
        self.push_work_item(node, Some(work));
    }

    fn push_work_item(
        &mut self,
        node: SearchNode,
        pending_choice_work: Option<PendingChoiceActionWork>,
    ) {
        let entry = QueueEntry {
            priority: priority_for_node(&node),
            sequence_id: self.next_sequence_id,
            node,
            pending_choice_work,
        };
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
        self.push_entry(entry);
    }

    fn push_entry(&mut self, entry: QueueEntry) {
        self.heap.push(entry);
    }

    pub(in crate::ai::combat_search_v2) fn pop(&mut self) -> Option<QueueEntry> {
        self.heap.pop()
    }

    pub(in crate::ai::combat_search_v2) fn len(&self) -> usize {
        self.heap.len()
    }

    /// Concrete engine states and virtual action-prefix work are different
    /// units.  Multiple residual entries may carry clones of the same parent;
    /// reports count that parent once.
    pub(in crate::ai::combat_search_v2) fn concrete_state_count(&self) -> usize {
        self.iter()
            .map(|entry| combat_exact_state_key(&entry.node.engine, &entry.node.combat))
            .collect::<HashSet<_>>()
            .len()
    }

    pub(in crate::ai::combat_search_v2) fn pending_choice_work_item_count(&self) -> usize {
        self.iter()
            .filter(|entry| entry.pending_choice_work.is_some())
            .count()
    }

    pub(in crate::ai::combat_search_v2) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(in crate::ai::combat_search_v2) fn iter(&self) -> impl Iterator<Item = &QueueEntry> {
        self.heap.iter()
    }

    pub(in crate::ai::combat_search_v2) fn replace_exact_state_rollout_estimate(
        &mut self,
        target: &SearchNode,
        estimate: &super::super::RolloutNodeEstimate,
    ) {
        let target_key = combat_exact_state_key(&target.engine, &target.combat);
        let mut entries = self.heap.drain().collect::<Vec<_>>();
        for entry in &mut entries {
            if combat_exact_state_key(&entry.node.engine, &entry.node.combat) == target_key {
                entry.node.rollout_estimate = estimate.clone();
                entry.priority = priority_for_node(&entry.node);
            }
        }
        for entry in entries {
            self.push_entry(entry);
        }
    }
}
