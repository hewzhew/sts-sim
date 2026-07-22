use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Instant;

use sts_core::ai::combat_state_key::{combat_exact_state_key, CombatExactStateKey};
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::ClientInput;

use super::policy::{
    normalized_probabilities, uniform_policy, CombatPolicyChoice, SharedCombatActionPolicy,
};
use super::selection_transaction::SelectionTransactionCursor;
use super::types::{exact_hash, CombatDecisionRoot, TurnOptionAction};

/// Lab control: pure policy-tree search over exact stable combat inputs.
///
/// Unlike `OracleCombatWitnessSession`, this search has no complete-turn
/// generator and no state-guide lanes. Every stable simulator input is one
/// edge in a single Levin-priority frontier. It is intentionally not wired
/// into production run control: its purpose is to test whether an action
/// policy alone is strong enough. The policy changes ordering only; terminal
/// truth and every witness action are checked by the exact stepper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicLevinWitnessConfig {
    pub max_engine_steps_per_transition: usize,
    pub uniform_exploration_ppm: u32,
    pub rerooting: AtomicLevinRerooting,
}

impl Default for AtomicLevinWitnessConfig {
    fn default() -> Self {
        Self {
            max_engine_steps_per_transition: 250,
            uniform_exploration_ppm: 10_000,
            rerooting: AtomicLevinRerooting::Disabled,
        }
    }
}

/// Lab-only rerooting choices for the atomic policy tree.
///
/// `PlayerTurnBoundaries` treats entry into a new player turn as a structural
/// clue. To remain robust when there are indefinitely many such clues, its
/// q-th observed boundary receives weight `1 / q`, the robust input-weight
/// transform from root-LTS. Descendants may be ranked by either the
/// original-root Levin cost or the weighted cost relative to an earlier
/// player-turn boundary. This is not a hard reset: the original-root cost
/// remains in the minimum for every node.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AtomicLevinRerooting {
    #[default]
    Disabled,
    PlayerTurnBoundaries,
}

#[derive(Clone, Copy, Debug)]
pub struct AtomicLevinWitnessQuantum {
    pub additional_applied_transitions: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AtomicLevinWitnessCounters {
    pub work_pops: usize,
    pub expanded_exact_states: usize,
    pub applied_action_transitions: usize,
    pub engine_steps: usize,
    pub exact_states: usize,
    pub reopened_exact_states: usize,
    pub duplicate_or_dominated_successors: usize,
    pub structured_inputs_materialized: usize,
    pub reroot_points_assigned: usize,
    pub rerooted_action_transitions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicLevinWitnessInterruption {
    AppliedTransitionBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicLevinWitnessReplayError {
    IllegalInput { action_index: usize },
    TransitionStepLimit { action_index: usize },
    SuccessorMismatch { action_index: usize },
    FinalStateIsNotWin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicLevinWitnessStatus {
    WitnessFound,
    Partial(AtomicLevinWitnessInterruption),
    FrontierExhausted,
    ReplayMismatch(AtomicLevinWitnessReplayError),
}

#[derive(Clone, Debug)]
pub struct AtomicLevinWitness {
    pub actions: Vec<TurnOptionAction>,
    pub final_position: CombatPosition,
    pub negative_log_policy: f64,
    pub replay_engine_steps: usize,
}

#[derive(Clone, Debug)]
pub struct AtomicLevinWitnessReport {
    pub before: AtomicLevinWitnessCounters,
    pub after: AtomicLevinWitnessCounters,
    pub frontier_entries: usize,
    pub max_atomic_depth: usize,
    pub max_player_turn: u32,
    pub unsupported_stable_boundaries: usize,
    pub transition_step_limit_gaps: usize,
    pub status: AtomicLevinWitnessStatus,
    pub witness: Option<AtomicLevinWitness>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AtomicLevinWatchedState {
    pub discovered: bool,
    pub accepted: bool,
    pub expanded: bool,
    pub first_discovery_after_transitions: Option<usize>,
    pub first_expansion_after_work_pops: Option<usize>,
    pub best_atomic_depth: Option<usize>,
    pub best_negative_log_policy: Option<f64>,
    pub best_levin_log_priority: Option<f64>,
    pub reroot_ordinal: Option<usize>,
    pub reroot_weight: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct PathRank {
    atomic_depth: usize,
    negative_log_policy: f64,
    levin_log_priority: f64,
    rerooted_ancestor_depth: Option<usize>,
}

impl PathRank {
    fn root() -> Self {
        Self {
            atomic_depth: 0,
            negative_log_policy: 0.0,
            levin_log_priority: 0.0,
            rerooted_ancestor_depth: None,
        }
    }

    fn child(
        parent: Self,
        conditional_probability: f64,
        reroot_points: Option<&Arc<RerootPoint>>,
    ) -> Self {
        let atomic_depth = parent.atomic_depth.saturating_add(1);
        let negative_log_policy =
            parent.negative_log_policy - conditional_probability.max(f64::MIN_POSITIVE).ln();
        let mut levin_log_priority = (atomic_depth as f64).ln() + negative_log_policy;
        let mut rerooted_ancestor_depth = None;
        let mut cursor = reroot_points.map(Arc::as_ref);
        while let Some(point) = cursor {
            let relative_depth = atomic_depth.saturating_sub(point.atomic_depth);
            if relative_depth > 0 && point.weight > 0.0 {
                let local_priority = (relative_depth as f64).ln()
                    + (negative_log_policy - point.negative_log_policy)
                    - point.weight.ln();
                if local_priority.total_cmp(&levin_log_priority) == Ordering::Less {
                    levin_log_priority = local_priority;
                    rerooted_ancestor_depth = Some(point.atomic_depth);
                }
            }
            cursor = point.parent.as_deref();
        }
        Self {
            atomic_depth,
            negative_log_policy,
            levin_log_priority,
            rerooted_ancestor_depth,
        }
    }

    fn is_better_than(self, other: Self) -> bool {
        self.levin_log_priority
            .total_cmp(&other.levin_log_priority)
            .then_with(|| {
                self.negative_log_policy
                    .total_cmp(&other.negative_log_policy)
            })
            .then_with(|| self.atomic_depth.cmp(&other.atomic_depth))
            == Ordering::Less
    }

    fn same_as(self, other: Self) -> bool {
        self.atomic_depth == other.atomic_depth
            && self.negative_log_policy.to_bits() == other.negative_log_policy.to_bits()
            && self.levin_log_priority.to_bits() == other.levin_log_priority.to_bits()
            && self.rerooted_ancestor_depth == other.rerooted_ancestor_depth
    }
}

#[derive(Clone, Debug)]
struct RerootPoint {
    parent: Option<Arc<RerootPoint>>,
    atomic_depth: usize,
    negative_log_policy: f64,
    weight: f64,
}

#[derive(Clone, Debug)]
struct PathTrace {
    parent: Option<Arc<PathTrace>>,
    action: Option<TurnOptionAction>,
}

impl PathTrace {
    fn root() -> Arc<Self> {
        Arc::new(Self {
            parent: None,
            action: None,
        })
    }

    fn extend(parent: Arc<Self>, action: TurnOptionAction) -> Arc<Self> {
        Arc::new(Self {
            parent: Some(parent),
            action: Some(action),
        })
    }

    fn actions(&self) -> Vec<TurnOptionAction> {
        let mut reversed = Vec::new();
        let mut cursor = Some(self);
        while let Some(trace) = cursor {
            if let Some(action) = trace.action.as_ref() {
                reversed.push(action.clone());
            }
            cursor = trace.parent.as_deref();
        }
        reversed.reverse();
        reversed
    }
}

#[derive(Clone, Debug)]
struct ExactSearchNode {
    position: CombatPosition,
    exact_key: CombatExactStateKey,
    rank: PathRank,
    trace: Arc<PathTrace>,
    reroot_points: Arc<RerootPoint>,
    entered_new_player_turn: bool,
}

#[derive(Clone, Debug)]
struct ApplyActionWork {
    parent: Arc<ExactSearchNode>,
    input: ClientInput,
    rank: PathRank,
    reroot_points: Arc<RerootPoint>,
}

#[derive(Clone, Debug)]
struct StructuredSelectionWork {
    parent: Arc<ExactSearchNode>,
    cursor: SelectionTransactionCursor,
    rank: PathRank,
    reroot_points: Arc<RerootPoint>,
}

#[derive(Clone, Debug)]
enum AtomicLevinWork {
    Expand(Arc<ExactSearchNode>),
    Apply(ApplyActionWork),
    StructuredSelection(StructuredSelectionWork),
}

impl AtomicLevinWork {
    fn rank(&self) -> PathRank {
        match self {
            Self::Expand(node) => node.rank,
            Self::Apply(work) => work.rank,
            Self::StructuredSelection(work) => work.rank,
        }
    }

    fn owning_node(&self) -> &ExactSearchNode {
        match self {
            Self::Expand(node) => node,
            Self::Apply(work) => &work.parent,
            Self::StructuredSelection(work) => &work.parent,
        }
    }
}

#[derive(Clone, Debug)]
struct FrontierEntry {
    rank: PathRank,
    sequence_id: u64,
    work: AtomicLevinWork,
}

impl Eq for FrontierEntry {}

impl PartialEq for FrontierEntry {
    fn eq(&self, other: &Self) -> bool {
        self.rank.same_as(other.rank) && self.sequence_id == other.sequence_id
    }
}

impl Ord for FrontierEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .rank
            .levin_log_priority
            .total_cmp(&self.rank.levin_log_priority)
            .then_with(|| {
                other
                    .rank
                    .negative_log_policy
                    .total_cmp(&self.rank.negative_log_policy)
            })
            .then_with(|| other.rank.atomic_depth.cmp(&self.rank.atomic_depth))
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for FrontierEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct AtomicLevinWitnessSession {
    root: CombatPosition,
    config: AtomicLevinWitnessConfig,
    policy: SharedCombatActionPolicy,
    frontier: BinaryHeap<FrontierEntry>,
    best_state_ranks: HashMap<CombatExactStateKey, PathRank>,
    next_sequence_id: u64,
    used: AtomicLevinWitnessCounters,
    granted_applied_transitions: usize,
    granted_engine_steps: usize,
    max_atomic_depth: usize,
    max_player_turn: u32,
    unsupported_stable_boundaries: usize,
    transition_step_limit_gaps: usize,
    witness: Option<AtomicLevinWitness>,
    replay_failure: Option<AtomicLevinWitnessReplayError>,
    watched_states: HashMap<String, AtomicLevinWatchedState>,
}

impl AtomicLevinWitnessSession {
    pub fn new(root: CombatDecisionRoot, config: AtomicLevinWitnessConfig) -> Self {
        Self::with_policy(root, config, uniform_policy())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: AtomicLevinWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let position = root.position().clone();
        let exact_key = combat_exact_state_key(&position.engine, &position.combat);
        let rank = PathRank::root();
        let root_reroot_point = Arc::new(RerootPoint {
            parent: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
            weight: 1.0,
        });
        let node = Arc::new(ExactSearchNode {
            position: position.clone(),
            exact_key: exact_key.clone(),
            rank,
            trace: PathTrace::root(),
            reroot_points: root_reroot_point,
            entered_new_player_turn: false,
        });
        let mut session = Self {
            root: position,
            config,
            policy,
            frontier: BinaryHeap::new(),
            best_state_ranks: HashMap::from([(exact_key, rank)]),
            next_sequence_id: 0,
            used: AtomicLevinWitnessCounters {
                exact_states: 1,
                reroot_points_assigned: usize::from(
                    config.rerooting == AtomicLevinRerooting::PlayerTurnBoundaries,
                ),
                ..AtomicLevinWitnessCounters::default()
            },
            granted_applied_transitions: 0,
            granted_engine_steps: 0,
            max_atomic_depth: 0,
            max_player_turn: root.turn_count(),
            unsupported_stable_boundaries: 0,
            transition_step_limit_gaps: 0,
            witness: None,
            replay_failure: None,
            watched_states: HashMap::new(),
        };
        session.push_work(AtomicLevinWork::Expand(node));
        session
    }

    pub fn counters(&self) -> AtomicLevinWitnessCounters {
        self.used
    }

    pub fn watch_exact_state_hash(&mut self, exact_state_hash: impl Into<String>) {
        let exact_state_hash = exact_state_hash.into();
        let is_root = exact_hash(&self.root) == exact_state_hash;
        self.watched_states
            .entry(exact_state_hash)
            .or_insert_with(|| AtomicLevinWatchedState {
                discovered: is_root,
                accepted: is_root,
                expanded: false,
                first_discovery_after_transitions: is_root.then_some(0),
                best_atomic_depth: is_root.then_some(0),
                best_negative_log_policy: is_root.then_some(0.0),
                best_levin_log_priority: is_root.then_some(0.0),
                ..AtomicLevinWatchedState::default()
            });
    }

    pub fn watched_state(&self, exact_state_hash: &str) -> Option<AtomicLevinWatchedState> {
        self.watched_states.get(exact_state_hash).copied()
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: AtomicLevinWitnessQuantum,
    ) -> AtomicLevinWitnessReport {
        let before = self.used;
        self.granted_applied_transitions = self
            .granted_applied_transitions
            .saturating_add(quantum.additional_applied_transitions);
        self.granted_engine_steps = self
            .granted_engine_steps
            .saturating_add(quantum.additional_engine_steps);

        let status = loop {
            if self.witness.is_some() {
                break AtomicLevinWitnessStatus::WitnessFound;
            }
            if let Some(error) = self.replay_failure.clone() {
                break AtomicLevinWitnessStatus::ReplayMismatch(error);
            }
            if deadline_reached(quantum.deadline) {
                break AtomicLevinWitnessStatus::Partial(AtomicLevinWitnessInterruption::Deadline);
            }
            let Some(entry) = self.frontier.pop() else {
                break AtomicLevinWitnessStatus::FrontierExhausted;
            };
            if !self.work_is_current(&entry.work) {
                continue;
            }

            match entry.work {
                AtomicLevinWork::Expand(node) => {
                    self.used.work_pops = self.used.work_pops.saturating_add(1);
                    self.expand_node(stepper, node);
                }
                AtomicLevinWork::StructuredSelection(mut work) => {
                    self.used.work_pops = self.used.work_pops.saturating_add(1);
                    let Some(input) = work.cursor.next_input() else {
                        continue;
                    };
                    self.used.structured_inputs_materialized =
                        self.used.structured_inputs_materialized.saturating_add(1);
                    let apply = ApplyActionWork {
                        parent: work.parent.clone(),
                        input,
                        rank: work.rank,
                        reroot_points: work.reroot_points.clone(),
                    };
                    self.push_work(AtomicLevinWork::Apply(apply));
                    if !work.cursor.is_exhausted() {
                        self.push_work(AtomicLevinWork::StructuredSelection(work));
                    }
                }
                AtomicLevinWork::Apply(work) => {
                    if self.used.applied_action_transitions >= self.granted_applied_transitions {
                        self.push_work(AtomicLevinWork::Apply(work));
                        break AtomicLevinWitnessStatus::Partial(
                            AtomicLevinWitnessInterruption::AppliedTransitionBudget,
                        );
                    }
                    if self.used.engine_steps >= self.granted_engine_steps {
                        self.push_work(AtomicLevinWork::Apply(work));
                        break AtomicLevinWitnessStatus::Partial(
                            AtomicLevinWitnessInterruption::EngineStepBudget,
                        );
                    }
                    self.used.work_pops = self.used.work_pops.saturating_add(1);
                    if let Some(status) = self.apply_action(stepper, work, quantum.deadline) {
                        break status;
                    }
                }
            }
        };

        AtomicLevinWitnessReport {
            before,
            after: self.used,
            frontier_entries: self.frontier.len(),
            max_atomic_depth: self.max_atomic_depth,
            max_player_turn: self.max_player_turn,
            unsupported_stable_boundaries: self.unsupported_stable_boundaries,
            transition_step_limit_gaps: self.transition_step_limit_gaps,
            status,
            witness: self.witness.clone(),
        }
    }

    fn expand_node(&mut self, stepper: &dyn CombatStepper, node: Arc<ExactSearchNode>) {
        self.used.expanded_exact_states = self.used.expanded_exact_states.saturating_add(1);
        let node_hash = (!self.watched_states.is_empty()).then(|| exact_hash(&node.position));
        if let Some(watched) = node_hash
            .as_deref()
            .and_then(|hash| self.watched_states.get_mut(hash))
        {
            watched.expanded = true;
            watched
                .first_expansion_after_work_pops
                .get_or_insert(self.used.work_pops);
        }
        match stepper.terminal(&node.position) {
            CombatTerminal::Win => {
                self.finish_witness(stepper, &node);
                return;
            }
            CombatTerminal::Loss => return,
            CombatTerminal::Unresolved => {}
        }

        let surface = stepper.legal_action_surface(&node.position);
        if surface.atomic_actions.is_empty() && surface.selection_families.is_empty() {
            self.unsupported_stable_boundaries =
                self.unsupported_stable_boundaries.saturating_add(1);
            return;
        }
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
        let weights = self.policy.weights(&node.position, &choices);
        let weights = (weights.len() == choices.len())
            .then_some(weights)
            .unwrap_or_else(|| vec![1.0; choices.len()]);
        let probabilities = normalized_probabilities(weights, self.config.uniform_exploration_ppm);
        let atomic_action_count = surface.atomic_actions.len();
        let reroot_points = self.reroot_points_for_descendants(&node);

        for (input, probability) in surface
            .atomic_actions
            .into_iter()
            .zip(probabilities[..atomic_action_count].iter().copied())
        {
            self.push_work(AtomicLevinWork::Apply(ApplyActionWork {
                parent: node.clone(),
                input,
                rank: PathRank::child(node.rank, probability, Some(&reroot_points)),
                reroot_points: reroot_points.clone(),
            }));
        }

        if !surface.selection_families.is_empty()
            && !stepper.supports_canonical_pending_choice_actions()
        {
            self.unsupported_stable_boundaries =
                self.unsupported_stable_boundaries.saturating_add(1);
            return;
        }
        for (family, family_probability) in surface
            .selection_families
            .into_iter()
            .zip(probabilities[atomic_action_count..].iter().copied())
        {
            let Ok(mut cursor) = SelectionTransactionCursor::new(&family) else {
                self.unsupported_stable_boundaries =
                    self.unsupported_stable_boundaries.saturating_add(1);
                continue;
            };
            if cursor.is_exhausted() {
                continue;
            }
            if family.declared_min == 1 && family.effective_max == 1 {
                let members = std::iter::from_fn(|| cursor.next_input()).collect::<Vec<_>>();
                let member_weights = self.policy.structured_selection_member_weights(
                    &node.position,
                    &family,
                    &members,
                );
                let member_weights = (member_weights.len() == members.len())
                    .then_some(member_weights)
                    .unwrap_or_else(|| vec![1.0; members.len()]);
                let member_probabilities =
                    normalized_probabilities(member_weights, self.config.uniform_exploration_ppm);
                for (input, member_probability) in members.into_iter().zip(member_probabilities) {
                    self.used.structured_inputs_materialized =
                        self.used.structured_inputs_materialized.saturating_add(1);
                    self.push_work(AtomicLevinWork::Apply(ApplyActionWork {
                        parent: node.clone(),
                        input,
                        rank: PathRank::child(
                            node.rank,
                            family_probability * member_probability,
                            Some(&reroot_points),
                        ),
                        reroot_points: reroot_points.clone(),
                    }));
                }
            } else {
                let member_count = cursor.remaining_input_count();
                if member_count == 0 {
                    continue;
                }
                let member_probability = family_probability / member_count as f64;
                let rank = PathRank::child(node.rank, member_probability, Some(&reroot_points));
                self.push_work(AtomicLevinWork::StructuredSelection(
                    StructuredSelectionWork {
                        parent: node.clone(),
                        cursor,
                        rank,
                        reroot_points: reroot_points.clone(),
                    },
                ));
            }
        }
    }

    fn apply_action(
        &mut self,
        stepper: &dyn CombatStepper,
        work: ApplyActionWork,
        deadline: Option<Instant>,
    ) -> Option<AtomicLevinWitnessStatus> {
        if !stepper.is_legal_action(&work.parent.position, &work.input) {
            self.unsupported_stable_boundaries =
                self.unsupported_stable_boundaries.saturating_add(1);
            return None;
        }
        let result = stepper.apply_to_stable(
            &work.parent.position,
            work.input.clone(),
            CombatStepLimits {
                max_engine_steps: self.config.max_engine_steps_per_transition,
                deadline,
            },
        );
        self.used.engine_steps = self.used.engine_steps.saturating_add(result.engine_steps);
        if result.timed_out {
            self.push_work(AtomicLevinWork::Apply(work));
            return Some(AtomicLevinWitnessStatus::Partial(
                AtomicLevinWitnessInterruption::Deadline,
            ));
        }
        self.used.applied_action_transitions =
            self.used.applied_action_transitions.saturating_add(1);
        if work.rank.rerooted_ancestor_depth.is_some() {
            self.used.rerooted_action_transitions =
                self.used.rerooted_action_transitions.saturating_add(1);
        }
        if result.truncated {
            self.transition_step_limit_gaps = self.transition_step_limit_gaps.saturating_add(1);
            return None;
        }

        let successor_hash = exact_hash(&result.position);
        if let Some(watched) = self.watched_states.get_mut(&successor_hash) {
            watched.discovered = true;
            watched
                .first_discovery_after_transitions
                .get_or_insert(self.used.applied_action_transitions);
            if watched
                .best_levin_log_priority
                .is_none_or(|priority| work.rank.levin_log_priority < priority)
            {
                watched.best_atomic_depth = Some(work.rank.atomic_depth);
                watched.best_negative_log_policy = Some(work.rank.negative_log_policy);
                watched.best_levin_log_priority = Some(work.rank.levin_log_priority);
            }
        }
        let action = TurnOptionAction {
            input: work.input,
            expected_successor_hash: successor_hash.clone(),
            engine_steps: result.engine_steps,
        };
        let trace = PathTrace::extend(work.parent.trace.clone(), action);
        let exact_key = combat_exact_state_key(&result.position.engine, &result.position.combat);
        let node = Arc::new(ExactSearchNode {
            entered_new_player_turn: result.position.combat.turn.turn_count
                > work.parent.position.combat.turn.turn_count,
            position: result.position,
            exact_key: exact_key.clone(),
            rank: work.rank,
            trace,
            reroot_points: work.reroot_points,
        });
        self.max_atomic_depth = self.max_atomic_depth.max(work.rank.atomic_depth);
        self.max_player_turn = self
            .max_player_turn
            .max(node.position.combat.turn.turn_count);

        if stepper.terminal(&node.position) == CombatTerminal::Win {
            self.finish_witness(stepper, &node);
            return self
                .replay_failure
                .clone()
                .map(AtomicLevinWitnessStatus::ReplayMismatch)
                .or(Some(AtomicLevinWitnessStatus::WitnessFound));
        }
        if stepper.terminal(&node.position) == CombatTerminal::Loss {
            return None;
        }

        match self.best_state_ranks.get(&exact_key).copied() {
            None => {
                self.best_state_ranks.insert(exact_key, work.rank);
                self.used.exact_states = self.used.exact_states.saturating_add(1);
                if let Some(watched) = self.watched_states.get_mut(&successor_hash) {
                    watched.accepted = true;
                }
                self.push_work(AtomicLevinWork::Expand(node));
            }
            Some(previous) if work.rank.is_better_than(previous) => {
                self.best_state_ranks.insert(exact_key, work.rank);
                self.used.reopened_exact_states = self.used.reopened_exact_states.saturating_add(1);
                if let Some(watched) = self.watched_states.get_mut(&successor_hash) {
                    watched.accepted = true;
                }
                self.push_work(AtomicLevinWork::Expand(node));
            }
            Some(_) => {
                self.used.duplicate_or_dominated_successors = self
                    .used
                    .duplicate_or_dominated_successors
                    .saturating_add(1);
            }
        }
        None
    }

    fn finish_witness(&mut self, stepper: &dyn CombatStepper, node: &ExactSearchNode) {
        if self.witness.is_some() || self.replay_failure.is_some() {
            return;
        }
        let actions = node.trace.actions();
        match replay_exact_witness(
            stepper,
            &self.root,
            &actions,
            self.config.max_engine_steps_per_transition,
        ) {
            Ok((final_position, replay_engine_steps)) => {
                self.witness = Some(AtomicLevinWitness {
                    actions,
                    final_position,
                    negative_log_policy: node.rank.negative_log_policy,
                    replay_engine_steps,
                });
            }
            Err(error) => self.replay_failure = Some(error),
        }
    }

    fn push_work(&mut self, work: AtomicLevinWork) {
        let rank = work.rank();
        let sequence_id = self.next_sequence_id;
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
        self.frontier.push(FrontierEntry {
            rank,
            sequence_id,
            work,
        });
    }

    fn reroot_points_for_descendants(&mut self, node: &ExactSearchNode) -> Arc<RerootPoint> {
        let should_reroot = match self.config.rerooting {
            AtomicLevinRerooting::Disabled => false,
            AtomicLevinRerooting::PlayerTurnBoundaries => node.entered_new_player_turn,
        };
        if !should_reroot {
            return node.reroot_points.clone();
        }
        self.used.reroot_points_assigned = self.used.reroot_points_assigned.saturating_add(1);
        let weight = 1.0 / self.used.reroot_points_assigned as f64;
        if !self.watched_states.is_empty() {
            let node_hash = exact_hash(&node.position);
            if let Some(watched) = self.watched_states.get_mut(&node_hash) {
                watched
                    .reroot_ordinal
                    .get_or_insert(self.used.reroot_points_assigned);
                watched.reroot_weight.get_or_insert(weight);
            }
        }
        Arc::new(RerootPoint {
            parent: Some(node.reroot_points.clone()),
            atomic_depth: node.rank.atomic_depth,
            negative_log_policy: node.rank.negative_log_policy,
            weight,
        })
    }

    fn work_is_current(&self, work: &AtomicLevinWork) -> bool {
        let node = work.owning_node();
        self.best_state_ranks
            .get(&node.exact_key)
            .is_some_and(|rank| rank.same_as(node.rank))
    }
}

fn replay_exact_witness(
    stepper: &dyn CombatStepper,
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<(CombatPosition, usize), AtomicLevinWitnessReplayError> {
    let mut position = root.clone();
    let mut engine_steps = 0usize;
    for (action_index, action) in actions.iter().enumerate() {
        if !stepper.is_legal_action(&position, &action.input) {
            return Err(AtomicLevinWitnessReplayError::IllegalInput { action_index });
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.truncated {
            return Err(AtomicLevinWitnessReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash {
            return Err(AtomicLevinWitnessReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win {
        return Err(AtomicLevinWitnessReplayError::FinalStateIsNotWin);
    }
    Ok((position, engine_steps))
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

#[cfg(test)]
mod rank_tests {
    use super::*;

    #[test]
    fn root_only_rank_is_plain_levin_cost() {
        let root = Arc::new(RerootPoint {
            parent: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
            weight: 1.0,
        });
        let first = PathRank::child(PathRank::root(), 0.25, Some(&root));
        let second = PathRank::child(first, 0.5, Some(&root));

        let expected = 2.0_f64.ln() - (0.25_f64 * 0.5).ln();
        assert!((second.levin_log_priority - expected).abs() < 1e-12);
        assert_eq!(second.rerooted_ancestor_depth, None);
    }

    #[test]
    fn turn_boundary_reroot_keeps_root_cost_but_can_lower_suffix_cost() {
        let root = Arc::new(RerootPoint {
            parent: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
            weight: 1.0,
        });
        let unlikely_prefix = PathRank::child(PathRank::root(), 0.01, Some(&root));
        let boundary = Arc::new(RerootPoint {
            parent: Some(root),
            atomic_depth: unlikely_prefix.atomic_depth,
            negative_log_policy: unlikely_prefix.negative_log_policy,
            weight: 1.0,
        });
        let suffix = PathRank::child(unlikely_prefix, 0.5, Some(&boundary));

        let plain_root_cost = 2.0_f64.ln() - (0.01_f64 * 0.5).ln();
        let local_cost = 1.0_f64.ln() - 0.5_f64.ln();
        assert!((suffix.levin_log_priority - local_cost).abs() < 1e-12);
        assert!(suffix.levin_log_priority < plain_root_cost);
        assert_eq!(suffix.rerooted_ancestor_depth, Some(1));
    }

    #[test]
    fn weak_reroot_weight_cannot_displace_better_root_cost() {
        let root = Arc::new(RerootPoint {
            parent: None,
            atomic_depth: 0,
            negative_log_policy: 0.0,
            weight: 1.0,
        });
        let prefix = PathRank::child(PathRank::root(), 0.9, Some(&root));
        let boundary = Arc::new(RerootPoint {
            parent: Some(root),
            atomic_depth: prefix.atomic_depth,
            negative_log_policy: prefix.negative_log_policy,
            weight: 1e-9,
        });
        let suffix = PathRank::child(prefix, 0.9, Some(&boundary));

        let plain_root_cost = 2.0_f64.ln() - (0.9_f64 * 0.9).ln();
        assert!((suffix.levin_log_priority - plain_root_cost).abs() < 1e-12);
        assert_eq!(suffix.rerooted_ancestor_depth, None);
    }
}
