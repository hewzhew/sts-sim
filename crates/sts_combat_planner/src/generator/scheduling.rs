use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Instant;

use crate::policy::{CombatGuideLaneId, CombatStateGuide, CombatStateGuideRank};

use super::{elapsed_nanos_u64, GeneratorWork, TurnOptionGeneratorSession};

#[derive(Clone, Copy, Debug)]
pub(crate) enum TurnOptionGeneratorPreferredLane {
    Anchor,
    Guide(CombatGuideLaneId),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct GeneratorWorkPriority {
    pub(super) levin_log_priority: f64,
    pub(super) atomic_depth: usize,
    pub(super) negative_log_policy: f64,
}

impl GeneratorWorkPriority {
    pub(super) fn for_path(atomic_depth: usize, negative_log_policy: f64) -> Self {
        Self {
            levin_log_priority: (atomic_depth.max(1) as f64).ln() + negative_log_policy,
            atomic_depth,
            negative_log_policy,
        }
    }
}

impl Eq for GeneratorWorkPriority {}

impl PartialEq for GeneratorWorkPriority {
    fn eq(&self, other: &Self) -> bool {
        self.levin_log_priority.to_bits() == other.levin_log_priority.to_bits()
    }
}

impl Ord for GeneratorWorkPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap; reverse the finite Levin cost so the least
        // expensive retained path is selected first.
        other.levin_log_priority.total_cmp(&self.levin_log_priority)
    }
}

impl PartialOrd for GeneratorWorkPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) struct GeneratorQueueEntry {
    pub(super) priority: GeneratorWorkPriority,
    pub(super) sequence_id: u64,
    pub(super) work_id: usize,
}

#[derive(Clone, Debug)]
pub(super) struct GuidedGeneratorQueueEntry {
    pub(super) guide_lane: CombatGuideLaneId,
    pub(super) work_id: usize,
    pub(super) sequence_id: u64,
    pub(super) guide_rank: CombatStateGuideRank,
    pub(super) anchor_priority: GeneratorWorkPriority,
}

impl Eq for GuidedGeneratorQueueEntry {}

impl PartialEq for GuidedGeneratorQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.guide_lane == other.guide_lane
            && self.work_id == other.work_id
            && self.sequence_id == other.sequence_id
    }
}

impl Ord for GuidedGeneratorQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.guide_rank
            .cmp(&other.guide_rank)
            .then_with(|| self.anchor_priority.cmp(&other.anchor_priority))
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for GuidedGeneratorQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) struct GuidedGeneratorFrontier {
    pub(super) lane: CombatGuideLaneId,
    pub(super) entries: BinaryHeap<GuidedGeneratorQueueEntry>,
}

impl Eq for GeneratorQueueEntry {}

impl PartialEq for GeneratorQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence_id == other.sequence_id
    }
}

impl Ord for GeneratorQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for GeneratorQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PushWorkTiming {
    pub(super) guide_elapsed_ns: u64,
    pub(super) retain_elapsed_ns: u64,
    pub(super) agenda_elapsed_ns: u64,
}

pub(super) fn guides_with_lookahead(
    base: Arc<[CombatStateGuide]>,
    lookahead: Option<&CombatStateGuide>,
) -> Arc<[CombatStateGuide]> {
    let Some(lookahead) = lookahead else {
        return base;
    };
    let mut guides = base.to_vec();
    if let Some(existing) = guides.iter_mut().find(|guide| guide.lane == lookahead.lane) {
        *existing = lookahead.clone();
    } else {
        guides.push(lookahead.clone());
    }
    guides.into()
}

impl TurnOptionGeneratorSession {
    pub(crate) fn prefer_lane(&mut self, preferred: TurnOptionGeneratorPreferredLane) {
        self.next_scheduler_lane = match preferred {
            TurnOptionGeneratorPreferredLane::Anchor => 0,
            TurnOptionGeneratorPreferredLane::Guide(lane) => self
                .guide_frontier_index(lane)
                .map_or(0, |frontier_index| frontier_index.saturating_add(1)),
        };
    }

    pub(super) fn push_work(
        &mut self,
        work: GeneratorWork,
        priority: GeneratorWorkPriority,
    ) -> usize {
        self.push_work_measured(work, priority, false).0
    }

    pub(super) fn push_work_measured(
        &mut self,
        mut work: GeneratorWork,
        priority: GeneratorWorkPriority,
        measure: bool,
    ) -> (usize, PushWorkTiming) {
        debug_assert!(priority.levin_log_priority.is_finite());
        let guide_started = measure.then(Instant::now);
        let base_guides = match &mut work {
            GeneratorWork::AtomicActions(cursor) => cursor.guides.clone(),
            GeneratorWork::Expand(partial) => {
                if let Some(guides) = partial.generation_guides.as_ref() {
                    guides.clone()
                } else {
                    let guides: Arc<[CombatStateGuide]> =
                        self.policy.turn_generation_guides(&partial.position).into();
                    partial.generation_guides = Some(guides.clone());
                    guides
                }
            }
            GeneratorWork::ApplyAction(action) => {
                action.parent.generation_guides.clone().unwrap_or_else(|| {
                    self.policy
                        .turn_generation_guides(&action.parent.position)
                        .into()
                })
            }
            GeneratorWork::StructuredSelection(selection) => selection
                .parent
                .generation_guides
                .clone()
                .unwrap_or_else(|| {
                    self.policy
                        .turn_generation_guides(&selection.parent.position)
                        .into()
                }),
        };
        let guides = guides_with_lookahead(
            base_guides,
            match &work {
                GeneratorWork::Expand(partial) => partial.lookahead_guide.as_ref(),
                _ => None,
            },
        );
        let guide_elapsed_ns = guide_started.map(elapsed_nanos_u64).unwrap_or(0);

        let retain_started = measure.then(Instant::now);
        let work_id = self.work.len();
        self.work.push(Some(work));
        self.guide_entries_per_work.push(guides.len());
        let entry = GeneratorQueueEntry {
            priority,
            sequence_id: self.next_sequence_id,
            work_id,
        };
        let retain_elapsed_ns = retain_started.map(elapsed_nanos_u64).unwrap_or(0);

        let agenda_started = measure.then(Instant::now);
        self.anchor_frontier.push(entry);
        for guide in guides.iter() {
            let frontier_index = self.ensure_guide_frontier(guide.lane);
            self.guided_frontiers[frontier_index]
                .entries
                .push(GuidedGeneratorQueueEntry {
                    guide_lane: guide.lane,
                    work_id,
                    sequence_id: self.next_sequence_id,
                    guide_rank: guide.rank.clone(),
                    anchor_priority: priority,
                });
        }
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
        self.live_work_items = self.live_work_items.saturating_add(1);
        self.live_guide_entries = self.live_guide_entries.saturating_add(guides.len());
        let agenda_elapsed_ns = agenda_started.map(elapsed_nanos_u64).unwrap_or(0);
        (
            work_id,
            PushWorkTiming {
                guide_elapsed_ns,
                retain_elapsed_ns,
                agenda_elapsed_ns,
            },
        )
    }

    #[cfg(test)]
    pub(super) fn pop_scheduled_work(&mut self) -> Option<GeneratorWork> {
        let lane_count = self.guided_frontiers.len().saturating_add(1);
        for offset in 0..lane_count {
            let lane = (self.next_scheduler_lane + offset) % lane_count;
            let work_id = if lane == 0 {
                self.pop_anchor_work_id()
            } else {
                self.pop_guided_work_id(lane - 1)
            };
            let Some(work_id) = work_id else {
                continue;
            };
            let work = self.take_live_work(work_id);
            if lane == 0 {
                self.anchor_work_pops = self.anchor_work_pops.saturating_add(1);
            } else {
                self.guided_work_pops = self.guided_work_pops.saturating_add(1);
            }
            self.next_scheduler_lane = (lane + 1) % lane_count;
            return Some(work);
        }
        None
    }

    pub(super) fn take_live_work(&mut self, work_id: usize) -> GeneratorWork {
        debug_assert_eq!(self.guide_entries_per_work.len(), self.work.len());
        let work = self.work[work_id]
            .take()
            .expect("scheduled generator work must still be live");
        self.live_work_items = self.live_work_items.saturating_sub(1);
        let guide_entries = self.guide_entries_per_work[work_id];
        debug_assert!(self.live_guide_entries >= guide_entries);
        self.live_guide_entries = self.live_guide_entries.saturating_sub(guide_entries);
        work
    }

    /// Rebuilds lazy scheduling heaps only between frozen service rounds.
    ///
    /// A strict stale majority both amortizes the linear retain and bounds
    /// retained garbage without changing any live entry's `Ord` key.
    pub(super) fn reclaim_stale_scheduling_entries(&mut self) {
        debug_assert!(self.scheduled_round.is_empty());

        let stale_anchor_entries = self
            .anchor_frontier
            .len()
            .saturating_sub(self.live_work_items);
        let guide_entries = self
            .guided_frontiers
            .iter()
            .map(|frontier| frontier.entries.len())
            .sum::<usize>();
        let stale_guide_entries = guide_entries.saturating_sub(self.live_guide_entries);
        let rebuild_anchor = stale_anchor_entries > self.live_work_items;
        let rebuild_guides = stale_guide_entries > self.live_guide_entries;
        if !rebuild_anchor && !rebuild_guides {
            return;
        }

        let work = &self.work;
        let mut reclaimed_anchor_entries = 0;
        if rebuild_anchor {
            let before = self.anchor_frontier.len();
            self.anchor_frontier
                .retain(|entry| work.get(entry.work_id).is_some_and(Option::is_some));
            reclaimed_anchor_entries = before.saturating_sub(self.anchor_frontier.len());
            self.anchor_frontier.shrink_to_fit();
        }

        let mut reclaimed_guide_entries = 0usize;
        if rebuild_guides {
            for frontier in &mut self.guided_frontiers {
                let before = frontier.entries.len();
                frontier
                    .entries
                    .retain(|entry| work.get(entry.work_id).is_some_and(Option::is_some));
                let reclaimed = before.saturating_sub(frontier.entries.len());
                reclaimed_guide_entries = reclaimed_guide_entries.saturating_add(reclaimed);
                if reclaimed > 0 {
                    frontier.entries.shrink_to_fit();
                }
            }
        }

        self.scheduling_rebuilds = self.scheduling_rebuilds.saturating_add(1);
        self.reclaimed_anchor_entries = self
            .reclaimed_anchor_entries
            .saturating_add(reclaimed_anchor_entries);
        self.reclaimed_guide_entries = self
            .reclaimed_guide_entries
            .saturating_add(reclaimed_guide_entries);
    }

    pub(super) fn peek_anchor_work_id(&mut self) -> Option<usize> {
        while let Some(entry) = self.anchor_frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some(entry.work_id);
            }
            self.anchor_frontier.pop();
        }
        None
    }

    #[cfg(test)]
    pub(super) fn pop_anchor_work_id(&mut self) -> Option<usize> {
        self.peek_anchor_work_id()?;
        self.anchor_frontier.pop().map(|entry| entry.work_id)
    }

    pub(super) fn peek_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
        let frontier = &mut self.guided_frontiers.get_mut(guide_index)?.entries;
        while let Some(entry) = frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some(entry.work_id);
            }
            frontier.pop();
        }
        None
    }

    #[cfg(test)]
    pub(super) fn pop_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
        self.peek_guided_work_id(guide_index)?;
        self.guided_frontiers[guide_index]
            .entries
            .pop()
            .map(|entry| entry.work_id)
    }

    pub(super) fn guide_frontier_index(&self, lane: CombatGuideLaneId) -> Option<usize> {
        self.guided_frontiers
            .iter()
            .position(|frontier| frontier.lane == lane)
    }

    pub(super) fn ensure_guide_frontier(&mut self, lane: CombatGuideLaneId) -> usize {
        if let Some(index) = self.guide_frontier_index(lane) {
            return index;
        }
        self.guided_frontiers.push(GuidedGeneratorFrontier {
            lane,
            entries: BinaryHeap::new(),
        });
        self.guided_frontiers.len() - 1
    }
}
