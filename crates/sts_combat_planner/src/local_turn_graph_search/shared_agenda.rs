use super::*;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

#[derive(Clone, Debug)]
struct AnchorEntry {
    node_id: usize,
    service_cost: f64,
}

impl PartialEq for AnchorEntry {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.service_cost.total_cmp(&other.service_cost).is_eq()
    }
}

impl Eq for AnchorEntry {}

impl PartialOrd for AnchorEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnchorEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max heap. Smaller search cost and then smaller node
        // id are therefore ordered as the greater entry.
        other
            .service_cost
            .total_cmp(&self.service_cost)
            .then_with(|| other.node_id.cmp(&self.node_id))
    }
}

#[derive(Clone, Debug)]
struct GuideEntry {
    node_id: usize,
    rank: CombatStateGuideRank,
    path_cost: f64,
}

impl PartialEq for GuideEntry {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.rank == other.rank
            && self.path_cost.total_cmp(&other.path_cost).is_eq()
    }
}

impl Eq for GuideEntry {}

impl PartialOrd for GuideEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GuideEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BTreeSet iterates in ascending order. Put the strongest semantic
        // rank first, then the cheapest retained exact path.
        other
            .rank
            .cmp(&self.rank)
            .then_with(|| self.path_cost.total_cmp(&other.path_cost))
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

/// Independent scheduling views over one shared exact-state graph.
///
/// A view chooses a boundary node. It never owns expansion below that node:
/// the selected node's turn generator chooses its own local generation lane.
pub(super) struct SharedBoundaryAgenda {
    anchor: BinaryHeap<AnchorEntry>,
    guides: BTreeMap<CombatGuideLaneId, BTreeSet<GuideEntry>>,
    next_view: usize,
    lookahead_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SharedAgendaPosition {
    pub(super) ordinal_rank: Option<usize>,
    pub(super) candidate_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SharedAnchorPosition {
    pub(super) agenda: SharedAgendaPosition,
    pub(super) service_cost: Option<f64>,
    pub(super) best_service_cost: Option<f64>,
}

impl SharedBoundaryAgenda {
    pub(super) fn new(lookahead_enabled: bool) -> Self {
        Self {
            anchor: BinaryHeap::new(),
            guides: BTreeMap::new(),
            next_view: 0,
            lookahead_enabled,
        }
    }

    pub(super) fn publish_node(
        &mut self,
        node_id: usize,
        node: &GraphNode,
        lookahead_lane: Option<CombatGuideLaneId>,
    ) {
        if node.generator.is_finished() || node.exhausted {
            return;
        }
        self.anchor.push(AnchorEntry {
            node_id,
            service_cost: anchor_service_cost(node),
        });
        self.publish_guide_entries(node_id, node, lookahead_lane);
    }

    pub(super) fn publish_guide_entries(
        &mut self,
        node_id: usize,
        node: &GraphNode,
        lookahead_lane: Option<CombatGuideLaneId>,
    ) {
        if node.generator.is_finished() || node.exhausted {
            return;
        }
        for guide in &node.guides {
            if Some(guide.lane) == lookahead_lane && node.lookahead_pending_lane == Some(guide.lane)
            {
                continue;
            }
            self.publish_guide_entry(node_id, node, guide.lane);
        }
    }

    pub(super) fn publish_guide_entry(
        &mut self,
        node_id: usize,
        node: &GraphNode,
        lane: CombatGuideLaneId,
    ) {
        if node.generator.is_finished() || node.exhausted {
            return;
        }
        let Some(rank) = guide_rank(node, lane) else {
            return;
        };
        self.guides.entry(lane).or_default().insert(GuideEntry {
            node_id,
            rank: rank.clone(),
            path_cost: node.path_cost(),
        });
    }

    pub(super) fn remove_guide_entries(
        &mut self,
        node_id: usize,
        node: &GraphNode,
        lookahead_lane: Option<CombatGuideLaneId>,
    ) {
        for guide in &node.guides {
            if Some(guide.lane) == lookahead_lane && node.lookahead_pending_lane == Some(guide.lane)
            {
                continue;
            }
            if let Some(entries) = self.guides.get_mut(&guide.lane) {
                entries.remove(&GuideEntry {
                    node_id,
                    rank: guide.rank.clone(),
                    path_cost: node.path_cost(),
                });
            }
        }
    }

    pub(super) fn republish_anchor(&mut self, node_id: usize, node: &GraphNode) {
        if node.generator.is_finished() || node.exhausted {
            return;
        }
        self.anchor.push(AnchorEntry {
            node_id,
            service_cost: anchor_service_cost(node),
        });
    }

    pub(super) fn next_service_view(&mut self) -> LocalServiceView {
        let mut views = Vec::with_capacity(self.guides.len().saturating_add(2));
        views.push(LocalServiceView::Anchor);
        if self.lookahead_enabled {
            views.push(LocalServiceView::LookaheadEvaluation);
        }
        views.extend(self.guides.keys().copied().map(LocalServiceView::Guide));
        let view = views[self.next_view % views.len()];
        self.next_view = self.next_view.saturating_add(1);
        view
    }

    pub(super) fn view_count(&self) -> usize {
        self.guides
            .len()
            .saturating_add(1)
            .saturating_add(usize::from(self.lookahead_enabled))
    }

    pub(super) fn select_anchor(&mut self, nodes: &[GraphNode]) -> Option<usize> {
        while let Some(entry) = self.anchor.pop() {
            let node = &nodes[entry.node_id];
            if !node.exhausted
                && !node.generator.is_finished()
                && entry
                    .service_cost
                    .total_cmp(&anchor_service_cost(node))
                    .is_eq()
            {
                return Some(entry.node_id);
            }
        }
        None
    }

    pub(super) fn select_guide(
        &mut self,
        lane: CombatGuideLaneId,
        nodes: &[GraphNode],
    ) -> Option<usize> {
        let entries = self.guides.get_mut(&lane)?;
        let selected = entries
            .iter()
            .filter(|entry| {
                let node = &nodes[entry.node_id];
                !node.exhausted
                    && !node.generator.is_finished()
                    && guide_rank(node, lane) == Some(&entry.rank)
                    && node.path_cost().total_cmp(&entry.path_cost).is_eq()
            })
            .next()
            .cloned()?;
        // One inadmissible guide grants one coherent expansion to a shared
        // exact state, then moves on. Repeated/completeness service belongs to
        // the anchor queue, matching the ownership split in Shared MHA*.
        entries.remove(&selected);
        Some(selected.node_id)
    }

    pub(super) fn select_pending_lookahead(&self, nodes: &[GraphNode]) -> Option<usize> {
        nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                !node.exhausted
                    && node.lookahead_pending_lane.is_some()
                    && node.generator.counters().generation_work > 0
            })
            .min_by(|(left_id, left), (right_id, right)| {
                left.path_cost()
                    .total_cmp(&right.path_cost())
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(node_id, _)| node_id)
    }

    pub(super) fn anchor_position(
        &self,
        node_id: usize,
        nodes: &[GraphNode],
    ) -> SharedAnchorPosition {
        let mut candidates = self
            .anchor
            .iter()
            .filter_map(|entry| {
                let node = &nodes[entry.node_id];
                (!node.exhausted
                    && !node.generator.is_finished()
                    && entry
                        .service_cost
                        .total_cmp(&anchor_service_cost(node))
                        .is_eq())
                .then_some((entry.node_id, entry.service_cost))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|(left_id, left_cost), (right_id, right_cost)| {
            left_cost
                .total_cmp(right_cost)
                .then_with(|| left_id.cmp(right_id))
        });
        candidates.dedup_by_key(|(candidate_id, _)| *candidate_id);
        SharedAnchorPosition {
            agenda: SharedAgendaPosition {
                ordinal_rank: candidates
                    .iter()
                    .position(|(candidate_id, _)| *candidate_id == node_id)
                    .map(|index| index.saturating_add(1)),
                candidate_count: candidates.len(),
            },
            service_cost: candidates
                .iter()
                .find(|(candidate_id, _)| *candidate_id == node_id)
                .map(|(_, cost)| *cost),
            best_service_cost: candidates.first().map(|(_, cost)| *cost),
        }
    }

    pub(super) fn guide_position(
        &self,
        node_id: usize,
        lane: CombatGuideLaneId,
        nodes: &[GraphNode],
    ) -> (SharedAgendaPosition, Option<&CombatStateGuideRank>) {
        let Some(entries) = self.guides.get(&lane) else {
            return (
                SharedAgendaPosition {
                    ordinal_rank: None,
                    candidate_count: 0,
                },
                None,
            );
        };
        let candidates = entries
            .iter()
            .filter(|entry| {
                let node = &nodes[entry.node_id];
                !node.exhausted
                    && !node.generator.is_finished()
                    && guide_rank(node, lane) == Some(&entry.rank)
                    && node.path_cost().total_cmp(&entry.path_cost).is_eq()
            })
            .collect::<Vec<_>>();
        (
            SharedAgendaPosition {
                ordinal_rank: candidates
                    .iter()
                    .position(|entry| entry.node_id == node_id)
                    .map(|index| index.saturating_add(1)),
                candidate_count: candidates.len(),
            },
            candidates.first().map(|entry| &entry.rank),
        )
    }
}

fn anchor_service_cost(node: &GraphNode) -> f64 {
    node.path_cost() + (node.widen_anchor_visits.saturating_add(1) as f64).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_entries_order_stronger_rank_before_cheaper_weaker_path() {
        let strong = GuideEntry {
            node_id: 7,
            rank: CombatStateGuideRank::new(vec![2, 0]),
            path_cost: 100.0,
        };
        let weak = GuideEntry {
            node_id: 3,
            rank: CombatStateGuideRank::new(vec![1, 999]),
            path_cost: 0.0,
        };
        let entries = BTreeSet::from([weak, strong]);
        assert_eq!(entries.first().map(|entry| entry.node_id), Some(7));
    }

    #[test]
    fn anchor_heap_prefers_lower_service_cost() {
        let mut entries = BinaryHeap::from([
            AnchorEntry {
                node_id: 1,
                service_cost: 5.0,
            },
            AnchorEntry {
                node_id: 2,
                service_cost: 1.0,
            },
        ]);
        assert_eq!(entries.pop().map(|entry| entry.node_id), Some(2));
    }
}
