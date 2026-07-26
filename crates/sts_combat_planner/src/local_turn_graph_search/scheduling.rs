use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum LocalWorkChoice {
    Widen {
        view: LocalServiceView,
    },
    Edge {
        edge_index: usize,
        view: LocalServiceView,
    },
}

pub(super) fn select_path_service_view(
    inherited: Option<LocalServiceView>,
    available: &[LocalServiceView],
    next_view: &mut usize,
) -> LocalServiceView {
    if let Some(view) = inherited {
        return view;
    }
    let view = available[*next_view % available.len()];
    *next_view = next_view.saturating_add(1);
    view
}

pub(super) fn select_local_work(
    node: &GraphNode,
    nodes: &[GraphNode],
    view: LocalServiceView,
    allow_widen: bool,
    progressive_guide_lane: Option<CombatGuideLaneId>,
) -> Option<LocalWorkChoice> {
    match view {
        LocalServiceView::Anchor => select_anchor_work(node, nodes, allow_widen),
        LocalServiceView::LookaheadEvaluation => select_pending_lookahead_work(node, nodes),
        LocalServiceView::Guide(lane) => select_guide_work(
            node,
            nodes,
            lane,
            allow_widen,
            guide_uses_progressive_service(lane, progressive_guide_lane),
        ),
    }
}

pub(super) fn guide_uses_progressive_service(
    lane: CombatGuideLaneId,
    configured_progressive_lane: Option<CombatGuideLaneId>,
) -> bool {
    // Progressive widening belongs to the explicitly configured expensive
    // acquisition lane. Cheap semantic guides exploit their current best
    // exact successor; anchor service and the other independent guide lanes
    // retain global exploration.
    configured_progressive_lane == Some(lane)
}

pub(super) fn select_pending_lookahead_work(
    node: &GraphNode,
    nodes: &[GraphNode],
) -> Option<LocalWorkChoice> {
    node.children
        .iter()
        .enumerate()
        .filter(|(_, edge)| {
            !nodes[edge.successor].exhausted
                && nodes[edge.successor].lookahead_pending_lane.is_some()
        })
        .map(|(edge_index, edge)| {
            (
                local_path_base(edge.actions.len(), edge.negative_log_policy),
                edge.visits,
                edge.successor,
                LocalWorkChoice::Edge {
                    edge_index,
                    view: LocalServiceView::LookaheadEvaluation,
                },
            )
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        })
        .map(|(_, _, _, choice)| choice)
}

pub(super) fn select_anchor_work(
    node: &GraphNode,
    nodes: &[GraphNode],
    allow_widen: bool,
) -> Option<LocalWorkChoice> {
    let widen = allow_widen
        .then(|| node.generator.best_retained_path_bound_snapshot())
        .flatten()
        .map(|(atomic_depth, negative_log_policy)| {
            (
                local_path_service_cost(
                    atomic_depth,
                    negative_log_policy,
                    node.widen_anchor_visits,
                ),
                LocalWorkChoice::Widen {
                    view: LocalServiceView::Anchor,
                },
            )
        });
    let best_edge = node
        .children
        .iter()
        .enumerate()
        .filter(|(_, edge)| !nodes[edge.successor].exhausted)
        .map(|(edge_index, edge)| {
            (
                local_path_service_cost(
                    edge.actions.len(),
                    edge.negative_log_policy,
                    edge.anchor_visits,
                ),
                edge.visits,
                edge.successor,
                LocalWorkChoice::Edge {
                    edge_index,
                    view: LocalServiceView::Anchor,
                },
            )
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
    match (widen, best_edge) {
        (Some((widen_cost, widen)), Some((edge_cost, _, _, edge))) => {
            Some(if widen_cost.total_cmp(&edge_cost).is_le() {
                widen
            } else {
                edge
            })
        }
        (Some((_, widen)), None) => Some(widen),
        (None, Some((_, _, _, edge))) => Some(edge),
        (None, None) => None,
    }
}

pub(super) fn select_backed_edge(node: &GraphNode, nodes: &[GraphNode]) -> Option<usize> {
    let mut ranked = node
        .children
        .iter()
        .enumerate()
        .filter(|(_, edge)| !nodes[edge.successor].exhausted)
        .filter_map(|(edge_index, edge)| {
            edge.backed_lookahead_rank
                .as_ref()
                .map(|rank| (edge_index, rank))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_index, left_rank), (right_index, right_rank)| {
        let left = &node.children[*left_index];
        let right = &node.children[*right_index];
        guide_choice_order(
            left_rank,
            local_path_base(left.actions.len(), left.negative_log_policy),
            left.backed_visits,
            left.successor,
            right_rank,
            local_path_base(right.actions.len(), right.negative_log_policy),
            right.backed_visits,
            right.successor,
        )
    });
    let total_service = ranked.iter().fold(0usize, |total, (edge_index, _)| {
        total.saturating_add(node.children[*edge_index].backed_visits)
    });
    let active_width = progressive_guide_width(total_service).max(1);
    ranked
        .iter()
        .take(active_width)
        .enumerate()
        .min_by_key(|(ordinal, (edge_index, _))| {
            (node.children[*edge_index].backed_visits, *ordinal)
        })
        .map(|(_, (edge_index, _))| *edge_index)
}

pub(super) fn select_pending_lookahead_edge(
    node: &GraphNode,
    nodes: &[GraphNode],
    view: LocalServiceView,
    active_width: usize,
) -> Option<usize> {
    let mut ranked = node
        .children
        .iter()
        .enumerate()
        .filter(|(_, edge)| !nodes[edge.successor].exhausted)
        .filter_map(|(edge_index, edge)| match view {
            LocalServiceView::Anchor => Some((edge_index, None)),
            // Acquisition compares the candidate boundary's own cheap,
            // immutable evidence. Using descendant Max-backup here lets
            // explored branches continually move the admission frontier and
            // starve an unevaluated sibling. Backed values still own
            // exploitation after expensive evidence exists.
            LocalServiceView::Guide(lane) => {
                guide_rank(&nodes[edge.successor], lane).map(|rank| (edge_index, Some(rank)))
            }
            LocalServiceView::LookaheadEvaluation => None,
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_index, left_rank), (right_index, right_rank)| {
        let left = &node.children[*left_index];
        let right = &node.children[*right_index];
        match (left_rank, right_rank) {
            (Some(left_rank), Some(right_rank)) => guide_choice_order(
                left_rank,
                local_path_base(left.actions.len(), left.negative_log_policy),
                0,
                left.successor,
                right_rank,
                local_path_base(right.actions.len(), right.negative_log_policy),
                0,
                right.successor,
            ),
            (None, None) => left
                .negative_log_policy
                .total_cmp(&right.negative_log_policy)
                .then_with(|| left.actions.len().cmp(&right.actions.len()))
                .then_with(|| left.successor.cmp(&right.successor))
                .then_with(|| left_index.cmp(right_index)),
            _ => unreachable!("one acquisition view gives every candidate one rank shape"),
        }
    });
    ranked
        .into_iter()
        .take(active_width.max(1))
        .find(|(edge_index, _)| {
            nodes[node.children[*edge_index].successor]
                .lookahead_pending_lane
                .is_some()
        })
        .map(|(edge_index, _)| edge_index)
}

pub(super) fn round_robin_available_index(start: usize, available: &[bool]) -> Option<usize> {
    if available.is_empty() {
        return None;
    }
    (0..available.len())
        .map(|offset| start.wrapping_add(offset) % available.len())
        .find(|index| available[*index])
}

pub(super) fn backed_widen_due(
    widen_services: usize,
    deepen_services: usize,
    can_widen: bool,
) -> bool {
    can_widen && guide_widen_service_due(widen_services, deepen_services)
}

pub(super) fn backed_widen_quantum(
    node_id: usize,
    regular_work: usize,
    backed_work: usize,
) -> usize {
    if node_id == 0 {
        regular_work
    } else {
        backed_work
    }
}

pub(super) fn generator_needs_initial_grounding(
    generation_work: usize,
    generator_finished: bool,
) -> bool {
    generation_work == 0 && !generator_finished
}

pub(super) fn progressive_rollout_width(total_service: usize) -> usize {
    ((total_service.saturating_add(1) as f64).sqrt() as usize).max(1)
}

pub(super) fn update_max_rank(
    current: &mut Option<CombatStateGuideRank>,
    candidate: &CombatStateGuideRank,
) -> bool {
    if current
        .as_ref()
        .is_some_and(|existing| existing >= candidate)
    {
        return false;
    }
    *current = Some(candidate.clone());
    true
}

pub(super) fn update_max_guide(
    current: &mut BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    lane: CombatGuideLaneId,
    candidate: &CombatStateGuideRank,
) -> bool {
    if current
        .get(&lane)
        .is_some_and(|existing| existing >= candidate)
    {
        return false;
    }
    current.insert(lane, candidate.clone());
    true
}

pub(super) fn select_guide_work(
    node: &GraphNode,
    nodes: &[GraphNode],
    lane: CombatGuideLaneId,
    allow_widen: bool,
    progressive_service: bool,
) -> Option<LocalWorkChoice> {
    if progressive_service {
        let mut ranked_candidates = node
            .children
            .iter()
            .enumerate()
            .filter(|(_, edge)| !nodes[edge.successor].exhausted)
            .filter_map(|(edge_index, edge)| {
                backed_guide_rank(edge, &nodes[edge.successor], lane)
                    .cloned()
                    .map(|rank| {
                        (
                            LocalWorkChoice::Edge {
                                edge_index,
                                view: LocalServiceView::Guide(lane),
                            },
                            rank,
                            local_path_base(edge.actions.len(), edge.negative_log_policy),
                            edge.visits,
                            edge.successor,
                            edge.guide_visits.get(&lane).copied().unwrap_or_default(),
                        )
                    })
            })
            .collect::<Vec<_>>();
        if allow_widen {
            let retained_path = node.generator.best_retained_path_bound_snapshot();
            let retained_guide = node.generator.best_retained_guide_promise_snapshot(lane);
            let widen_rank = retained_guide
                .as_ref()
                .map(|promise| promise.rank.clone())
                .or_else(|| guide_rank(node, lane).cloned());
            let widen_path = retained_guide
                .as_ref()
                .map(|promise| (promise.atomic_depth, promise.negative_log_policy))
                .or(retained_path);
            if let (Some(rank), Some((atomic_depth, negative_log_policy))) =
                (widen_rank, widen_path)
            {
                ranked_candidates.push((
                    LocalWorkChoice::Widen {
                        view: LocalServiceView::Guide(lane),
                    },
                    rank,
                    local_path_base(atomic_depth, negative_log_policy),
                    node.widen_guide_visits
                        .get(&lane)
                        .copied()
                        .unwrap_or_default(),
                    usize::MAX,
                    node.widen_guide_visits
                        .get(&lane)
                        .copied()
                        .unwrap_or_default(),
                ));
            }
        }
        ranked_candidates.sort_by(|left, right| {
            guide_choice_order(
                &left.1, left.2, left.3, left.4, &right.1, right.2, right.3, right.4,
            )
        });
        if !ranked_candidates.is_empty() {
            let total_service = ranked_candidates
                .iter()
                .fold(0usize, |total, candidate| total.saturating_add(candidate.5));
            let selected = progressive_candidate_index(
                total_service,
                ranked_candidates.iter().map(|candidate| candidate.5),
            )?;
            return Some(ranked_candidates[selected].0);
        }
    }

    let edge_ranks = node
        .children
        .iter()
        .map(|edge| {
            (!nodes[edge.successor].exhausted)
                .then(|| backed_guide_rank(edge, &nodes[edge.successor], lane).cloned())
                .flatten()
        })
        .collect::<Vec<_>>();
    let best_edge = edge_ranks
        .iter()
        .enumerate()
        .filter_map(|(edge_index, rank)| {
            let rank = rank.as_ref()?;
            let edge = &node.children[edge_index];
            Some((
                rank,
                local_path_base(edge.actions.len(), edge.negative_log_policy),
                edge.visits,
                edge.successor,
                LocalWorkChoice::Edge {
                    edge_index,
                    view: LocalServiceView::Guide(lane),
                },
            ))
        })
        .min_by(|left, right| {
            guide_choice_order(
                left.0, left.1, left.2, left.3, right.0, right.1, right.2, right.3,
            )
        })
        .map(|(rank, anchor, visits, successor, edge)| (rank, anchor, visits, successor, edge));
    let retained_promise = allow_widen
        .then(|| node.generator.best_retained_guide_promise_snapshot(lane))
        .flatten();
    match (retained_promise, best_edge) {
        (Some(promise), Some((edge_rank, edge_anchor, _edge_visits, successor, edge))) => {
            let promise_anchor = local_path_base(promise.atomic_depth, promise.negative_log_policy);
            let promise_visits = node
                .widen_guide_visits
                .get(&lane)
                .copied()
                .unwrap_or_default();
            let deepen_visits = node.children.iter().fold(0usize, |total, child| {
                total.saturating_add(child.guide_visits.get(&lane).copied().unwrap_or_default())
            });
            let promise_preferred = guide_choice_order(
                &promise.rank,
                promise_anchor,
                0,
                usize::MAX,
                edge_rank,
                edge_anchor,
                0,
                successor,
            )
            .is_lt();
            Some(
                if promise_preferred && guide_widen_service_due(promise_visits, deepen_visits) {
                    LocalWorkChoice::Widen {
                        view: LocalServiceView::Guide(lane),
                    }
                } else {
                    edge
                },
            )
        }
        (Some(_), None) => Some(LocalWorkChoice::Widen {
            view: LocalServiceView::Guide(lane),
        }),
        (None, Some((_, _, _, _, edge))) => Some(edge),
        (None, None) => None,
    }
}

pub(super) fn progressive_guide_width(total_service: usize) -> usize {
    (usize::BITS - total_service.saturating_add(1).leading_zeros()) as usize
}

pub(super) fn progressive_candidate_index(
    total_service: usize,
    service_counts_in_rank_order: impl IntoIterator<Item = usize>,
) -> Option<usize> {
    service_counts_in_rank_order
        .into_iter()
        .take(progressive_rollout_width(total_service))
        .enumerate()
        .min_by_key(|(ordinal, services)| (*services, *ordinal))
        .map(|(ordinal, _)| ordinal)
}

pub(super) fn guide_widen_service_due(widen_visits: usize, deepen_visits: usize) -> bool {
    widen_visits <= deepen_visits
}

pub(super) fn guide_choice_order(
    left_rank: &CombatStateGuideRank,
    left_anchor: f64,
    left_visits: usize,
    left_successor: usize,
    right_rank: &CombatStateGuideRank,
    right_anchor: f64,
    right_visits: usize,
    right_successor: usize,
) -> std::cmp::Ordering {
    // The policy-only anchor already owns completeness and fair service. An
    // auxiliary guide must remain exploitative; charging it service debt at
    // every tree level makes a good multi-turn corridor lose a fresh fraction
    // of its budget at every parent.
    right_rank
        .cmp(left_rank)
        .then_with(|| left_anchor.total_cmp(&right_anchor))
        .then_with(|| left_visits.cmp(&right_visits))
        .then_with(|| left_successor.cmp(&right_successor))
}

pub(super) fn local_path_base(atomic_depth: usize, negative_log_policy: f64) -> f64 {
    negative_log_policy + (atomic_depth.max(1) as f64).ln()
}

pub(super) fn local_path_service_cost(
    atomic_depth: usize,
    negative_log_policy: f64,
    services: usize,
) -> f64 {
    local_path_base(atomic_depth, negative_log_policy) + (services.saturating_add(1) as f64).ln()
}

pub(super) fn guide_rank(
    node: &GraphNode,
    lane: CombatGuideLaneId,
) -> Option<&CombatStateGuideRank> {
    node.guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| &guide.rank)
}

pub(super) fn backed_guide_rank<'a>(
    edge: &'a GraphEdge,
    successor: &'a GraphNode,
    lane: CombatGuideLaneId,
) -> Option<&'a CombatStateGuideRank> {
    edge.backed_guides
        .get(&lane)
        .or_else(|| guide_rank(successor, lane))
}

pub(super) fn guides_with_pending_lookahead(
    policy: &dyn crate::policy::CombatActionPolicy,
    evaluator: Option<&dyn crate::policy::CombatLookaheadEvaluator>,
    position: &CombatPosition,
) -> (Vec<CombatStateGuide>, Option<CombatGuideLaneId>) {
    let mut guides = policy.state_guides(position);
    let pending_lane = evaluator
        .and_then(|evaluator| evaluator.pending_guide(position))
        .and_then(|pending| {
            if guides.iter().any(|guide| guide.lane == pending.lane) {
                None
            } else {
                let lane = pending.lane;
                guides.push(pending);
                Some(lane)
            }
        });
    (guides, pending_lane)
}

pub(super) fn guide_rank_map(
    guides: &[CombatStateGuide],
) -> BTreeMap<CombatGuideLaneId, CombatStateGuideRank> {
    guides
        .iter()
        .map(|guide| (guide.lane, guide.rank.clone()))
        .collect()
}

pub(super) fn boundary_service_views_from_guides(
    guides: &[CombatStateGuide],
    pending_lookahead_lane: Option<CombatGuideLaneId>,
) -> Vec<LocalServiceView> {
    let lanes = guides
        .iter()
        .map(|guide| guide.lane)
        .filter(|lane| Some(*lane) != pending_lookahead_lane)
        .collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(
            pending_lookahead_lane
                .is_some()
                .then_some(LocalServiceView::LookaheadEvaluation),
        )
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

pub(super) fn lookahead_acquisition_views_from_guides(
    guides: &[CombatStateGuide],
    pending_lookahead_lane: Option<CombatGuideLaneId>,
) -> Vec<LocalServiceView> {
    let lanes = guides
        .iter()
        .map(|guide| guide.lane)
        .filter(|lane| Some(*lane) != pending_lookahead_lane)
        .collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

pub(super) fn generation_service_views_from_lanes(
    lanes: impl IntoIterator<Item = CombatGuideLaneId>,
) -> Vec<LocalServiceView> {
    let lanes = lanes.into_iter().collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

pub(super) fn replay_witness(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    negative_log_policy: f64,
    discovery_source: OracleCombatWitnessDiscoverySource,
    stepper: &dyn CombatStepper,
) -> Result<OracleCombatWitness, OracleCombatWitnessReplayError> {
    let mut position = root.clone();
    let mut engine_steps = 0usize;
    for (action_index, action) in actions.iter().enumerate() {
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(OracleCombatWitnessReplayError::IllegalInput { action_index });
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: action.engine_steps.max(1),
                deadline: None,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.truncated || result.timed_out {
            return Err(OracleCombatWitnessReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash.as_str() {
            return Err(OracleCombatWitnessReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win {
        return Err(OracleCombatWitnessReplayError::FinalStateIsNotWin);
    }
    Ok(OracleCombatWitness {
        actions: actions.to_vec(),
        final_position: position,
        negative_log_policy,
        replay_engine_steps: engine_steps,
        discovery_source,
    })
}

pub(super) fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

pub(super) fn elapsed_nanos_u64(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

pub(super) fn witness_better(left: &OracleCombatWitness, right: &OracleCombatWitness) -> bool {
    left.final_position
        .combat
        .entities
        .player
        .current_hp
        .cmp(&right.final_position.combat.entities.player.current_hp)
        .then_with(|| right.actions.len().cmp(&left.actions.len()))
        .then_with(|| {
            right
                .negative_log_policy
                .total_cmp(&left.negative_log_policy)
        })
        == std::cmp::Ordering::Greater
}

pub(super) fn local_deep_state_snapshot(
    node: &GraphNode,
    path_atomic_depth: usize,
) -> OracleCombatDeepStateSnapshot {
    let combat = &node.generator.root().position().combat;
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .collect::<Vec<_>>();
    OracleCombatDeepStateSnapshot {
        player_turn: combat.turn.turn_count,
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        alive_enemy_count: alive_monsters.len(),
        enemy_total_hp: alive_monsters
            .into_iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        hand_size: combat.zones.hand.len(),
        draw_pile_size: combat.zones.draw_pile.len(),
        discard_pile_size: combat.zones.discard_pile.len(),
        exhaust_pile_size: combat.zones.exhaust_pile.len(),
        path_atomic_depth,
    }
}
