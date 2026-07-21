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

#[derive(Clone, Copy)]
enum LayerServiceView {
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
    let original_root = root.position().clone();
    let root_state = BeamState {
        exact_state_hash: root.exact_state_hash().to_owned(),
        position: root.position().clone(),
        trail: None,
        atomic_depth: 0,
        negative_log_policy: 0.0,
    };
    let mut counters = LayeredCombatWitnessCounters::default();
    let mut layers = Vec::new();
    let mut generation_gaps = Vec::new();
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
    let mut depth_limited = Vec::new();

    loop {
        if deadline_reached(budget.deadline) {
            return report(
                LayeredCombatWitnessStatus::Partial(LayeredCombatWitnessInterruption::Deadline),
                counters,
                layers,
                best_frontier(&agenda, &depth_limited),
                generation_gaps,
                None,
            );
        }
        if counters.generation_work >= budget.max_generation_work {
            return report(
                LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::GenerationWorkBudget,
                ),
                counters,
                layers,
                best_frontier(&agenda, &depth_limited),
                generation_gaps,
                None,
            );
        }
        if counters.engine_steps >= budget.max_engine_steps {
            return report(
                LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::EngineStepBudget,
                ),
                counters,
                layers,
                best_frontier(&agenda, &depth_limited),
                generation_gaps,
                None,
            );
        }

        let Some((_, cohort)) = agenda.pop_first() else {
            let status = if !depth_limited.is_empty() {
                LayeredCombatWitnessStatus::Partial(
                    LayeredCombatWitnessInterruption::TurnLayerBudget,
                )
            } else if generation_gaps.is_empty() {
                LayeredCombatWitnessStatus::FrontierExhausted
            } else {
                LayeredCombatWitnessStatus::MechanicsGap
            };
            return report(
                status,
                counters,
                layers,
                best_frontier(&agenda, &depth_limited),
                generation_gaps,
                None,
            );
        };
        if cohort.relative_turn_depth >= config.max_turn_layers {
            depth_limited.push(cohort);
            continue;
        }
        if cohort.source_window_index > 0 {
            counters.recovered_window_expansions =
                counters.recovered_window_expansions.saturating_add(1);
        }

        let beam = cohort.states;

        let player_turn = beam
            .first()
            .map(|state| state.position.combat.turn.turn_count)
            .unwrap_or_default();
        debug_assert!(beam
            .iter()
            .all(|state| state.position.combat.turn.turn_count == player_turn));
        let parent_states = beam.len();
        let parent_exact_state_hashes = beam
            .iter()
            .map(|state| state.exact_state_hash.clone())
            .collect::<Vec<_>>();
        let layer_before = counters.clone();
        let mut next_by_hash = HashMap::<String, BeamState>::new();
        let mut layer_gap_count = 0usize;

        let mut guide_lanes = BTreeSet::new();
        let mut workers = beam
            .iter()
            .filter_map(|parent| {
                let parent_root = CombatDecisionRoot::new(parent.position.clone()).ok()?;
                let generator = TurnOptionGeneratorSession::with_policy(
                    parent_root,
                    config.generator,
                    policy.clone(),
                );
                for guide in policy.turn_generation_guides(&parent.position) {
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
        let layer_work_limit = config.maximum_generation_work_per_layer.max(1).min(
            budget
                .max_generation_work
                .saturating_sub(counters.generation_work),
        );
        let candidate_pool_target = config
            .beam_width
            .max(1)
            .saturating_mul(config.candidate_pool_multiplier.max(1));
        let mut layer_work = 0usize;
        let mut next_service_view = 0usize;

        while layer_work < layer_work_limit
            && (layer_work < config.minimum_generation_work_per_layer
                || next_by_hash.len() < candidate_pool_target)
            && workers.iter().any(|worker| !worker.generator.is_finished())
        {
            if deadline_reached(budget.deadline) {
                return report(
                    LayeredCombatWitnessStatus::Partial(LayeredCombatWitnessInterruption::Deadline),
                    counters,
                    layers,
                    beam,
                    generation_gaps,
                    None,
                );
            }
            let remaining_steps = budget
                .max_engine_steps
                .saturating_sub(counters.engine_steps);
            if remaining_steps == 0 {
                return report(
                    LayeredCombatWitnessStatus::Partial(
                        LayeredCombatWitnessInterruption::EngineStepBudget,
                    ),
                    counters,
                    layers,
                    beam,
                    generation_gaps,
                    None,
                );
            }
            let requested_view = service_views[next_service_view % service_views.len()];
            next_service_view = next_service_view.saturating_add(1);
            let (worker_index, actual_view) = select_layer_worker(&workers, requested_view)
                .or_else(|| select_layer_worker(&workers, LayerServiceView::Anchor))
                .expect("a live layer has a serviceable generator");
            let worker = &mut workers[worker_index];
            worker.generator.prefer_lane(match actual_view {
                LayerServiceView::Anchor => TurnOptionGeneratorPreferredLane::Anchor,
                LayerServiceView::Guide(lane) => TurnOptionGeneratorPreferredLane::Guide(lane),
            });
            let work = config
                .generation_quantum_work
                .max(1)
                .min(layer_work_limit.saturating_sub(layer_work));
            let before = worker.generator.counters();
            worker.generator.advance(
                stepper,
                CombatPlanningQuantum {
                    additional_generation_work: work,
                    additional_engine_steps: remaining_steps.min(
                        work.saturating_mul(
                            config.generator.max_engine_steps_per_transition.max(1),
                        ),
                    ),
                    deadline: budget.deadline,
                },
            );
            let after = worker.generator.counters();
            let used_work = after.generation_work.saturating_sub(before.generation_work);
            let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
            if used_work == 0 && used_steps == 0 {
                break;
            }
            layer_work = layer_work.saturating_add(used_work);
            counters.generation_work = counters.generation_work.saturating_add(used_work);
            counters.engine_steps = counters.engine_steps.saturating_add(used_steps);

            let options = worker.generator.take_completed_options();
            counters.completed_turn_options = counters
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
                        match replay_witness(&original_root, &actions, negative_log_policy, stepper)
                        {
                            Ok(witness) => {
                                return report(
                                    LayeredCombatWitnessStatus::WitnessFound,
                                    counters,
                                    layers,
                                    beam,
                                    generation_gaps,
                                    Some(witness),
                                );
                            }
                            Err(error) => {
                                return report(
                                    LayeredCombatWitnessStatus::ReplayMismatch(error),
                                    counters,
                                    layers,
                                    beam,
                                    generation_gaps,
                                    None,
                                );
                            }
                        }
                    }
                    CompleteTurnOptionBoundary::NextPlayerTurn => {
                        let candidate = BeamState {
                            exact_state_hash: option.exact_successor_hash().to_owned(),
                            position: option.exact_successor().clone(),
                            trail,
                            atomic_depth,
                            negative_log_policy,
                        };
                        match next_by_hash.entry(candidate.exact_state_hash.clone()) {
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert(candidate);
                            }
                            std::collections::hash_map::Entry::Occupied(mut entry) => {
                                counters.duplicate_next_turn_states =
                                    counters.duplicate_next_turn_states.saturating_add(1);
                                if path_is_better(&candidate, entry.get()) {
                                    entry.insert(candidate);
                                }
                            }
                        }
                    }
                    CompleteTurnOptionBoundary::TerminalLoss
                    | CompleteTurnOptionBoundary::Escape => {}
                }
            }
        }

        let expanded_parents = workers
            .iter()
            .filter(|worker| worker.generator.counters().generation_work > 0)
            .count();
        let truncated_parents = workers
            .iter()
            .filter(|worker| !worker.generator.is_finished())
            .count();
        counters.expanded_parents = counters.expanded_parents.saturating_add(expanded_parents);
        counters.truncated_parents = counters.truncated_parents.saturating_add(truncated_parents);
        for worker in &workers {
            layer_gap_count = layer_gap_count.saturating_add(worker.generator.gaps().len());
            generation_gaps.extend_from_slice(worker.generator.gaps());
        }
        let parent_work = workers
            .iter()
            .map(|worker| LayeredCombatParentWorkReport {
                exact_state_hash: worker.parent.exact_state_hash.clone(),
                generation_work: worker.generator.counters().generation_work,
                completed_turn_options: worker.generator.total_completed_options(),
                finished: worker.generator.is_finished(),
            })
            .collect::<Vec<_>>();

        let unique_next = next_by_hash.len();
        counters.unique_next_turn_states =
            counters.unique_next_turn_states.saturating_add(unique_next);
        let next_candidates = next_by_hash.into_values().collect::<Vec<_>>();
        if next_candidates.is_empty() {
            layers.push(LayeredCombatLayerReport {
                relative_turn_depth: cohort.relative_turn_depth,
                window_discrepancy: cohort.window_discrepancy,
                source_window_index: cohort.source_window_index,
                player_turn,
                parent_states,
                parent_exact_state_hashes,
                parent_work,
                expanded_parents: counters
                    .expanded_parents
                    .saturating_sub(layer_before.expanded_parents),
                generation_work: counters
                    .generation_work
                    .saturating_sub(layer_before.generation_work),
                completed_turn_options: counters
                    .completed_turn_options
                    .saturating_sub(layer_before.completed_turn_options),
                unique_next_turn_states: 0,
                duplicate_next_turn_states: counters
                    .duplicate_next_turn_states
                    .saturating_sub(layer_before.duplicate_next_turn_states),
                retained_next_turn_states: 0,
                retained_exact_state_hashes: Vec::new(),
                truncated_parents: counters
                    .truncated_parents
                    .saturating_sub(layer_before.truncated_parents),
                emitted_windows: 0,
            });
            counters.completed_layers = counters.completed_layers.saturating_add(1);
            continue;
        }

        let ranked_candidates = rank_multi_view(next_candidates, config, policy.as_ref());
        let window_width = config.beam_width.max(1);
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
            let window_discrepancy = cohort.window_discrepancy.saturating_add(window_index);
            counters.maximum_window_discrepancy =
                counters.maximum_window_discrepancy.max(window_discrepancy);
            if window_index > 0 {
                counters.deferred_windows = counters.deferred_windows.saturating_add(1);
            }
            enqueue_cohort(
                &mut agenda,
                BeamCohort {
                    states,
                    relative_turn_depth: cohort.relative_turn_depth.saturating_add(1),
                    window_discrepancy,
                    source_window_index: window_index,
                },
                &mut next_cohort_id,
            );
        }
        layers.push(LayeredCombatLayerReport {
            relative_turn_depth: cohort.relative_turn_depth,
            window_discrepancy: cohort.window_discrepancy,
            source_window_index: cohort.source_window_index,
            player_turn,
            parent_states,
            parent_exact_state_hashes,
            parent_work,
            expanded_parents: counters
                .expanded_parents
                .saturating_sub(layer_before.expanded_parents),
            generation_work: counters
                .generation_work
                .saturating_sub(layer_before.generation_work),
            completed_turn_options: counters
                .completed_turn_options
                .saturating_sub(layer_before.completed_turn_options),
            unique_next_turn_states: unique_next,
            duplicate_next_turn_states: counters
                .duplicate_next_turn_states
                .saturating_sub(layer_before.duplicate_next_turn_states),
            retained_next_turn_states,
            retained_exact_state_hashes,
            truncated_parents: counters
                .truncated_parents
                .saturating_sub(layer_before.truncated_parents),
            emitted_windows,
        });
        counters.completed_layers = counters.completed_layers.saturating_add(1);
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
