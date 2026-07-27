mod config;
mod diagnostics;
mod policy_line;
mod potion_budget;
mod reporting;
mod scheduling;
mod session;

pub use config::LocalTurnGraphWitnessConfig;
use potion_budget::*;
pub use reporting::*;
use scheduling::*;

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use sts_combat_strategy::{
    combat_plan_action_timing_v1, combat_plan_has_timed_action_preference_v1,
    combat_plan_projection_v1, combat_plan_selection_member_timing_v1,
    combat_plan_transition_annotation_v1, CombatPlanActionTimingV1,
    CombatPlanTransitionAnnotationV1,
};
use sts_core::ai::combat_state_key::combat_exact_state_key;
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::ClientInput;

use super::generator::TurnOptionGeneratorPreferredLane;
use super::policy::{
    normalized_probabilities, CombatGuideLaneId, CombatPolicyChoice, CombatPolicyWitnessProposal,
    CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy,
    SharedCombatLookaheadEvaluator,
};
use super::selection_transaction::SelectionTransactionCursor;
use super::types::{
    exact_hash, CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOption,
    CompleteTurnOptionBoundary, TurnOptionAction, TurnOptionGenerationGap,
};
use super::witness_search::{
    OracleCombatDeepStateSnapshot, OracleCombatWitness, OracleCombatWitnessDiscoverySource,
    OracleCombatWitnessProgressSnapshot, OracleCombatWitnessReplayError,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessStateProgressSnapshot,
};
use super::TurnOptionGeneratorSession;

#[derive(Clone)]
struct LocalRootActionFamilyAccumulator {
    first_action: ClientInput,
    best_root_negative_log_policy: Option<f64>,
    completed_root_turn_options: usize,
    terminal_wins: usize,
    terminal_losses: usize,
    escapes: usize,
}

struct GraphNode {
    generator: TurnOptionGeneratorSession,
    /// Potion resources already expended on the retained path to this exact
    /// boundary. It is part of constrained search identity whenever the
    /// caller supplied a finite combat budget.
    potion_expenditures: u32,
    /// One exact incoming path retained for diagnostics only. Search ownership
    /// and scheduling continue to use the shared exact node.
    diagnostic_parent: Option<(usize, usize)>,
    relative_turn_depth: usize,
    visits: usize,
    generated_options: usize,
    children: Vec<GraphEdge>,
    guides: Vec<CombatStateGuide>,
    boundary_service_views: Vec<LocalServiceView>,
    next_boundary_service_view: usize,
    lookahead_acquisition_views: Vec<LocalServiceView>,
    next_lookahead_acquisition_view: usize,
    generation_service_views: Vec<LocalServiceView>,
    next_generation_service_view: usize,
    widen_anchor_visits: usize,
    widen_guide_visits: BTreeMap<CombatGuideLaneId, usize>,
    lookahead_pending_lane: Option<CombatGuideLaneId>,
    /// Best exact descendant observed for each cheap semantic guide.
    backed_guides: BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    /// Best bounded rollout value observed at this exact boundary. This is
    /// search guidance only; terminal authority still belongs to exact replay.
    backed_lookahead_rank: Option<CombatStateGuideRank>,
    synced_gaps: usize,
    exhausted: bool,
}

struct GraphEdge {
    successor: usize,
    actions: Vec<TurnOptionAction>,
    negative_log_policy: f64,
    plan_transition_annotation: Option<CombatPlanTransitionAnnotationV1>,
    visits: usize,
    anchor_visits: usize,
    guide_visits: BTreeMap<CombatGuideLaneId, usize>,
    /// Best exact descendant observed through this edge for each cheap guide.
    backed_guides: BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    /// Best evaluated descendant reached through this exact edge.
    backed_lookahead_rank: Option<CombatStateGuideRank>,
    backed_visits: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalServiceView {
    Anchor,
    LookaheadEvaluation,
    Guide(CombatGuideLaneId),
}

enum SelectedWork {
    Widen {
        node_id: usize,
        path: Vec<(usize, usize)>,
        view: LocalServiceView,
        requested_work: usize,
    },
    Evaluate {
        node_id: usize,
        path: Vec<(usize, usize)>,
    },
    Restart,
    Exhausted,
}

/// A resumable session. Exact successor nodes and their service statistics are
/// shared across all incoming edges.
pub struct LocalTurnGraphWitnessSession {
    original_root: CombatPosition,
    config: LocalTurnGraphWitnessConfig,
    policy: SharedCombatActionPolicy,
    lookahead_evaluator: Option<SharedCombatLookaheadEvaluator>,
    collect_plan_transition_annotations: bool,
    lookahead_lane: Option<CombatGuideLaneId>,
    nodes: Vec<GraphNode>,
    nodes_by_exact_key: HashMap<ConstrainedExactStateKey, usize>,
    used: LocalTurnGraphWitnessCounters,
    performance_timing: LocalTurnGraphPerformanceTiming,
    granted_selections: usize,
    granted_generation_work: usize,
    granted_engine_steps: usize,
    generation_gaps: Vec<TurnOptionGenerationGap>,
    root_action_families: Vec<LocalRootActionFamilyAccumulator>,
    witness: Option<OracleCombatWitness>,
    replay_failure: Option<OracleCombatWitnessReplayError>,
}

impl LocalTurnGraphWitnessSession {
    pub fn advance(
        &mut self,
        quantum: LocalTurnGraphWitnessQuantum,
        stepper: &dyn CombatStepper,
    ) -> LocalTurnGraphWitnessReport {
        self.granted_selections = self
            .granted_selections
            .saturating_add(quantum.additional_selections);
        self.granted_generation_work = self
            .granted_generation_work
            .saturating_add(quantum.additional_generation_work);
        self.granted_engine_steps = self
            .granted_engine_steps
            .saturating_add(quantum.additional_engine_steps);

        let status = loop {
            if self.witness_satisfies() {
                break LocalTurnGraphWitnessStatus::WitnessFound;
            }
            if let Some(error) = self.replay_failure.clone() {
                break LocalTurnGraphWitnessStatus::ReplayMismatch(error);
            }
            if deadline_reached(quantum.deadline) {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::Deadline,
                );
            }
            if self.used.selections >= self.granted_selections {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::SelectionBudget,
                );
            }
            if self
                .used
                .generation_work
                .saturating_add(self.used.lookahead_work)
                >= self.granted_generation_work
            {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::GenerationWorkBudget,
                );
            }
            if self.used.engine_steps >= self.granted_engine_steps {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::EngineStepBudget,
                );
            }

            let selection_started = Instant::now();
            let selected_work = self.select_work();
            self.performance_timing.selection_elapsed_ns = self
                .performance_timing
                .selection_elapsed_ns
                .saturating_add(elapsed_nanos_u64(selection_started));
            match selected_work {
                SelectedWork::Widen {
                    node_id,
                    path,
                    view,
                    requested_work,
                } => {
                    self.used.selections = self.used.selections.saturating_add(1);
                    if !self.widen(
                        node_id,
                        &path,
                        view,
                        requested_work,
                        quantum.deadline,
                        stepper,
                    ) {
                        break LocalTurnGraphWitnessStatus::Partial(
                            if deadline_reached(quantum.deadline) {
                                LocalTurnGraphWitnessInterruption::Deadline
                            } else {
                                LocalTurnGraphWitnessInterruption::EngineStepBudget
                            },
                        );
                    }
                }
                SelectedWork::Evaluate { node_id, path } => {
                    self.used.selections = self.used.selections.saturating_add(1);
                    if !self.evaluate_lookahead(node_id, &path, quantum.deadline) {
                        break LocalTurnGraphWitnessStatus::Partial(
                            if deadline_reached(quantum.deadline) {
                                LocalTurnGraphWitnessInterruption::Deadline
                            } else {
                                LocalTurnGraphWitnessInterruption::GenerationWorkBudget
                            },
                        );
                    }
                    // An expensive boundary observation must be grounded by
                    // at least one exact expansion. Otherwise the evaluator
                    // can label a state and leave it with zero exact children,
                    // forcing the global scheduler to rediscover the same
                    // boundary before any real evidence exists.
                    if generator_needs_initial_grounding(
                        self.nodes[node_id].generator.counters().generation_work,
                        self.nodes[node_id].generator.is_finished(),
                    ) && !self.widen(
                        node_id,
                        &path,
                        LocalServiceView::Anchor,
                        self.config.backed_generation_quantum_work,
                        quantum.deadline,
                        stepper,
                    ) {
                        break LocalTurnGraphWitnessStatus::Partial(
                            if deadline_reached(quantum.deadline) {
                                LocalTurnGraphWitnessInterruption::Deadline
                            } else {
                                LocalTurnGraphWitnessInterruption::GenerationWorkBudget
                            },
                        );
                    }
                }
                SelectedWork::Restart => continue,
                SelectedWork::Exhausted => {
                    break if self.generation_gaps.is_empty() {
                        LocalTurnGraphWitnessStatus::FrontierExhausted
                    } else {
                        LocalTurnGraphWitnessStatus::MechanicsGap
                    };
                }
            }
        };
        self.snapshot(status)
    }

    fn record_root_option(&mut self, option: &CompleteTurnOption) {
        let Some(first_action) = option.actions().first() else {
            return;
        };
        let family_index = self
            .root_action_families
            .iter()
            .position(|family| family.first_action == first_action.input)
            .unwrap_or_else(|| {
                self.root_action_families
                    .push(LocalRootActionFamilyAccumulator {
                        first_action: first_action.input.clone(),
                        best_root_negative_log_policy: None,
                        completed_root_turn_options: 0,
                        terminal_wins: 0,
                        terminal_losses: 0,
                        escapes: 0,
                    });
                self.root_action_families.len() - 1
            });
        let family = &mut self.root_action_families[family_index];
        family.best_root_negative_log_policy = Some(
            family
                .best_root_negative_log_policy
                .map_or(option.negative_log_policy(), |current| {
                    current.min(option.negative_log_policy())
                }),
        );
        family.completed_root_turn_options = family.completed_root_turn_options.saturating_add(1);
        match option.boundary() {
            CompleteTurnOptionBoundary::TerminalWin => {
                family.terminal_wins = family.terminal_wins.saturating_add(1);
            }
            CompleteTurnOptionBoundary::TerminalLoss => {
                family.terminal_losses = family.terminal_losses.saturating_add(1);
            }
            CompleteTurnOptionBoundary::Escape => {
                family.escapes = family.escapes.saturating_add(1);
            }
            CompleteTurnOptionBoundary::NextPlayerTurn => {}
        }
    }

    fn select_work(&mut self) -> SelectedWork {
        let mut node_id = 0usize;
        let mut path = Vec::new();
        let mut path_view = None;
        loop {
            self.refresh_exhaustion(node_id);
            if self.nodes[node_id].exhausted {
                return if node_id == 0 {
                    SelectedWork::Exhausted
                } else {
                    SelectedWork::Restart
                };
            }

            self.nodes[node_id].visits = self.nodes[node_id].visits.saturating_add(1);
            self.used.node_visits = self.used.node_visits.saturating_add(1);
            let generator_counters = self.nodes[node_id].generator.counters();
            if generator_needs_initial_grounding(
                generator_counters.generation_work,
                self.nodes[node_id].generator.is_finished(),
            ) {
                self.nodes[node_id].widen_anchor_visits =
                    self.nodes[node_id].widen_anchor_visits.saturating_add(1);
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: LocalServiceView::Anchor,
                    requested_work: if node_id == 0 {
                        self.config.root_initial_expansion_work
                    } else {
                        self.config.initial_expansion_work
                    },
                };
            }
            let requested_view = {
                let node = &mut self.nodes[node_id];
                select_path_service_view(
                    path_view,
                    &node.boundary_service_views,
                    &mut node.next_boundary_service_view,
                )
            };
            if requested_view == LocalServiceView::LookaheadEvaluation {
                // Rollout is one portfolio member, not the sole authority.
                // Its service owns Widen/Deepen jointly and backs values along
                // one exact path; the other root services preserve the proven
                // anchor and typed semantic guides.
                return self.select_backed_work();
            }
            let selected = select_local_work(
                &self.nodes[node_id],
                &self.nodes,
                requested_view,
                true,
                self.lookahead_lane,
            )
            .or_else(|| {
                select_local_work(
                    &self.nodes[node_id],
                    &self.nodes,
                    LocalServiceView::Anchor,
                    true,
                    self.lookahead_lane,
                )
            });
            let Some(selected) = selected else {
                self.nodes[node_id].exhausted = true;
                self.used.exhausted_nodes = self.used.exhausted_nodes.saturating_add(1);
                return SelectedWork::Restart;
            };
            let LocalWorkChoice::Edge {
                edge_index,
                view: actual_view,
            } = selected
            else {
                let LocalWorkChoice::Widen { view } = selected else {
                    unreachable!()
                };
                let node = &mut self.nodes[node_id];
                let generation_view = match view {
                    LocalServiceView::Anchor => {
                        node.widen_anchor_visits = node.widen_anchor_visits.saturating_add(1);
                        let generation_view = node.generation_service_views[node
                            .next_generation_service_view
                            % node.generation_service_views.len()];
                        node.next_generation_service_view =
                            node.next_generation_service_view.saturating_add(1);
                        generation_view
                    }
                    LocalServiceView::Guide(lane) => {
                        let visits = node.widen_guide_visits.entry(lane).or_default();
                        *visits = visits.saturating_add(1);
                        LocalServiceView::Guide(lane)
                    }
                    LocalServiceView::LookaheadEvaluation => {
                        unreachable!("lookahead evaluation selects an existing boundary child")
                    }
                };
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: generation_view,
                    requested_work: self.config.generation_quantum_work,
                };
            };
            self.nodes[node_id].children[edge_index].visits = self.nodes[node_id].children
                [edge_index]
                .visits
                .saturating_add(1);
            match actual_view {
                LocalServiceView::Anchor => {
                    self.nodes[node_id].children[edge_index].anchor_visits = self.nodes[node_id]
                        .children[edge_index]
                        .anchor_visits
                        .saturating_add(1);
                }
                LocalServiceView::Guide(lane) => {
                    let visits = self.nodes[node_id].children[edge_index]
                        .guide_visits
                        .entry(lane)
                        .or_default();
                    *visits = visits.saturating_add(1);
                }
                LocalServiceView::LookaheadEvaluation => {}
            }
            let successor = self.nodes[node_id].children[edge_index].successor;
            // One root service chooses one semantic view. Preserve that view
            // through the selected path instead of independently rotating at
            // every depth; independent rotations dilute an N-lane guide by
            // another factor of N at each player turn. If a node had to fall
            // back to Anchor, `actual_view` carries that explicit downgrade.
            path_view = Some(actual_view);
            path.push((node_id, edge_index));
            node_id = successor;
        }
    }

    /// Selects one exact unit of work for the rollout-backed graph.
    ///
    /// Complete player turns remain lazy proposals, but Widen and Deepen now
    /// have one owner. Widen and Deepen share one service currency, while only
    /// a progressively widened child window pays for rollout evaluation.
    /// Descendant values are max-backed through the exact incoming path. This
    /// is deliberately separate from the legacy multi-lane traversal above.
    fn select_backed_work(&mut self) -> SelectedWork {
        let mut node_id = 0usize;
        let mut path = Vec::new();
        loop {
            self.refresh_exhaustion(node_id);
            if self.nodes[node_id].exhausted {
                return if node_id == 0 {
                    SelectedWork::Exhausted
                } else {
                    SelectedWork::Restart
                };
            }

            self.nodes[node_id].visits = self.nodes[node_id].visits.saturating_add(1);
            self.used.node_visits = self.used.node_visits.saturating_add(1);
            let generator_counters = self.nodes[node_id].generator.counters();
            if generator_needs_initial_grounding(
                generator_counters.generation_work,
                self.nodes[node_id].generator.is_finished(),
            ) {
                self.nodes[node_id].widen_anchor_visits =
                    self.nodes[node_id].widen_anchor_visits.saturating_add(1);
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: LocalServiceView::Anchor,
                    requested_work: if node_id == 0 {
                        self.config.root_initial_expansion_work
                    } else {
                        self.config.initial_expansion_work
                    },
                };
            }

            if node_id != 0
                && self.nodes[node_id].lookahead_pending_lane.is_some()
                && self.used.boundary_lookahead_evaluations < self.config.lookahead_max_evaluations
            {
                return SelectedWork::Evaluate { node_id, path };
            }

            let backed_services = self.nodes[node_id]
                .children
                .iter()
                .map(|edge| edge.backed_visits)
                .sum::<usize>();
            if self.used.boundary_lookahead_evaluations < self.config.lookahead_max_evaluations {
                let active_width = progressive_rollout_width(backed_services);
                let acquisition_views = self.nodes[node_id].lookahead_acquisition_views.clone();
                let pending_by_view = acquisition_views
                    .iter()
                    .copied()
                    .map(|view| {
                        select_pending_lookahead_edge(
                            &self.nodes[node_id],
                            &self.nodes,
                            view,
                            active_width,
                        )
                    })
                    .collect::<Vec<_>>();
                let available = pending_by_view
                    .iter()
                    .map(Option::is_some)
                    .collect::<Vec<_>>();
                let start = self.nodes[node_id].next_lookahead_acquisition_view;
                if let Some(view_index) = round_robin_available_index(start, &available) {
                    self.nodes[node_id].next_lookahead_acquisition_view =
                        view_index.saturating_add(1);
                    let edge_index = pending_by_view[view_index]
                        .expect("available acquisition view must own a pending edge");
                    let successor = {
                        let edge = &mut self.nodes[node_id].children[edge_index];
                        edge.visits = edge.visits.saturating_add(1);
                        edge.backed_visits = edge.backed_visits.saturating_add(1);
                        edge.successor
                    };
                    path.push((node_id, edge_index));
                    node_id = successor;
                    continue;
                }
            }

            let can_widen = !self.nodes[node_id].generator.is_finished();
            let widen_due = backed_widen_due(
                self.nodes[node_id].widen_anchor_visits,
                backed_services,
                can_widen,
            );
            if widen_due {
                self.nodes[node_id].widen_anchor_visits =
                    self.nodes[node_id].widen_anchor_visits.saturating_add(1);
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: LocalServiceView::Anchor,
                    requested_work: backed_widen_quantum(
                        node_id,
                        self.config.generation_quantum_work,
                        self.config.backed_generation_quantum_work,
                    ),
                };
            }

            if let Some(edge_index) = select_backed_edge(&self.nodes[node_id], &self.nodes) {
                let successor = {
                    let edge = &mut self.nodes[node_id].children[edge_index];
                    edge.visits = edge.visits.saturating_add(1);
                    edge.backed_visits = edge.backed_visits.saturating_add(1);
                    edge.successor
                };
                path.push((node_id, edge_index));
                node_id = successor;
                continue;
            }

            if can_widen {
                self.nodes[node_id].widen_anchor_visits =
                    self.nodes[node_id].widen_anchor_visits.saturating_add(1);
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: LocalServiceView::Anchor,
                    requested_work: backed_widen_quantum(
                        node_id,
                        self.config.generation_quantum_work,
                        self.config.backed_generation_quantum_work,
                    ),
                };
            }

            self.nodes[node_id].exhausted = true;
            self.used.exhausted_nodes = self.used.exhausted_nodes.saturating_add(1);
            return SelectedWork::Restart;
        }
    }

    fn evaluate_lookahead(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        deadline: Option<Instant>,
    ) -> bool {
        let Some(evaluator) = self.lookahead_evaluator.as_ref() else {
            self.nodes[node_id].lookahead_pending_lane = None;
            return true;
        };
        let Some(expected_lane) = self.nodes[node_id].lookahead_pending_lane else {
            return true;
        };
        let remaining_work = self.granted_generation_work.saturating_sub(
            self.used
                .generation_work
                .saturating_add(self.used.lookahead_work),
        );
        let max_work = self
            .config
            .lookahead_work_per_evaluation
            .max(1)
            .min(remaining_work);
        if max_work == 0 {
            return false;
        }
        let position = self.nodes[node_id].generator.root().position();
        let Some(evaluation) = evaluator.evaluate(position, max_work, deadline) else {
            return false;
        };
        if evaluation.guide.lane != expected_lane {
            self.nodes[node_id].lookahead_pending_lane = None;
            return true;
        }
        let backed_rank = evaluation.guide.rank.clone();
        if let Some(guide) = self.nodes[node_id]
            .guides
            .iter_mut()
            .find(|guide| guide.lane == expected_lane)
        {
            *guide = evaluation.guide;
        } else {
            self.nodes[node_id].guides.push(evaluation.guide);
        }
        update_max_rank(&mut self.nodes[node_id].backed_lookahead_rank, &backed_rank);
        for (parent_id, edge_index) in path {
            let edge = &mut self.nodes[*parent_id].children[*edge_index];
            update_max_rank(&mut edge.backed_lookahead_rank, &backed_rank);
        }
        self.nodes[node_id].lookahead_pending_lane = None;
        self.used.lookahead_evaluations = self.used.lookahead_evaluations.saturating_add(1);
        self.used.lookahead_work = self
            .used
            .lookahead_work
            .saturating_add(evaluation.work.max(1));
        self.used.boundary_lookahead_evaluations =
            self.used.boundary_lookahead_evaluations.saturating_add(1);
        self.used.boundary_lookahead_work = self
            .used
            .boundary_lookahead_work
            .saturating_add(evaluation.work.max(1));
        true
    }

    fn widen(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        view: LocalServiceView,
        requested_work: usize,
        deadline: Option<Instant>,
        stepper: &dyn CombatStepper,
    ) -> bool {
        let remaining_work = self.granted_generation_work.saturating_sub(
            self.used
                .generation_work
                .saturating_add(self.used.lookahead_work),
        );
        let remaining_steps = self
            .granted_engine_steps
            .saturating_sub(self.used.engine_steps);
        let generator_work = self.nodes[node_id].generator.counters().generation_work;
        let requested_work = if node_id == 0 && generator_work == 0 {
            self.config.root_initial_expansion_work
        } else if generator_work == 0 {
            self.config.initial_expansion_work.max(requested_work)
        } else {
            requested_work
        };
        let work = requested_work.max(1).min(remaining_work);
        if work == 0 || remaining_steps == 0 {
            return false;
        }
        let remaining_lookahead_evaluations = self
            .config
            .lookahead_max_evaluations
            .saturating_sub(self.used.atomic_lookahead_evaluations);
        let remaining_lookahead_work = remaining_work.saturating_sub(work);

        let generation_started = Instant::now();
        let (
            before,
            after,
            before_lookahead_evaluations,
            after_lookahead_evaluations,
            before_lookahead_work,
            after_lookahead_work,
            before_diagnostics,
            after_diagnostics,
            before_timing,
            after_timing,
            options,
            new_gaps,
        ) = {
            let node = &mut self.nodes[node_id];
            node.generator.prefer_lane(match view {
                LocalServiceView::Anchor => TurnOptionGeneratorPreferredLane::Anchor,
                LocalServiceView::Guide(lane) => TurnOptionGeneratorPreferredLane::Guide(lane),
                LocalServiceView::LookaheadEvaluation => {
                    unreachable!("lookahead evaluation never widens a turn generator")
                }
            });
            let before = node.generator.counters();
            let before_lookahead_evaluations = node.generator.lookahead_evaluations();
            let before_lookahead_work = node.generator.lookahead_work();
            let before_diagnostics = node.generator.diagnostics();
            let before_timing = node.generator.timing();
            node.generator.advance_with_lookahead(
                stepper,
                CombatPlanningQuantum {
                    additional_generation_work: work,
                    additional_engine_steps: remaining_steps.min(work.saturating_mul(
                        self.config.generator.max_engine_steps_per_transition.max(1),
                    )),
                    deadline,
                },
                remaining_lookahead_evaluations,
                remaining_lookahead_work,
                self.config.lookahead_work_per_evaluation,
            );
            let after = node.generator.counters();
            let after_lookahead_evaluations = node.generator.lookahead_evaluations();
            let after_lookahead_work = node.generator.lookahead_work();
            for lane in node.generator.retained_guide_lanes() {
                let view = LocalServiceView::Guide(lane);
                if !node.generation_service_views.contains(&view) {
                    node.generation_service_views.push(view);
                }
            }
            let after_diagnostics = node.generator.diagnostics();
            let after_timing = node.generator.timing();
            let options = node.generator.take_completed_options();
            let gaps = node.generator.gaps()[node.synced_gaps..].to_vec();
            node.synced_gaps = node.generator.gaps().len();
            (
                before,
                after,
                before_lookahead_evaluations,
                after_lookahead_evaluations,
                before_lookahead_work,
                after_lookahead_work,
                before_diagnostics,
                after_diagnostics,
                before_timing,
                after_timing,
                options,
                gaps,
            )
        };
        self.performance_timing.generation_elapsed_ns = self
            .performance_timing
            .generation_elapsed_ns
            .saturating_add(elapsed_nanos_u64(generation_started));

        let used_work = after.generation_work.saturating_sub(before.generation_work);
        let used_lookahead_evaluations =
            after_lookahead_evaluations.saturating_sub(before_lookahead_evaluations);
        let used_lookahead_work = after_lookahead_work.saturating_sub(before_lookahead_work);
        let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
        if used_work == 0 && used_lookahead_work == 0 && used_steps == 0 {
            return false;
        }
        self.used.generation_work = self.used.generation_work.saturating_add(used_work);
        self.used.lookahead_evaluations = self
            .used
            .lookahead_evaluations
            .saturating_add(used_lookahead_evaluations);
        self.used.lookahead_work = self.used.lookahead_work.saturating_add(used_lookahead_work);
        self.used.atomic_lookahead_evaluations = self
            .used
            .atomic_lookahead_evaluations
            .saturating_add(used_lookahead_evaluations);
        self.used.atomic_lookahead_work = self
            .used
            .atomic_lookahead_work
            .saturating_add(used_lookahead_work);
        self.used.engine_steps = self.used.engine_steps.saturating_add(used_steps);
        self.used.applied_action_transitions = self.used.applied_action_transitions.saturating_add(
            after_diagnostics
                .applied_action_transitions
                .saturating_sub(before_diagnostics.applied_action_transitions),
        );
        self.used.unique_successor_states = self.used.unique_successor_states.saturating_add(
            after_diagnostics
                .unique_successor_states
                .saturating_sub(before_diagnostics.unique_successor_states),
        );
        self.used.duplicate_exact_successors = self.used.duplicate_exact_successors.saturating_add(
            after_diagnostics
                .duplicate_exact_successors
                .saturating_sub(before_diagnostics.duplicate_exact_successors),
        );
        self.performance_timing.atomic_expand_elapsed_ns = self
            .performance_timing
            .atomic_expand_elapsed_ns
            .saturating_add(
                after_timing
                    .atomic_expand_elapsed_ns
                    .saturating_sub(before_timing.atomic_expand_elapsed_ns),
            );
        self.performance_timing.transition_simulation_elapsed_ns = self
            .performance_timing
            .transition_simulation_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_simulation_elapsed_ns
                    .saturating_sub(before_timing.transition_simulation_elapsed_ns),
            );
        self.performance_timing.transition_identity_elapsed_ns = self
            .performance_timing
            .transition_identity_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_identity_elapsed_ns
                    .saturating_sub(before_timing.transition_identity_elapsed_ns),
            );
        self.performance_timing.transition_key_build_elapsed_ns = self
            .performance_timing
            .transition_key_build_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_key_build_elapsed_ns
                    .saturating_sub(before_timing.transition_key_build_elapsed_ns),
            );
        self.performance_timing.transition_key_index_elapsed_ns = self
            .performance_timing
            .transition_key_index_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_key_index_elapsed_ns
                    .saturating_sub(before_timing.transition_key_index_elapsed_ns),
            );
        self.performance_timing.transition_admission_elapsed_ns = self
            .performance_timing
            .transition_admission_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_admission_elapsed_ns
                    .saturating_sub(before_timing.transition_admission_elapsed_ns),
            );
        self.performance_timing.transition_trace_elapsed_ns = self
            .performance_timing
            .transition_trace_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_trace_elapsed_ns
                    .saturating_sub(before_timing.transition_trace_elapsed_ns),
            );
        self.performance_timing.transition_seen_elapsed_ns = self
            .performance_timing
            .transition_seen_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_seen_elapsed_ns
                    .saturating_sub(before_timing.transition_seen_elapsed_ns),
            );
        self.performance_timing.transition_publish_elapsed_ns = self
            .performance_timing
            .transition_publish_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_elapsed_ns),
            );
        self.performance_timing
            .transition_publish_trace_node_elapsed_ns = self
            .performance_timing
            .transition_publish_trace_node_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_trace_node_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_trace_node_elapsed_ns),
            );
        self.performance_timing
            .transition_publish_boundary_elapsed_ns = self
            .performance_timing
            .transition_publish_boundary_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_boundary_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_boundary_elapsed_ns),
            );
        self.performance_timing
            .transition_publish_complete_elapsed_ns = self
            .performance_timing
            .transition_publish_complete_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_complete_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_complete_elapsed_ns),
            );
        self.performance_timing.transition_publish_push_elapsed_ns = self
            .performance_timing
            .transition_publish_push_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_push_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_push_elapsed_ns),
            );
        self.performance_timing.transition_publish_guide_elapsed_ns = self
            .performance_timing
            .transition_publish_guide_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_guide_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_guide_elapsed_ns),
            );
        self.performance_timing.transition_publish_retain_elapsed_ns = self
            .performance_timing
            .transition_publish_retain_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_retain_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_retain_elapsed_ns),
            );
        self.performance_timing.transition_publish_agenda_elapsed_ns = self
            .performance_timing
            .transition_publish_agenda_elapsed_ns
            .saturating_add(
                after_timing
                    .transition_publish_agenda_elapsed_ns
                    .saturating_sub(before_timing.transition_publish_agenda_elapsed_ns),
            );
        self.generation_gaps.extend(new_gaps);

        let admission_started = Instant::now();
        for option in options {
            let root_option_started = Instant::now();
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            self.performance_timing.admission_root_option_elapsed_ns = self
                .performance_timing
                .admission_root_option_elapsed_ns
                .saturating_add(elapsed_nanos_u64(root_option_started));
            match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin => {
                    self.used.terminal_win_options =
                        self.used.terminal_win_options.saturating_add(1);
                    let witness_filter_started = Instant::now();
                    let (mut actions, prefix_negative_log_policy) = self.path_actions(path);
                    actions.extend_from_slice(option.actions());
                    let negative_log_policy =
                        prefix_negative_log_policy + option.negative_log_policy();
                    let candidate_is_dominated = self.witness.as_ref().is_some_and(|current| {
                        !terminal_candidate_could_improve_witness(
                            current,
                            option.exact_successor().combat.entities.player.current_hp,
                            actions.len(),
                            negative_log_policy,
                            actions_potion_expenditures(&actions),
                            self.config.max_potions_used,
                        )
                    });
                    self.performance_timing.admission_witness_filter_elapsed_ns = self
                        .performance_timing
                        .admission_witness_filter_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(witness_filter_started));
                    if candidate_is_dominated {
                        self.used.witness_replay_dominated_skips =
                            self.used.witness_replay_dominated_skips.saturating_add(1);
                        continue;
                    }
                    self.used.witness_replay_attempts =
                        self.used.witness_replay_attempts.saturating_add(1);
                    let witness_replay_started = Instant::now();
                    let improved = match replay_witness(
                        &self.original_root,
                        &actions,
                        negative_log_policy,
                        OracleCombatWitnessDiscoverySource::PlannerSearch,
                        stepper,
                    ) {
                        Ok(witness) => self.remember_witness(witness),
                        Err(error) => {
                            self.replay_failure = Some(error);
                            false
                        }
                    };
                    if improved {
                        self.used.witness_replay_improvements =
                            self.used.witness_replay_improvements.saturating_add(1);
                    }
                    self.performance_timing.admission_witness_replay_elapsed_ns = self
                        .performance_timing
                        .admission_witness_replay_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(witness_replay_started));
                    if self.witness_satisfies() {
                        self.performance_timing.admission_elapsed_ns = self
                            .performance_timing
                            .admission_elapsed_ns
                            .saturating_add(elapsed_nanos_u64(admission_started));
                        return true;
                    }
                }
                CompleteTurnOptionBoundary::TerminalLoss => {
                    self.used.terminal_losses = self.used.terminal_losses.saturating_add(1);
                }
                CompleteTurnOptionBoundary::Escape => {}
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    let _ = self.accept_successor(node_id, path, option);
                }
            }
        }
        let refresh_started = Instant::now();
        self.refresh_exhaustion(node_id);
        self.performance_timing.admission_refresh_elapsed_ns = self
            .performance_timing
            .admission_refresh_elapsed_ns
            .saturating_add(elapsed_nanos_u64(refresh_started));
        self.performance_timing.admission_elapsed_ns = self
            .performance_timing
            .admission_elapsed_ns
            .saturating_add(elapsed_nanos_u64(admission_started));
        true
    }

    fn accept_successor(
        &mut self,
        parent_id: usize,
        path: &[(usize, usize)],
        option: CompleteTurnOption,
    ) -> Option<usize> {
        let relative_turn_depth = self.nodes[parent_id].relative_turn_depth.saturating_add(1);
        if relative_turn_depth > self.config.max_turn_depth {
            self.used.depth_limited_successors =
                self.used.depth_limited_successors.saturating_add(1);
            return None;
        }

        let successor_identity_started = Instant::now();
        let successor_identity = option.exact_successor_identity().clone();
        let successor_exact_key = successor_identity.exact_key().cloned().unwrap_or_else(|| {
            Arc::new(combat_exact_state_key(
                &option.exact_successor().engine,
                &option.exact_successor().combat,
            ))
        });
        let successor_potion_expenditures = self.nodes[parent_id]
            .potion_expenditures
            .saturating_add(actions_potion_expenditures(option.actions()));
        self.performance_timing.successor_identity_elapsed_ns = self
            .performance_timing
            .successor_identity_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_identity_started));
        if self
            .config
            .max_potions_used
            .is_some_and(|limit| successor_potion_expenditures > limit)
        {
            return None;
        }
        let constrained_successor_key = ConstrainedExactStateKey::new(
            successor_exact_key,
            self.config.max_potions_used,
            successor_potion_expenditures,
        );
        let successor_lookup_started = Instant::now();
        let existing = self
            .nodes_by_exact_key
            .get(&constrained_successor_key)
            .copied();
        self.performance_timing.successor_lookup_elapsed_ns = self
            .performance_timing
            .successor_lookup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_lookup_started));
        let successor = if let Some(existing) = existing {
            existing
        } else {
            let successor_node_build_started = Instant::now();
            let Ok(root) = CombatDecisionRoot::with_exact_state_identity(
                option.exact_successor().clone(),
                successor_identity,
            ) else {
                self.performance_timing.successor_node_build_elapsed_ns = self
                    .performance_timing
                    .successor_node_build_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(successor_node_build_started));
                return None;
            };
            let (guides, lookahead_pending_lane) = guides_with_pending_lookahead(
                self.policy.as_ref(),
                self.lookahead_evaluator.as_deref(),
                root.position(),
            );
            let backed_guides = guide_rank_map(&guides);
            let boundary_service_views =
                boundary_service_views_from_guides(&guides, lookahead_pending_lane);
            let lookahead_acquisition_views =
                lookahead_acquisition_views_from_guides(&guides, lookahead_pending_lane);
            let node_id = self.nodes.len();
            let generator = turn_generator_for_potion_budget(
                root.clone(),
                self.config.generator,
                self.policy.clone(),
                self.config.max_potions_used,
                successor_potion_expenditures,
            );
            let generation_service_views =
                generation_service_views_from_lanes(generator.retained_guide_lanes());
            self.nodes.push(GraphNode {
                generator,
                potion_expenditures: successor_potion_expenditures,
                diagnostic_parent: Some((parent_id, self.nodes[parent_id].children.len())),
                relative_turn_depth,
                visits: 0,
                generated_options: 0,
                children: Vec::new(),
                guides,
                boundary_service_views,
                next_boundary_service_view: 0,
                lookahead_acquisition_views,
                next_lookahead_acquisition_view: 0,
                generation_service_views,
                next_generation_service_view: 0,
                widen_anchor_visits: 0,
                widen_guide_visits: BTreeMap::new(),
                lookahead_pending_lane,
                backed_guides,
                backed_lookahead_rank: None,
                synced_gaps: 0,
                exhausted: false,
            });
            self.nodes_by_exact_key
                .insert(constrained_successor_key, node_id);
            self.used.exact_nodes = self.nodes.len();
            self.used.maximum_turn_depth = self.used.maximum_turn_depth.max(relative_turn_depth);
            self.performance_timing.successor_node_build_elapsed_ns = self
                .performance_timing
                .successor_node_build_elapsed_ns
                .saturating_add(elapsed_nanos_u64(successor_node_build_started));
            node_id
        };

        let successor_edge_started = Instant::now();
        let successor_lanes = self.nodes[successor]
            .guides
            .iter()
            .map(|guide| guide.lane)
            .collect::<BTreeSet<_>>();
        let successor_backed_guides = self.nodes[successor].backed_guides.clone();
        let successor_backed_rank = self.nodes[successor].backed_lookahead_rank.clone();
        let existing_edge_index = self.nodes[parent_id]
            .children
            .iter()
            .position(|edge| edge.successor == successor);
        let edge_index = if let Some(edge_index) = existing_edge_index {
            self.used.duplicate_successor_edges =
                self.used.duplicate_successor_edges.saturating_add(1);
            let edge = &mut self.nodes[parent_id].children[edge_index];
            if option
                .negative_log_policy()
                .total_cmp(&edge.negative_log_policy)
                .is_lt()
            {
                edge.actions = option.actions().to_vec();
                edge.negative_log_policy = option.negative_log_policy();
            }
            edge_index
        } else {
            let plan_transition_annotation = self
                .collect_plan_transition_annotations
                .then(|| {
                    combat_plan_transition_annotation_v1(
                        self.nodes[parent_id].generator.root().position(),
                        option.exact_successor(),
                    )
                })
                .flatten();
            let parent = &mut self.nodes[parent_id];
            let edge_index = parent.children.len();
            parent.children.push(GraphEdge {
                successor,
                actions: option.actions().to_vec(),
                negative_log_policy: option.negative_log_policy(),
                plan_transition_annotation: plan_transition_annotation.clone(),
                visits: 0,
                anchor_visits: 0,
                guide_visits: BTreeMap::new(),
                backed_guides: successor_backed_guides.clone(),
                backed_lookahead_rank: successor_backed_rank,
                backed_visits: 0,
            });
            for lane in successor_lanes {
                if Some(lane) == self.lookahead_lane {
                    continue;
                }
                let view = LocalServiceView::Guide(lane);
                if !parent.boundary_service_views.contains(&view) {
                    parent.boundary_service_views.push(view);
                }
            }
            parent.exhausted = false;
            self.used.exact_edges = self.used.exact_edges.saturating_add(1);
            if plan_transition_annotation.is_some() {
                self.used.annotated_exact_edges = self.used.annotated_exact_edges.saturating_add(1);
            }
            edge_index
        };
        self.performance_timing.successor_edge_elapsed_ns = self
            .performance_timing
            .successor_edge_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_edge_started));
        let successor_backup_started = Instant::now();
        self.backup_guides_along_path(path, parent_id, edge_index, &successor_backed_guides);
        self.performance_timing.successor_backup_elapsed_ns = self
            .performance_timing
            .successor_backup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_backup_started));
        Some(successor)
    }

    fn backup_guides_along_path(
        &mut self,
        path: &[(usize, usize)],
        parent_id: usize,
        edge_index: usize,
        guides: &BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    ) {
        for (node_id, selected_edge) in path
            .iter()
            .copied()
            .chain(std::iter::once((parent_id, edge_index)))
        {
            for (lane, rank) in guides {
                update_max_guide(
                    &mut self.nodes[node_id].children[selected_edge].backed_guides,
                    *lane,
                    rank,
                );
                update_max_guide(&mut self.nodes[node_id].backed_guides, *lane, rank);
            }
        }
    }

    fn path_actions(&self, path: &[(usize, usize)]) -> (Vec<TurnOptionAction>, f64) {
        let action_count = path
            .iter()
            .map(|(node_id, edge_index)| self.nodes[*node_id].children[*edge_index].actions.len())
            .sum();
        let mut actions = Vec::with_capacity(action_count);
        let mut negative_log_policy = 0.0;
        for (node_id, edge_index) in path {
            let edge = &self.nodes[*node_id].children[*edge_index];
            actions.extend_from_slice(&edge.actions);
            negative_log_policy += edge.negative_log_policy;
        }
        (actions, negative_log_policy)
    }

    fn refresh_exhaustion(&mut self, node_id: usize) {
        if self.nodes[node_id].exhausted || !self.nodes[node_id].generator.is_finished() {
            return;
        }
        let all_children_exhausted = self.nodes[node_id]
            .children
            .iter()
            .all(|edge| self.nodes[edge.successor].exhausted);
        if all_children_exhausted {
            self.nodes[node_id].exhausted = true;
            self.used.exhausted_nodes = self.used.exhausted_nodes.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests;
