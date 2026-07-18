use super::super::PendingChoiceActionWork;
use super::node::{RootLineage, RootLineageId, SearchNode};
use super::priority::{priority_for_node, QueueEntry};
use crate::ai::combat_state_key::combat_exact_state_key;
use std::collections::{BTreeMap, BinaryHeap, HashSet};

pub(in crate::ai::combat_search_v2) struct FrontierQueue {
    unattributed: BinaryHeap<QueueEntry>,
    by_root_action: BTreeMap<RootLineageId, BinaryHeap<QueueEntry>>,
    next_sequence_id: u64,
}

impl FrontierQueue {
    pub(in crate::ai::combat_search_v2) fn new() -> Self {
        Self {
            unattributed: BinaryHeap::new(),
            by_root_action: BTreeMap::new(),
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
        match entry.node.root_lineage {
            RootLineage::Unmaterialized => self.unattributed.push(entry),
            RootLineage::Action(id) => self.by_root_action.entry(id).or_default().push(entry),
        }
    }

    pub(in crate::ai::combat_search_v2) fn pop(&mut self) -> Option<QueueEntry> {
        let mut selected_root = None;
        let mut best = self.unattributed.peek();
        for (id, heap) in &self.by_root_action {
            let Some(candidate) = heap.peek() else {
                continue;
            };
            if best.is_none_or(|current| candidate > current) {
                selected_root = Some(*id);
                best = Some(candidate);
            }
        }
        match selected_root {
            Some(id) => self.pop_root_action(id),
            None => self.unattributed.pop(),
        }
    }

    pub(in crate::ai::combat_search_v2) fn pop_unattributed(&mut self) -> Option<QueueEntry> {
        self.unattributed.pop()
    }

    pub(in crate::ai::combat_search_v2) fn pop_root_action(
        &mut self,
        id: RootLineageId,
    ) -> Option<QueueEntry> {
        let (entry, empty) = {
            let heap = self.by_root_action.get_mut(&id)?;
            let entry = heap.pop();
            (entry, heap.is_empty())
        };
        if empty {
            self.by_root_action.remove(&id);
        }
        entry
    }

    pub(in crate::ai::combat_search_v2) fn has_root_action_work(&self, id: RootLineageId) -> bool {
        self.by_root_action
            .get(&id)
            .is_some_and(|heap| !heap.is_empty())
    }

    pub(in crate::ai::combat_search_v2) fn len(&self) -> usize {
        self.unattributed.len()
            + self
                .by_root_action
                .values()
                .map(BinaryHeap::len)
                .sum::<usize>()
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
        self.unattributed
            .iter()
            .chain(self.by_root_action.values().flat_map(|heap| heap.iter()))
    }

    pub(in crate::ai::combat_search_v2) fn replace_exact_state_rollout_estimate(
        &mut self,
        target: &SearchNode,
        estimate: &super::super::RolloutNodeEstimate,
    ) {
        let target_key = combat_exact_state_key(&target.engine, &target.combat);
        let mut entries = std::mem::take(&mut self.unattributed).into_vec();
        for (_, heap) in std::mem::take(&mut self.by_root_action) {
            entries.extend(heap.into_vec());
        }
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
