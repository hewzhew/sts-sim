use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::time::Instant;

use serde::Serialize;
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::ClientInput;

use super::generator::TurnOptionGeneratorPreferredLane;
use super::policy::{
    CombatGuideLaneId, CombatPolicyWitnessProposal, CombatStateGuide, CombatStateGuideRank,
    SharedCombatActionPolicy, SharedCombatLookaheadEvaluator,
};
use super::types::{
    exact_hash, CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOption,
    CompleteTurnOptionBoundary, TurnOptionAction, TurnOptionGenerationGap,
    TurnOptionGeneratorConfig,
};
use super::witness_search::{
    OracleCombatDeepStateSnapshot, OracleCombatWitness, OracleCombatWitnessDiscoverySource,
    OracleCombatWitnessProgressSnapshot, OracleCombatWitnessReplayError,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessStateProgressSnapshot,
};
use super::TurnOptionGeneratorSession;

/// Resumable search over a shared graph of exact player-turn boundaries.
///
/// Complete-turn generation remains lazy, but Widen and Deepen are decided at
/// the node that owns the alternatives. A deep path therefore does not have
/// to compete against every shallower generator in one global queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    /// One deterministic service unit for a selected node's resumable turn
    /// generator. This controls preemption granularity, not search quality.
    pub generation_quantum_work: usize,
    /// Coherent generator service after an exact boundary has earned backed
    /// exploitation. It remains preemptible at the graph level while avoiding
    /// repeated four-work drips on the selected expensive edge.
    pub backed_generation_quantum_work: usize,
    /// Deterministic work reserved for the first expansion of a selected exact
    /// turn-boundary node. Later resumptions return to the small quantum.
    pub initial_expansion_work: usize,
    /// Root-only discovery batch. Root proposals gate every deeper path, so
    /// they receive a wider but still bounded first expansion.
    pub root_initial_expansion_work: usize,
    /// Maximum number of exact states that may receive an optional expensive
    /// lookahead evaluation during this session.
    pub lookahead_max_evaluations: usize,
    /// Maximum deterministic evaluator work charged to one exact state.
    pub lookahead_work_per_evaluation: usize,
    pub max_turn_depth: usize,
    pub satisfaction: OracleCombatWitnessSatisfaction,
}

impl Default for LocalTurnGraphWitnessConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            generation_quantum_work: 4,
            backed_generation_quantum_work: 256,
            initial_expansion_work: 64,
            root_initial_expansion_work: 2_048,
            lookahead_max_evaluations: 384,
            lookahead_work_per_evaluation: 24,
            max_turn_depth: 32,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        }
    }
}

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

#[derive(Clone, Debug)]
pub struct LocalTurnGraphWitnessReport {
    pub status: LocalTurnGraphWitnessStatus,
    pub counters: LocalTurnGraphWitnessCounters,
    pub root_visits: usize,
    pub root_generated_options: usize,
    pub root_children: usize,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub witness: Option<OracleCombatWitness>,
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
    lookahead_lane: Option<CombatGuideLaneId>,
    nodes: Vec<GraphNode>,
    nodes_by_hash: HashMap<String, usize>,
    used: LocalTurnGraphWitnessCounters,
    granted_selections: usize,
    granted_generation_work: usize,
    granted_engine_steps: usize,
    generation_gaps: Vec<TurnOptionGenerationGap>,
    root_action_families: Vec<LocalRootActionFamilyAccumulator>,
    witness: Option<OracleCombatWitness>,
    replay_failure: Option<OracleCombatWitnessReplayError>,
}

impl LocalTurnGraphWitnessSession {
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
        let root_hash = root.exact_state_hash().to_owned();
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
        let root_generation_service_views =
            generation_service_views(policy.as_ref(), root.position());
        // Expensive lookahead evaluates exact player-turn boundaries. Atomic
        // partial states remain the generator's private proposal mechanism;
        // evaluating them here would reintroduce an independent inner search.
        let generator =
            TurnOptionGeneratorSession::with_policy(root.clone(), config.generator, policy.clone());
        Self {
            original_root,
            config,
            policy,
            lookahead_evaluator,
            lookahead_lane: root_lookahead_pending_lane,
            nodes: vec![GraphNode {
                generator,
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
            nodes_by_hash: HashMap::from([(root_hash, 0)]),
            used: LocalTurnGraphWitnessCounters {
                exact_nodes: 1,
                ..LocalTurnGraphWitnessCounters::default()
            },
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
        let replace = self
            .witness
            .as_ref()
            .is_none_or(|current| witness_better(&witness, current));
        if replace {
            self.witness = Some(witness);
        }
        Ok(replace)
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
        if self
            .witness
            .as_ref()
            .is_none_or(|current| witness_better(&witness, current))
        {
            self.witness = Some(witness);
        }
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

            match self.select_work() {
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
                    if lookahead_needs_exact_grounding(
                        self.nodes[node_id].generated_options,
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

    pub fn state_snapshot_by_exact_hash(
        &self,
        exact_state_hash: &str,
    ) -> Option<LocalTurnGraphStateSnapshot> {
        let node_id = *self.nodes_by_hash.get(exact_state_hash)?;
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
        let parent_id = *self.nodes_by_hash.get(parent_exact_state_hash)?;
        let successor_id = *self.nodes_by_hash.get(successor_exact_state_hash)?;
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
        let requested_work = if node_id == 0 && self.nodes[node_id].generated_options == 0 {
            self.config.root_initial_expansion_work
        } else if self.nodes[node_id].generated_options == 0 {
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

        let (
            before,
            after,
            before_lookahead_evaluations,
            after_lookahead_evaluations,
            before_lookahead_work,
            after_lookahead_work,
            before_diagnostics,
            after_diagnostics,
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
                options,
                gaps,
            )
        };

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
        self.generation_gaps.extend(new_gaps);

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
                            if self
                                .witness
                                .as_ref()
                                .is_none_or(|current| witness_better(&witness, current))
                            {
                                self.witness = Some(witness);
                            }
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
                    self.accept_successor(node_id, path, option);
                }
            }
        }
        self.refresh_exhaustion(node_id);
        true
    }

    fn witness_satisfies(&self) -> bool {
        let Some(witness) = self.witness.as_ref() else {
            return false;
        };
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

    fn accept_successor(
        &mut self,
        parent_id: usize,
        path: &[(usize, usize)],
        option: CompleteTurnOption,
    ) {
        let relative_turn_depth = self.nodes[parent_id].relative_turn_depth.saturating_add(1);
        if relative_turn_depth > self.config.max_turn_depth {
            self.used.depth_limited_successors =
                self.used.depth_limited_successors.saturating_add(1);
            return;
        }

        let successor_hash = option.exact_successor_hash().to_owned();
        let successor = if let Some(existing) = self.nodes_by_hash.get(&successor_hash) {
            *existing
        } else {
            let Ok(root) = CombatDecisionRoot::new(option.exact_successor().clone()) else {
                return;
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
            let generation_service_views =
                generation_service_views(self.policy.as_ref(), root.position());
            let node_id = self.nodes.len();
            let generator = TurnOptionGeneratorSession::with_policy(
                root.clone(),
                self.config.generator,
                self.policy.clone(),
            );
            self.nodes.push(GraphNode {
                generator,
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
            self.nodes_by_hash.insert(successor_hash, node_id);
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
            let parent = &mut self.nodes[parent_id];
            let edge_index = parent.children.len();
            parent.children.push(GraphEdge {
                successor,
                actions: option.actions().to_vec(),
                negative_log_policy: option.negative_log_policy(),
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
            edge_index
        };
        self.backup_guides_along_path(path, parent_id, edge_index, &successor_backed_guides);
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
            root_visits: self.nodes[0].visits,
            root_generated_options: self.nodes[0].generated_options,
            root_children: self.nodes[0].children.len(),
            generation_gaps: self.generation_gaps.clone(),
            witness: self.witness.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum LocalWorkChoice {
    Widen {
        view: LocalServiceView,
    },
    Edge {
        edge_index: usize,
        view: LocalServiceView,
    },
}

fn select_path_service_view(
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

fn select_local_work(
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
            progressive_guide_lane == Some(lane),
        ),
    }
}

fn select_pending_lookahead_work(node: &GraphNode, nodes: &[GraphNode]) -> Option<LocalWorkChoice> {
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

fn select_anchor_work(
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

fn select_backed_edge(node: &GraphNode, nodes: &[GraphNode]) -> Option<usize> {
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

fn select_pending_lookahead_edge(
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

fn round_robin_available_index(start: usize, available: &[bool]) -> Option<usize> {
    if available.is_empty() {
        return None;
    }
    (0..available.len())
        .map(|offset| start.wrapping_add(offset) % available.len())
        .find(|index| available[*index])
}

fn backed_widen_due(widen_services: usize, deepen_services: usize, can_widen: bool) -> bool {
    can_widen && guide_widen_service_due(widen_services, deepen_services)
}

fn backed_widen_quantum(node_id: usize, regular_work: usize, backed_work: usize) -> usize {
    if node_id == 0 {
        regular_work
    } else {
        backed_work
    }
}

fn lookahead_needs_exact_grounding(generated_options: usize, generator_finished: bool) -> bool {
    generated_options == 0 && !generator_finished
}

fn progressive_rollout_width(total_service: usize) -> usize {
    ((total_service.saturating_add(1) as f64).sqrt() as usize).max(1)
}

fn update_max_rank(
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

fn update_max_guide(
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

fn select_guide_work(
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

fn progressive_guide_width(total_service: usize) -> usize {
    (usize::BITS - total_service.saturating_add(1).leading_zeros()) as usize
}

fn progressive_candidate_index(
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

fn guide_widen_service_due(widen_visits: usize, deepen_visits: usize) -> bool {
    widen_visits <= deepen_visits
}

fn guide_choice_order(
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

fn local_path_base(atomic_depth: usize, negative_log_policy: f64) -> f64 {
    negative_log_policy + (atomic_depth.max(1) as f64).ln()
}

fn local_path_service_cost(atomic_depth: usize, negative_log_policy: f64, services: usize) -> f64 {
    local_path_base(atomic_depth, negative_log_policy) + (services.saturating_add(1) as f64).ln()
}

fn guide_rank(node: &GraphNode, lane: CombatGuideLaneId) -> Option<&CombatStateGuideRank> {
    node.guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| &guide.rank)
}

fn backed_guide_rank<'a>(
    edge: &'a GraphEdge,
    successor: &'a GraphNode,
    lane: CombatGuideLaneId,
) -> Option<&'a CombatStateGuideRank> {
    edge.backed_guides
        .get(&lane)
        .or_else(|| guide_rank(successor, lane))
}

fn guides_with_pending_lookahead(
    policy: &dyn super::policy::CombatActionPolicy,
    evaluator: Option<&dyn super::policy::CombatLookaheadEvaluator>,
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

fn guide_rank_map(
    guides: &[CombatStateGuide],
) -> BTreeMap<CombatGuideLaneId, CombatStateGuideRank> {
    guides
        .iter()
        .map(|guide| (guide.lane, guide.rank.clone()))
        .collect()
}

fn boundary_service_views_from_guides(
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

fn lookahead_acquisition_views_from_guides(
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

fn generation_service_views(
    policy: &dyn super::policy::CombatActionPolicy,
    position: &CombatPosition,
) -> Vec<LocalServiceView> {
    let lanes = policy
        .turn_generation_guides(position)
        .into_iter()
        .map(|guide| guide.lane)
        .collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

fn replay_witness(
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
        discovery_source,
    })
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn witness_better(left: &OracleCombatWitness, right: &OracleCombatWitness) -> bool {
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

fn local_deep_state_snapshot(
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

#[cfg(test)]
mod tests {
    use super::{
        backed_widen_due, backed_widen_quantum, boundary_service_views_from_guides,
        guide_choice_order, guide_widen_service_due, local_path_service_cost,
        lookahead_acquisition_views_from_guides, lookahead_needs_exact_grounding,
        progressive_candidate_index, progressive_guide_width, progressive_rollout_width,
        round_robin_available_index, select_path_service_view, update_max_guide, update_max_rank,
        GraphEdge, LocalServiceView,
    };
    use crate::policy::{CombatGuideLaneId, CombatStateGuide, CombatStateGuideRank};

    fn edge(negative_log_policy: f64, visits: usize) -> GraphEdge {
        GraphEdge {
            successor: 0,
            actions: Vec::new(),
            negative_log_policy,
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
    fn evaluated_live_boundary_receives_one_exact_grounding_expansion() {
        assert!(lookahead_needs_exact_grounding(0, false));
        assert!(!lookahead_needs_exact_grounding(1, false));
        assert!(!lookahead_needs_exact_grounding(0, true));
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
