use super::super::plugins::CombatSearchFrontierPluginId;
use super::super::transition::terminal_label;
use super::super::value::{
    combat_eval_from_rollout_estimate, CombatEvalOutcomeClass, CombatEvalProgressBucket,
    CombatEvalSurvivalBucket,
};
use super::super::PendingChoiceActionWork;
use super::super::SearchTerminalLabel;
use super::node::SearchNode;
use super::priority::{priority_for_node, QueueEntry};
use crate::ai::combat_state_key::combat_exact_state_key;
use std::collections::{BinaryHeap, HashSet};

pub(in crate::ai::combat_search_v2) struct FrontierQueue {
    policy: CombatSearchFrontierPluginId,
    single: BinaryHeap<QueueEntry>,
    lanes: FrontierLanes,
    next_sequence_id: u64,
}

impl FrontierQueue {
    pub(in crate::ai::combat_search_v2) fn new(
        policy: impl Into<CombatSearchFrontierPluginId>,
    ) -> Self {
        Self {
            policy: policy.into(),
            single: BinaryHeap::new(),
            lanes: FrontierLanes::new(),
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
        match self.policy {
            CombatSearchFrontierPluginId::SingleQueue => self.single.push(entry),
            CombatSearchFrontierPluginId::RoundRobinEvalBuckets => self.lanes.push(entry),
        }
    }

    pub(in crate::ai::combat_search_v2) fn pop(&mut self) -> Option<QueueEntry> {
        match self.policy {
            CombatSearchFrontierPluginId::SingleQueue => self.single.pop(),
            CombatSearchFrontierPluginId::RoundRobinEvalBuckets => self.lanes.pop(),
        }
    }

    pub(in crate::ai::combat_search_v2) fn len(&self) -> usize {
        match self.policy {
            CombatSearchFrontierPluginId::SingleQueue => self.single.len(),
            CombatSearchFrontierPluginId::RoundRobinEvalBuckets => self.lanes.len(),
        }
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
        self.single.iter().chain(self.lanes.iter())
    }
}

struct FrontierLanes {
    exact_win: BinaryHeap<QueueEntry>,
    estimated_win: BinaryHeap<QueueEntry>,
    survival: BinaryHeap<QueueEntry>,
    progress: BinaryHeap<QueueEntry>,
    balanced: BinaryHeap<QueueEntry>,
    cursor: usize,
    pop_count: usize,
}

impl FrontierLanes {
    const BEST_FIRST_POP_CYCLE: usize = 8;
    const ROUND_ROBIN_OFFSET: usize = Self::BEST_FIRST_POP_CYCLE - 1;

    fn new() -> Self {
        Self {
            exact_win: BinaryHeap::new(),
            estimated_win: BinaryHeap::new(),
            survival: BinaryHeap::new(),
            progress: BinaryHeap::new(),
            balanced: BinaryHeap::new(),
            cursor: 0,
            pop_count: 0,
        }
    }

    fn push(&mut self, entry: QueueEntry) {
        match classify_lane(&entry) {
            FrontierLane::ExactWin => self.exact_win.push(entry),
            FrontierLane::EstimatedWin => self.estimated_win.push(entry),
            FrontierLane::Survival => self.survival.push(entry),
            FrontierLane::Progress => self.progress.push(entry),
            FrontierLane::Balanced => self.balanced.push(entry),
        }
    }

    fn pop(&mut self) -> Option<QueueEntry> {
        if let Some(entry) = self.exact_win.pop() {
            return Some(entry);
        }

        if self.pop_count % Self::BEST_FIRST_POP_CYCLE != Self::ROUND_ROBIN_OFFSET {
            if let Some(entry) = self.pop_best_non_exact_lane() {
                self.pop_count = self.pop_count.saturating_add(1);
                return Some(entry);
            }
        }

        let entry = self.pop_round_robin_lane();
        if entry.is_some() {
            self.pop_count = self.pop_count.saturating_add(1);
        }
        entry
    }

    fn pop_round_robin_lane(&mut self) -> Option<QueueEntry> {
        const ROUND_ROBIN_LANES: [FrontierLane; 4] = [
            FrontierLane::Survival,
            FrontierLane::Progress,
            FrontierLane::EstimatedWin,
            FrontierLane::Balanced,
        ];
        for offset in 0..ROUND_ROBIN_LANES.len() {
            let index = (self.cursor + offset) % ROUND_ROBIN_LANES.len();
            if let Some(entry) = self.pop_lane(ROUND_ROBIN_LANES[index]) {
                self.cursor = (index + 1) % ROUND_ROBIN_LANES.len();
                return Some(entry);
            }
        }
        None
    }

    fn pop_best_non_exact_lane(&mut self) -> Option<QueueEntry> {
        const NON_EXACT_LANES: [FrontierLane; 4] = [
            FrontierLane::EstimatedWin,
            FrontierLane::Survival,
            FrontierLane::Progress,
            FrontierLane::Balanced,
        ];
        let best_lane = NON_EXACT_LANES
            .into_iter()
            .filter_map(|lane| self.peek_lane(lane).map(|entry| (lane, entry)))
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(lane, _)| lane)?;
        self.pop_lane(best_lane)
    }

    fn pop_lane(&mut self, lane: FrontierLane) -> Option<QueueEntry> {
        match lane {
            FrontierLane::ExactWin => self.exact_win.pop(),
            FrontierLane::EstimatedWin => self.estimated_win.pop(),
            FrontierLane::Survival => self.survival.pop(),
            FrontierLane::Progress => self.progress.pop(),
            FrontierLane::Balanced => self.balanced.pop(),
        }
    }

    fn peek_lane(&self, lane: FrontierLane) -> Option<&QueueEntry> {
        match lane {
            FrontierLane::ExactWin => self.exact_win.peek(),
            FrontierLane::EstimatedWin => self.estimated_win.peek(),
            FrontierLane::Survival => self.survival.peek(),
            FrontierLane::Progress => self.progress.peek(),
            FrontierLane::Balanced => self.balanced.peek(),
        }
    }

    fn len(&self) -> usize {
        self.exact_win
            .len()
            .saturating_add(self.estimated_win.len())
            .saturating_add(self.survival.len())
            .saturating_add(self.progress.len())
            .saturating_add(self.balanced.len())
    }

    fn iter(&self) -> impl Iterator<Item = &QueueEntry> {
        self.exact_win
            .iter()
            .chain(self.estimated_win.iter())
            .chain(self.survival.iter())
            .chain(self.progress.iter())
            .chain(self.balanced.iter())
    }
}

#[derive(Clone, Copy)]
enum FrontierLane {
    ExactWin,
    EstimatedWin,
    Survival,
    Progress,
    Balanced,
}

fn classify_lane(entry: &QueueEntry) -> FrontierLane {
    if terminal_label(&entry.node.engine, &entry.node.combat) == SearchTerminalLabel::Win {
        return FrontierLane::ExactWin;
    }

    if !entry.node.rollout_estimate.is_evaluated() {
        return FrontierLane::Balanced;
    }

    let eval = combat_eval_from_rollout_estimate(&entry.node.rollout_estimate);
    if eval.outcome_class() == CombatEvalOutcomeClass::Win {
        return FrontierLane::EstimatedWin;
    }
    if eval.survival_bucket() != CombatEvalSurvivalBucket::DeadOrForcedLoss
        && matches!(
            eval.progress_bucket(),
            CombatEvalProgressBucket::RaceFavored
                | CombatEvalProgressBucket::LethalNextTurnLikely
                | CombatEvalProgressBucket::LethalNow
        )
    {
        return FrontierLane::Progress;
    }
    if matches!(
        eval.survival_bucket(),
        CombatEvalSurvivalBucket::DeadOrForcedLoss
            | CombatEvalSurvivalBucket::LethalVisible
            | CombatEvalSurvivalBucket::Critical
    ) {
        return FrontierLane::Survival;
    }
    FrontierLane::Balanced
}
