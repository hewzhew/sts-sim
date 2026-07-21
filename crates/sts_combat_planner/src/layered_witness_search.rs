use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};

use super::generator::{RetainedGuidePromise, TurnOptionGeneratorPreferredLane};
use super::policy::{CombatGuideLaneId, CombatStateGuideRank, SharedCombatActionPolicy};
use super::types::{
    exact_hash, CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOptionBoundary,
    TurnOptionAction, TurnOptionGenerationGap, TurnOptionGeneratorConfig,
};
use super::witness_search::{
    OracleCombatWitness, OracleCombatWitnessDiscoverySource, OracleCombatWitnessReplayError,
};
use super::TurnOptionGeneratorSession;

/// Lab-only control that keeps tactical generation and strategic depth on
/// separate clocks. Every retained state in one player-turn layer receives a
/// bounded complete-turn generation allowance before any successor is
/// expanded into the following player turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayeredCombatWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    pub beam_width: usize,
    pub retained_per_view: usize,
    /// Do not close a turn layer before this much shared generator work has
    /// been available, even if a shallow candidate pool appears full.
    pub minimum_generation_work_per_layer: usize,
    /// Hard ceiling for one layer. This is independent of retained parent
    /// count so a wide beam cannot multiply the tactical budget by itself.
    pub maximum_generation_work_per_layer: usize,
    /// A layer may close after the minimum allowance once its exact candidate
    /// pool reaches this multiple of the retained beam width.
    pub candidate_pool_multiplier: usize,
    pub generation_quantum_work: usize,
    pub max_turn_layers: usize,
}

impl Default for LayeredCombatWitnessConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            beam_width: 32,
            retained_per_view: 6,
            minimum_generation_work_per_layer: 640,
            maximum_generation_work_per_layer: 8_192,
            candidate_pool_multiplier: 8,
            generation_quantum_work: 8,
            max_turn_layers: 32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayeredCombatWitnessBudget {
    pub max_generation_work: usize,
    pub max_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug)]
pub struct LayeredCombatWitnessQuantum {
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayeredCombatWitnessInterruption {
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
    TurnLayerBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayeredCombatWitnessStatus {
    WitnessFound,
    Partial(LayeredCombatWitnessInterruption),
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayeredCombatWitnessCounters {
    pub generation_work: usize,
    pub engine_steps: usize,
    pub expanded_parents: usize,
    pub completed_turn_options: usize,
    pub unique_next_turn_states: usize,
    pub duplicate_next_turn_states: usize,
    pub truncated_parents: usize,
    pub completed_layers: usize,
    pub deferred_windows: usize,
    pub recovered_window_expansions: usize,
    pub maximum_window_discrepancy: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayeredCombatLayerReport {
    pub relative_turn_depth: usize,
    pub window_discrepancy: usize,
    pub source_window_index: usize,
    pub player_turn: u32,
    pub parent_states: usize,
    pub parent_exact_state_hashes: Vec<String>,
    pub parent_work: Vec<LayeredCombatParentWorkReport>,
    pub expanded_parents: usize,
    pub generation_work: usize,
    pub completed_turn_options: usize,
    pub unique_next_turn_states: usize,
    pub duplicate_next_turn_states: usize,
    pub retained_next_turn_states: usize,
    pub retained_exact_state_hashes: Vec<String>,
    pub truncated_parents: usize,
    pub emitted_windows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayeredCombatParentWorkReport {
    pub exact_state_hash: String,
    pub generation_work: usize,
    pub completed_turn_options: usize,
    pub finished: bool,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatFrontierState {
    pub exact_state_hash: String,
    pub position: CombatPosition,
    pub actions: Vec<TurnOptionAction>,
    pub negative_log_policy: f64,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatWitnessReport {
    pub status: LayeredCombatWitnessStatus,
    pub counters: LayeredCombatWitnessCounters,
    pub layers: Vec<LayeredCombatLayerReport>,
    pub frontier: Vec<LayeredCombatFrontierState>,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatDeferredWindow {
    pub relative_turn_depth: usize,
    pub window_discrepancy: usize,
    pub source_window_index: usize,
    pub candidates: Vec<LayeredCombatFrontierState>,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatLineageWindow {
    pub parent_candidate_index: usize,
    pub parent_exact_state_hash: String,
    pub window: LayeredCombatDeferredWindow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayeredCombatLineageParentRank {
    pub parent_candidate_index: usize,
    pub parent_exact_state_hash: String,
    pub consensus_rank: usize,
    pub rank_sum: usize,
    pub anchor_rank: usize,
    pub guide_ranks: Vec<(CombatGuideLaneId, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayeredCombatCandidateRaceConfig {
    pub continuation: LayeredCombatWitnessConfig,
    pub service_quantum_work: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayeredCombatCandidateRaceStatus {
    WitnessFound,
    Partial(LayeredCombatWitnessInterruption),
    CandidatesExhausted,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayeredCombatCandidateRaceCounters {
    pub generation_work: usize,
    pub engine_steps: usize,
    pub services: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayeredCombatCandidateRaceEntryReport {
    pub candidate_index: usize,
    pub exact_state_hash: String,
    pub generation_work: usize,
    pub engine_steps: usize,
    pub completed_layers: usize,
    pub terminal: bool,
    pub found_witness: bool,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatCandidateRaceReport {
    pub status: LayeredCombatCandidateRaceStatus,
    pub counters: LayeredCombatCandidateRaceCounters,
    pub candidates: Vec<LayeredCombatCandidateRaceEntryReport>,
    pub witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayeredCombatLineagePortfolioConfig {
    pub candidate_race: LayeredCombatCandidateRaceConfig,
    pub parents_per_view: usize,
    pub windows_per_parent: usize,
    pub service_quantum_work: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayeredCombatLineagePortfolioStatus {
    WitnessFound,
    Partial(LayeredCombatWitnessInterruption),
    SelectedPortfolioExhausted,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayeredCombatLineagePortfolioCounters {
    pub generation_work: usize,
    pub engine_steps: usize,
    pub services: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayeredCombatLineagePortfolioEntryReport {
    pub parent_candidate_index: usize,
    pub parent_consensus_rank: usize,
    pub source_window_index: usize,
    pub window_discrepancy: usize,
    pub generation_work: usize,
    pub engine_steps: usize,
    pub terminal: bool,
    pub found_witness: bool,
}

#[derive(Clone, Debug)]
pub struct LayeredCombatLineagePortfolioReport {
    pub status: LayeredCombatLineagePortfolioStatus,
    pub counters: LayeredCombatLineagePortfolioCounters,
    pub selected_parent_count: usize,
    pub deferred_parent_count: usize,
    pub deferred_window_count: usize,
    pub entries: Vec<LayeredCombatLineagePortfolioEntryReport>,
    pub witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Debug)]
struct BeamState {
    exact_state_hash: String,
    position: CombatPosition,
    trail: Option<Arc<ActionTrailNode>>,
    atomic_depth: usize,
    negative_log_policy: f64,
}

#[derive(Clone, Debug)]
struct ActionTrailNode {
    parent: Option<Arc<ActionTrailNode>>,
    turn_actions: Arc<[TurnOptionAction]>,
}

struct BeamCohort {
    states: Vec<BeamState>,
    relative_turn_depth: usize,
    window_discrepancy: usize,
    source_window_index: usize,
}

struct LayerWorker {
    parent: BeamState,
    generator: TurnOptionGeneratorSession,
}

struct ActiveLayer {
    relative_turn_depth: usize,
    window_discrepancy: usize,
    source_window_index: usize,
    player_turn: u32,
    parent_states: usize,
    parent_exact_state_hashes: Vec<String>,
    layer_before: LayeredCombatWitnessCounters,
    workers: Vec<LayerWorker>,
    service_views: Vec<LayerServiceView>,
    next_service_view: usize,
    next_by_hash: HashMap<String, BeamState>,
    generation_work: usize,
}

#[derive(Clone, Copy)]
enum LayerServiceView {
    Anchor,
    Guide(CombatGuideLaneId),
}

pub struct LayeredCombatWitnessSession {
    original_root: CombatPosition,
    config: LayeredCombatWitnessConfig,
    policy: SharedCombatActionPolicy,
    agenda: CohortAgenda,
    next_cohort_id: usize,
    depth_limited: Vec<BeamCohort>,
    active_layer: Option<ActiveLayer>,
    counters: LayeredCombatWitnessCounters,
    layers: Vec<LayeredCombatLayerReport>,
    generation_gaps: Vec<TurnOptionGenerationGap>,
    terminal_status: Option<LayeredCombatWitnessStatus>,
    witness: Option<OracleCombatWitness>,
}

struct CandidateRaceEntry {
    candidate_index: usize,
    exact_state_hash: String,
    prefix_actions: Vec<TurnOptionAction>,
    prefix_negative_log_policy: f64,
    session: Box<LayeredCombatWitnessSession>,
    continuation_anchor_cost: Option<f64>,
    continuation_guides: BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    found_witness: bool,
}

pub struct LayeredCombatCandidateRaceSession {
    original_root: CombatPosition,
    config: LayeredCombatCandidateRaceConfig,
    entries: Vec<CandidateRaceEntry>,
    counters: LayeredCombatCandidateRaceCounters,
    next_service_view: usize,
    terminal_status: Option<LayeredCombatCandidateRaceStatus>,
    witness: Option<OracleCombatWitness>,
}

struct LineagePortfolioEntry {
    parent_rank: LayeredCombatLineageParentRank,
    source_window_index: usize,
    window_discrepancy: usize,
    race: Box<LayeredCombatCandidateRaceSession>,
    found_witness: bool,
}

pub struct LayeredCombatLineagePortfolioSession {
    config: LayeredCombatLineagePortfolioConfig,
    entries: Vec<LineagePortfolioEntry>,
    selected_parent_count: usize,
    deferred_parent_count: usize,
    deferred_window_count: usize,
    service_views: Vec<CandidateRaceServiceView>,
    next_service_view: usize,
    counters: LayeredCombatLineagePortfolioCounters,
    terminal_status: Option<LayeredCombatLineagePortfolioStatus>,
    witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Copy)]
enum CandidateRaceServiceView {
    Anchor,
    Guide(CombatGuideLaneId),
}

impl BeamState {
    fn policy_priority(&self) -> f64 {
        self.negative_log_policy + (self.atomic_depth.max(1) as f64).ln()
    }

    fn public_snapshot(&self) -> LayeredCombatFrontierState {
        LayeredCombatFrontierState {
            exact_state_hash: self.exact_state_hash.clone(),
            position: self.position.clone(),
            actions: flatten_action_trail(self.trail.as_ref()),
            negative_log_policy: self.negative_log_policy,
        }
    }
}

/// Runs the independent, turn-synchronous control. This intentionally does
/// not share the production witness agenda or its legacy suffix donor.
pub fn search_layered_combat_witness(
    root: CombatDecisionRoot,
    config: LayeredCombatWitnessConfig,
    budget: LayeredCombatWitnessBudget,
    policy: SharedCombatActionPolicy,
    stepper: &dyn CombatStepper,
) -> LayeredCombatWitnessReport {
    let mut session = LayeredCombatWitnessSession::with_policy(root, config, policy);
    session.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: budget.max_generation_work,
            additional_engine_steps: budget.max_engine_steps,
            deadline: budget.deadline,
        },
        stepper,
    )
}

impl LayeredCombatWitnessSession {
    pub fn with_policy(
        root: CombatDecisionRoot,
        config: LayeredCombatWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let original_root = root.position().clone();
        let root_state = BeamState {
            exact_state_hash: root.exact_state_hash().to_owned(),
            position: root.position().clone(),
            trail: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
        };
        let mut agenda = BTreeMap::new();
        let mut next_cohort_id = 0usize;
        enqueue_cohort(
            &mut agenda,
            BeamCohort {
                states: vec![root_state],
                relative_turn_depth: 0,
                window_discrepancy: 0,
                source_window_index: 0,
            },
            &mut next_cohort_id,
        );
        Self {
            original_root,
            config,
            policy,
            agenda,
            next_cohort_id,
            depth_limited: Vec::new(),
            active_layer: None,
            counters: LayeredCombatWitnessCounters::default(),
            layers: Vec::new(),
            generation_gaps: Vec::new(),
            terminal_status: None,
            witness: None,
        }
    }

    pub fn counters(&self) -> &LayeredCombatWitnessCounters {
        &self.counters
    }

    pub fn completed_layers(&self) -> usize {
        self.counters.completed_layers
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal_status.is_some()
    }

    pub fn deferred_windows(&self) -> Vec<LayeredCombatDeferredWindow> {
        let mut windows = self
            .agenda
            .values()
            .chain(self.depth_limited.iter())
            .map(|cohort| LayeredCombatDeferredWindow {
                relative_turn_depth: cohort.relative_turn_depth,
                window_discrepancy: cohort.window_discrepancy,
                source_window_index: cohort.source_window_index,
                candidates: cohort
                    .states
                    .iter()
                    .map(BeamState::public_snapshot)
                    .collect(),
            })
            .collect::<Vec<_>>();
        windows.sort_by_key(|window| {
            (
                window.relative_turn_depth,
                window.window_discrepancy,
                window.source_window_index,
            )
        });
        windows
    }

    fn visit_frontier_states(&self, mut visit: impl FnMut(&BeamState)) {
        for cohort in self.agenda.values().chain(self.depth_limited.iter()) {
            for state in &cohort.states {
                visit(state);
            }
        }
        if let Some(active) = self.active_layer.as_ref() {
            for worker in &active.workers {
                visit(&worker.parent);
            }
        }
    }

    pub fn advance(
        &mut self,
        quantum: LayeredCombatWitnessQuantum,
        stepper: &dyn CombatStepper,
    ) -> LayeredCombatWitnessReport {
        if let Some(status) = self.terminal_status.clone() {
            return self.snapshot(status);
        }
        let work_limit = self
            .counters
            .generation_work
            .saturating_add(quantum.additional_generation_work);
        let engine_step_limit = self
            .counters
            .engine_steps
            .saturating_add(quantum.additional_engine_steps);

        loop {
            if deadline_reached(quantum.deadline) {
                return self.snapshot(LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::Deadline,
                ));
            }
            if self.counters.generation_work >= work_limit {
                return self.snapshot(LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::GenerationWorkBudget,
                ));
            }
            if self.counters.engine_steps >= engine_step_limit {
                return self.snapshot(LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                ));
            }

            if self.active_layer.is_none() && !self.start_next_layer() {
                let status = if !self.depth_limited.is_empty() {
                    LayeredCombatWitnessStatus::Partial(
                        LayeredCombatWitnessInterruption::TurnLayerBudget,
                    )
                } else if self.generation_gaps.is_empty() {
                    LayeredCombatWitnessStatus::FrontierExhausted
                } else {
                    LayeredCombatWitnessStatus::MechanicsGap
                };
                self.terminal_status = Some(status.clone());
                return self.snapshot(status);
            }

            if self.active_layer_is_complete() {
                self.finish_active_layer();
                continue;
            }

            let remaining_work = work_limit.saturating_sub(self.counters.generation_work);
            let remaining_steps = engine_step_limit.saturating_sub(self.counters.engine_steps);
            if !self.service_active_layer(
                remaining_work,
                remaining_steps,
                quantum.deadline,
                stepper,
            ) {
                self.finish_active_layer();
                continue;
            }
            if let Some(status) = self.terminal_status.clone() {
                return self.snapshot(status);
            }
        }
    }

    fn start_next_layer(&mut self) -> bool {
        loop {
            let Some((_, cohort)) = self.agenda.pop_first() else {
                return false;
            };
            if cohort.relative_turn_depth >= self.config.max_turn_layers {
                self.depth_limited.push(cohort);
                continue;
            }
            if cohort.source_window_index > 0 {
                self.counters.recovered_window_expansions =
                    self.counters.recovered_window_expansions.saturating_add(1);
            }

            let player_turn = cohort
                .states
                .first()
                .map(|state| state.position.combat.turn.turn_count)
                .unwrap_or_default();
            debug_assert!(cohort
                .states
                .iter()
                .all(|state| state.position.combat.turn.turn_count == player_turn));
            let parent_exact_state_hashes = cohort
                .states
                .iter()
                .map(|state| state.exact_state_hash.clone())
                .collect::<Vec<_>>();
            let mut guide_lanes = BTreeSet::new();
            let workers = cohort
                .states
                .iter()
                .filter_map(|parent| {
                    let parent_root = CombatDecisionRoot::new(parent.position.clone()).ok()?;
                    let generator = TurnOptionGeneratorSession::with_policy(
                        parent_root,
                        self.config.generator,
                        self.policy.clone(),
                    );
                    for guide in self.policy.turn_generation_guides(&parent.position) {
                        guide_lanes.insert(guide.lane);
                    }
                    Some(LayerWorker {
                        parent: parent.clone(),
                        generator,
                    })
                })
                .collect::<Vec<_>>();
            let mut service_views = vec![LayerServiceView::Anchor];
            service_views.extend(guide_lanes.into_iter().map(LayerServiceView::Guide));
            self.active_layer = Some(ActiveLayer {
                relative_turn_depth: cohort.relative_turn_depth,
                window_discrepancy: cohort.window_discrepancy,
                source_window_index: cohort.source_window_index,
                player_turn,
                parent_states: cohort.states.len(),
                parent_exact_state_hashes,
                layer_before: self.counters.clone(),
                workers,
                service_views,
                next_service_view: 0,
                next_by_hash: HashMap::new(),
                generation_work: 0,
            });
            return true;
        }
    }

    fn active_layer_is_complete(&self) -> bool {
        let Some(active) = self.active_layer.as_ref() else {
            return false;
        };
        let candidate_pool_target = self
            .config
            .beam_width
            .max(1)
            .saturating_mul(self.config.candidate_pool_multiplier.max(1));
        active.generation_work >= self.config.maximum_generation_work_per_layer.max(1)
            || (active.generation_work >= self.config.minimum_generation_work_per_layer
                && active.next_by_hash.len() >= candidate_pool_target)
            || active
                .workers
                .iter()
                .all(|worker| worker.generator.is_finished())
    }

    fn service_active_layer(
        &mut self,
        remaining_work: usize,
        remaining_steps: usize,
        deadline: Option<Instant>,
        stepper: &dyn CombatStepper,
    ) -> bool {
        let Some(active) = self.active_layer.as_mut() else {
            return false;
        };
        if active.workers.is_empty() || active.service_views.is_empty() {
            return false;
        }
        let requested_view =
            active.service_views[active.next_service_view % active.service_views.len()];
        active.next_service_view = active.next_service_view.saturating_add(1);
        let Some((worker_index, actual_view)) =
            select_layer_worker(&active.workers, requested_view)
                .or_else(|| select_layer_worker(&active.workers, LayerServiceView::Anchor))
        else {
            return false;
        };
        let worker = &mut active.workers[worker_index];
        worker.generator.prefer_lane(match actual_view {
            LayerServiceView::Anchor => TurnOptionGeneratorPreferredLane::Anchor,
            LayerServiceView::Guide(lane) => TurnOptionGeneratorPreferredLane::Guide(lane),
        });
        let layer_remaining = self
            .config
            .maximum_generation_work_per_layer
            .max(1)
            .saturating_sub(active.generation_work);
        let work = self
            .config
            .generation_quantum_work
            .max(1)
            .min(layer_remaining)
            .min(remaining_work);
        if work == 0 || remaining_steps == 0 {
            return false;
        }
        let before = worker.generator.counters();
        worker.generator.advance(
            stepper,
            CombatPlanningQuantum {
                additional_generation_work: work,
                additional_engine_steps: remaining_steps.min(
                    work.saturating_mul(
                        self.config.generator.max_engine_steps_per_transition.max(1),
                    ),
                ),
                deadline,
            },
        );
        let after = worker.generator.counters();
        let used_work = after.generation_work.saturating_sub(before.generation_work);
        let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
        if used_work == 0 && used_steps == 0 {
            return false;
        }
        active.generation_work = active.generation_work.saturating_add(used_work);
        self.counters.generation_work = self.counters.generation_work.saturating_add(used_work);
        self.counters.engine_steps = self.counters.engine_steps.saturating_add(used_steps);

        let options = worker.generator.take_completed_options();
        self.counters.completed_turn_options = self
            .counters
            .completed_turn_options
            .saturating_add(options.len());
        for option in options {
            let trail = Some(Arc::new(ActionTrailNode {
                parent: worker.parent.trail.clone(),
                turn_actions: Arc::from(option.actions().to_vec()),
            }));
            let atomic_depth = worker
                .parent
                .atomic_depth
                .saturating_add(option.actions().len());
            let negative_log_policy =
                worker.parent.negative_log_policy + option.negative_log_policy();
            match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin => {
                    let actions = flatten_action_trail(trail.as_ref());
                    match replay_witness(
                        &self.original_root,
                        &actions,
                        negative_log_policy,
                        stepper,
                    ) {
                        Ok(witness) => {
                            self.witness = Some(witness);
                            self.terminal_status = Some(LayeredCombatWitnessStatus::WitnessFound);
                        }
                        Err(error) => {
                            self.terminal_status =
                                Some(LayeredCombatWitnessStatus::ReplayMismatch(error));
                        }
                    }
                    break;
                }
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    let candidate = BeamState {
                        exact_state_hash: option.exact_successor_hash().to_owned(),
                        position: option.exact_successor().clone(),
                        trail,
                        atomic_depth,
                        negative_log_policy,
                    };
                    match active
                        .next_by_hash
                        .entry(candidate.exact_state_hash.clone())
                    {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(candidate);
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            self.counters.duplicate_next_turn_states =
                                self.counters.duplicate_next_turn_states.saturating_add(1);
                            if path_is_better(&candidate, entry.get()) {
                                entry.insert(candidate);
                            }
                        }
                    }
                }
                CompleteTurnOptionBoundary::TerminalLoss | CompleteTurnOptionBoundary::Escape => {}
            }
        }
        true
    }

    fn finish_active_layer(&mut self) {
        let Some(active) = self.active_layer.take() else {
            return;
        };
        let expanded_parents = active
            .workers
            .iter()
            .filter(|worker| worker.generator.counters().generation_work > 0)
            .count();
        let truncated_parents = active
            .workers
            .iter()
            .filter(|worker| !worker.generator.is_finished())
            .count();
        self.counters.expanded_parents = self
            .counters
            .expanded_parents
            .saturating_add(expanded_parents);
        self.counters.truncated_parents = self
            .counters
            .truncated_parents
            .saturating_add(truncated_parents);
        for worker in &active.workers {
            self.generation_gaps
                .extend_from_slice(worker.generator.gaps());
        }
        let parent_work = active
            .workers
            .iter()
            .map(|worker| LayeredCombatParentWorkReport {
                exact_state_hash: worker.parent.exact_state_hash.clone(),
                generation_work: worker.generator.counters().generation_work,
                completed_turn_options: worker.generator.total_completed_options(),
                finished: worker.generator.is_finished(),
            })
            .collect::<Vec<_>>();
        let unique_next = active.next_by_hash.len();
        self.counters.unique_next_turn_states = self
            .counters
            .unique_next_turn_states
            .saturating_add(unique_next);
        let ranked_candidates = rank_multi_view(
            active.next_by_hash.into_values().collect(),
            self.config,
            self.policy.as_ref(),
        );
        let window_width = self.config.beam_width.max(1);
        let emitted_windows = ranked_candidates.len().div_ceil(window_width);
        let retained_exact_state_hashes = ranked_candidates
            .iter()
            .take(window_width)
            .map(|state| state.exact_state_hash.clone())
            .collect::<Vec<_>>();
        let retained_next_turn_states = retained_exact_state_hashes.len();
        for (window_index, states) in ranked_candidates
            .chunks(window_width)
            .map(<[BeamState]>::to_vec)
            .enumerate()
        {
            let window_discrepancy = active.window_discrepancy.saturating_add(window_index);
            self.counters.maximum_window_discrepancy = self
                .counters
                .maximum_window_discrepancy
                .max(window_discrepancy);
            if window_index > 0 {
                self.counters.deferred_windows = self.counters.deferred_windows.saturating_add(1);
            }
            enqueue_cohort(
                &mut self.agenda,
                BeamCohort {
                    states,
                    relative_turn_depth: active.relative_turn_depth.saturating_add(1),
                    window_discrepancy,
                    source_window_index: window_index,
                },
                &mut self.next_cohort_id,
            );
        }
        self.layers.push(LayeredCombatLayerReport {
            relative_turn_depth: active.relative_turn_depth,
            window_discrepancy: active.window_discrepancy,
            source_window_index: active.source_window_index,
            player_turn: active.player_turn,
            parent_states: active.parent_states,
            parent_exact_state_hashes: active.parent_exact_state_hashes,
            parent_work,
            expanded_parents: self
                .counters
                .expanded_parents
                .saturating_sub(active.layer_before.expanded_parents),
            generation_work: self
                .counters
                .generation_work
                .saturating_sub(active.layer_before.generation_work),
            completed_turn_options: self
                .counters
                .completed_turn_options
                .saturating_sub(active.layer_before.completed_turn_options),
            unique_next_turn_states: unique_next,
            duplicate_next_turn_states: self
                .counters
                .duplicate_next_turn_states
                .saturating_sub(active.layer_before.duplicate_next_turn_states),
            retained_next_turn_states,
            retained_exact_state_hashes,
            truncated_parents: self
                .counters
                .truncated_parents
                .saturating_sub(active.layer_before.truncated_parents),
            emitted_windows,
        });
        self.counters.completed_layers = self.counters.completed_layers.saturating_add(1);
    }

    fn snapshot(&self, status: LayeredCombatWitnessStatus) -> LayeredCombatWitnessReport {
        let frontier = self
            .active_layer
            .as_ref()
            .map(|active| {
                active
                    .workers
                    .iter()
                    .map(|worker| worker.parent.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| best_frontier(&self.agenda, &self.depth_limited));
        report(
            status,
            self.counters.clone(),
            self.layers.clone(),
            frontier,
            self.generation_gaps.clone(),
            self.witness.clone(),
        )
    }
}

impl LayeredCombatCandidateRaceSession {
    pub fn from_window(
        original_root: CombatDecisionRoot,
        window: LayeredCombatDeferredWindow,
        config: LayeredCombatCandidateRaceConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let entries = window
            .candidates
            .into_iter()
            .enumerate()
            .filter_map(|(candidate_index, candidate)| {
                let root = CombatDecisionRoot::new(candidate.position).ok()?;
                Some(CandidateRaceEntry {
                    candidate_index,
                    exact_state_hash: candidate.exact_state_hash,
                    prefix_actions: candidate.actions,
                    prefix_negative_log_policy: candidate.negative_log_policy,
                    session: Box::new(LayeredCombatWitnessSession::with_policy(
                        root,
                        config.continuation,
                        policy.clone(),
                    )),
                    continuation_anchor_cost: None,
                    continuation_guides: BTreeMap::new(),
                    found_witness: false,
                })
            })
            .collect::<Vec<_>>();
        let terminal_status = entries
            .is_empty()
            .then_some(LayeredCombatCandidateRaceStatus::CandidatesExhausted);
        Self {
            original_root: original_root.position().clone(),
            config,
            entries,
            counters: LayeredCombatCandidateRaceCounters::default(),
            next_service_view: 0,
            terminal_status,
            witness: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal_status.is_some()
    }

    /// Returns deferred windows without merging candidates from different
    /// parent lineages. Prefix actions and policy cost are rebased to the
    /// race's original root so a later witness remains exactly replayable.
    pub fn deferred_lineage_windows(&self) -> Vec<LayeredCombatLineageWindow> {
        self.entries
            .iter()
            .flat_map(|entry| {
                entry.session.deferred_windows().into_iter().map(|window| {
                    let candidates = window
                        .candidates
                        .into_iter()
                        .map(|candidate| {
                            let mut actions = entry.prefix_actions.clone();
                            actions.extend(candidate.actions);
                            LayeredCombatFrontierState {
                                exact_state_hash: candidate.exact_state_hash,
                                position: candidate.position,
                                actions,
                                negative_log_policy: entry.prefix_negative_log_policy
                                    + candidate.negative_log_policy,
                            }
                        })
                        .collect();
                    LayeredCombatLineageWindow {
                        parent_candidate_index: entry.candidate_index,
                        parent_exact_state_hash: entry.exact_state_hash.clone(),
                        window: LayeredCombatDeferredWindow {
                            relative_turn_depth: window.relative_turn_depth.saturating_add(1),
                            window_discrepancy: window.window_discrepancy,
                            source_window_index: window.source_window_index,
                            candidates,
                        },
                    }
                })
            })
            .collect()
    }

    pub fn advance(
        &mut self,
        quantum: LayeredCombatWitnessQuantum,
        stepper: &dyn CombatStepper,
    ) -> LayeredCombatCandidateRaceReport {
        if let Some(status) = self.terminal_status.clone() {
            return self.snapshot(status);
        }
        let work_limit = self
            .counters
            .generation_work
            .saturating_add(quantum.additional_generation_work);
        let engine_step_limit = self
            .counters
            .engine_steps
            .saturating_add(quantum.additional_engine_steps);

        loop {
            if deadline_reached(quantum.deadline) {
                return self.snapshot(LayeredCombatCandidateRaceStatus::Partial(
                    LayeredCombatWitnessInterruption::Deadline,
                ));
            }
            if self.counters.generation_work >= work_limit {
                return self.snapshot(LayeredCombatCandidateRaceStatus::Partial(
                    LayeredCombatWitnessInterruption::GenerationWorkBudget,
                ));
            }
            if self.counters.engine_steps >= engine_step_limit {
                return self.snapshot(LayeredCombatCandidateRaceStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                ));
            }
            let Some(entry_index) = self.next_entry_index() else {
                let status = LayeredCombatCandidateRaceStatus::CandidatesExhausted;
                self.terminal_status = Some(status.clone());
                return self.snapshot(status);
            };

            let remaining_work = work_limit.saturating_sub(self.counters.generation_work);
            let remaining_steps = engine_step_limit.saturating_sub(self.counters.engine_steps);
            let service_work = self.config.service_quantum_work.max(1).min(remaining_work);
            let before = self.entries[entry_index].session.counters().clone();
            let candidate_report = self.entries[entry_index].session.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: service_work,
                    additional_engine_steps: remaining_steps.min(
                        service_work.saturating_mul(
                            self.config
                                .continuation
                                .generator
                                .max_engine_steps_per_transition
                                .max(1),
                        ),
                    ),
                    deadline: quantum.deadline,
                },
                stepper,
            );
            let after = self.entries[entry_index].session.counters().clone();
            let used_work = after.generation_work.saturating_sub(before.generation_work);
            let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
            self.counters.generation_work = self.counters.generation_work.saturating_add(used_work);
            self.counters.engine_steps = self.counters.engine_steps.saturating_add(used_steps);
            self.counters.services = self.counters.services.saturating_add(1);
            self.refresh_entry_guidance(entry_index);

            if let Some(candidate_witness) = candidate_report.witness {
                let entry = &mut self.entries[entry_index];
                let mut actions = entry.prefix_actions.clone();
                actions.extend(candidate_witness.actions);
                let negative_log_policy =
                    entry.prefix_negative_log_policy + candidate_witness.negative_log_policy;
                match replay_witness(&self.original_root, &actions, negative_log_policy, stepper) {
                    Ok(witness) => {
                        entry.found_witness = true;
                        self.witness = Some(witness);
                        let status = LayeredCombatCandidateRaceStatus::WitnessFound;
                        self.terminal_status = Some(status.clone());
                        return self.snapshot(status);
                    }
                    Err(error) => {
                        let status = LayeredCombatCandidateRaceStatus::ReplayMismatch(error);
                        self.terminal_status = Some(status.clone());
                        return self.snapshot(status);
                    }
                }
            }
            if used_work == 0 && used_steps == 0 && !self.entries[entry_index].session.is_terminal()
            {
                let status = LayeredCombatCandidateRaceStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                );
                return self.snapshot(status);
            }
        }
    }

    fn next_entry_index(&mut self) -> Option<usize> {
        let needs_first_layer = self
            .entries
            .iter()
            .any(|entry| !entry.session.is_terminal() && entry.session.completed_layers() == 0);
        if needs_first_layer {
            return self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    !entry.session.is_terminal() && entry.session.completed_layers() == 0
                })
                .min_by_key(|(_, entry)| {
                    let work = entry.session.counters().generation_work as u128;
                    let rank = entry.candidate_index.saturating_add(1) as u128;
                    // A retained candidate must first receive enough service
                    // to expose one exact next-turn layer. Before that point,
                    // its presentation index is not evidence that the whole
                    // continuation is weak.
                    (work, rank)
                })
                .map(|(index, _)| index);
        }

        let mut service_views = vec![CandidateRaceServiceView::Anchor];
        let guide_lanes = self
            .entries
            .iter()
            .filter(|entry| !entry.session.is_terminal())
            .flat_map(|entry| entry.continuation_guides.keys().copied())
            .collect::<BTreeSet<_>>();
        service_views.extend(guide_lanes.into_iter().map(CandidateRaceServiceView::Guide));
        let requested_view = service_views[self.next_service_view % service_views.len()];
        self.next_service_view = self.next_service_view.saturating_add(1);
        select_candidate_race_entry(&self.entries, requested_view).or_else(|| {
            select_candidate_race_entry(&self.entries, CandidateRaceServiceView::Anchor)
        })
    }

    fn refresh_entry_guidance(&mut self, entry_index: usize) {
        let entry = &mut self.entries[entry_index];
        let policy = entry.session.policy.clone();
        let prefix_negative_log_policy = entry.prefix_negative_log_policy;
        let mut anchor_cost = None::<f64>;
        let mut guides = BTreeMap::<CombatGuideLaneId, CombatStateGuideRank>::new();
        entry.session.visit_frontier_states(|state| {
            let candidate_anchor = prefix_negative_log_policy + state.policy_priority();
            if anchor_cost.is_none_or(|incumbent| candidate_anchor < incumbent) {
                anchor_cost = Some(candidate_anchor);
            }
            for guide in policy.state_guides(&state.position) {
                match guides.entry(guide.lane) {
                    std::collections::btree_map::Entry::Vacant(vacant) => {
                        vacant.insert(guide.rank);
                    }
                    std::collections::btree_map::Entry::Occupied(mut occupied) => {
                        if guide.rank > *occupied.get() {
                            occupied.insert(guide.rank);
                        }
                    }
                }
            }
        });
        entry.continuation_anchor_cost = anchor_cost;
        entry.continuation_guides = guides;
    }

    fn snapshot(
        &self,
        status: LayeredCombatCandidateRaceStatus,
    ) -> LayeredCombatCandidateRaceReport {
        LayeredCombatCandidateRaceReport {
            status,
            counters: self.counters.clone(),
            candidates: self
                .entries
                .iter()
                .map(|entry| LayeredCombatCandidateRaceEntryReport {
                    candidate_index: entry.candidate_index,
                    exact_state_hash: entry.exact_state_hash.clone(),
                    generation_work: entry.session.counters().generation_work,
                    engine_steps: entry.session.counters().engine_steps,
                    completed_layers: entry.session.completed_layers(),
                    terminal: entry.session.is_terminal(),
                    found_witness: entry.found_witness,
                })
                .collect(),
            witness: self.witness.clone(),
        }
    }
}

/// Orders parent lineages using ordinal evidence from every independent guide
/// view. Raw guide components are never added or compared across lanes. This
/// is a scheduling hint only: every exact child remains available to a later
/// resumable pass.
pub fn rank_layered_combat_lineage_parents(
    windows: &[LayeredCombatLineageWindow],
    policy: &dyn super::policy::CombatActionPolicy,
) -> Vec<LayeredCombatLineageParentRank> {
    struct ParentEvidence {
        parent_candidate_index: usize,
        parent_exact_state_hash: String,
        best_anchor_cost: f64,
        best_guides: BTreeMap<CombatGuideLaneId, CombatStateGuideRank>,
    }

    let mut by_parent = BTreeMap::<usize, ParentEvidence>::new();
    let mut guide_lanes = BTreeSet::<CombatGuideLaneId>::new();
    for lineage in windows {
        let evidence = by_parent
            .entry(lineage.parent_candidate_index)
            .or_insert_with(|| ParentEvidence {
                parent_candidate_index: lineage.parent_candidate_index,
                parent_exact_state_hash: lineage.parent_exact_state_hash.clone(),
                best_anchor_cost: f64::INFINITY,
                best_guides: BTreeMap::new(),
            });
        debug_assert_eq!(
            evidence.parent_exact_state_hash,
            lineage.parent_exact_state_hash
        );
        for candidate in &lineage.window.candidates {
            let anchor_cost =
                candidate.negative_log_policy + (candidate.actions.len().max(1) as f64).ln();
            evidence.best_anchor_cost = evidence.best_anchor_cost.min(anchor_cost);
            for guide in policy.state_guides(&candidate.position) {
                guide_lanes.insert(guide.lane);
                match evidence.best_guides.entry(guide.lane) {
                    std::collections::btree_map::Entry::Vacant(vacant) => {
                        vacant.insert(guide.rank);
                    }
                    std::collections::btree_map::Entry::Occupied(mut occupied) => {
                        if guide.rank > *occupied.get() {
                            occupied.insert(guide.rank);
                        }
                    }
                }
            }
        }
    }
    let evidence = by_parent.into_values().collect::<Vec<_>>();
    let missing_rank = evidence.len().saturating_add(1);
    let mut ranked = evidence
        .iter()
        .map(|candidate| {
            let anchor_rank = 1usize.saturating_add(
                evidence
                    .iter()
                    .filter(|other| other.best_anchor_cost < candidate.best_anchor_cost)
                    .count(),
            );
            let guide_ranks = guide_lanes
                .iter()
                .copied()
                .map(|lane| {
                    let rank = candidate
                        .best_guides
                        .get(&lane)
                        .map_or(missing_rank, |rank| {
                            1usize.saturating_add(
                                evidence
                                    .iter()
                                    .filter_map(|other| other.best_guides.get(&lane))
                                    .filter(|other_rank| *other_rank > rank)
                                    .count(),
                            )
                        });
                    (lane, rank)
                })
                .collect::<Vec<_>>();
            let rank_sum = guide_ranks
                .iter()
                .fold(anchor_rank, |sum, (_, rank)| sum.saturating_add(*rank));
            LayeredCombatLineageParentRank {
                parent_candidate_index: candidate.parent_candidate_index,
                parent_exact_state_hash: candidate.parent_exact_state_hash.clone(),
                consensus_rank: 0,
                rank_sum,
                anchor_rank,
                guide_ranks,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|parent| {
        (
            parent.rank_sum,
            parent.anchor_rank,
            parent.parent_candidate_index,
        )
    });
    for (index, parent) in ranked.iter_mut().enumerate() {
        parent.consensus_rank = index.saturating_add(1);
    }
    ranked
}

impl LayeredCombatLineagePortfolioSession {
    pub fn from_lineage_windows(
        original_root: CombatDecisionRoot,
        windows: Vec<LayeredCombatLineageWindow>,
        config: LayeredCombatLineagePortfolioConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let parent_ranks = rank_layered_combat_lineage_parents(&windows, policy.as_ref());
        let parents_per_view = config.parents_per_view.max(1);
        let selected_parents = parent_ranks
            .iter()
            .filter(|parent| {
                parent.anchor_rank <= parents_per_view
                    || parent
                        .guide_ranks
                        .iter()
                        .any(|(_, rank)| *rank <= parents_per_view)
            })
            .map(|parent| parent.parent_candidate_index)
            .collect::<BTreeSet<_>>();
        let selected_parent_count = selected_parents.len();
        let deferred_parent_count = parent_ranks.len().saturating_sub(selected_parent_count);
        let parent_rank_by_index = parent_ranks
            .into_iter()
            .map(|rank| (rank.parent_candidate_index, rank))
            .collect::<BTreeMap<_, _>>();
        let mut windows_by_parent = BTreeMap::<usize, Vec<LayeredCombatLineageWindow>>::new();
        for window in windows {
            windows_by_parent
                .entry(window.parent_candidate_index)
                .or_default()
                .push(window);
        }
        for parent_windows in windows_by_parent.values_mut() {
            parent_windows.sort_by_key(|lineage| {
                (
                    lineage.window.window_discrepancy,
                    lineage.window.source_window_index,
                )
            });
        }

        let original_position = original_root.position().clone();
        let windows_per_parent = config.windows_per_parent.max(1);
        let mut deferred_window_count = 0usize;
        let mut entries = Vec::new();
        for (parent_candidate_index, parent_windows) in windows_by_parent {
            if !selected_parents.contains(&parent_candidate_index) {
                deferred_window_count = deferred_window_count.saturating_add(parent_windows.len());
                continue;
            }
            let Some(parent_rank) = parent_rank_by_index.get(&parent_candidate_index).cloned()
            else {
                deferred_window_count = deferred_window_count.saturating_add(parent_windows.len());
                continue;
            };
            for (window_index, lineage) in parent_windows.into_iter().enumerate() {
                if window_index >= windows_per_parent {
                    deferred_window_count = deferred_window_count.saturating_add(1);
                    continue;
                }
                let source_window_index = lineage.window.source_window_index;
                let window_discrepancy = lineage.window.window_discrepancy;
                let root = CombatDecisionRoot::new(original_position.clone())
                    .expect("portfolio root was already validated");
                entries.push(LineagePortfolioEntry {
                    parent_rank: parent_rank.clone(),
                    source_window_index,
                    window_discrepancy,
                    race: Box::new(LayeredCombatCandidateRaceSession::from_window(
                        root,
                        lineage.window,
                        config.candidate_race,
                        policy.clone(),
                    )),
                    found_witness: false,
                });
            }
        }
        entries.sort_by_key(|entry| {
            (
                entry.parent_rank.consensus_rank,
                entry.window_discrepancy,
                entry.source_window_index,
                entry.parent_rank.parent_candidate_index,
            )
        });
        let mut service_views = vec![CandidateRaceServiceView::Anchor];
        let guide_lanes = entries
            .iter()
            .flat_map(|entry| entry.parent_rank.guide_ranks.iter().map(|(lane, _)| *lane))
            .collect::<BTreeSet<_>>();
        service_views.extend(guide_lanes.into_iter().map(CandidateRaceServiceView::Guide));
        let terminal_status = entries
            .is_empty()
            .then_some(LayeredCombatLineagePortfolioStatus::SelectedPortfolioExhausted);
        Self {
            config,
            entries,
            selected_parent_count,
            deferred_parent_count,
            deferred_window_count,
            service_views,
            next_service_view: 0,
            counters: LayeredCombatLineagePortfolioCounters::default(),
            terminal_status,
            witness: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal_status.is_some()
    }

    pub fn advance(
        &mut self,
        quantum: LayeredCombatWitnessQuantum,
        stepper: &dyn CombatStepper,
    ) -> LayeredCombatLineagePortfolioReport {
        if let Some(status) = self.terminal_status.clone() {
            return self.snapshot(status);
        }
        let work_limit = self
            .counters
            .generation_work
            .saturating_add(quantum.additional_generation_work);
        let engine_step_limit = self
            .counters
            .engine_steps
            .saturating_add(quantum.additional_engine_steps);
        loop {
            if deadline_reached(quantum.deadline) {
                return self.snapshot(LayeredCombatLineagePortfolioStatus::Partial(
                    LayeredCombatWitnessInterruption::Deadline,
                ));
            }
            if self.counters.generation_work >= work_limit {
                return self.snapshot(LayeredCombatLineagePortfolioStatus::Partial(
                    LayeredCombatWitnessInterruption::GenerationWorkBudget,
                ));
            }
            if self.counters.engine_steps >= engine_step_limit {
                return self.snapshot(LayeredCombatLineagePortfolioStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                ));
            }
            let Some(entry_index) = self.next_entry_index() else {
                let status = LayeredCombatLineagePortfolioStatus::SelectedPortfolioExhausted;
                self.terminal_status = Some(status.clone());
                return self.snapshot(status);
            };
            let remaining_work = work_limit.saturating_sub(self.counters.generation_work);
            let remaining_steps = engine_step_limit.saturating_sub(self.counters.engine_steps);
            let service_work = self.config.service_quantum_work.max(1).min(remaining_work);
            let before = self.entries[entry_index].race.counters.clone();
            let report = self.entries[entry_index].race.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: service_work,
                    additional_engine_steps: remaining_steps.min(
                        service_work.saturating_mul(
                            self.config
                                .candidate_race
                                .continuation
                                .generator
                                .max_engine_steps_per_transition
                                .max(1),
                        ),
                    ),
                    deadline: quantum.deadline,
                },
                stepper,
            );
            let after = self.entries[entry_index].race.counters.clone();
            let used_work = after.generation_work.saturating_sub(before.generation_work);
            let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
            self.counters.generation_work = self.counters.generation_work.saturating_add(used_work);
            self.counters.engine_steps = self.counters.engine_steps.saturating_add(used_steps);
            self.counters.services = self.counters.services.saturating_add(1);
            if let Some(witness) = report.witness {
                self.entries[entry_index].found_witness = true;
                self.witness = Some(witness);
                let status = LayeredCombatLineagePortfolioStatus::WitnessFound;
                self.terminal_status = Some(status.clone());
                return self.snapshot(status);
            }
            if let LayeredCombatCandidateRaceStatus::ReplayMismatch(error) = report.status {
                let status = LayeredCombatLineagePortfolioStatus::ReplayMismatch(error);
                self.terminal_status = Some(status.clone());
                return self.snapshot(status);
            }
            if used_work == 0 && used_steps == 0 && !self.entries[entry_index].race.is_terminal() {
                return self.snapshot(LayeredCombatLineagePortfolioStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                ));
            }
        }
    }

    fn next_entry_index(&mut self) -> Option<usize> {
        if self.service_views.is_empty() {
            return None;
        }
        let requested_view = self.service_views[self.next_service_view % self.service_views.len()];
        self.next_service_view = self.next_service_view.saturating_add(1);
        select_lineage_portfolio_entry(&self.entries, requested_view).or_else(|| {
            select_lineage_portfolio_entry(&self.entries, CandidateRaceServiceView::Anchor)
        })
    }

    fn snapshot(
        &self,
        status: LayeredCombatLineagePortfolioStatus,
    ) -> LayeredCombatLineagePortfolioReport {
        LayeredCombatLineagePortfolioReport {
            status,
            counters: self.counters.clone(),
            selected_parent_count: self.selected_parent_count,
            deferred_parent_count: self.deferred_parent_count,
            deferred_window_count: self.deferred_window_count,
            entries: self
                .entries
                .iter()
                .map(|entry| LayeredCombatLineagePortfolioEntryReport {
                    parent_candidate_index: entry.parent_rank.parent_candidate_index,
                    parent_consensus_rank: entry.parent_rank.consensus_rank,
                    source_window_index: entry.source_window_index,
                    window_discrepancy: entry.window_discrepancy,
                    generation_work: entry.race.counters.generation_work,
                    engine_steps: entry.race.counters.engine_steps,
                    terminal: entry.race.is_terminal(),
                    found_witness: entry.found_witness,
                })
                .collect(),
            witness: self.witness.clone(),
        }
    }
}

fn select_lineage_portfolio_entry(
    entries: &[LineagePortfolioEntry],
    view: CandidateRaceServiceView,
) -> Option<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !entry.race.is_terminal())
        .filter_map(|(index, entry)| {
            let view_rank = match view {
                CandidateRaceServiceView::Anchor => Some(entry.parent_rank.anchor_rank),
                CandidateRaceServiceView::Guide(lane) => entry
                    .parent_rank
                    .guide_ranks
                    .iter()
                    .find_map(|(entry_lane, rank)| (*entry_lane == lane).then_some(*rank)),
            }?;
            let work = entry.race.counters.generation_work as u128;
            let view_rank = view_rank.max(1) as u128;
            let window_rank = entry.window_discrepancy.saturating_add(1) as u128;
            Some((
                (
                    work.saturating_add(1)
                        .saturating_mul(view_rank)
                        .saturating_mul(window_rank),
                    view_rank,
                    window_rank,
                    entry.parent_rank.consensus_rank,
                    entry.parent_rank.parent_candidate_index,
                ),
                index,
            ))
        })
        .min_by_key(|(key, _)| *key)
        .map(|(_, index)| index)
}

fn select_candidate_race_entry(
    entries: &[CandidateRaceEntry],
    view: CandidateRaceServiceView,
) -> Option<usize> {
    let mut ranked = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            if entry.session.is_terminal() {
                return false;
            }
            match view {
                CandidateRaceServiceView::Anchor => entry.continuation_anchor_cost.is_some(),
                CandidateRaceServiceView::Guide(lane) => {
                    entry.continuation_guides.contains_key(&lane)
                }
            }
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        let left = &entries[*left];
        let right = &entries[*right];
        match view {
            CandidateRaceServiceView::Anchor => left
                .continuation_anchor_cost
                .expect("anchor entry was checked for eligibility")
                .total_cmp(
                    &right
                        .continuation_anchor_cost
                        .expect("anchor entry was checked for eligibility"),
                ),
            CandidateRaceServiceView::Guide(lane) => right
                .continuation_guides
                .get(&lane)
                .expect("guide entry was checked for eligibility")
                .cmp(
                    left.continuation_guides
                        .get(&lane)
                        .expect("guide entry was checked for eligibility"),
                )
                .then_with(|| {
                    left.continuation_anchor_cost
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&right.continuation_anchor_cost.unwrap_or(f64::INFINITY))
                }),
        }
        .then_with(|| left.candidate_index.cmp(&right.candidate_index))
    });
    let generation_work = entries
        .iter()
        .map(|entry| entry.session.counters().generation_work)
        .collect::<Vec<_>>();
    select_ranked_service_index(&ranked, &generation_work)
}

fn select_ranked_service_index(ranked: &[usize], generation_work: &[usize]) -> Option<usize> {
    ranked
        .iter()
        .copied()
        .enumerate()
        .min_by_key(|(ordinal_rank, entry_index)| {
            let work = generation_work
                .get(*entry_index)
                .copied()
                .unwrap_or(usize::MAX) as u128;
            let rank = ordinal_rank.saturating_add(1) as u128;
            (
                work.saturating_add(1).saturating_mul(rank),
                *ordinal_rank,
                *entry_index,
            )
        })
        .map(|(_, entry_index)| entry_index)
}

#[cfg(test)]
mod candidate_race_priority_tests {
    use super::select_ranked_service_index;

    #[test]
    fn guide_rank_accelerates_service_without_permanently_starving_a_sibling() {
        // Entry 1 is first in this guide view even though its stable index is
        // larger, so it receives the first service quantum.
        assert_eq!(select_ranked_service_index(&[1, 0], &[0, 0]), Some(1));

        // Once entry 1 has consumed enough deterministic generator work, the
        // lower-ranked sibling becomes due. Rank guides service; it does not
        // revoke resumability from another live exact lineage.
        assert_eq!(select_ranked_service_index(&[1, 0], &[0, 4]), Some(0));
    }
}

type CohortAgenda = BTreeMap<(usize, usize, usize, usize), BeamCohort>;

fn enqueue_cohort(agenda: &mut CohortAgenda, cohort: BeamCohort, next_cohort_id: &mut usize) {
    let key = (
        cohort
            .relative_turn_depth
            .saturating_add(cohort.window_discrepancy),
        cohort.window_discrepancy,
        cohort.relative_turn_depth,
        *next_cohort_id,
    );
    *next_cohort_id = next_cohort_id.saturating_add(1);
    agenda.insert(key, cohort);
}

fn best_frontier(agenda: &CohortAgenda, depth_limited: &[BeamCohort]) -> Vec<BeamState> {
    agenda
        .first_key_value()
        .map(|(_, cohort)| cohort.states.clone())
        .or_else(|| {
            depth_limited
                .iter()
                .min_by_key(|cohort| {
                    (
                        cohort
                            .relative_turn_depth
                            .saturating_add(cohort.window_discrepancy),
                        cohort.window_discrepancy,
                        cohort.relative_turn_depth,
                    )
                })
                .map(|cohort| cohort.states.clone())
        })
        .unwrap_or_default()
}

fn select_layer_worker(
    workers: &[LayerWorker],
    view: LayerServiceView,
) -> Option<(usize, LayerServiceView)> {
    let mut best = None;
    for (index, worker) in workers.iter().enumerate() {
        if worker.generator.is_finished() {
            continue;
        }
        let eligible = match view {
            LayerServiceView::Anchor => worker
                .generator
                .best_retained_path_bound_snapshot()
                .is_some(),
            LayerServiceView::Guide(lane) => worker
                .generator
                .best_retained_guide_promise_snapshot(lane)
                .is_some(),
        };
        if !eligible {
            continue;
        }
        if best.is_none_or(|incumbent| worker_is_better(worker, &workers[incumbent], view)) {
            best = Some(index);
        }
    }
    best.map(|index| (index, view))
}

fn worker_is_better(
    candidate: &LayerWorker,
    incumbent: &LayerWorker,
    view: LayerServiceView,
) -> bool {
    match view {
        LayerServiceView::Anchor => {
            layer_worker_anchor_cost(candidate) < layer_worker_anchor_cost(incumbent)
        }
        LayerServiceView::Guide(lane) => {
            let candidate_promise = candidate
                .generator
                .best_retained_guide_promise_snapshot(lane)
                .expect("guide worker was checked for eligibility");
            let incumbent_promise = incumbent
                .generator
                .best_retained_guide_promise_snapshot(lane)
                .expect("guide worker was checked for eligibility");
            guide_promise_is_better(candidate, &candidate_promise, incumbent, &incumbent_promise)
        }
    }
}

fn guide_promise_is_better(
    candidate: &LayerWorker,
    candidate_promise: &RetainedGuidePromise,
    incumbent: &LayerWorker,
    incumbent_promise: &RetainedGuidePromise,
) -> bool {
    candidate_promise
        .rank
        .cmp(&incumbent_promise.rank)
        .then_with(|| {
            incumbent
                .generator
                .counters()
                .generation_work
                .cmp(&candidate.generator.counters().generation_work)
        })
        .then_with(|| {
            layer_worker_anchor_cost(incumbent).total_cmp(&layer_worker_anchor_cost(candidate))
        })
        .is_gt()
}

fn layer_worker_anchor_cost(worker: &LayerWorker) -> f64 {
    let (partial_depth, partial_negative_log_policy) = worker
        .generator
        .best_retained_path_bound_snapshot()
        .unwrap_or((0, f64::INFINITY));
    let total_depth = worker
        .parent
        .atomic_depth
        .saturating_add(partial_depth)
        .max(1);
    let total_negative_log_policy = worker.parent.negative_log_policy + partial_negative_log_policy;
    let service_debt = worker
        .generator
        .counters()
        .generation_work
        .saturating_add(1) as f64;
    total_negative_log_policy + (total_depth as f64).ln() + service_debt.ln()
}

fn rank_multi_view(
    candidates: Vec<BeamState>,
    config: LayeredCombatWitnessConfig,
    policy: &dyn super::policy::CombatActionPolicy,
) -> Vec<BeamState> {
    let first_window_width = config.beam_width.max(1).min(candidates.len());
    let per_view = config.retained_per_view.max(1);
    let mut views = Vec::<Vec<usize>>::new();

    let mut policy_view = (0..candidates.len()).collect::<Vec<_>>();
    policy_view.sort_by(|left, right| compare_policy(&candidates[*left], &candidates[*right]));
    views.push(policy_view.clone());

    let mut guide_views = BTreeMap::<CombatGuideLaneId, Vec<(CombatStateGuideRank, usize)>>::new();
    for (index, candidate) in candidates.iter().enumerate() {
        for guide in policy.state_guides(&candidate.position) {
            guide_views
                .entry(guide.lane)
                .or_default()
                .push((guide.rank, index));
        }
    }
    for (_, mut ranked) in guide_views {
        ranked.sort_by(|(left_rank, left_index), (right_rank, right_index)| {
            right_rank
                .cmp(left_rank)
                .then_with(|| compare_policy(&candidates[*left_index], &candidates[*right_index]))
        });
        views.push(ranked.into_iter().map(|(_, index)| index).collect());
    }
    let mut selected = Vec::with_capacity(candidates.len());
    let mut seen = HashSet::new();
    for rank in 0..per_view {
        for view in &views {
            let Some(index) = view.get(rank).copied() else {
                continue;
            };
            if seen.insert(index) {
                selected.push(index);
                if selected.len() == first_window_width {
                    break;
                }
            }
        }
        if selected.len() == first_window_width {
            break;
        }
    }
    if selected.len() < first_window_width {
        for index in policy_view.iter().copied() {
            if seen.insert(index) {
                selected.push(index);
                if selected.len() == first_window_width {
                    break;
                }
            }
        }
    }

    let remaining_view_depth = views.iter().map(Vec::len).max().unwrap_or_default();
    for rank in 0..remaining_view_depth {
        for view in &views {
            let Some(index) = view.get(rank).copied() else {
                continue;
            };
            if seen.insert(index) {
                selected.push(index);
            }
        }
    }
    for index in policy_view {
        if seen.insert(index) {
            selected.push(index);
        }
    }

    debug_assert_eq!(selected.len(), candidates.len());

    selected
        .into_iter()
        .map(|index| candidates[index].clone())
        .collect()
}

fn compare_policy(left: &BeamState, right: &BeamState) -> std::cmp::Ordering {
    left.policy_priority()
        .total_cmp(&right.policy_priority())
        .then_with(|| {
            left.negative_log_policy
                .total_cmp(&right.negative_log_policy)
        })
        .then_with(|| left.atomic_depth.cmp(&right.atomic_depth))
        .then_with(|| left.exact_state_hash.cmp(&right.exact_state_hash))
}

fn path_is_better(candidate: &BeamState, incumbent: &BeamState) -> bool {
    compare_policy(candidate, incumbent).is_lt()
}

fn flatten_action_trail(tip: Option<&Arc<ActionTrailNode>>) -> Vec<TurnOptionAction> {
    let mut turns = Vec::new();
    let mut current = tip.cloned();
    while let Some(node) = current {
        turns.push(node.turn_actions.clone());
        current = node.parent.clone();
    }
    turns.reverse();
    let action_count = turns.iter().map(|turn| turn.len()).sum();
    let mut actions = Vec::with_capacity(action_count);
    for turn in turns {
        actions.extend_from_slice(&turn);
    }
    actions
}

fn replay_witness(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    negative_log_policy: f64,
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
        if exact_hash(&result.position) != action.expected_successor_hash {
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
        discovery_source: OracleCombatWitnessDiscoverySource::PlannerSearch,
    })
}

fn report(
    status: LayeredCombatWitnessStatus,
    counters: LayeredCombatWitnessCounters,
    layers: Vec<LayeredCombatLayerReport>,
    beam: Vec<BeamState>,
    generation_gaps: Vec<TurnOptionGenerationGap>,
    witness: Option<OracleCombatWitness>,
) -> LayeredCombatWitnessReport {
    LayeredCombatWitnessReport {
        status,
        counters,
        layers,
        frontier: beam.iter().map(BeamState::public_snapshot).collect(),
        generation_gaps,
        witness,
    }
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}
