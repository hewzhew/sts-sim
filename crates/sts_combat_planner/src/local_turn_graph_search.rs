mod config;
mod potion_budget;
mod scheduling;

pub use config::LocalTurnGraphWitnessConfig;
use potion_budget::*;
use scheduling::*;

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use sts_combat_strategy::{
    combat_plan_action_timing_v1, combat_plan_has_timed_action_preference_v1,
    combat_plan_projection_v1, combat_plan_selection_member_timing_v1,
    combat_plan_transition_annotation_v1, CombatPlanActionTimingV1, CombatPlanProjectionV1,
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

#[derive(Clone, Copy, Debug)]
pub struct LocalTurnGraphWitnessQuantum {
    pub additional_selections: usize,
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalTurnGraphWitnessInterruption {
    SelectionBudget,
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalTurnGraphWitnessStatus {
    WitnessFound,
    Partial(LocalTurnGraphWitnessInterruption),
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalTurnGraphPlanAnnotationEnableError {
    EdgesAlreadyMaterialized,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessCounters {
    pub selections: usize,
    pub node_visits: usize,
    pub generation_work: usize,
    pub lookahead_evaluations: usize,
    pub lookahead_work: usize,
    pub atomic_lookahead_evaluations: usize,
    pub atomic_lookahead_work: usize,
    pub boundary_lookahead_evaluations: usize,
    pub boundary_lookahead_work: usize,
    pub engine_steps: usize,
    pub exact_nodes: usize,
    pub exact_edges: usize,
    /// Newly materialized exact edges carrying read-only combat-plan facts.
    /// This counter never participates in scheduling or stopping.
    pub annotated_exact_edges: usize,
    pub completed_turn_options: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub duplicate_successor_edges: usize,
    pub terminal_losses: usize,
    pub depth_limited_successors: usize,
    pub exhausted_nodes: usize,
    pub maximum_turn_depth: usize,
    /// Complete tactical lines proposed by an external policy and then
    /// replayed from this session's unchanged root.
    pub policy_witness_proposals: usize,
    /// Exact simulator steps spent authoritatively replaying policy proposals.
    pub policy_witness_replay_engine_steps: usize,
}

/// Wall-clock diagnostics kept outside deterministic search counters.
///
/// Search budgets and equality contracts never read this structure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalTurnGraphPerformanceTiming {
    pub selection_elapsed_ns: u64,
    pub generation_elapsed_ns: u64,
    pub admission_elapsed_ns: u64,
    pub atomic_expand_elapsed_ns: u64,
    pub transition_simulation_elapsed_ns: u64,
    pub transition_identity_elapsed_ns: u64,
    pub transition_admission_elapsed_ns: u64,
    pub transition_trace_elapsed_ns: u64,
    pub transition_seen_elapsed_ns: u64,
    pub transition_publish_elapsed_ns: u64,
}

impl LocalTurnGraphPerformanceTiming {
    fn accumulate(&mut self, other: Self) {
        self.selection_elapsed_ns = self
            .selection_elapsed_ns
            .saturating_add(other.selection_elapsed_ns);
        self.generation_elapsed_ns = self
            .generation_elapsed_ns
            .saturating_add(other.generation_elapsed_ns);
        self.admission_elapsed_ns = self
            .admission_elapsed_ns
            .saturating_add(other.admission_elapsed_ns);
        self.atomic_expand_elapsed_ns = self
            .atomic_expand_elapsed_ns
            .saturating_add(other.atomic_expand_elapsed_ns);
        self.transition_simulation_elapsed_ns = self
            .transition_simulation_elapsed_ns
            .saturating_add(other.transition_simulation_elapsed_ns);
        self.transition_identity_elapsed_ns = self
            .transition_identity_elapsed_ns
            .saturating_add(other.transition_identity_elapsed_ns);
        self.transition_admission_elapsed_ns = self
            .transition_admission_elapsed_ns
            .saturating_add(other.transition_admission_elapsed_ns);
        self.transition_trace_elapsed_ns = self
            .transition_trace_elapsed_ns
            .saturating_add(other.transition_trace_elapsed_ns);
        self.transition_seen_elapsed_ns = self
            .transition_seen_elapsed_ns
            .saturating_add(other.transition_seen_elapsed_ns);
        self.transition_publish_elapsed_ns = self
            .transition_publish_elapsed_ns
            .saturating_add(other.transition_publish_elapsed_ns);
    }
}

#[derive(Clone, Debug)]
pub struct LocalTurnGraphWitnessReport {
    pub status: LocalTurnGraphWitnessStatus,
    pub counters: LocalTurnGraphWitnessCounters,
    pub performance_timing: LocalTurnGraphPerformanceTiming,
    pub root_visits: usize,
    pub root_generated_options: usize,
    pub root_children: usize,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub witness: Option<OracleCombatWitness>,
}

/// Exact work used to materialize one bounded policy mainline at player-turn
/// boundaries before ordinary graph search.
///
/// A proposal is not a witness. It merely leaves replayable edges in the
/// shared graph; terminal truth still comes from exact simulation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LocalTurnGraphSuffixProbeAttempt {
    pub exact_state_hash: String,
    pub player_turn: u32,
    pub plan_projection: Option<CombatPlanProjectionV1>,
    pub generation_work: usize,
    pub engine_steps: usize,
    pub witness_found: bool,
    pub final_hp: Option<i32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalTurnGraphPolicyLineReport {
    pub proposed_turns: usize,
    pub chosen_action_transitions: usize,
    pub proposed_actions: Vec<ClientInput>,
    pub rejected_preview_transitions: usize,
    pub deferred_actions: usize,
    pub engine_steps: usize,
    pub legal_surface_elapsed_ns: u64,
    pub policy_ranking_elapsed_ns: u64,
    pub transition_preview_elapsed_ns: u64,
    pub action_identity_elapsed_ns: u64,
    pub plan_annotation_elapsed_ns: u64,
    pub successor_admission_elapsed_ns: u64,
    pub suffix_probe_attempts: usize,
    pub suffix_probe_generation_work: usize,
    pub suffix_probe_engine_steps: usize,
    pub suffix_probe_completed_turn_options: usize,
    pub suffix_probe_applied_action_transitions: usize,
    pub suffix_probe_unique_successor_states: usize,
    pub suffix_probe_exact_nodes: usize,
    pub suffix_probe_exact_edges: usize,
    pub suffix_probe_performance_timing: LocalTurnGraphPerformanceTiming,
    pub suffix_probe_setup_elapsed_ns: u64,
    pub suffix_probe_advance_elapsed_ns: u64,
    pub suffix_probe_replay_elapsed_ns: u64,
    pub suffix_probe_witness_found: bool,
    pub suffix_probe_details: Vec<LocalTurnGraphSuffixProbeAttempt>,
    pub reached_terminal_win: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct LocalTurnGraphStateSnapshot {
    pub exact_state_hash: String,
    pub relative_turn_depth: usize,
    pub visits: usize,
    pub generation_work: usize,
    pub generator_engine_steps: usize,
    pub retained_generator_work_items: usize,
    pub generator_anchor_work_pops: usize,
    pub generator_guided_work_pops: usize,
    pub best_retained_anchor_atomic_depth: Option<usize>,
    pub retained_guide_promises: Vec<LocalTurnGraphRetainedGuidePromiseSnapshot>,
    pub retained_lookahead_guides: usize,
    pub lookahead_pending_lane: Option<u32>,
    pub generated_options: usize,
    pub children: usize,
    pub exhausted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct LocalTurnGraphRetainedGuidePromiseSnapshot {
    pub lane: u32,
    pub rank: Vec<i32>,
    pub atomic_depth: usize,
}

/// Read-only root-action attribution using the local graph's own semantics.
///
/// Descendant counts are non-exclusive reachability counts: an exact node
/// shared by two root-action families is truthfully reachable from both.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphRootActionFamilySnapshot {
    pub first_action: ClientInput,
    pub best_root_negative_log_policy: Option<f64>,
    pub completed_root_turn_options: usize,
    pub terminal_wins: usize,
    pub terminal_losses: usize,
    pub escapes: usize,
    pub unique_next_turn_successors: usize,
    pub retained_next_turn_successors: usize,
    pub reachable_exact_states: usize,
    pub reachable_retained_states: usize,
    pub reachable_generation_work: usize,
    pub reachable_completed_turn_options: usize,
    pub max_player_turn: u32,
    pub best_hp_at_max_turn: Option<i32>,
    pub lowest_enemy_hp_at_max_turn: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphGuideServiceSnapshot {
    pub lane: u32,
    pub edge_visits: usize,
    pub ordinal_rank: usize,
    pub candidate_count: usize,
    pub successor_rank: Vec<i32>,
    pub best_rank: Vec<i32>,
}

/// One already-materialized exact edge in the local turn graph.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphEdgeSnapshot {
    pub parent_visits: usize,
    pub parent_generated_options: usize,
    pub parent_children: usize,
    pub parent_widen_anchor_visits: usize,
    pub actions: Vec<TurnOptionAction>,
    pub negative_log_policy: f64,
    pub plan_transition_annotation: Option<CombatPlanTransitionAnnotationV1>,
    pub visits: usize,
    pub anchor_visits: usize,
    pub backed_visits: usize,
    pub backed_lookahead_rank: Option<Vec<i32>>,
    pub lookahead_pending_rank: Option<usize>,
    pub lookahead_pending_candidates: usize,
    pub guide_service: Vec<LocalTurnGraphGuideServiceSnapshot>,
    pub successor_visits: usize,
    pub successor_generated_options: usize,
    pub successor_children: usize,
    pub successor_exhausted: bool,
}

/// One exact graph edge carrying an encounter-owned plan annotation.
///
/// This diagnostic view deliberately exposes service facts without
/// interpreting the annotation. Encounter semantics remain owned by the
/// strategy crate and are never read by local-graph scheduling.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphPlanTransitionEdgeSnapshot {
    pub parent_exact_state_hash: String,
    pub successor_exact_state_hash: String,
    pub parent_relative_turn_depth: usize,
    pub action_count: usize,
    pub negative_log_policy: f64,
    pub plan_transition_annotation: CombatPlanTransitionAnnotationV1,
    pub edge_visits: usize,
    pub anchor_visits: usize,
    pub guide_visits: usize,
    pub backed_visits: usize,
    pub successor_visits: usize,
}

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
    pub fn set_satisfaction(&mut self, satisfaction: OracleCombatWitnessSatisfaction) {
        self.config.satisfaction = satisfaction;
    }

    /// Enables read-only plan facts on subsequently materialized exact edges.
    ///
    /// Enabling after graph construction would leave a mixture of annotated
    /// and unannotated edges, so the session rejects that ambiguous state.
    pub fn enable_plan_transition_annotations(
        &mut self,
    ) -> Result<(), LocalTurnGraphPlanAnnotationEnableError> {
        if self.used.exact_edges > 0 {
            return Err(LocalTurnGraphPlanAnnotationEnableError::EdgesAlreadyMaterialized);
        }
        self.collect_plan_transition_annotations = true;
        Ok(())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, None)
    }

    pub fn with_policy_and_lookahead(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: SharedCombatLookaheadEvaluator,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, Some(lookahead_evaluator))
    }

    fn with_optional_lookahead(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: Option<SharedCombatLookaheadEvaluator>,
    ) -> Self {
        let original_root = root.position().clone();
        let root_exact_key = root
            .exact_state_key()
            .expect("a newly constructed combat root retains its exact key")
            .clone();
        let (root_guides, root_lookahead_pending_lane) = guides_with_pending_lookahead(
            policy.as_ref(),
            lookahead_evaluator.as_deref(),
            root.position(),
        );
        let root_backed_guides = guide_rank_map(&root_guides);
        let root_boundary_service_views =
            boundary_service_views_from_guides(&root_guides, root_lookahead_pending_lane);
        let root_lookahead_acquisition_views =
            lookahead_acquisition_views_from_guides(&root_guides, root_lookahead_pending_lane);
        // Expensive lookahead evaluates exact player-turn boundaries. Atomic
        // partial states remain the generator's private proposal mechanism;
        // evaluating them here would reintroduce an independent inner search.
        let generator = turn_generator_for_potion_budget(
            root.clone(),
            config.generator,
            policy.clone(),
            config.max_potions_used,
            0,
        );
        let root_generation_service_views =
            generation_service_views_from_lanes(generator.retained_guide_lanes());
        Self {
            original_root,
            config,
            policy,
            lookahead_evaluator,
            collect_plan_transition_annotations: false,
            lookahead_lane: root_lookahead_pending_lane,
            nodes: vec![GraphNode {
                generator,
                potion_expenditures: 0,
                diagnostic_parent: None,
                relative_turn_depth: 0,
                visits: 0,
                generated_options: 0,
                children: Vec::new(),
                guides: root_guides,
                boundary_service_views: root_boundary_service_views,
                next_boundary_service_view: 0,
                lookahead_acquisition_views: root_lookahead_acquisition_views,
                next_lookahead_acquisition_view: 0,
                generation_service_views: root_generation_service_views,
                next_generation_service_view: 0,
                widen_anchor_visits: 0,
                widen_guide_visits: BTreeMap::new(),
                lookahead_pending_lane: root_lookahead_pending_lane,
                backed_guides: root_backed_guides,
                backed_lookahead_rank: None,
                synced_gaps: 0,
                exhausted: false,
            }],
            nodes_by_exact_key: HashMap::from([(
                ConstrainedExactStateKey::new(root_exact_key, config.max_potions_used, 0),
                0,
            )]),
            used: LocalTurnGraphWitnessCounters {
                exact_nodes: 1,
                ..LocalTurnGraphWitnessCounters::default()
            },
            performance_timing: LocalTurnGraphPerformanceTiming::default(),
            granted_selections: 0,
            granted_generation_work: 0,
            granted_engine_steps: 0,
            generation_gaps: Vec::new(),
            root_action_families: Vec::new(),
            witness: None,
            replay_failure: None,
        }
    }

    pub fn witness(&self) -> Option<&OracleCombatWitness> {
        self.witness.as_ref()
    }

    /// Offers one complete tactical line as an untrusted candidate.
    ///
    /// Policy code may discover a useful line cheaply, but it owns neither
    /// legality nor terminal truth. This session replays every action and
    /// expected exact successor from its unchanged root before installing a
    /// witness. Independent local-graph search remains available to improve
    /// or replace the candidate.
    pub fn offer_witness_proposal(
        &mut self,
        proposal: CombatPolicyWitnessProposal,
        stepper: &dyn CombatStepper,
    ) -> Result<bool, OracleCombatWitnessReplayError> {
        self.used.policy_witness_proposals = self.used.policy_witness_proposals.saturating_add(1);
        let witness = replay_witness(
            &self.original_root,
            &proposal.actions,
            proposal.actions.len() as f64,
            OracleCombatWitnessDiscoverySource::PolicyProposal,
            stepper,
        )?;
        self.used.policy_witness_replay_engine_steps = self
            .used
            .policy_witness_replay_engine_steps
            .saturating_add(witness.replay_engine_steps);
        self.used.engine_steps = self
            .used
            .engine_steps
            .saturating_add(witness.replay_engine_steps);
        Ok(self.remember_witness(witness))
    }

    /// Materializes one bounded, exact policy mainline as ordinary graph
    /// edges.
    ///
    /// At each stable action surface the existing action policy still selects
    /// the greedy action. An encounter plan may reject a preview only when
    /// exact before/after projections prove that it prematurely spends a
    /// reserved resource. The next policy-ranked compatible action is tried;
    /// EndTurn therefore wins naturally only when no better compatible play
    /// remains. Every rejected action remains available to normal search.
    ///
    /// This is intentionally a single line, not another scheduler, beam, or
    /// pruning rule. If the line loses or encounters an unsupported
    /// structured choice, already materialized turn boundaries remain useful
    /// and ordinary search continues unchanged.
    pub fn offer_plan_compatible_policy_line(
        &mut self,
        max_turns: usize,
        max_actions: usize,
        stepper: &dyn CombatStepper,
    ) -> Result<LocalTurnGraphPolicyLineReport, String> {
        self.offer_plan_compatible_policy_line_with_suffix_probes(
            max_turns,
            max_actions,
            0,
            stepper,
        )
    }

    /// Materializes the same exact policy line and, immediately before that
    /// line would cross a typed combat-plan milestone, gives the current
    /// exact state one bounded deterministic suffix search.
    ///
    /// This is a hierarchical laboratory control, not a second global
    /// scheduler. The policy line cheaply carries the combat through states
    /// where its plan stage is unchanged; exact branching is paid only at a
    /// semantic handoff. A suffix can become authoritative only after its
    /// actions are joined to the exact prefix and replayed from the unchanged
    /// combat root.
    pub fn offer_plan_compatible_policy_line_with_suffix_probes(
        &mut self,
        max_turns: usize,
        max_actions: usize,
        suffix_generation_work: usize,
        stepper: &dyn CombatStepper,
    ) -> Result<LocalTurnGraphPolicyLineReport, String> {
        let mut report = LocalTurnGraphPolicyLineReport::default();
        if max_turns == 0
            || max_actions == 0
            || combat_plan_projection_v1(&self.original_root).is_none()
        {
            return Ok(report);
        }

        let mut node_id = 0usize;
        let mut path = Vec::<(usize, usize)>::new();
        let mut total_actions = 0usize;

        'turns: for _ in 0..max_turns {
            if total_actions >= max_actions {
                break;
            }
            let segment_root = self.nodes[node_id].generator.root().position().clone();
            let segment_identity_started = Instant::now();
            let segment_root_hash = exact_hash(&segment_root);
            report.action_identity_elapsed_ns = report
                .action_identity_elapsed_ns
                .saturating_add(elapsed_nanos_u64(segment_identity_started));
            let root_turn = segment_root.combat.turn.turn_count;
            let mut position = segment_root.clone();
            let mut actions = Vec::<TurnOptionAction>::new();
            let mut negative_log_policy = 0.0f64;

            while total_actions < max_actions {
                if stepper.terminal(&position) != CombatTerminal::Unresolved {
                    break;
                }
                let legal_surface_started = Instant::now();
                let surface = stepper.legal_action_surface(&position);
                report.legal_surface_elapsed_ns = report
                    .legal_surface_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(legal_surface_started));
                let policy_ranking_started = Instant::now();
                let choices = surface
                    .atomic_actions
                    .iter()
                    .map(CombatPolicyChoice::Atomic)
                    .chain(
                        surface
                            .selection_families
                            .iter()
                            .map(CombatPolicyChoice::StructuredSelection),
                    )
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    break;
                }
                let weights = self.policy.weights(&position, &choices);
                let weights = (weights.len() == choices.len())
                    .then_some(weights)
                    .unwrap_or_else(|| vec![1.0; choices.len()]);
                let probabilities = normalized_probabilities(
                    weights,
                    self.config.generator.uniform_exploration_ppm,
                );
                let mut ranked_indices = (0..choices.len()).collect::<Vec<_>>();
                ranked_indices.sort_by(|left, right| {
                    probabilities[*right]
                        .total_cmp(&probabilities[*left])
                        .then_with(|| left.cmp(right))
                });
                let mut selected = None;
                let mut first_neutral = None;
                let seek_timed_preference = combat_plan_has_timed_action_preference_v1(&position);
                report.policy_ranking_elapsed_ns = report
                    .policy_ranking_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(policy_ranking_started));
                let mut blocked_by_structured_family = false;
                let transition_preview_started = Instant::now();
                for candidate_index in ranked_indices {
                    if candidate_index >= surface.atomic_actions.len() {
                        let family = &surface.selection_families
                            [candidate_index - surface.atomic_actions.len()];
                        // A mandatory singleton choice is still a small exact
                        // action surface. Reuse the policy's member ordering
                        // and materialize only its principal member. Variable
                        // subsets remain lazy generator work: eagerly choosing
                        // one here would hide the very combinatorial boundary
                        // this proposer is meant to avoid.
                        if family.declared_min != 1 || family.effective_max != 1 {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let Ok(mut cursor) = SelectionTransactionCursor::new(family) else {
                            blocked_by_structured_family = true;
                            break;
                        };
                        let members =
                            std::iter::from_fn(|| cursor.next_input()).collect::<Vec<_>>();
                        if members.is_empty() {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let member_weights = self
                            .policy
                            .structured_selection_member_weights(&position, family, &members);
                        let member_weights = (member_weights.len() == members.len())
                            .then_some(member_weights)
                            .unwrap_or_else(|| vec![1.0; members.len()]);
                        let member_probabilities = normalized_probabilities(
                            member_weights,
                            self.config.generator.uniform_exploration_ppm,
                        );
                        let mut ranked_member_indices = (0..members.len()).collect::<Vec<_>>();
                        ranked_member_indices.sort_by(|left, right| {
                            member_probabilities[*right]
                                .total_cmp(&member_probabilities[*left])
                                .then_with(|| left.cmp(right))
                        });
                        let member_index = ranked_member_indices
                            .iter()
                            .copied()
                            .find(|member_index| {
                                let timing = combat_plan_selection_member_timing_v1(
                                    &position,
                                    family,
                                    &members[*member_index],
                                );
                                if matches!(timing, CombatPlanActionTimingV1::Defer(_)) {
                                    report.rejected_preview_transitions =
                                        report.rejected_preview_transitions.saturating_add(1);
                                    report.deferred_actions =
                                        report.deferred_actions.saturating_add(1);
                                    false
                                } else {
                                    true
                                }
                            })
                            // A mandatory choice remains executable even if
                            // every member consumes a reserved plan asset.
                            .unwrap_or(ranked_member_indices[0]);
                        let input = members[member_index].clone();
                        let step = stepper.apply_to_stable(
                            &position,
                            input.clone(),
                            CombatStepLimits {
                                max_engine_steps: self
                                    .config
                                    .generator
                                    .max_engine_steps_per_transition
                                    .max(1),
                                deadline: None,
                            },
                        );
                        report.engine_steps = report.engine_steps.saturating_add(step.engine_steps);
                        self.used.applied_action_transitions =
                            self.used.applied_action_transitions.saturating_add(1);
                        self.used.engine_steps =
                            self.used.engine_steps.saturating_add(step.engine_steps);
                        if step.truncated || step.timed_out {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let probability =
                            probabilities[candidate_index] * member_probabilities[member_index];
                        match combat_plan_action_timing_v1(&position, &step.position) {
                            CombatPlanActionTimingV1::PreferNow => {
                                selected = Some((input, probability, step));
                                break;
                            }
                            CombatPlanActionTimingV1::Neutral if seek_timed_preference => {
                                first_neutral.get_or_insert((input, probability, step));
                            }
                            CombatPlanActionTimingV1::Neutral => {
                                selected = Some((input, probability, step));
                                break;
                            }
                            CombatPlanActionTimingV1::Defer(_) => {
                                report.rejected_preview_transitions =
                                    report.rejected_preview_transitions.saturating_add(1);
                                report.deferred_actions = report.deferred_actions.saturating_add(1);
                            }
                        }
                        continue;
                    }
                    let input = surface.atomic_actions[candidate_index].clone();
                    let step = stepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: self
                                .config
                                .generator
                                .max_engine_steps_per_transition
                                .max(1),
                            deadline: None,
                        },
                    );
                    report.engine_steps = report.engine_steps.saturating_add(step.engine_steps);
                    self.used.applied_action_transitions =
                        self.used.applied_action_transitions.saturating_add(1);
                    self.used.engine_steps =
                        self.used.engine_steps.saturating_add(step.engine_steps);
                    if step.truncated || step.timed_out {
                        break;
                    }
                    match combat_plan_action_timing_v1(&position, &step.position) {
                        CombatPlanActionTimingV1::PreferNow => {
                            selected = Some((input, probabilities[candidate_index], step));
                            break;
                        }
                        CombatPlanActionTimingV1::Neutral if seek_timed_preference => {
                            first_neutral.get_or_insert((
                                input,
                                probabilities[candidate_index],
                                step,
                            ));
                        }
                        CombatPlanActionTimingV1::Neutral => {
                            selected = Some((input, probabilities[candidate_index], step));
                            break;
                        }
                        CombatPlanActionTimingV1::Defer(_) => {
                            report.rejected_preview_transitions =
                                report.rejected_preview_transitions.saturating_add(1);
                            report.deferred_actions = report.deferred_actions.saturating_add(1);
                        }
                    }
                }
                report.transition_preview_elapsed_ns = report
                    .transition_preview_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(transition_preview_started));
                if selected.is_none() && !blocked_by_structured_family {
                    selected = first_neutral;
                }
                let Some((selected_input, selected_probability, selected_step)) = selected else {
                    break;
                };
                report.chosen_action_transitions =
                    report.chosen_action_transitions.saturating_add(1);
                report.proposed_actions.push(selected_input.clone());

                negative_log_policy -= selected_probability.max(f64::MIN_POSITIVE).ln();
                let action_identity_started = Instant::now();
                actions.push(TurnOptionAction {
                    input: selected_input,
                    expected_successor_hash: exact_hash(&selected_step.position).into(),
                    engine_steps: selected_step.engine_steps,
                });
                report.action_identity_elapsed_ns = report
                    .action_identity_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(action_identity_started));
                total_actions = total_actions.saturating_add(1);
                position = selected_step.position;

                if stepper.terminal(&position) != CombatTerminal::Unresolved
                    || (matches!(
                        position.engine,
                        sts_core::state::core::EngineState::CombatPlayerTurn
                    ) && position.combat.turn.turn_count > root_turn)
                {
                    break;
                }
            }

            if actions.is_empty() {
                break;
            }
            let boundary = match stepper.terminal(&position) {
                CombatTerminal::Win => CompleteTurnOptionBoundary::TerminalWin,
                CombatTerminal::Loss => CompleteTurnOptionBoundary::TerminalLoss,
                CombatTerminal::Unresolved if position.combat.runtime.combat_smoked => {
                    CompleteTurnOptionBoundary::Escape
                }
                CombatTerminal::Unresolved
                    if matches!(
                        position.engine,
                        sts_core::state::core::EngineState::CombatPlayerTurn
                    ) && position.combat.turn.turn_count > root_turn =>
                {
                    CompleteTurnOptionBoundary::NextPlayerTurn
                }
                CombatTerminal::Unresolved => break,
            };
            let option = CompleteTurnOption::new(
                segment_root_hash,
                actions,
                boundary,
                position,
                negative_log_policy,
            );
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            report.proposed_turns = report.proposed_turns.saturating_add(1);

            match boundary {
                CompleteTurnOptionBoundary::TerminalWin => {
                    let (mut all_actions, prefix_negative_log_policy) = self.path_actions(&path);
                    all_actions.extend_from_slice(option.actions());
                    let witness = replay_witness(
                        &self.original_root,
                        &all_actions,
                        prefix_negative_log_policy + option.negative_log_policy(),
                        OracleCombatWitnessDiscoverySource::PlannerSearch,
                        stepper,
                    )
                    .map_err(|error| format!("policy-line terminal replay failed: {error:?}"))?;
                    self.remember_witness(witness);
                    report.reached_terminal_win = true;
                    break;
                }
                CompleteTurnOptionBoundary::TerminalLoss => {
                    self.used.terminal_losses = self.used.terminal_losses.saturating_add(1);
                    break;
                }
                CompleteTurnOptionBoundary::Escape => break,
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    let plan_annotation_started = Instant::now();
                    let crosses_plan_milestone = combat_plan_transition_annotation_v1(
                        &segment_root,
                        option.exact_successor(),
                    )
                    .is_some_and(|annotation| !annotation.completed_milestones().is_empty());
                    report.plan_annotation_elapsed_ns = report
                        .plan_annotation_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(plan_annotation_started));
                    if suffix_generation_work > 0
                        && crosses_plan_milestone
                        && self.offer_exact_suffix_probe(
                            node_id,
                            &path,
                            suffix_generation_work,
                            stepper,
                            &mut report,
                        )?
                    {
                        break 'turns;
                    }
                    let successor_admission_started = Instant::now();
                    let Some(successor_id) = self.accept_successor(node_id, &path, option) else {
                        return Err("policy-line successor was not admitted".to_owned());
                    };
                    let Some(edge_index) = self.nodes[node_id]
                        .children
                        .iter()
                        .position(|edge| edge.successor == successor_id)
                    else {
                        return Err("policy-line successor edge was not admitted".to_owned());
                    };
                    path.push((node_id, edge_index));
                    node_id = successor_id;
                    report.successor_admission_elapsed_ns = report
                        .successor_admission_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(successor_admission_started));
                }
            }
        }

        Ok(report)
    }

    fn offer_exact_suffix_probe(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        suffix_generation_work: usize,
        stepper: &dyn CombatStepper,
        report: &mut LocalTurnGraphPolicyLineReport,
    ) -> Result<bool, String> {
        let suffix_setup_started = Instant::now();
        let root = CombatDecisionRoot::new(self.nodes[node_id].generator.root().position().clone())
            .map_err(|error| format!("suffix probe root is not a decision boundary: {error:?}"))?;
        let mut suffix = LocalTurnGraphWitnessSession::with_policy(
            root,
            LocalTurnGraphWitnessConfig {
                satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
                ..self.config
            },
            self.policy.clone(),
        );
        report.suffix_probe_setup_elapsed_ns = report
            .suffix_probe_setup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_setup_started));
        let suffix_advance_started = Instant::now();
        let suffix_report = suffix.advance(
            LocalTurnGraphWitnessQuantum {
                additional_selections: suffix_generation_work.max(1),
                additional_generation_work: suffix_generation_work,
                additional_engine_steps: suffix_generation_work
                    .saturating_mul(self.config.generator.max_engine_steps_per_transition.max(1)),
                deadline: None,
            },
            stepper,
        );
        report.suffix_probe_advance_elapsed_ns = report
            .suffix_probe_advance_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_advance_started));
        report.suffix_probe_attempts = report.suffix_probe_attempts.saturating_add(1);
        report.suffix_probe_generation_work = report
            .suffix_probe_generation_work
            .saturating_add(suffix_report.counters.generation_work);
        report.suffix_probe_engine_steps = report
            .suffix_probe_engine_steps
            .saturating_add(suffix_report.counters.engine_steps);
        report.suffix_probe_completed_turn_options = report
            .suffix_probe_completed_turn_options
            .saturating_add(suffix_report.counters.completed_turn_options);
        report.suffix_probe_applied_action_transitions = report
            .suffix_probe_applied_action_transitions
            .saturating_add(suffix_report.counters.applied_action_transitions);
        report.suffix_probe_unique_successor_states = report
            .suffix_probe_unique_successor_states
            .saturating_add(suffix_report.counters.unique_successor_states);
        report.suffix_probe_exact_nodes = report
            .suffix_probe_exact_nodes
            .saturating_add(suffix_report.counters.exact_nodes);
        report.suffix_probe_exact_edges = report
            .suffix_probe_exact_edges
            .saturating_add(suffix_report.counters.exact_edges);
        report
            .suffix_probe_performance_timing
            .accumulate(suffix_report.performance_timing);
        let witness_found = suffix_report.witness.is_some();
        let final_hp = suffix_report
            .witness
            .as_ref()
            .map(|witness| witness.final_position.combat.entities.player.current_hp);
        report
            .suffix_probe_details
            .push(LocalTurnGraphSuffixProbeAttempt {
                exact_state_hash: exact_hash(self.nodes[node_id].generator.root().position()),
                player_turn: self.nodes[node_id]
                    .generator
                    .root()
                    .position()
                    .combat
                    .turn
                    .turn_count,
                plan_projection: combat_plan_projection_v1(
                    self.nodes[node_id].generator.root().position(),
                ),
                generation_work: suffix_report.counters.generation_work,
                engine_steps: suffix_report.counters.engine_steps,
                witness_found,
                final_hp,
            });

        let Some(suffix_witness) = suffix_report.witness else {
            return Ok(false);
        };
        let suffix_replay_started = Instant::now();
        let (mut actions, _) = self.path_actions(path);
        actions.extend(suffix_witness.actions);
        let final_hp_hint = suffix_witness
            .final_position
            .combat
            .entities
            .player
            .current_hp;
        let accepted = self
            .offer_witness_proposal(
                CombatPolicyWitnessProposal {
                    actions,
                    final_hp_hint,
                },
                stepper,
            )
            .map_err(|error| format!("combined suffix witness replay failed: {error:?}"))?;
        report.suffix_probe_replay_elapsed_ns = report
            .suffix_probe_replay_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_replay_started));
        report.suffix_probe_witness_found = accepted || self.witness.is_some();
        report.reached_terminal_win = report.suffix_probe_witness_found;
        Ok(report.suffix_probe_witness_found)
    }

    pub fn restore_verified_witness(&mut self, witness: OracleCombatWitness) -> Result<(), String> {
        if witness.final_position.combat.runtime.combat_smoked {
            return Err(
                "restored local-turn-graph witness is a Smoke Bomb escape, not a terminal victory"
                    .to_string(),
            );
        }
        if sts_core::sim::combat::combat_terminal(
            &witness.final_position.engine,
            &witness.final_position.combat,
        ) != CombatTerminal::Win
        {
            return Err("restored local-turn-graph witness is not terminal victory".to_string());
        }
        self.remember_witness(witness);
        Ok(())
    }

    pub fn counters(&self) -> LocalTurnGraphWitnessCounters {
        self.used.clone()
    }

    pub fn retained_state_work(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.generator.retained_work_items())
            .sum::<usize>()
            .saturating_add(self.nodes.iter().filter(|node| !node.exhausted).count())
    }

    pub fn progress_snapshot(&self) -> OracleCombatWitnessProgressSnapshot {
        let root = &self.nodes[0];
        let root_counters = root.generator.counters();
        let mut survival_by_turn =
            BTreeMap::<u32, (OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>::new();
        let mut deepest_survival = None::<(OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>;
        let mut deepest_progress = None::<(OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>;
        let mut max_path_atomic_depth = 0usize;
        for node_id in 0..self.nodes.len() {
            let actions = self.diagnostic_actions_to_node(node_id);
            max_path_atomic_depth = max_path_atomic_depth.max(actions.len());
            let state = local_deep_state_snapshot(&self.nodes[node_id], actions.len());
            let replace_turn =
                survival_by_turn
                    .get(&state.player_turn)
                    .is_none_or(|(current, _)| {
                        (state.player_hp, -state.enemy_total_hp, state.player_block)
                            > (
                                current.player_hp,
                                -current.enemy_total_hp,
                                current.player_block,
                            )
                    });
            if replace_turn {
                survival_by_turn.insert(state.player_turn, (state.clone(), actions.clone()));
            }
            let replace_survival = deepest_survival.as_ref().is_none_or(|(current, _)| {
                (
                    state.player_turn,
                    state.player_hp,
                    -state.enemy_total_hp,
                    state.player_block,
                ) > (
                    current.player_turn,
                    current.player_hp,
                    -current.enemy_total_hp,
                    current.player_block,
                )
            });
            if replace_survival {
                deepest_survival = Some((state.clone(), actions.clone()));
            }
            let replace_progress = deepest_progress.as_ref().is_none_or(|(current, _)| {
                (
                    state.player_turn,
                    -state.enemy_total_hp,
                    state.player_hp,
                    state.player_block,
                ) > (
                    current.player_turn,
                    -current.enemy_total_hp,
                    current.player_hp,
                    current.player_block,
                )
            });
            if replace_progress {
                deepest_progress = Some((state, actions));
            }
        }
        let recent_turn_survival_envelope = survival_by_turn
            .into_values()
            .rev()
            .take(32)
            .map(|(state, _)| state)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        OracleCombatWitnessProgressSnapshot {
            retained_states: self.nodes.iter().filter(|node| !node.exhausted).count(),
            queued_anchor_entries: self.nodes.iter().filter(|node| !node.exhausted).count(),
            queued_guided_entries: Vec::new(),
            guide_queues: Vec::new(),
            generation_gap_count: self.generation_gaps.len(),
            pending_witness_replay: false,
            root_state: Some(OracleCombatWitnessStateProgressSnapshot {
                exact_state_hash: exact_hash(root.generator.root().position()),
                path_atomic_depth: 0,
                path_negative_log_policy: 0.0,
                generator_work: root_counters.generation_work,
                generator_engine_steps: root_counters.engine_steps,
                completed_turn_options: root.generator.total_completed_options(),
                retained_generator_work_items: root.generator.retained_work_items(),
                synced_options: root.generated_options,
                anchor_states_ahead: None,
                guided_states_ahead: None,
                guided_lane_ranks: None,
            }),
            max_player_turn: self
                .nodes
                .iter()
                .map(|node| node.generator.root().position().combat.turn.turn_count)
                .max()
                .unwrap_or_default(),
            deepest_survival_state: deepest_survival.as_ref().map(|(state, _)| state.clone()),
            deepest_progress_state: deepest_progress.as_ref().map(|(state, _)| state.clone()),
            deepest_survival_actions: deepest_survival
                .map(|(_, actions)| actions)
                .unwrap_or_default(),
            deepest_progress_actions: deepest_progress
                .map(|(_, actions)| actions)
                .unwrap_or_default(),
            recent_turn_survival_envelope,
            max_path_atomic_depth,
            max_completed_turn_options_at_state: self
                .nodes
                .iter()
                .map(|node| node.generator.total_completed_options())
                .max()
                .unwrap_or_default(),
            ..OracleCombatWitnessProgressSnapshot::default()
        }
    }

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

    fn node_id_by_exact_hash(&self, exact_state_hash: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| node.generator.root().exact_state_hash() == exact_state_hash)
    }

    pub fn state_snapshot_by_exact_hash(
        &self,
        exact_state_hash: &str,
    ) -> Option<LocalTurnGraphStateSnapshot> {
        let node_id = self.node_id_by_exact_hash(exact_state_hash)?;
        let node = &self.nodes[node_id];
        let counters = node.generator.counters();
        let retained_guide_promises = node
            .generation_service_views
            .iter()
            .filter_map(|view| {
                let LocalServiceView::Guide(lane) = view else {
                    return None;
                };
                node.generator
                    .best_retained_guide_promise_snapshot(*lane)
                    .map(|promise| LocalTurnGraphRetainedGuidePromiseSnapshot {
                        lane: lane.value(),
                        rank: promise.rank.components().to_vec(),
                        atomic_depth: promise.atomic_depth,
                    })
            })
            .collect();
        Some(LocalTurnGraphStateSnapshot {
            exact_state_hash: exact_state_hash.to_owned(),
            relative_turn_depth: node.relative_turn_depth,
            visits: node.visits,
            generation_work: counters.generation_work,
            generator_engine_steps: counters.engine_steps,
            retained_generator_work_items: node.generator.retained_work_items(),
            generator_anchor_work_pops: node.generator.anchor_work_pops(),
            generator_guided_work_pops: node.generator.guided_work_pops(),
            best_retained_anchor_atomic_depth: node
                .generator
                .best_retained_path_bound_snapshot()
                .map(|(atomic_depth, _)| atomic_depth),
            retained_guide_promises,
            retained_lookahead_guides: node.generator.retained_lookahead_guides(),
            lookahead_pending_lane: node.lookahead_pending_lane.map(CombatGuideLaneId::value),
            generated_options: node.generated_options,
            children: node.children.len(),
            exhausted: node.exhausted,
        })
    }

    pub fn root_action_families(&self) -> Vec<LocalTurnGraphRootActionFamilySnapshot> {
        let mut snapshots = self
            .root_action_families
            .iter()
            .map(|family| self.root_action_family_snapshot(family))
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            left.best_root_negative_log_policy
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.best_root_negative_log_policy.unwrap_or(f64::INFINITY))
        });
        snapshots
    }

    pub fn edge_snapshot_by_exact_hashes(
        &self,
        parent_exact_state_hash: &str,
        successor_exact_state_hash: &str,
    ) -> Option<LocalTurnGraphEdgeSnapshot> {
        let parent_id = self.node_id_by_exact_hash(parent_exact_state_hash)?;
        let successor_id = self.node_id_by_exact_hash(successor_exact_state_hash)?;
        let parent = &self.nodes[parent_id];
        let edge = parent
            .children
            .iter()
            .find(|edge| edge.successor == successor_id)?;
        let successor = &self.nodes[successor_id];
        let mut pending_lookahead = parent
            .children
            .iter()
            .filter(|candidate| {
                !self.nodes[candidate.successor].exhausted
                    && self.nodes[candidate.successor]
                        .lookahead_pending_lane
                        .is_some()
            })
            .collect::<Vec<_>>();
        pending_lookahead.sort_by(|left, right| {
            local_path_base(left.actions.len(), left.negative_log_policy)
                .total_cmp(&local_path_base(
                    right.actions.len(),
                    right.negative_log_policy,
                ))
                .then_with(|| left.visits.cmp(&right.visits))
                .then_with(|| left.successor.cmp(&right.successor))
        });
        let lookahead_pending_rank = pending_lookahead
            .iter()
            .position(|candidate| candidate.successor == successor_id)
            .map(|index| index.saturating_add(1));
        let guide_service = successor
            .guides
            .iter()
            .map(|guide| {
                let mut candidates = parent
                    .children
                    .iter()
                    .filter(|candidate| !self.nodes[candidate.successor].exhausted)
                    .filter_map(|candidate| {
                        backed_guide_rank(candidate, &self.nodes[candidate.successor], guide.lane)
                            .map(|rank| (candidate, rank))
                    })
                    .collect::<Vec<_>>();
                candidates.sort_by(|(left_edge, left_rank), (right_edge, right_rank)| {
                    guide_choice_order(
                        left_rank,
                        local_path_base(left_edge.actions.len(), left_edge.negative_log_policy),
                        left_edge.visits,
                        left_edge.successor,
                        right_rank,
                        local_path_base(right_edge.actions.len(), right_edge.negative_log_policy),
                        right_edge.visits,
                        right_edge.successor,
                    )
                });
                let ordinal_rank = candidates
                    .iter()
                    .position(|(candidate, _)| candidate.successor == successor_id)
                    .map(|index| index.saturating_add(1))
                    .unwrap_or(0);
                LocalTurnGraphGuideServiceSnapshot {
                    lane: guide.lane.value(),
                    edge_visits: edge.guide_visits.get(&guide.lane).copied().unwrap_or(0),
                    ordinal_rank,
                    candidate_count: candidates.len(),
                    successor_rank: backed_guide_rank(edge, successor, guide.lane)
                        .unwrap_or(&guide.rank)
                        .components()
                        .to_vec(),
                    best_rank: candidates
                        .first()
                        .map(|(_, rank)| rank.components().to_vec())
                        .unwrap_or_default(),
                }
            })
            .collect();
        Some(LocalTurnGraphEdgeSnapshot {
            parent_visits: parent.visits,
            parent_generated_options: parent.generated_options,
            parent_children: parent.children.len(),
            parent_widen_anchor_visits: parent.widen_anchor_visits,
            actions: edge.actions.clone(),
            negative_log_policy: edge.negative_log_policy,
            plan_transition_annotation: edge.plan_transition_annotation.clone(),
            visits: edge.visits,
            anchor_visits: edge.anchor_visits,
            backed_visits: edge.backed_visits,
            backed_lookahead_rank: edge
                .backed_lookahead_rank
                .as_ref()
                .map(|rank| rank.components().to_vec()),
            lookahead_pending_rank,
            lookahead_pending_candidates: pending_lookahead.len(),
            guide_service,
            successor_visits: successor.visits,
            successor_generated_options: successor.generated_options,
            successor_children: successor.children.len(),
            successor_exhausted: successor.exhausted,
        })
    }

    pub fn plan_transition_edge_snapshots(&self) -> Vec<LocalTurnGraphPlanTransitionEdgeSnapshot> {
        let exact_hashes = self
            .nodes
            .iter()
            .map(|node| node.generator.root().exact_state_hash())
            .collect::<Vec<_>>();
        let mut snapshots = self
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(parent_id, parent)| {
                let exact_hashes = &exact_hashes;
                parent.children.iter().filter_map(move |edge| {
                    let plan_transition_annotation =
                        edge.plan_transition_annotation.as_ref()?.clone();
                    Some(LocalTurnGraphPlanTransitionEdgeSnapshot {
                        parent_exact_state_hash: exact_hashes[parent_id].to_owned(),
                        successor_exact_state_hash: exact_hashes[edge.successor].to_owned(),
                        parent_relative_turn_depth: parent.relative_turn_depth,
                        action_count: edge.actions.len(),
                        negative_log_policy: edge.negative_log_policy,
                        plan_transition_annotation,
                        edge_visits: edge.visits,
                        anchor_visits: edge.anchor_visits,
                        guide_visits: edge.guide_visits.values().copied().sum(),
                        backed_visits: edge.backed_visits,
                        successor_visits: self.nodes[edge.successor].visits,
                    })
                })
            })
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            left.parent_relative_turn_depth
                .cmp(&right.parent_relative_turn_depth)
                .then_with(|| {
                    left.parent_exact_state_hash
                        .cmp(&right.parent_exact_state_hash)
                })
                .then_with(|| {
                    left.successor_exact_state_hash
                        .cmp(&right.successor_exact_state_hash)
                })
        });
        snapshots
    }

    fn root_action_family_snapshot(
        &self,
        family: &LocalRootActionFamilyAccumulator,
    ) -> LocalTurnGraphRootActionFamilySnapshot {
        let root_successors = self.nodes[0]
            .children
            .iter()
            .filter(|edge| {
                edge.actions
                    .first()
                    .is_some_and(|action| action.input == family.first_action)
            })
            .map(|edge| edge.successor)
            .collect::<BTreeSet<_>>();
        let retained_next_turn_successors = root_successors
            .iter()
            .filter(|node_id| !self.nodes[**node_id].exhausted)
            .count();
        let mut pending = root_successors.iter().copied().collect::<VecDeque<_>>();
        let mut reachable = BTreeSet::new();
        while let Some(node_id) = pending.pop_front() {
            if !reachable.insert(node_id) {
                continue;
            }
            pending.extend(
                self.nodes[node_id]
                    .children
                    .iter()
                    .map(|edge| edge.successor),
            );
        }

        let mut max_player_turn = 0;
        let mut best_hp_at_max_turn = None;
        let mut lowest_enemy_hp_at_max_turn = None;
        let mut reachable_generation_work = 0usize;
        let mut reachable_completed_turn_options = 0usize;
        let mut reachable_retained_states = 0usize;
        for node_id in &reachable {
            let node = &self.nodes[*node_id];
            let position = node.generator.root().position();
            let turn = position.combat.turn.turn_count;
            let hp = position.combat.entities.player.current_hp;
            let enemy_hp = position
                .combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .map(|monster| monster.current_hp.max(0))
                .sum::<i32>();
            if turn > max_player_turn {
                max_player_turn = turn;
                best_hp_at_max_turn = Some(hp);
                lowest_enemy_hp_at_max_turn = Some(enemy_hp);
            } else if turn == max_player_turn {
                best_hp_at_max_turn =
                    Some(best_hp_at_max_turn.map_or(hp, |current| current.max(hp)));
                lowest_enemy_hp_at_max_turn = Some(
                    lowest_enemy_hp_at_max_turn.map_or(enemy_hp, |current| current.min(enemy_hp)),
                );
            }
            let counters = node.generator.counters();
            reachable_generation_work =
                reachable_generation_work.saturating_add(counters.generation_work);
            reachable_completed_turn_options = reachable_completed_turn_options
                .saturating_add(node.generator.total_completed_options());
            if !node.exhausted {
                reachable_retained_states = reachable_retained_states.saturating_add(1);
            }
        }

        LocalTurnGraphRootActionFamilySnapshot {
            first_action: family.first_action.clone(),
            best_root_negative_log_policy: family.best_root_negative_log_policy,
            completed_root_turn_options: family.completed_root_turn_options,
            terminal_wins: family.terminal_wins,
            terminal_losses: family.terminal_losses,
            escapes: family.escapes,
            unique_next_turn_successors: root_successors.len(),
            retained_next_turn_successors,
            reachable_exact_states: reachable.len(),
            reachable_retained_states,
            reachable_generation_work,
            reachable_completed_turn_options,
            max_player_turn,
            best_hp_at_max_turn,
            lowest_enemy_hp_at_max_turn,
        }
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
        self.generation_gaps.extend(new_gaps);

        let admission_started = Instant::now();
        for option in options {
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin => {
                    let (mut actions, prefix_negative_log_policy) = self.path_actions(path);
                    actions.extend_from_slice(option.actions());
                    match replay_witness(
                        &self.original_root,
                        &actions,
                        prefix_negative_log_policy + option.negative_log_policy(),
                        OracleCombatWitnessDiscoverySource::PlannerSearch,
                        stepper,
                    ) {
                        Ok(witness) => {
                            self.remember_witness(witness);
                        }
                        Err(error) => self.replay_failure = Some(error),
                    }
                    if self.witness_satisfies() {
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
        self.refresh_exhaustion(node_id);
        self.performance_timing.admission_elapsed_ns = self
            .performance_timing
            .admission_elapsed_ns
            .saturating_add(elapsed_nanos_u64(admission_started));
        true
    }

    fn witness_satisfies(&self) -> bool {
        let Some(witness) = self.witness.as_ref() else {
            return false;
        };
        if !witness_within_potion_budget(witness, self.config.max_potions_used) {
            return false;
        }
        match self.config.satisfaction {
            OracleCombatWitnessSatisfaction::FirstWitness => true,
            OracleCombatWitnessSatisfaction::HpLossAtMost(limit) => {
                let initial_hp = self.original_root.combat.entities.player.current_hp;
                let final_hp = witness.final_position.combat.entities.player.current_hp;
                initial_hp.saturating_sub(final_hp).max(0) as u32 <= limit
            }
            OracleCombatWitnessSatisfaction::BudgetOrExhaustion => false,
        }
    }

    fn remember_witness(&mut self, witness: OracleCombatWitness) -> bool {
        let replace = self.witness.as_ref().is_none_or(|current| {
            witness_better_with_potion_budget(&witness, current, self.config.max_potions_used)
        });
        if replace {
            self.witness = Some(witness);
        }
        replace
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
        let successor = if let Some(existing) =
            self.nodes_by_exact_key.get(&constrained_successor_key)
        {
            *existing
        } else {
            let Ok(root) = CombatDecisionRoot::with_exact_state_identity(
                option.exact_successor().clone(),
                successor_identity,
            ) else {
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
            node_id
        };

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
        self.backup_guides_along_path(path, parent_id, edge_index, &successor_backed_guides);
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

    fn diagnostic_actions_to_node(&self, mut node_id: usize) -> Vec<TurnOptionAction> {
        let mut path = Vec::new();
        while let Some(parent) = self.nodes[node_id].diagnostic_parent {
            path.push(parent);
            node_id = parent.0;
        }
        path.reverse();
        self.path_actions(&path).0
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

    fn snapshot(&self, status: LocalTurnGraphWitnessStatus) -> LocalTurnGraphWitnessReport {
        LocalTurnGraphWitnessReport {
            status,
            counters: self.used.clone(),
            performance_timing: self.performance_timing,
            root_visits: self.nodes[0].visits,
            root_generated_options: self.nodes[0].generated_options,
            root_children: self.nodes[0].children.len(),
            generation_gaps: self.generation_gaps.clone(),
            witness: self.witness.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backed_widen_due, backed_widen_quantum, boundary_service_views_from_guides,
        generator_needs_initial_grounding, guide_choice_order, guide_uses_progressive_service,
        guide_widen_service_due, local_path_service_cost, lookahead_acquisition_views_from_guides,
        progressive_candidate_index, progressive_guide_width, progressive_rollout_width,
        round_robin_available_index, select_path_service_view, update_max_guide, update_max_rank,
        GraphEdge, LocalServiceView,
    };
    use crate::policy::{
        CombatGuideLaneId, CombatStateGuide, CombatStateGuideRank, COMBAT_PLAN_STATE_GUIDE_LANE_V1,
    };

    fn edge(negative_log_policy: f64, visits: usize) -> GraphEdge {
        GraphEdge {
            successor: 0,
            actions: Vec::new(),
            negative_log_policy,
            plan_transition_annotation: None,
            visits,
            anchor_visits: visits,
            guide_visits: Default::default(),
            backed_guides: Default::default(),
            backed_lookahead_rank: None,
            backed_visits: 0,
        }
    }

    #[test]
    fn virtual_widen_and_materialized_child_share_one_local_service_currency() {
        let widen_before = local_path_service_cost(2, 0.5, 0);
        let child_before = local_path_service_cost(3, 0.7, 0);
        assert!(widen_before < child_before);

        let widen_after_service = local_path_service_cost(2, 0.5, 2);
        assert!(child_before < widen_after_service);
    }

    #[test]
    fn local_policy_service_cannot_permanently_starve_lower_prior_child() {
        let preferred = edge(0.0, 0);
        let alternate = edge(1.0, 0);
        let preferred_cost =
            preferred.negative_log_policy + (preferred.anchor_visits.saturating_add(1) as f64).ln();
        let alternate_cost =
            alternate.negative_log_policy + (alternate.anchor_visits.saturating_add(1) as f64).ln();
        assert!(preferred_cost < alternate_cost);

        let preferred_after_service = edge(0.0, 3);
        let preferred_after_cost = preferred_after_service.negative_log_policy
            + (preferred_after_service.anchor_visits.saturating_add(1) as f64).ln();
        assert!(alternate_cost < preferred_after_cost);
    }

    #[test]
    fn guide_exploits_its_best_child_while_anchor_owns_fairness() {
        let best = CombatStateGuideRank::new(vec![1, 0]);
        let alternate = CombatStateGuideRank::new(vec![0, 10_000]);

        assert!(
            guide_choice_order(&best, 100.0, usize::MAX, 9, &alternate, 0.0, 0, 1).is_lt(),
            "guide service debt must not overturn the guide's semantic ordering"
        );
    }

    #[test]
    fn guide_can_continue_a_stronger_unfinished_turn_before_deepening_a_child() {
        let retained_partial = CombatStateGuideRank::new(vec![2, 0]);
        let completed_child = CombatStateGuideRank::new(vec![1, 10_000]);

        assert!(
            guide_choice_order(
                &retained_partial,
                10.0,
                usize::MAX,
                usize::MAX,
                &completed_child,
                0.0,
                0,
                1,
            )
            .is_lt(),
            "a guide must compare its retained partial promise with completed boundary children"
        );
    }

    #[test]
    fn guide_interleaves_widen_and_deepen_service_when_both_are_live() {
        assert!(guide_widen_service_due(0, 0));
        assert!(!guide_widen_service_due(1, 0));
        assert!(guide_widen_service_due(1, 1));
        assert!(!guide_widen_service_due(2, 1));
    }

    #[test]
    fn expensive_guide_opens_competitors_logarithmically() {
        assert_eq!(progressive_guide_width(0), 1);
        assert_eq!(progressive_guide_width(1), 2);
        assert_eq!(progressive_guide_width(3), 3);
        assert_eq!(progressive_guide_width(7), 4);
        assert_eq!(progressive_guide_width(15), 5);
    }

    #[test]
    fn expensive_guide_services_widen_as_a_ranked_peer() {
        // Widen is first in guide order. The first services go to it; once the
        // square-root progressive window opens, the unserved materialized
        // child gets service instead of either side permanently monopolizing
        // the lane.
        assert_eq!(progressive_candidate_index(0, [0, 0]), Some(0));
        assert_eq!(progressive_candidate_index(1, [1, 0]), Some(0));
        assert_eq!(progressive_candidate_index(3, [1, 0]), Some(1));
    }

    #[test]
    fn only_the_configured_expensive_guide_is_progressive() {
        let lookahead = CombatGuideLaneId::new(91);
        let ordinary = CombatGuideLaneId::new(92);

        assert!(!guide_uses_progressive_service(
            COMBAT_PLAN_STATE_GUIDE_LANE_V1,
            Some(lookahead)
        ));
        assert!(guide_uses_progressive_service(lookahead, Some(lookahead)));
        assert!(!guide_uses_progressive_service(ordinary, Some(lookahead)));
    }

    #[test]
    fn one_tree_service_preserves_its_semantic_view_across_depth() {
        let available = [
            LocalServiceView::Anchor,
            LocalServiceView::Guide(crate::policy::CombatGuideLaneId::new(6)),
        ];
        let mut next = 0;
        let root = select_path_service_view(None, &available, &mut next);
        assert_eq!(root, LocalServiceView::Anchor);
        assert_eq!(next, 1);

        let inherited = LocalServiceView::Guide(crate::policy::CombatGuideLaneId::new(6));
        assert_eq!(
            select_path_service_view(Some(inherited), &available, &mut next),
            inherited
        );
        assert_eq!(
            next, 1,
            "a descendant must not consume a fresh local lane rotation"
        );
    }

    #[test]
    fn backed_value_is_monotone_and_keeps_the_best_descendant() {
        let weak = CombatStateGuideRank::new(vec![1, 2]);
        let strong = CombatStateGuideRank::new(vec![1, 3]);
        let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
        let mut backed = None;

        assert!(update_max_rank(&mut backed, &weak));
        assert!(update_max_rank(&mut backed, &strong));
        assert!(!update_max_rank(&mut backed, &weaker_later));
        assert_eq!(backed, Some(strong));
    }

    #[test]
    fn semantic_guide_backup_is_monotone_per_lane() {
        let lane = crate::policy::CombatGuideLaneId::new(4);
        let weak = CombatStateGuideRank::new(vec![1, 2]);
        let strong = CombatStateGuideRank::new(vec![1, 3]);
        let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
        let mut backed = std::collections::BTreeMap::new();

        assert!(update_max_guide(&mut backed, lane, &weak));
        assert!(update_max_guide(&mut backed, lane, &strong));
        assert!(!update_max_guide(&mut backed, lane, &weaker_later));
        assert_eq!(backed.get(&lane), Some(&strong));
    }

    #[test]
    fn backed_search_balances_widen_and_deepen_service() {
        assert!(backed_widen_due(0, 0, true));
        assert!(!backed_widen_due(1, 0, true));
        assert!(backed_widen_due(1, 1, true));
        assert!(!backed_widen_due(2, 1, true));
        assert!(!backed_widen_due(2, usize::MAX, false));
    }

    #[test]
    fn backed_burst_deepens_selected_subtrees_without_widening_the_root() {
        assert_eq!(backed_widen_quantum(0, 4, 256), 4);
        assert_eq!(backed_widen_quantum(1, 4, 256), 256);
        assert_eq!(backed_widen_quantum(17, 4, 256), 256);
    }

    #[test]
    fn live_generator_receives_initial_grounding_even_if_an_external_edge_exists() {
        assert!(generator_needs_initial_grounding(0, false));
        assert!(!generator_needs_initial_grounding(1, false));
        assert!(!generator_needs_initial_grounding(0, true));
    }

    #[test]
    fn each_acquisition_view_gets_a_progressively_widened_rollout_window() {
        assert_eq!(progressive_rollout_width(0), 1);
        assert_eq!(progressive_rollout_width(3), 2);
        assert_eq!(progressive_rollout_width(8), 3);
        assert_eq!(progressive_rollout_width(783), 28);
    }

    #[test]
    fn lookahead_acquisition_rotates_across_available_semantic_views() {
        let available = [true, true, false];
        assert_eq!(round_robin_available_index(0, &available), Some(0));
        assert_eq!(round_robin_available_index(1, &available), Some(1));
        assert_eq!(round_robin_available_index(2, &available), Some(0));
        assert_eq!(round_robin_available_index(7, &available), Some(1));
        assert_eq!(round_robin_available_index(0, &[false, false, false]), None);
    }

    #[test]
    fn cheap_guides_acquire_lookahead_and_keep_their_proven_traversal() {
        let cheap_lane = CombatGuideLaneId::new(4);
        let lookahead_lane = CombatGuideLaneId::new(6);
        let guides = vec![
            CombatStateGuide::new(cheap_lane, vec![1]),
            CombatStateGuide::new(lookahead_lane, vec![1]),
        ];

        let traversal = boundary_service_views_from_guides(&guides, Some(lookahead_lane));
        assert_eq!(
            traversal,
            vec![
                LocalServiceView::Anchor,
                LocalServiceView::LookaheadEvaluation,
                LocalServiceView::Guide(cheap_lane),
            ]
        );

        assert_eq!(
            lookahead_acquisition_views_from_guides(&guides, Some(lookahead_lane)),
            vec![
                LocalServiceView::Anchor,
                LocalServiceView::Guide(cheap_lane)
            ]
        );
    }
}
