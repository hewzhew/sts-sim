use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use rustc_hash::{FxBuildHasher, FxHasher};
use sts_core::ai::combat_state_key::{combat_exact_state_key, CombatExactStateKey};
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::{ClientInput, EngineState};

use super::policy::{
    normalized_probabilities, uniform_policy, CombatGuideLaneId, CombatPolicyChoice,
    CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy,
    SharedCombatLookaheadEvaluator,
};
use super::selection_transaction::SelectionTransactionCursor;
use super::types::{
    exact_hash, supported_boundary, CombatDecisionRoot, CombatPlanningCounters,
    CombatPlanningQuantum, CompleteTurnOption, GenerationInterruption, ReplaySuccessorHash,
    TurnOptionAction, TurnOptionGenerationDiagnostics, TurnOptionGenerationGap,
    TurnOptionGenerationGapKind, TurnOptionGenerationReport, TurnOptionGenerationStatus,
    TurnOptionGeneratorConfig,
};

/// High-frequency diagnostic sub-timers sample one transition per interval.
/// Parent stage timers remain exhaustive; this keeps Windows QPC overhead
/// from becoming a material part of the search being measured.
pub const DETAIL_TIMING_SAMPLE_INTERVAL: usize = 16;

#[derive(Clone, Debug)]
struct PartialTurnOption {
    position: CombatPosition,
    trace: Option<Arc<PendingActionTrace>>,
    atomic_depth: usize,
    negative_log_policy: f64,
    potion_expenditures: u32,
    generation_guides: Option<Arc<[CombatStateGuide]>>,
    lookahead_guide: Option<CombatStateGuide>,
}

#[derive(Debug)]
struct PendingActionTrace {
    // Generator branches share exact prefixes. Durable replay hashes are
    // materialized only when a complete segment is published, then cached on
    // the shared prefix node so sibling segments never re-hash that state.
    parent: Option<Arc<Self>>,
    input: ClientInput,
    successor_key: Arc<CombatExactStateKey>,
    engine_steps: usize,
    depth: usize,
}

impl PartialTurnOption {
    fn action_depth(&self) -> usize {
        self.trace.as_ref().map_or(0, |trace| trace.depth)
    }

    fn materialize_actions(&self) -> Vec<TurnOptionAction> {
        let mut actions = Vec::with_capacity(self.action_depth());
        let mut cursor = self.trace.clone();
        while let Some(trace) = cursor {
            actions.push(TurnOptionAction {
                input: trace.input.clone(),
                expected_successor_hash: ReplaySuccessorHash::from_exact_key(
                    trace.successor_key.clone(),
                ),
                engine_steps: trace.engine_steps,
            });
            cursor = trace.parent.clone();
        }
        actions.reverse();
        actions
    }
}

#[derive(Clone, Debug)]
struct ActionTransitionWork {
    parent: Arc<PartialTurnOption>,
    input: ClientInput,
    atomic_depth: usize,
    negative_log_policy: f64,
}

#[derive(Clone, Debug)]
struct AtomicActionCandidate {
    input: ClientInput,
    conditional_probability: f64,
    negative_log_policy: f64,
}

#[derive(Clone, Debug)]
struct AtomicActionCursorWork {
    parent: Arc<PartialTurnOption>,
    candidates: Vec<AtomicActionCandidate>,
    next_candidate: usize,
    guides: Arc<[CombatStateGuide]>,
}

impl AtomicActionCursorWork {
    fn new(
        parent: Arc<PartialTurnOption>,
        inputs: Vec<ClientInput>,
        probabilities: Vec<f64>,
        guides: impl Into<Arc<[CombatStateGuide]>>,
    ) -> Option<Self> {
        let mut candidates = inputs
            .into_iter()
            .zip(probabilities)
            .map(|(input, conditional_probability)| AtomicActionCandidate {
                input,
                conditional_probability,
                negative_log_policy: parent.negative_log_policy - conditional_probability.ln(),
            })
            .collect::<Vec<_>>();
        // Stable ordering preserves the simulator's canonical surface order
        // for equal policy mass while exposing the most likely concrete edge
        // first.
        candidates.sort_by(|left, right| {
            right
                .conditional_probability
                .total_cmp(&left.conditional_probability)
        });
        (!candidates.is_empty()).then_some(Self {
            parent,
            candidates,
            next_candidate: 0,
            guides: guides.into(),
        })
    }

    fn current_transition(&self) -> Option<ActionTransitionWork> {
        let candidate = self.candidates.get(self.next_candidate)?;
        Some(ActionTransitionWork {
            parent: self.parent.clone(),
            input: candidate.input.clone(),
            atomic_depth: self.parent.atomic_depth.saturating_add(1),
            negative_log_policy: candidate.negative_log_policy,
        })
    }

    fn consume_current(&mut self) {
        self.next_candidate = self.next_candidate.saturating_add(1);
    }

    fn remaining_candidate_count(&self) -> usize {
        self.candidates.len().saturating_sub(self.next_candidate)
    }

    fn priority(&self) -> Option<GeneratorWorkPriority> {
        let remaining_probability = self.candidates[self.next_candidate..]
            .iter()
            .map(|candidate| candidate.conditional_probability)
            .sum::<f64>();
        (remaining_probability > 0.0).then(|| {
            GeneratorWorkPriority::for_path(
                self.parent.atomic_depth.saturating_add(1),
                self.parent.negative_log_policy - remaining_probability.ln(),
            )
        })
    }
}

#[derive(Clone, Debug)]
struct StructuredSelectionWork {
    parent: Arc<PartialTurnOption>,
    cursor: SelectionTransactionCursor,
    family_negative_log_policy: f64,
    remaining_conditional_mass: f64,
}

#[derive(Clone, Debug)]
enum GeneratorWork {
    Expand(PartialTurnOption),
    AtomicActions(AtomicActionCursorWork),
    ApplyAction(ActionTransitionWork),
    StructuredSelection(StructuredSelectionWork),
}

impl GeneratorWork {
    fn position(&self) -> &CombatPosition {
        match self {
            Self::Expand(partial) => &partial.position,
            Self::AtomicActions(actions) => &actions.parent.position,
            Self::ApplyAction(action) => &action.parent.position,
            Self::StructuredSelection(selection) => &selection.parent.position,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionTransitionStatus {
    Consumed,
    TimedOut,
}

#[derive(Clone, Debug)]
struct IndexedExactStateKey {
    // This hash is private to the in-memory transposition set. Equality still
    // compares the complete typed key, so it cannot change exact-state
    // semantics or the durable v1 hashes written to witnesses.
    structural_hash: u64,
    key: Arc<CombatExactStateKey>,
    /// Finite caller-owned resources are part of constrained search identity.
    /// Without a finite potion contract, `None` preserves ordinary exact-state
    /// transposition.
    potion_expenditures: Option<u32>,
}

impl PartialEq for IndexedExactStateKey {
    fn eq(&self, other: &Self) -> bool {
        self.structural_hash == other.structural_hash
            && self.key == other.key
            && self.potion_expenditures == other.potion_expenditures
    }
}

impl Eq for IndexedExactStateKey {}

impl Hash for IndexedExactStateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.structural_hash.hash(state);
        self.potion_expenditures.hash(state);
    }
}

impl IndexedExactStateKey {
    #[cfg(test)]
    fn new(key: CombatExactStateKey, potion_expenditures: Option<u32>) -> Self {
        Self::from_arc(Arc::new(key), potion_expenditures)
    }

    fn from_arc(key: Arc<CombatExactStateKey>, potion_expenditures: Option<u32>) -> Self {
        // This is a trusted, process-local bucket hash. Full typed equality is
        // still checked below, so collisions cannot merge simulator states.
        let mut hasher = FxHasher::default();
        key.hash(&mut hasher);
        Self {
            structural_hash: hasher.finish(),
            key,
            potion_expenditures,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GeneratorWorkPriority {
    levin_log_priority: f64,
    atomic_depth: usize,
    negative_log_policy: f64,
}

impl GeneratorWorkPriority {
    fn for_path(atomic_depth: usize, negative_log_policy: f64) -> Self {
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

struct GeneratorQueueEntry {
    priority: GeneratorWorkPriority,
    sequence_id: u64,
    work_id: usize,
}

#[derive(Clone, Debug)]
struct GuidedGeneratorQueueEntry {
    guide_lane: CombatGuideLaneId,
    work_id: usize,
    sequence_id: u64,
    guide_rank: CombatStateGuideRank,
    anchor_priority: GeneratorWorkPriority,
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

struct GuidedGeneratorFrontier {
    lane: CombatGuideLaneId,
    entries: BinaryHeap<GuidedGeneratorQueueEntry>,
}

#[derive(Clone, Debug)]
pub(crate) struct RetainedGuidePromise {
    pub(crate) rank: CombatStateGuideRank,
    pub(crate) atomic_depth: usize,
    pub(crate) negative_log_policy: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TurnOptionGeneratorPreferredLane {
    Anchor,
    Guide(CombatGuideLaneId),
}

/// Read-only lifecycle information for one exact atomic transition which is
/// still waiting inside a lazy turn generator.  This distinguishes a missing
/// action from an action hidden behind a resumable sibling cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveActionTransitionSnapshot {
    pub candidate_ordinal: usize,
    pub remaining_candidate_count: usize,
    pub conditional_probability: f64,
    pub candidate_negative_log_policy: f64,
    pub cursor_negative_log_policy: f64,
    pub anchor_queue_rank: usize,
    pub guide_queue_ranks: Vec<usize>,
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

pub struct TurnOptionGeneratorSession {
    root: CombatDecisionRoot,
    config: TurnOptionGeneratorConfig,
    /// Maximum potion uses/discards allowed inside this generated turn.
    ///
    /// The run-level graph supplies the remaining combat allowance at each
    /// exact turn boundary. Enforcing it here prevents an over-budget potion
    /// prefix from consuming the complete-turn search before a legal line is
    /// ever released.
    max_potion_expenditures: Option<u32>,
    policy: SharedCombatActionPolicy,
    work: Vec<Option<GeneratorWork>>,
    anchor_frontier: BinaryHeap<GeneratorQueueEntry>,
    guided_frontiers: Vec<GuidedGeneratorFrontier>,
    next_scheduler_lane: usize,
    /// Frozen heads which still belong to the current scheduling round.
    ///
    /// This must survive `advance` boundaries. Re-snapshotting after every
    /// wall-clock or work interruption lets newly published heads repeatedly
    /// overtake the unserved tail of the prior round, making split quanta
    /// semantically different from one continuous grant.
    scheduled_round: VecDeque<(usize, usize)>,
    live_work_items: usize,
    next_sequence_id: u64,
    seen: HashSet<IndexedExactStateKey, FxBuildHasher>,
    completed: Vec<CompleteTurnOption>,
    total_completed_options: usize,
    gaps: Vec<TurnOptionGenerationGap>,
    applied_action_transitions: usize,
    duplicate_exact_successors: usize,
    atomic_state_expansions: usize,
    atomic_expand_services: usize,
    anchor_work_pops: usize,
    guided_work_pops: usize,
    lookahead_evaluator: Option<SharedCombatLookaheadEvaluator>,
    lookahead_evaluations: usize,
    lookahead_work: usize,
    atomic_expand_elapsed_ns: u64,
    transition_simulation_elapsed_ns: u64,
    transition_identity_elapsed_ns: u64,
    transition_key_build_elapsed_ns: u64,
    transition_key_index_elapsed_ns: u64,
    transition_admission_elapsed_ns: u64,
    transition_trace_elapsed_ns: u64,
    transition_seen_elapsed_ns: u64,
    transition_publish_elapsed_ns: u64,
    transition_publish_trace_node_elapsed_ns: u64,
    transition_publish_boundary_elapsed_ns: u64,
    transition_publish_complete_elapsed_ns: u64,
    transition_publish_push_elapsed_ns: u64,
    transition_publish_guide_elapsed_ns: u64,
    transition_publish_retain_elapsed_ns: u64,
    transition_publish_agenda_elapsed_ns: u64,
    used: CombatPlanningCounters,
    granted: CombatPlanningCounters,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TurnOptionGeneratorTiming {
    pub atomic_expand_elapsed_ns: u64,
    pub transition_simulation_elapsed_ns: u64,
    pub transition_identity_elapsed_ns: u64,
    pub transition_key_build_elapsed_ns: u64,
    pub transition_key_index_elapsed_ns: u64,
    pub transition_admission_elapsed_ns: u64,
    pub transition_trace_elapsed_ns: u64,
    pub transition_seen_elapsed_ns: u64,
    pub transition_publish_elapsed_ns: u64,
    pub transition_publish_trace_node_elapsed_ns: u64,
    pub transition_publish_boundary_elapsed_ns: u64,
    pub transition_publish_complete_elapsed_ns: u64,
    pub transition_publish_push_elapsed_ns: u64,
    pub transition_publish_guide_elapsed_ns: u64,
    pub transition_publish_retain_elapsed_ns: u64,
    pub transition_publish_agenda_elapsed_ns: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct PushWorkTiming {
    guide_elapsed_ns: u64,
    retain_elapsed_ns: u64,
    agenda_elapsed_ns: u64,
}

fn guides_with_lookahead(
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
    pub fn new(root: CombatDecisionRoot, config: TurnOptionGeneratorConfig) -> Self {
        Self::with_policy(root, config, uniform_policy())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: TurnOptionGeneratorConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, None, None)
    }

    pub(crate) fn with_policy_and_potion_limit(
        root: CombatDecisionRoot,
        config: TurnOptionGeneratorConfig,
        policy: SharedCombatActionPolicy,
        max_potion_expenditures: Option<u32>,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, None, max_potion_expenditures)
    }

    pub fn with_policy_and_lookahead(
        root: CombatDecisionRoot,
        config: TurnOptionGeneratorConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: SharedCombatLookaheadEvaluator,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, Some(lookahead_evaluator), None)
    }

    fn with_optional_lookahead(
        root: CombatDecisionRoot,
        config: TurnOptionGeneratorConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: Option<SharedCombatLookaheadEvaluator>,
        max_potion_expenditures: Option<u32>,
    ) -> Self {
        let max_potion_expenditures = if config.allow_potion_expenditure {
            max_potion_expenditures
        } else {
            Some(0)
        };
        let mut seen = HashSet::with_hasher(FxBuildHasher);
        let root_key = root.exact_state_key().cloned().unwrap_or_else(|| {
            Arc::new(combat_exact_state_key(
                &root.position().engine,
                &root.position().combat,
            ))
        });
        seen.insert(IndexedExactStateKey::from_arc(
            root_key,
            max_potion_expenditures.map(|_| 0),
        ));
        let root_work = GeneratorWork::Expand(PartialTurnOption {
            position: root.position().clone(),
            trace: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
            potion_expenditures: 0,
            generation_guides: None,
            lookahead_guide: None,
        });
        let mut session = Self {
            root,
            config,
            max_potion_expenditures,
            policy,
            work: Vec::new(),
            anchor_frontier: BinaryHeap::new(),
            guided_frontiers: Vec::new(),
            next_scheduler_lane: 0,
            scheduled_round: VecDeque::new(),
            live_work_items: 0,
            next_sequence_id: 0,
            seen,
            completed: Vec::new(),
            total_completed_options: 0,
            gaps: Vec::new(),
            applied_action_transitions: 0,
            duplicate_exact_successors: 0,
            atomic_state_expansions: 0,
            atomic_expand_services: 0,
            anchor_work_pops: 0,
            guided_work_pops: 0,
            lookahead_evaluator,
            lookahead_evaluations: 0,
            lookahead_work: 0,
            atomic_expand_elapsed_ns: 0,
            transition_simulation_elapsed_ns: 0,
            transition_identity_elapsed_ns: 0,
            transition_key_build_elapsed_ns: 0,
            transition_key_index_elapsed_ns: 0,
            transition_admission_elapsed_ns: 0,
            transition_trace_elapsed_ns: 0,
            transition_seen_elapsed_ns: 0,
            transition_publish_elapsed_ns: 0,
            transition_publish_trace_node_elapsed_ns: 0,
            transition_publish_boundary_elapsed_ns: 0,
            transition_publish_complete_elapsed_ns: 0,
            transition_publish_push_elapsed_ns: 0,
            transition_publish_guide_elapsed_ns: 0,
            transition_publish_retain_elapsed_ns: 0,
            transition_publish_agenda_elapsed_ns: 0,
            used: CombatPlanningCounters::default(),
            granted: CombatPlanningCounters::default(),
        };
        session.push_work(root_work, GeneratorWorkPriority::for_path(0, 0.0));
        session
    }

    pub fn root(&self) -> &CombatDecisionRoot {
        &self.root
    }

    pub fn completed_options(&self) -> &[CompleteTurnOption] {
        &self.completed
    }

    /// Diagnostic membership for one exact partial-turn position.  `seen`
    /// records both live and already-expanded states, so this distinguishes a
    /// prefix that was never generated from one that was generated and later
    /// consumed.  It does not change retention or scheduling.
    pub fn has_seen_exact_position(&self, position: &CombatPosition) -> bool {
        let key = combat_exact_state_key(&position.engine, &position.combat);
        self.seen.iter().any(|seen| *seen.key == key)
    }

    /// Counts still-live generator work rooted at one exact partial-turn
    /// position as `(expand, pending_atomic_actions, structured_selection)`.
    /// Atomic siblings share one resumable cursor, but the second component
    /// remains a count of concrete transitions so membership reports stay
    /// comparable. This is a diagnostic view only.
    pub fn live_work_counts_at_exact_position(
        &self,
        position: &CombatPosition,
    ) -> (usize, usize, usize) {
        let target = combat_exact_state_key(&position.engine, &position.combat);
        let counts = self
            .work
            .iter()
            .filter_map(Option::as_ref)
            .filter(|work| {
                combat_exact_state_key(&work.position().engine, &work.position().combat) == target
            })
            .fold((0, 0, 0), |mut counts, work| {
                match work {
                    GeneratorWork::Expand(_) => counts.0 += 1,
                    GeneratorWork::AtomicActions(actions) => {
                        counts.1 += actions.remaining_candidate_count()
                    }
                    GeneratorWork::ApplyAction(_) => counts.1 += 1,
                    GeneratorWork::StructuredSelection(_) => counts.2 += 1,
                }
                counts
            });
        counts
    }

    /// Locates an exact pending action without changing queue order or work.
    /// `candidate_ordinal` is one-based within the cursor's still-unapplied
    /// members; queue ranks refer to the shared cursor work item.
    pub fn live_action_transition_snapshot(
        &self,
        position: &CombatPosition,
        input: &ClientInput,
    ) -> Option<LiveActionTransitionSnapshot> {
        let target = combat_exact_state_key(&position.engine, &position.combat);
        self.work
            .iter()
            .enumerate()
            .find_map(|(work_id, work)| match work.as_ref()? {
                GeneratorWork::AtomicActions(actions)
                    if combat_exact_state_key(
                        &actions.parent.position.engine,
                        &actions.parent.position.combat,
                    ) == target =>
                {
                    let remaining = &actions.candidates[actions.next_candidate..];
                    let (offset, candidate) = remaining
                        .iter()
                        .enumerate()
                        .find(|(_, candidate)| candidate.input == *input)?;
                    let cursor_priority = actions.priority()?;
                    let (anchor_queue_rank, guide_queue_ranks) =
                        self.live_queue_ranks_for_work_id(work_id)?;
                    Some(LiveActionTransitionSnapshot {
                        candidate_ordinal: offset.saturating_add(1),
                        remaining_candidate_count: remaining.len(),
                        conditional_probability: candidate.conditional_probability,
                        candidate_negative_log_policy: candidate.negative_log_policy,
                        cursor_negative_log_policy: cursor_priority.negative_log_policy,
                        anchor_queue_rank,
                        guide_queue_ranks,
                    })
                }
                GeneratorWork::ApplyAction(action)
                    if combat_exact_state_key(
                        &action.parent.position.engine,
                        &action.parent.position.combat,
                    ) == target
                        && action.input == *input =>
                {
                    let (anchor_queue_rank, guide_queue_ranks) =
                        self.live_queue_ranks_for_work_id(work_id)?;
                    Some(LiveActionTransitionSnapshot {
                        candidate_ordinal: 1,
                        remaining_candidate_count: 1,
                        conditional_probability: 1.0,
                        candidate_negative_log_policy: action.negative_log_policy,
                        cursor_negative_log_policy: action.negative_log_policy,
                        anchor_queue_rank,
                        guide_queue_ranks,
                    })
                }
                _ => None,
            })
    }

    /// One-based live queue ranks for an exact pending expansion, returned as
    /// `(anchor_rank, guide_ranks)`. Lower is scheduled earlier within that
    /// view. This exposes queue placement without mutating queues.
    pub fn live_expand_queue_ranks_at_exact_position(
        &self,
        position: &CombatPosition,
    ) -> Option<(usize, Vec<usize>)> {
        let target_key = combat_exact_state_key(&position.engine, &position.combat);
        let target_work_id = self
            .work
            .iter()
            .enumerate()
            .find_map(|(work_id, work)| match work.as_ref()? {
                GeneratorWork::Expand(partial)
                    if combat_exact_state_key(
                        &partial.position.engine,
                        &partial.position.combat,
                    ) == target_key =>
                {
                    Some(work_id)
                }
                _ => None,
            })?;
        self.live_queue_ranks_for_work_id(target_work_id)
    }

    fn live_queue_ranks_for_work_id(&self, target_work_id: usize) -> Option<(usize, Vec<usize>)> {
        let target_anchor = self
            .anchor_frontier
            .iter()
            .find(|entry| entry.work_id == target_work_id)?;
        let anchor_rank = 1 + self
            .anchor_frontier
            .iter()
            .filter(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
            .filter(|entry| *entry > target_anchor)
            .count();
        let guide_ranks = self
            .guided_frontiers
            .iter()
            .map(|frontier| {
                let Some(target) = frontier
                    .entries
                    .iter()
                    .find(|entry| entry.work_id == target_work_id)
                else {
                    return 0;
                };
                1 + frontier
                    .entries
                    .iter()
                    .filter(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
                    .filter(|entry| *entry > target)
                    .count()
            })
            .collect();
        Some((anchor_rank, guide_ranks))
    }

    pub fn gaps(&self) -> &[TurnOptionGenerationGap] {
        &self.gaps
    }

    pub fn counters(&self) -> CombatPlanningCounters {
        self.used
    }

    pub fn atomic_state_expansions(&self) -> usize {
        self.atomic_state_expansions
    }

    pub fn anchor_work_pops(&self) -> usize {
        self.anchor_work_pops
    }

    pub fn guided_work_pops(&self) -> usize {
        self.guided_work_pops
    }

    pub fn lookahead_evaluations(&self) -> usize {
        self.lookahead_evaluations
    }

    pub fn lookahead_work(&self) -> usize {
        self.lookahead_work
    }

    pub fn retained_lookahead_guides(&self) -> usize {
        self.work
            .iter()
            .filter_map(Option::as_ref)
            .filter(|work| {
                matches!(
                    work,
                    GeneratorWork::Expand(PartialTurnOption {
                        lookahead_guide: Some(_),
                        ..
                    })
                )
            })
            .count()
    }

    pub(crate) fn retained_guide_lanes(&self) -> Vec<CombatGuideLaneId> {
        self.guided_frontiers
            .iter()
            .filter(|frontier| {
                frontier
                    .entries
                    .iter()
                    .any(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
            })
            .map(|frontier| frontier.lane)
            .collect()
    }

    pub fn granted_budget(&self) -> CombatPlanningCounters {
        self.granted
    }

    pub fn retained_work_items(&self) -> usize {
        self.live_work_items
    }

    pub fn diagnostics(&self) -> TurnOptionGenerationDiagnostics {
        TurnOptionGenerationDiagnostics {
            applied_action_transitions: self.applied_action_transitions,
            unique_successor_states: self.seen.len().saturating_sub(1),
            duplicate_exact_successors: self.duplicate_exact_successors,
            completed_turn_options: self.total_completed_options,
        }
    }

    pub(crate) fn timing(&self) -> TurnOptionGeneratorTiming {
        TurnOptionGeneratorTiming {
            atomic_expand_elapsed_ns: self.atomic_expand_elapsed_ns,
            transition_simulation_elapsed_ns: self.transition_simulation_elapsed_ns,
            transition_identity_elapsed_ns: self.transition_identity_elapsed_ns,
            transition_key_build_elapsed_ns: self.transition_key_build_elapsed_ns,
            transition_key_index_elapsed_ns: self.transition_key_index_elapsed_ns,
            transition_admission_elapsed_ns: self.transition_admission_elapsed_ns,
            transition_trace_elapsed_ns: self.transition_trace_elapsed_ns,
            transition_seen_elapsed_ns: self.transition_seen_elapsed_ns,
            transition_publish_elapsed_ns: self.transition_publish_elapsed_ns,
            transition_publish_trace_node_elapsed_ns: self.transition_publish_trace_node_elapsed_ns,
            transition_publish_boundary_elapsed_ns: self.transition_publish_boundary_elapsed_ns,
            transition_publish_complete_elapsed_ns: self.transition_publish_complete_elapsed_ns,
            transition_publish_push_elapsed_ns: self.transition_publish_push_elapsed_ns,
            transition_publish_guide_elapsed_ns: self.transition_publish_guide_elapsed_ns,
            transition_publish_retain_elapsed_ns: self.transition_publish_retain_elapsed_ns,
            transition_publish_agenda_elapsed_ns: self.transition_publish_agenda_elapsed_ns,
        }
    }

    pub(crate) fn take_completed_options(&mut self) -> Vec<CompleteTurnOption> {
        std::mem::take(&mut self.completed)
    }

    pub(crate) fn total_completed_options(&self) -> usize {
        self.total_completed_options
    }

    pub fn is_finished(&self) -> bool {
        self.live_work_items == 0
    }

    pub(crate) fn best_retained_path_bound(&mut self) -> Option<(usize, f64)> {
        while let Some(entry) = self.anchor_frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                break;
            }
            self.anchor_frontier.pop();
        }
        self.anchor_frontier.peek().map(|entry| {
            (
                entry.priority.atomic_depth,
                entry.priority.negative_log_policy,
            )
        })
    }

    pub(crate) fn best_retained_path_bound_snapshot(&self) -> Option<(usize, f64)> {
        let anchor = self
            .anchor_frontier
            .iter()
            .filter(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
            .min_by(|left, right| {
                left.priority
                    .levin_log_priority
                    .total_cmp(&right.priority.levin_log_priority)
                    .then_with(|| {
                        left.priority
                            .negative_log_policy
                            .total_cmp(&right.priority.negative_log_policy)
                    })
                    .then_with(|| left.priority.atomic_depth.cmp(&right.priority.atomic_depth))
            })
            .map(|entry| {
                (
                    entry.priority.atomic_depth,
                    entry.priority.negative_log_policy,
                )
            });
        anchor
    }

    pub(crate) fn has_guide_lane(&self, lane: CombatGuideLaneId) -> bool {
        self.guided_frontiers
            .iter()
            .any(|frontier| frontier.lane == lane)
    }

    /// The best still-live partial expansion for one semantically identical
    /// guide lane.  This is the partial-expansion promise published to the
    /// outer search; it is not a terminal estimate and changes no legality.
    pub(crate) fn best_retained_guide_promise(
        &mut self,
        lane: CombatGuideLaneId,
    ) -> Option<RetainedGuidePromise> {
        let frontier_index = self.guide_frontier_index(lane)?;
        self.peek_guided_work_id(frontier_index)?;
        self.guided_frontiers[frontier_index]
            .entries
            .peek()
            .map(|entry| RetainedGuidePromise {
                rank: entry.guide_rank.clone(),
                atomic_depth: entry.anchor_priority.atomic_depth,
                negative_log_policy: entry.anchor_priority.negative_log_policy,
            })
    }

    pub(crate) fn best_retained_guide_promise_snapshot(
        &self,
        lane: CombatGuideLaneId,
    ) -> Option<RetainedGuidePromise> {
        self.guided_frontiers
            .iter()
            .find(|frontier| frontier.lane == lane)?
            .entries
            .iter()
            .filter(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
            .max()
            .map(|entry| RetainedGuidePromise {
                rank: entry.guide_rank.clone(),
                atomic_depth: entry.anchor_priority.atomic_depth,
                negative_log_policy: entry.anchor_priority.negative_log_policy,
            })
    }

    pub(crate) fn prefer_lane(&mut self, preferred: TurnOptionGeneratorPreferredLane) {
        self.next_scheduler_lane = match preferred {
            TurnOptionGeneratorPreferredLane::Anchor => 0,
            TurnOptionGeneratorPreferredLane::Guide(lane) => self
                .guide_frontier_index(lane)
                .map_or(0, |frontier_index| frontier_index.saturating_add(1)),
        };
    }

    pub(crate) fn release_unused_grant(&mut self) -> CombatPlanningCounters {
        let released = CombatPlanningCounters {
            generation_work: self
                .granted
                .generation_work
                .saturating_sub(self.used.generation_work),
            engine_steps: self
                .granted
                .engine_steps
                .saturating_sub(self.used.engine_steps),
        };
        self.granted = self.used;
        released
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlanningQuantum,
    ) -> TurnOptionGenerationReport {
        self.advance_internal(stepper, quantum, 0, 0, 0)
    }

    pub fn advance_with_lookahead(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlanningQuantum,
        lookahead_evaluations: usize,
        lookahead_work: usize,
        lookahead_work_per_evaluation: usize,
    ) -> TurnOptionGenerationReport {
        self.advance_internal(
            stepper,
            quantum,
            lookahead_evaluations,
            lookahead_work,
            lookahead_work_per_evaluation,
        )
    }

    fn advance_internal(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlanningQuantum,
        mut remaining_lookahead_evaluations: usize,
        mut remaining_lookahead_work: usize,
        lookahead_work_per_evaluation: usize,
    ) -> TurnOptionGenerationReport {
        let before = self.used;
        let before_diagnostics = self.diagnostics();
        let completed_before = self.total_completed_options;
        self.granted = self.granted.saturating_add(CombatPlanningCounters {
            generation_work: quantum.additional_generation_work,
            engine_steps: quantum.additional_engine_steps,
        });
        // Freeze one current head from every scheduling view before serving
        // the round. Without this boundary, work expanded by an earlier lane
        // can publish a new head into a later lane and repeatedly overtake the
        // item which was already first there. A finite pending transaction can
        // consequently remain live forever despite round-robin lane service.
        let interruption = loop {
            if self.is_finished() {
                break None;
            }
            if deadline_reached(quantum.deadline) {
                break Some(GenerationInterruption::Deadline);
            }
            if self.used.generation_work >= self.granted.generation_work {
                break Some(GenerationInterruption::GenerationWorkBudget);
            }
            while self
                .scheduled_round
                .front()
                .is_some_and(|(_, work_id)| !self.work.get(*work_id).is_some_and(Option::is_some))
            {
                self.scheduled_round.pop_front();
            }
            if self.scheduled_round.is_empty() {
                self.scheduled_round = self.snapshot_scheduling_round();
                if self.scheduled_round.is_empty() {
                    debug_assert!(self.is_finished());
                    break None;
                }
            }
            let (lane, work_id) = *self
                .scheduled_round
                .front()
                .expect("a non-empty scheduling round has a head");
            let transition_reservation = self.config.max_engine_steps_per_transition.max(1);
            if self.work[work_id].as_ref().is_some_and(|work| {
                matches!(
                    work,
                    GeneratorWork::AtomicActions(_) | GeneratorWork::ApplyAction(_)
                )
            }) && self
                .granted
                .engine_steps
                .saturating_sub(self.used.engine_steps)
                < transition_reservation
            {
                break Some(GenerationInterruption::EngineStepBudget);
            }

            self.scheduled_round.pop_front();
            let work = self.work[work_id]
                .take()
                .expect("a reserved generator work item must still be live");
            self.live_work_items = self.live_work_items.saturating_sub(1);
            if lane == 0 {
                self.anchor_work_pops = self.anchor_work_pops.saturating_add(1);
            } else {
                self.guided_work_pops = self.guided_work_pops.saturating_add(1);
            }
            self.next_scheduler_lane = (lane + 1) % self.guided_frontiers.len().saturating_add(1);
            self.used.generation_work = self.used.generation_work.saturating_add(1);
            match work {
                GeneratorWork::Expand(mut partial) => {
                    let expand_service_ordinal = self.atomic_expand_services;
                    self.atomic_expand_services = self.atomic_expand_services.saturating_add(1);
                    let should_evaluate = partial.lookahead_guide.is_none()
                        && self.lookahead_evaluator.as_ref().is_some_and(|evaluator| {
                            evaluator.admit_atomic_state(&partial.position, expand_service_ordinal)
                        });
                    if should_evaluate
                        && remaining_lookahead_evaluations > 0
                        && remaining_lookahead_work > 0
                    {
                        let priority = GeneratorWorkPriority::for_path(
                            partial.atomic_depth,
                            partial.negative_log_policy,
                        );
                        let max_work = lookahead_work_per_evaluation
                            .max(1)
                            .min(remaining_lookahead_work);
                        let evaluation = self.lookahead_evaluator.as_ref().and_then(|evaluator| {
                            evaluator.evaluate(&partial.position, max_work, quantum.deadline)
                        });
                        let Some(evaluation) = evaluation else {
                            self.push_work(GeneratorWork::Expand(partial), priority);
                            break Some(if deadline_reached(quantum.deadline) {
                                GenerationInterruption::Deadline
                            } else {
                                GenerationInterruption::GenerationWorkBudget
                            });
                        };
                        debug_assert!(evaluation.work <= max_work);
                        let charged_work = evaluation.work.max(1).min(max_work);
                        partial.lookahead_guide = Some(evaluation.guide);
                        self.lookahead_evaluations = self.lookahead_evaluations.saturating_add(1);
                        self.lookahead_work = self.lookahead_work.saturating_add(charged_work);
                        remaining_lookahead_evaluations =
                            remaining_lookahead_evaluations.saturating_sub(1);
                        remaining_lookahead_work =
                            remaining_lookahead_work.saturating_sub(charged_work);
                        self.push_work(GeneratorWork::Expand(partial), priority);
                        continue;
                    }
                    self.atomic_state_expansions = self.atomic_state_expansions.saturating_add(1);
                    let expand_started = Instant::now();
                    self.expand(stepper, partial);
                    self.atomic_expand_elapsed_ns = self
                        .atomic_expand_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(expand_started));
                }
                GeneratorWork::AtomicActions(mut cursor) => {
                    let action = cursor
                        .current_transition()
                        .expect("a scheduled atomic cursor has a candidate");
                    if self.apply_action_transition(
                        stepper,
                        action,
                        transition_reservation,
                        quantum.deadline,
                    ) == ActionTransitionStatus::TimedOut
                    {
                        let priority = cursor
                            .priority()
                            .expect("a timed-out cursor retains its candidate");
                        self.push_work(GeneratorWork::AtomicActions(cursor), priority);
                        break Some(GenerationInterruption::Deadline);
                    }
                    cursor.consume_current();
                    if let Some(priority) = cursor.priority() {
                        self.push_work(GeneratorWork::AtomicActions(cursor), priority);
                    }
                }
                GeneratorWork::StructuredSelection(mut selection) => {
                    let remaining_inputs = selection.cursor.remaining_input_count().max(1);
                    if let Some(input) = selection.cursor.next_input() {
                        // Every concrete member of a finite symbolic family
                        // receives equal conditional mass. The former
                        // geometric split made enumeration order an
                        // exponential strategic prior (1/2, 1/4, 1/8, ...).
                        let input_conditional_mass =
                            selection.remaining_conditional_mass / remaining_inputs as f64;
                        if !selection.cursor.is_exhausted() {
                            selection.remaining_conditional_mass -= input_conditional_mass;
                            let residual_negative_log = selection.family_negative_log_policy
                                - selection.remaining_conditional_mass.ln();
                            let residual_priority = GeneratorWorkPriority::for_path(
                                selection.parent.atomic_depth.saturating_add(1),
                                residual_negative_log,
                            );
                            self.push_work(
                                GeneratorWork::StructuredSelection(selection.clone()),
                                residual_priority,
                            );
                        }
                        let negative_log_policy =
                            selection.family_negative_log_policy - input_conditional_mass.ln();
                        let atomic_depth = selection.parent.atomic_depth.saturating_add(1);
                        let priority =
                            GeneratorWorkPriority::for_path(atomic_depth, negative_log_policy);
                        self.push_work(
                            GeneratorWork::ApplyAction(ActionTransitionWork {
                                parent: selection.parent,
                                input,
                                atomic_depth,
                                negative_log_policy,
                            }),
                            priority,
                        );
                    }
                }
                GeneratorWork::ApplyAction(action) => {
                    let priority = GeneratorWorkPriority::for_path(
                        action.atomic_depth,
                        action.negative_log_policy,
                    );
                    if self.apply_action_transition(
                        stepper,
                        action.clone(),
                        transition_reservation,
                        quantum.deadline,
                    ) == ActionTransitionStatus::TimedOut
                    {
                        self.push_work(GeneratorWork::ApplyAction(action), priority);
                        break Some(GenerationInterruption::Deadline);
                    }
                }
            }
        };

        let status = if let Some(cause) = interruption {
            TurnOptionGenerationStatus::Partial(cause)
        } else if self.gaps.is_empty() {
            TurnOptionGenerationStatus::Complete
        } else {
            TurnOptionGenerationStatus::PartialWithMechanicsGaps
        };
        TurnOptionGenerationReport {
            before,
            after: self.used,
            granted: self.granted,
            before_diagnostics,
            after_diagnostics: self.diagnostics(),
            retained_work_items: self.retained_work_items(),
            newly_completed_options: self
                .total_completed_options
                .saturating_sub(completed_before),
            total_completed_options: self.total_completed_options,
            gaps: self.gaps.clone(),
            status,
        }
    }

    /// Captures the current head of every scheduling view as one finite
    /// service round. Duplicate views of the same shared work item collapse
    /// to one service; newly published work waits for the next round.
    fn snapshot_scheduling_round(&mut self) -> VecDeque<(usize, usize)> {
        let lane_count = self.guided_frontiers.len().saturating_add(1);
        let mut work_ids = HashSet::new();
        let mut round = VecDeque::new();
        for offset in 0..lane_count {
            let lane = (self.next_scheduler_lane + offset) % lane_count;
            let work_id = if lane == 0 {
                self.peek_anchor_work_id()
            } else {
                self.peek_guided_work_id(lane - 1)
            };
            if let Some(work_id) = work_id.filter(|work_id| work_ids.insert(*work_id)) {
                round.push_back((lane, work_id));
            }
        }
        round
    }

    fn apply_action_transition(
        &mut self,
        stepper: &dyn CombatStepper,
        action: ActionTransitionWork,
        transition_reservation: usize,
        deadline: Option<Instant>,
    ) -> ActionTransitionStatus {
        let simulation_started = Instant::now();
        if stepper
            .choice_for_legal_input(&action.parent.position, &action.input)
            .is_none()
        {
            self.record_gap(
                TurnOptionGenerationGapKind::GeneratedInputRejected,
                &action.parent,
            );
            self.transition_simulation_elapsed_ns = self
                .transition_simulation_elapsed_ns
                .saturating_add(elapsed_nanos_u64(simulation_started));
            return ActionTransitionStatus::Consumed;
        }
        let result = stepper.apply_to_stable(
            &action.parent.position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: transition_reservation,
                deadline,
            },
        );
        self.used.engine_steps = self.used.engine_steps.saturating_add(result.engine_steps);
        self.transition_simulation_elapsed_ns = self
            .transition_simulation_elapsed_ns
            .saturating_add(elapsed_nanos_u64(simulation_started));
        if result.timed_out {
            return ActionTransitionStatus::TimedOut;
        }
        if result.truncated {
            self.record_gap(
                TurnOptionGenerationGapKind::TransitionStepLimit,
                &action.parent,
            );
            return ActionTransitionStatus::Consumed;
        }

        self.applied_action_transitions = self.applied_action_transitions.saturating_add(1);
        let detail_timing_scale = detail_timing_scale(self.applied_action_transitions);
        let identity_started = Instant::now();
        let key_build_started = detail_timing_scale.map(|_| Instant::now());
        let key = combat_exact_state_key(&result.position.engine, &result.position.combat);
        self.transition_key_build_elapsed_ns =
            self.transition_key_build_elapsed_ns
                .saturating_add(sampled_elapsed_nanos_u64(
                    key_build_started,
                    detail_timing_scale,
                ));
        let key_index_started = detail_timing_scale.map(|_| Instant::now());
        let successor_key = Arc::new(key);
        let successor_potion_expenditures = action
            .parent
            .potion_expenditures
            .saturating_add(u32::from(is_potion_expenditure(&action.input)));
        let indexed_key = IndexedExactStateKey::from_arc(
            successor_key.clone(),
            self.max_potion_expenditures
                .map(|_| successor_potion_expenditures),
        );
        self.transition_key_index_elapsed_ns =
            self.transition_key_index_elapsed_ns
                .saturating_add(sampled_elapsed_nanos_u64(
                    key_index_started,
                    detail_timing_scale,
                ));
        self.transition_identity_elapsed_ns = self
            .transition_identity_elapsed_ns
            .saturating_add(elapsed_nanos_u64(identity_started));
        let admission_started = Instant::now();
        let seen_started = detail_timing_scale.map(|_| Instant::now());
        let unseen = self.seen.insert(indexed_key);
        self.transition_seen_elapsed_ns = self
            .transition_seen_elapsed_ns
            .saturating_add(sampled_elapsed_nanos_u64(seen_started, detail_timing_scale));
        let publish_started = Instant::now();
        if unseen {
            let trace_node_started = detail_timing_scale.map(|_| Instant::now());
            let partial = PartialTurnOption {
                position: result.position,
                trace: Some(Arc::new(PendingActionTrace {
                    parent: action.parent.trace.clone(),
                    input: action.input,
                    successor_key,
                    engine_steps: result.engine_steps,
                    depth: action.parent.action_depth().saturating_add(1),
                })),
                atomic_depth: action.atomic_depth,
                negative_log_policy: action.negative_log_policy,
                potion_expenditures: successor_potion_expenditures,
                generation_guides: None,
                lookahead_guide: None,
            };
            self.transition_publish_trace_node_elapsed_ns = self
                .transition_publish_trace_node_elapsed_ns
                .saturating_add(sampled_elapsed_nanos_u64(
                    trace_node_started,
                    detail_timing_scale,
                ));
            let boundary_started = detail_timing_scale.map(|_| Instant::now());
            let terminal = stepper.terminal(&partial.position);
            let boundary = supported_boundary(&self.root, &partial.position, terminal);
            self.transition_publish_boundary_elapsed_ns =
                self.transition_publish_boundary_elapsed_ns.saturating_add(
                    sampled_elapsed_nanos_u64(boundary_started, detail_timing_scale),
                );
            if let Some(boundary) = boundary {
                // A stable atomic transition has already paid the simulator
                // cost and reached the requested exact boundary. Publish it
                // now instead of routing it back through the private atomic
                // agenda.
                let trace_started = Instant::now();
                let actions = partial.materialize_actions();
                self.transition_trace_elapsed_ns = self
                    .transition_trace_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(trace_started));
                // These mutually exclusive coarse timers are exhaustive. The
                // branch costs are heavy-tailed enough that sparse estimates
                // are misleading; nested hot-path timers remain sampled.
                let complete_started = Instant::now();
                self.publish_completed(CompleteTurnOption::new(
                    self.root.exact_state_identity().clone(),
                    actions,
                    boundary,
                    partial.position,
                    partial.negative_log_policy,
                ));
                self.transition_publish_complete_elapsed_ns = self
                    .transition_publish_complete_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(complete_started));
            } else {
                let priority = GeneratorWorkPriority::for_path(
                    action.atomic_depth,
                    action.negative_log_policy,
                );
                let push_started = Instant::now();
                let (_, push_timing) = self.push_work_measured(
                    GeneratorWork::Expand(partial),
                    priority,
                    detail_timing_scale.is_some(),
                );
                self.transition_publish_push_elapsed_ns = self
                    .transition_publish_push_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(push_started));
                let scale = detail_timing_scale.unwrap_or(0);
                self.transition_publish_guide_elapsed_ns = self
                    .transition_publish_guide_elapsed_ns
                    .saturating_add(push_timing.guide_elapsed_ns.saturating_mul(scale));
                self.transition_publish_retain_elapsed_ns = self
                    .transition_publish_retain_elapsed_ns
                    .saturating_add(push_timing.retain_elapsed_ns.saturating_mul(scale));
                self.transition_publish_agenda_elapsed_ns = self
                    .transition_publish_agenda_elapsed_ns
                    .saturating_add(push_timing.agenda_elapsed_ns.saturating_mul(scale));
            }
        } else {
            self.duplicate_exact_successors = self.duplicate_exact_successors.saturating_add(1);
        }
        self.transition_publish_elapsed_ns = self
            .transition_publish_elapsed_ns
            .saturating_add(elapsed_nanos_u64(publish_started));
        self.transition_admission_elapsed_ns = self
            .transition_admission_elapsed_ns
            .saturating_add(elapsed_nanos_u64(admission_started));
        ActionTransitionStatus::Consumed
    }

    fn expand(&mut self, stepper: &dyn CombatStepper, mut partial: PartialTurnOption) {
        let terminal = stepper.terminal(&partial.position);
        if let Some(boundary) = supported_boundary(&self.root, &partial.position, terminal) {
            let trace_started = Instant::now();
            let actions = partial.materialize_actions();
            self.transition_trace_elapsed_ns = self
                .transition_trace_elapsed_ns
                .saturating_add(elapsed_nanos_u64(trace_started));
            self.publish_completed(CompleteTurnOption::new(
                self.root.exact_state_identity().clone(),
                actions,
                boundary,
                partial.position,
                partial.negative_log_policy,
            ));
            return;
        }

        if terminal != CombatTerminal::Unresolved
            || !matches!(
                partial.position.engine,
                EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
            )
            || (matches!(partial.position.engine, EngineState::CombatPlayerTurn)
                && partial.position.combat.turn.turn_count != self.root.turn_count())
        {
            self.record_gap(
                TurnOptionGenerationGapKind::UnsupportedStableBoundary,
                &partial,
            );
            return;
        }

        let mut surface = stepper.legal_action_surface(&partial.position);
        if self
            .max_potion_expenditures
            .is_some_and(|limit| partial.potion_expenditures >= limit)
        {
            surface
                .atomic_actions
                .retain(|input| !is_potion_expenditure(input));
        }
        let surface_is_empty =
            surface.atomic_actions.is_empty() && surface.selection_families.is_empty();
        let policy_choices = surface
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
        let weights = self.policy.weights(&partial.position, &policy_choices);
        let weights = (weights.len() == policy_choices.len())
            .then_some(weights)
            .unwrap_or_else(|| vec![1.0; policy_choices.len()]);
        let probabilities = normalized_probabilities(weights, self.config.uniform_exploration_ppm);
        let atomic_action_count = surface.atomic_actions.len();
        let atomic_probabilities = probabilities[..atomic_action_count].to_vec();
        let selection_probabilities = probabilities[atomic_action_count..].to_vec();
        let base_generation_guides = partial
            .generation_guides
            .get_or_insert_with(|| self.policy.turn_generation_guides(&partial.position).into())
            .clone();
        let parent_guides =
            guides_with_lookahead(base_generation_guides, partial.lookahead_guide.as_ref());
        // Every outgoing action observes the same immutable parent position.
        // Sharing it avoids one full combat-state and action-prefix clone for
        // every legal action while preserving the exact search graph.
        let parent = Arc::new(partial);
        if let Some(cursor) = AtomicActionCursorWork::new(
            parent.clone(),
            surface.atomic_actions,
            atomic_probabilities,
            parent_guides.clone(),
        ) {
            let priority = cursor
                .priority()
                .expect("a new atomic cursor contains probability mass");
            self.push_work(GeneratorWork::AtomicActions(cursor), priority);
        }
        if !surface.selection_families.is_empty()
            && !stepper.supports_canonical_pending_choice_actions()
        {
            self.record_gap(
                TurnOptionGenerationGapKind::UnsupportedStructuredChoice,
                &parent,
            );
        } else {
            for (family, probability) in surface
                .selection_families
                .into_iter()
                .zip(selection_probabilities)
            {
                match SelectionTransactionCursor::new(&family) {
                    Ok(mut cursor) if family.declared_min == 1 && family.effective_max == 1 => {
                        let members =
                            std::iter::from_fn(|| cursor.next_input()).collect::<Vec<_>>();
                        let weights = self.policy.structured_selection_member_weights(
                            &parent.position,
                            &family,
                            &members,
                        );
                        let weights = (weights.len() == members.len())
                            .then_some(weights)
                            .unwrap_or_else(|| vec![1.0; members.len()]);
                        let probabilities =
                            normalized_probabilities(weights, self.config.uniform_exploration_ppm)
                                .into_iter()
                                .map(|member_probability| probability * member_probability)
                                .collect::<Vec<_>>();
                        if let Some(cursor) = AtomicActionCursorWork::new(
                            parent.clone(),
                            members,
                            probabilities,
                            parent_guides.clone(),
                        ) {
                            let priority = cursor
                                .priority()
                                .expect("ranked selection retains probability mass");
                            self.push_work(GeneratorWork::AtomicActions(cursor), priority);
                        }
                    }
                    Ok(cursor) if !cursor.is_exhausted() => {
                        let family_negative_log_policy =
                            parent.negative_log_policy - probability.ln();
                        self.push_work(
                            GeneratorWork::StructuredSelection(StructuredSelectionWork {
                                parent: parent.clone(),
                                cursor,
                                family_negative_log_policy,
                                remaining_conditional_mass: 1.0,
                            }),
                            GeneratorWorkPriority::for_path(
                                parent.atomic_depth.saturating_add(1),
                                family_negative_log_policy,
                            ),
                        );
                    }
                    Ok(_) => {}
                    Err(kind) => self.record_gap(kind, &parent),
                }
            }
        }
        if surface_is_empty {
            self.record_gap(
                TurnOptionGenerationGapKind::EmptyLegalActionSurface,
                &parent,
            );
        }
    }

    fn record_gap(&mut self, kind: TurnOptionGenerationGapKind, partial: &PartialTurnOption) {
        self.gaps.push(TurnOptionGenerationGap {
            kind,
            exact_state_hash: exact_hash(&partial.position),
            action_depth: partial.action_depth(),
        });
    }

    fn publish_completed(&mut self, option: CompleteTurnOption) {
        self.total_completed_options = self.total_completed_options.saturating_add(1);
        self.completed.push(option);
    }

    fn push_work(&mut self, work: GeneratorWork, priority: GeneratorWorkPriority) -> usize {
        self.push_work_measured(work, priority, false).0
    }

    fn push_work_measured(
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
    fn pop_scheduled_work(&mut self) -> Option<GeneratorWork> {
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
            let work = self.work[work_id]
                .take()
                .expect("scheduled generator work must still be live");
            self.live_work_items = self.live_work_items.saturating_sub(1);
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

    fn peek_anchor_work_id(&mut self) -> Option<usize> {
        while let Some(entry) = self.anchor_frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some(entry.work_id);
            }
            self.anchor_frontier.pop();
        }
        None
    }

    #[cfg(test)]
    fn pop_anchor_work_id(&mut self) -> Option<usize> {
        self.peek_anchor_work_id()?;
        self.anchor_frontier.pop().map(|entry| entry.work_id)
    }

    fn peek_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
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
    fn pop_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
        self.peek_guided_work_id(guide_index)?;
        self.guided_frontiers[guide_index]
            .entries
            .pop()
            .map(|entry| entry.work_id)
    }

    fn guide_frontier_index(&self, lane: CombatGuideLaneId) -> Option<usize> {
        self.guided_frontiers
            .iter()
            .position(|frontier| frontier.lane == lane)
    }

    fn ensure_guide_frontier(&mut self, lane: CombatGuideLaneId) -> usize {
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

fn sampled_elapsed_nanos_u64(started: Option<Instant>, scale: Option<u64>) -> u64 {
    started
        .zip(scale)
        .map(|(started, scale)| elapsed_nanos_u64(started).saturating_mul(scale))
        .unwrap_or(0)
}

fn detail_timing_scale(transition_ordinal: usize) -> Option<u64> {
    debug_assert!(DETAIL_TIMING_SAMPLE_INTERVAL.is_power_of_two());
    // SplitMix64 finalizer: deterministic and cheap, while avoiding a fixed
    // relationship between the sample and canonical action-family order.
    let mut mixed = transition_ordinal as u64;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    ((mixed & (DETAIL_TIMING_SAMPLE_INTERVAL as u64 - 1)) == 0)
        .then_some(DETAIL_TIMING_SAMPLE_INTERVAL as u64)
}

fn is_potion_expenditure(input: &ClientInput) -> bool {
    matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    )
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn elapsed_nanos_u64(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod priority_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use sts_core::sim::combat::EngineCombatStepper;

    struct CountingLookahead {
        calls: Arc<AtomicUsize>,
    }

    struct CountingGenerationGuides {
        calls: Arc<AtomicUsize>,
    }

    impl super::super::policy::CombatActionPolicy for CountingGenerationGuides {
        fn weights(
            &self,
            _position: &CombatPosition,
            choices: &[super::super::policy::CombatPolicyChoice<'_>],
        ) -> Vec<f64> {
            vec![1.0; choices.len()]
        }

        fn turn_generation_guides(&self, _position: &CombatPosition) -> Vec<CombatStateGuide> {
            self.calls.fetch_add(1, AtomicOrdering::Relaxed);
            vec![CombatStateGuide::new(CombatGuideLaneId::new(98), vec![0])]
        }
    }

    impl super::super::policy::CombatLookaheadEvaluator for CountingLookahead {
        fn pending_guide(&self, _position: &CombatPosition) -> Option<CombatStateGuide> {
            Some(CombatStateGuide::new(CombatGuideLaneId::new(99), vec![0]))
        }

        fn admit_atomic_state(
            &self,
            _position: &CombatPosition,
            _atomic_expansions_before: usize,
        ) -> bool {
            true
        }

        fn evaluate(
            &self,
            _position: &CombatPosition,
            max_work: usize,
            _deadline: Option<Instant>,
        ) -> Option<super::super::policy::CombatLookaheadEvaluation> {
            self.calls.fetch_add(1, AtomicOrdering::Relaxed);
            Some(super::super::policy::CombatLookaheadEvaluation {
                guide: CombatStateGuide::new(CombatGuideLaneId::new(99), vec![1]),
                work: 3.min(max_work),
            })
        }
    }

    fn test_root() -> CombatDecisionRoot {
        let mut combat = sts_core::test_support::blank_test_combat();
        combat.entities.monsters = vec![sts_core::test_support::test_monster(
            sts_core::content::monsters::EnemyId::JawWorm,
        )];
        CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
            .expect("test combat is a player-turn root")
    }

    fn guided_entry(
        guide: i32,
        cumulative_negative_log_policy: f64,
        atomic_depth: usize,
        sequence_id: u64,
    ) -> GuidedGeneratorQueueEntry {
        GuidedGeneratorQueueEntry {
            guide_lane: CombatGuideLaneId::new(0),
            work_id: sequence_id as usize,
            sequence_id,
            guide_rank: CombatStateGuideRank::new(vec![guide]),
            anchor_priority: GeneratorWorkPriority::for_path(
                atomic_depth,
                cumulative_negative_log_policy,
            ),
        }
    }

    #[test]
    fn guided_prefix_priority_uses_exact_state_before_anchor_policy() {
        let improved_after_setup = guided_entry(10, 8.0, 3, 0);
        let locally_greedy = guided_entry(9, 0.01, 1, 1);

        assert!(improved_after_setup > locally_greedy);
    }

    #[test]
    fn one_partial_state_computes_base_generation_guides_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let policy = Arc::new(CountingGenerationGuides {
            calls: calls.clone(),
        });
        let mut session = TurnOptionGeneratorSession::with_policy(
            test_root(),
            TurnOptionGeneratorConfig::default(),
            policy,
        );

        assert_eq!(calls.load(AtomicOrdering::Relaxed), 1);
        let report = session.advance(
            &EngineCombatStepper,
            CombatPlanningQuantum::deterministic(1, 250_000),
        );

        assert_eq!(report.after.generation_work, 1);
        assert_eq!(
            calls.load(AtomicOrdering::Relaxed),
            1,
            "expanding a queued partial must reuse the guide bundle computed at publication"
        );
    }

    #[test]
    fn expensive_lookahead_is_lazy_budgeted_and_does_not_expand_the_state() {
        let calls = Arc::new(AtomicUsize::new(0));
        let evaluator = Arc::new(CountingLookahead {
            calls: calls.clone(),
        });
        let mut session = TurnOptionGeneratorSession::with_policy_and_lookahead(
            test_root(),
            TurnOptionGeneratorConfig::default(),
            uniform_policy(),
            evaluator,
        );
        let report = session.advance_with_lookahead(
            &EngineCombatStepper,
            CombatPlanningQuantum::deterministic(1, 250_000),
            1,
            3,
            3,
        );

        assert_eq!(calls.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(session.lookahead_evaluations(), 1);
        assert_eq!(session.lookahead_work(), 3);
        assert_eq!(session.atomic_state_expansions(), 0);
        assert_eq!(session.retained_lookahead_guides(), 1);
        assert_eq!(report.after.generation_work, 1);
        assert!(session.retained_work_items() > 0);
    }

    #[test]
    fn atomic_cursor_conserves_residual_probability_mass() {
        let mut session =
            TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
        let GeneratorWork::Expand(parent) =
            session.pop_scheduled_work().expect("root expansion work")
        else {
            panic!("root work must be an expansion");
        };
        let mut cursor = AtomicActionCursorWork::new(
            Arc::new(parent),
            vec![
                ClientInput::EndTurn,
                ClientInput::Cancel,
                ClientInput::Proceed,
            ],
            vec![0.2, 0.5, 0.3],
            Vec::new(),
        )
        .expect("non-empty action surface");

        let initial = cursor.priority().unwrap();
        assert!(initial.negative_log_policy.abs() < 1.0e-12);
        assert_eq!(
            cursor.current_transition().unwrap().input,
            ClientInput::Cancel,
            "the most probable concrete edge is emitted first"
        );

        cursor.consume_current();
        let residual = cursor.priority().unwrap();
        assert!((residual.negative_log_policy - (-0.5_f64.ln())).abs() < 1.0e-12);
        let next_concrete = cursor.current_transition().unwrap();
        assert!(residual.negative_log_policy <= next_concrete.negative_log_policy);

        cursor.consume_current();
        let final_residual = cursor.priority().unwrap();
        let final_concrete = cursor.current_transition().unwrap();
        assert_eq!(
            final_residual.negative_log_policy.to_bits(),
            final_concrete.negative_log_policy.to_bits(),
            "one remaining edge has exactly the cursor's residual bound"
        );
        cursor.consume_current();
        assert!(cursor.priority().is_none());
    }

    #[test]
    fn action_transition_does_not_bypass_explicit_anchor_priority() {
        let mut session =
            TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
        let GeneratorWork::Expand(parent) =
            session.pop_scheduled_work().expect("root expansion work")
        else {
            panic!("root work must be an expansion");
        };

        for _ in 0..32 {
            session.push_work(
                GeneratorWork::Expand(parent.clone()),
                GeneratorWorkPriority::for_path(1, 0.0),
            );
        }
        session.push_work(
            GeneratorWork::ApplyAction(ActionTransitionWork {
                parent: Arc::new(parent),
                input: ClientInput::EndTurn,
                atomic_depth: 1,
                negative_log_policy: 100.0,
            }),
            GeneratorWorkPriority::for_path(1, 100.0),
        );
        session.prefer_lane(TurnOptionGeneratorPreferredLane::Anchor);

        assert!(matches!(
            session.pop_scheduled_work(),
            Some(GeneratorWork::Expand(_))
        ));
    }

    #[test]
    fn scheduling_round_heads_cannot_be_overtaken_by_new_arrivals() {
        let mut session =
            TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
        let GeneratorWork::Expand(parent) =
            session.pop_scheduled_work().expect("root expansion work")
        else {
            panic!("root work must be an expansion");
        };
        let anchor_head = session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, 0.0),
        );
        let guide_head = session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, 10.0),
        );
        let lane = CombatGuideLaneId::new(99);
        let guide_index = session.ensure_guide_frontier(lane);
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: anchor_head,
                sequence_id: 10_000,
                guide_rank: CombatStateGuideRank::new(vec![0]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
            });
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: guide_head,
                sequence_id: 10_001,
                guide_rank: CombatStateGuideRank::new(vec![10]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 10.0),
            });

        session.next_scheduler_lane = 0;
        let round = session.snapshot_scheduling_round();
        assert_eq!(
            round.iter().copied().collect::<Vec<_>>(),
            vec![(0, anchor_head), (1, guide_head)]
        );

        let newcomer = session.push_work(
            GeneratorWork::Expand(parent),
            GeneratorWorkPriority::for_path(1, 0.0),
        );
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: newcomer,
                sequence_id: 10_002,
                guide_rank: CombatStateGuideRank::new(vec![20]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
            });

        assert_eq!(
            round.iter().copied().collect::<Vec<_>>(),
            vec![(0, anchor_head), (1, guide_head)],
            "a later arrival belongs to the next round even when it becomes the new guide head"
        );
    }

    #[test]
    fn interrupted_scheduling_round_resumes_before_new_arrivals() {
        let mut session =
            TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
        let GeneratorWork::Expand(parent) =
            session.pop_scheduled_work().expect("root expansion work")
        else {
            panic!("root work must be an expansion");
        };
        let anchor_head = session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, 0.0),
        );
        let guide_head = session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, 10.0),
        );
        let lane = CombatGuideLaneId::new(99);
        let guide_index = session.ensure_guide_frontier(lane);
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: anchor_head,
                sequence_id: 10_000,
                guide_rank: CombatStateGuideRank::new(vec![0]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
            });
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: guide_head,
                sequence_id: 10_001,
                guide_rank: CombatStateGuideRank::new(vec![10]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 10.0),
            });

        session.next_scheduler_lane = 0;
        let first = session.advance(
            &EngineCombatStepper,
            CombatPlanningQuantum::deterministic(1, 250),
        );
        assert_eq!(
            first.status,
            TurnOptionGenerationStatus::Partial(GenerationInterruption::GenerationWorkBudget)
        );
        assert_eq!(
            session.scheduled_round.front().copied(),
            Some((1, guide_head)),
            "the unserved guide head remains frozen across the quantum boundary"
        );

        let newcomer = session.push_work(
            GeneratorWork::Expand(parent),
            GeneratorWorkPriority::for_path(1, 0.0),
        );
        session.guided_frontiers[guide_index]
            .entries
            .push(GuidedGeneratorQueueEntry {
                guide_lane: lane,
                work_id: newcomer,
                sequence_id: 10_002,
                guide_rank: CombatStateGuideRank::new(vec![20]),
                anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
            });

        session.advance(
            &EngineCombatStepper,
            CombatPlanningQuantum::deterministic(1, 250),
        );
        assert!(session.work[guide_head].is_none());
        assert!(session.work[newcomer].is_some());
    }

    #[test]
    fn finite_potion_allowance_is_part_of_generator_transposition_identity() {
        let root = test_root();
        let exact = combat_exact_state_key(&root.position().engine, &root.position().combat);
        let without_spend = IndexedExactStateKey::new(exact.clone(), Some(0));
        let after_one_spend = IndexedExactStateKey::new(exact, Some(1));

        assert_ne!(without_spend, after_one_spend);
        assert_eq!(
            HashSet::from([without_spend, after_one_spend]).len(),
            2,
            "equal simulator states with different remaining finite resources cannot transpose"
        );
    }

    #[test]
    fn structural_hash_collision_still_compares_the_complete_typed_state() {
        let root = test_root();
        let position = root.position();
        let player_turn = combat_exact_state_key(&position.engine, &position.combat);
        let processing = combat_exact_state_key(&EngineState::CombatProcessing, &position.combat);
        let first = IndexedExactStateKey::new(player_turn, None);
        let mut collided = IndexedExactStateKey::new(processing, None);

        collided.structural_hash = first.structural_hash;

        assert_ne!(first, collided);
        assert_eq!(
            HashSet::from([first, collided]).len(),
            2,
            "a private structural-hash collision must not merge exact simulator states"
        );
    }

    #[test]
    fn detail_timing_sampler_is_sparse_without_periodic_action_order_aliasing() {
        let sampled = (1..=16_384)
            .filter(|ordinal| detail_timing_scale(*ordinal).is_some())
            .collect::<Vec<_>>();

        assert!((900..=1_150).contains(&sampled.len()));
        assert!(
            sampled.windows(2).any(|pair| pair[1] - pair[0] != 16),
            "samples must not always select the same member of 16-wide action families"
        );
    }
}
