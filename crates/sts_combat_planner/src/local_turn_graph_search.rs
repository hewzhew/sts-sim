mod admission;
mod config;
mod diagnostics;
mod policy_line;
mod potion_budget;
mod reporting;
mod scheduling;
mod session;
mod shared_agenda;

pub use config::LocalTurnGraphWitnessConfig;
use potion_budget::*;
pub use reporting::*;
use scheduling::*;
use shared_agenda::SharedBoundaryAgenda;

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use rustc_hash::FxBuildHasher;
use smallvec::SmallVec;
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
use super::witness::{
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

/// Guide policies currently expose only a handful of fixed semantic lanes.
/// A tree map made every exact node and edge allocate a separate tree node,
/// while all lookups still touched only this tiny set. Keep the same exact
/// lane/rank association in a contiguous inline table; lane order has no
/// search authority and updates remain exact.
#[derive(Clone, Default)]
struct GuideRankMap(SmallVec<[(CombatGuideLaneId, CombatStateGuideRank); 4]>);

impl GuideRankMap {
    fn from_guides(guides: &[CombatStateGuide]) -> Self {
        let mut ranks = Self::default();
        for guide in guides {
            ranks.update_max(guide.lane, &guide.rank);
        }
        ranks
    }

    fn get(&self, lane: &CombatGuideLaneId) -> Option<&CombatStateGuideRank> {
        self.0
            .iter()
            .find_map(|(candidate, rank)| (candidate == lane).then_some(rank))
    }

    fn update_max(&mut self, lane: CombatGuideLaneId, candidate: &CombatStateGuideRank) -> bool {
        if let Some((_, existing)) = self
            .0
            .iter_mut()
            .find(|(candidate_lane, _)| *candidate_lane == lane)
        {
            if *existing >= *candidate {
                return false;
            }
            *existing = candidate.clone();
            return true;
        }
        self.0.push((lane, candidate.clone()));
        true
    }

    fn iter(&self) -> impl Iterator<Item = &(CombatGuideLaneId, CombatStateGuideRank)> {
        self.0.iter()
    }
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
    /// Retained exact path identity used by the shared boundary agendas.
    /// Selection views rank this shared node; they do not recursively own the
    /// path below it.
    path_negative_log_policy: f64,
    path_atomic_depth: usize,
    relative_turn_depth: usize,
    visits: usize,
    generated_options: usize,
    children: Vec<GraphEdge>,
    guides: Vec<CombatStateGuide>,
    generation_service_views: Vec<LocalServiceView>,
    next_generation_service_view: usize,
    widen_anchor_visits: usize,
    widen_guide_visits: BTreeMap<CombatGuideLaneId, usize>,
    lookahead_pending_lane: Option<CombatGuideLaneId>,
    /// Best exact descendant observed for each cheap semantic guide.
    backed_guides: GuideRankMap,
    /// Best bounded rollout value observed at this exact boundary. This is
    /// search guidance only; terminal authority still belongs to exact replay.
    backed_lookahead_rank: Option<CombatStateGuideRank>,
    synced_gaps: usize,
    exhausted: bool,
}

impl GraphNode {
    fn path_cost(&self) -> f64 {
        local_path_base(self.path_atomic_depth, self.path_negative_log_policy)
    }
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
    backed_guides: GuideRankMap,
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
    shared_agenda: SharedBoundaryAgenda,
    nodes: Vec<GraphNode>,
    nodes_by_exact_key: HashMap<ConstrainedExactStateKey, usize, FxBuildHasher>,
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
        let attempts = self.shared_agenda.view_count().max(1);
        for _ in 0..attempts {
            let service_view = self.shared_agenda.next_service_view();
            if service_view == LocalServiceView::LookaheadEvaluation {
                if self.used.boundary_lookahead_evaluations < self.config.lookahead_max_evaluations
                {
                    if let Some(node_id) = self.shared_agenda.select_pending_lookahead(&self.nodes)
                    {
                        let path = self.retained_path_to(node_id);
                        self.record_shared_service(node_id, &path, service_view);
                        return SelectedWork::Evaluate { node_id, path };
                    }
                }
                continue;
            }

            let selected = match service_view {
                LocalServiceView::Anchor => self.shared_agenda.select_anchor(&self.nodes),
                LocalServiceView::Guide(lane) => self.shared_agenda.select_guide(lane, &self.nodes),
                LocalServiceView::LookaheadEvaluation => unreachable!(),
            };
            let Some(node_id) = selected else {
                continue;
            };
            let path = self.retained_path_to(node_id);
            self.record_shared_service(node_id, &path, service_view);

            // The agenda view has completed its only job: selecting one
            // shared exact boundary. Expansion belongs to the selected node,
            // whose private generator rotates its own independent lanes.
            let generation_view = {
                let node = &mut self.nodes[node_id];
                let view = node.generation_service_views
                    [node.next_generation_service_view % node.generation_service_views.len()];
                node.next_generation_service_view =
                    node.next_generation_service_view.saturating_add(1);
                view
            };
            if service_view == LocalServiceView::Anchor {
                self.shared_agenda
                    .republish_anchor(node_id, &self.nodes[node_id]);
            }
            let generator_work = self.nodes[node_id].generator.counters().generation_work;
            return SelectedWork::Widen {
                node_id,
                path,
                view: generation_view,
                requested_work: if generator_work == 0 {
                    if node_id == 0 {
                        self.config.root_initial_expansion_work
                    } else {
                        self.config.initial_expansion_work
                    }
                } else {
                    match service_view {
                        LocalServiceView::Guide(_) => self.config.backed_generation_quantum_work,
                        LocalServiceView::Anchor => self.config.generation_quantum_work,
                        LocalServiceView::LookaheadEvaluation => unreachable!(),
                    }
                },
            };
        }

        // A guide can be temporarily empty while the anchor still owns live
        // work. Give the anchor one explicit fallback before declaring the
        // shared graph exhausted.
        if let Some(node_id) = self.shared_agenda.select_anchor(&self.nodes) {
            let path = self.retained_path_to(node_id);
            self.record_shared_service(node_id, &path, LocalServiceView::Anchor);
            let generation_view = {
                let node = &mut self.nodes[node_id];
                let view = node.generation_service_views
                    [node.next_generation_service_view % node.generation_service_views.len()];
                node.next_generation_service_view =
                    node.next_generation_service_view.saturating_add(1);
                view
            };
            self.shared_agenda
                .republish_anchor(node_id, &self.nodes[node_id]);
            return SelectedWork::Widen {
                node_id,
                path,
                view: generation_view,
                requested_work: self.config.generation_quantum_work,
            };
        }
        SelectedWork::Exhausted
    }

    fn retained_path_to(&self, mut node_id: usize) -> Vec<(usize, usize)> {
        let mut reversed = Vec::with_capacity(self.nodes[node_id].relative_turn_depth);
        while let Some((parent_id, edge_index)) = self.nodes[node_id].diagnostic_parent {
            reversed.push((parent_id, edge_index));
            node_id = parent_id;
        }
        reversed.reverse();
        reversed
    }

    fn record_shared_service(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        view: LocalServiceView,
    ) {
        self.nodes[node_id].visits = self.nodes[node_id].visits.saturating_add(1);
        self.used.node_visits = self.used.node_visits.saturating_add(1);
        match view {
            LocalServiceView::Anchor => {
                self.nodes[node_id].widen_anchor_visits =
                    self.nodes[node_id].widen_anchor_visits.saturating_add(1);
            }
            LocalServiceView::Guide(lane) => {
                let visits = self.nodes[node_id]
                    .widen_guide_visits
                    .entry(lane)
                    .or_default();
                *visits = visits.saturating_add(1);
            }
            LocalServiceView::LookaheadEvaluation => {}
        }
        for (parent_id, edge_index) in path {
            let edge = &mut self.nodes[*parent_id].children[*edge_index];
            edge.visits = edge.visits.saturating_add(1);
            match view {
                LocalServiceView::Anchor => {
                    edge.anchor_visits = edge.anchor_visits.saturating_add(1);
                }
                LocalServiceView::Guide(lane) => {
                    let visits = edge.guide_visits.entry(lane).or_default();
                    *visits = visits.saturating_add(1);
                }
                LocalServiceView::LookaheadEvaluation => {}
            }
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
        // The evaluated lane was deliberately absent while pending. Publish
        // only that newly grounded view: reinserting every guide here would
        // silently grant already-serviced one-shot views another expansion.
        self.shared_agenda
            .publish_guide_entry(node_id, &self.nodes[node_id], expected_lane);
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
        if self.nodes[node_id].generator.is_finished() || self.nodes[node_id].exhausted {
            self.shared_agenda.remove_guide_entries(
                node_id,
                &self.nodes[node_id],
                self.lookahead_lane,
            );
        }
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
