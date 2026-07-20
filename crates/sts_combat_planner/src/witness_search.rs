use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sts_core::ai::combat_state_key::{combat_exact_state_key, CombatExactStateKey};
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};

use super::policy::{uniform_policy, CombatStateGuideRank, SharedCombatActionPolicy};
use super::types::{
    exact_hash, CombatDecisionRoot, CompleteTurnOptionBoundary, TurnOptionAction,
    TurnOptionGenerationGap, TurnOptionGeneratorConfig,
};
use super::{CombatPlanningQuantum, TurnOptionGeneratorSession};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OracleCombatWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    pub generation_work_per_agenda_pop: usize,
    pub satisfaction: OracleCombatWitnessSatisfaction,
}

impl Default for OracleCombatWitnessConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OracleCombatWitnessSatisfaction {
    #[default]
    FirstWitness,
    HpLossAtMost(u32),
    BudgetOrExhaustion,
}

#[derive(Clone, Copy, Debug)]
pub struct OracleCombatWitnessQuantum {
    pub additional_agenda_pops: usize,
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

impl OracleCombatWitnessQuantum {
    pub fn deterministic(agenda_pops: usize, generation_work: usize, engine_steps: usize) -> Self {
        Self {
            additional_agenda_pops: agenda_pops,
            additional_generation_work: generation_work,
            additional_engine_steps: engine_steps,
            deadline: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OracleCombatWitnessCounters {
    pub agenda_pops: usize,
    pub generation_work: usize,
    pub engine_steps: usize,
    pub exact_states: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub completed_turn_options: usize,
    pub policy_witness_proposals: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleCombatWitnessInterruption {
    AgendaBudget,
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleCombatWitnessReplayError {
    IllegalInput { action_index: usize },
    TransitionStepLimit { action_index: usize },
    SuccessorMismatch { action_index: usize },
    FinalStateIsNotWin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleCombatWitnessStatus {
    WitnessFound,
    Partial(OracleCombatWitnessInterruption),
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OracleCombatWitness {
    pub actions: Vec<TurnOptionAction>,
    pub final_position: CombatPosition,
    pub negative_log_policy: f64,
    pub replay_engine_steps: usize,
}

#[derive(Clone, Debug)]
pub struct OracleCombatWitnessReport {
    pub before: OracleCombatWitnessCounters,
    pub after: OracleCombatWitnessCounters,
    pub retained_state_work: usize,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub status: OracleCombatWitnessStatus,
    pub witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OracleCombatWitnessProgressSnapshot {
    pub retained_states: usize,
    pub queued_anchor_entries: usize,
    pub queued_guided_entries: Vec<usize>,
    pub max_player_turn: u32,
    pub max_path_atomic_depth: usize,
    pub max_completed_turn_options_at_state: usize,
    pub generation_gap_count: usize,
    pub pending_witness_replay: bool,
    pub root_state: Option<OracleCombatWitnessStateProgressSnapshot>,
    pub deepest_survival_state: Option<OracleCombatDeepStateSnapshot>,
    pub deepest_progress_state: Option<OracleCombatDeepStateSnapshot>,
    /// Exact public action prefix that reaches `deepest_survival_state`.
    /// Diagnostic only; it has no authority over queue ordering.
    pub deepest_survival_actions: Vec<TurnOptionAction>,
    /// Exact public action prefix that reaches `deepest_progress_state`.
    /// Diagnostic only; it has no authority over queue ordering.
    pub deepest_progress_actions: Vec<TurnOptionAction>,
    /// For each of the most recent retained player turns, the state with the
    /// highest player HP (then least remaining enemy HP). This is diagnostic:
    /// it exposes whether deeper search is advancing only along a dying line
    /// without assigning that envelope any search authority.
    pub recent_turn_survival_envelope: Vec<OracleCombatDeepStateSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatDeepStateSnapshot {
    pub player_turn: u32,
    pub player_hp: i32,
    pub player_block: i32,
    pub alive_enemy_count: usize,
    pub enemy_total_hp: i32,
    pub hand_size: usize,
    pub draw_pile_size: usize,
    pub discard_pile_size: usize,
    pub exhaust_pile_size: usize,
    pub path_atomic_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatWitnessStateProgressSnapshot {
    pub exact_state_hash: String,
    pub path_atomic_depth: usize,
    pub path_negative_log_policy: f64,
    pub generator_work: usize,
    pub generator_engine_steps: usize,
    pub completed_turn_options: usize,
    pub retained_generator_work_items: usize,
    pub synced_options: usize,
    pub anchor_states_ahead: Option<usize>,
    pub guided_states_ahead: Option<Vec<usize>>,
}

#[derive(Clone, Copy, Debug)]
struct PathRank {
    atomic_depth: usize,
    negative_log_policy: f64,
}

impl PathRank {
    fn levin_log_priority(self) -> f64 {
        (self.atomic_depth.max(1) as f64).ln() + self.negative_log_policy
    }

    fn better_than(self, other: Self) -> bool {
        self.levin_log_priority()
            .total_cmp(&other.levin_log_priority())
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
    }
}

struct SearchState {
    exact_key: CombatExactStateKey,
    path: PathRank,
    guide_ranks: Vec<CombatStateGuideRank>,
    queue_revision: u64,
    actions: Vec<TurnOptionAction>,
    generator: TurnOptionGeneratorSession,
    synced_options: usize,
    synced_gaps: usize,
}

#[derive(Clone, Copy, Debug)]
struct StateQueueEntry {
    state_id: usize,
    revision: u64,
    sequence_id: u64,
    priority: f64,
}

impl Eq for StateQueueEntry {}

impl PartialEq for StateQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority.to_bits() == other.priority.to_bits() && self.sequence_id == other.sequence_id
    }
}

impl Ord for StateQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .total_cmp(&self.priority)
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for StateQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct GuidedStateQueueEntry {
    guide_index: usize,
    state_id: usize,
    revision: u64,
    sequence_id: u64,
    guide_rank: CombatStateGuideRank,
    anchor_priority: f64,
}

impl Eq for GuidedStateQueueEntry {}

impl PartialEq for GuidedStateQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.state_id == other.state_id
            && self.guide_index == other.guide_index
            && self.revision == other.revision
            && self.sequence_id == other.sequence_id
    }
}

impl Ord for GuidedStateQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.guide_rank
            .cmp(&other.guide_rank)
            // Guide ties retain the policy-only Levin ordering.
            .then_with(|| other.anchor_priority.total_cmp(&self.anchor_priority))
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for GuidedStateQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct PendingWitnessReplay {
    actions: Vec<TurnOptionAction>,
    negative_log_policy: f64,
    position: CombatPosition,
    next_action: usize,
    engine_steps: usize,
    final_hp_hint: i32,
}

pub struct OracleCombatWitnessSession {
    root: CombatDecisionRoot,
    config: OracleCombatWitnessConfig,
    policy: SharedCombatActionPolicy,
    states: Vec<Option<SearchState>>,
    anchor_frontier: BinaryHeap<StateQueueEntry>,
    guided_frontiers: Vec<BinaryHeap<GuidedStateQueueEntry>>,
    next_scheduler_lane: usize,
    best_paths: HashMap<CombatExactStateKey, PathRank>,
    next_sequence_id: u64,
    used: OracleCombatWitnessCounters,
    granted_agenda_pops: usize,
    granted_generation_work: usize,
    granted_engine_steps: usize,
    gaps: Vec<TurnOptionGenerationGap>,
    pending_witness: Option<PendingWitnessReplay>,
    witness: Option<OracleCombatWitness>,
    replay_failure: Option<OracleCombatWitnessReplayError>,
}

// A policy proposal is a root-level capability donor, not another search
// frontier. Re-running a bounded legacy rollout for every popped exact state
// consumed most of the improvement budget after the first verified incumbent.
// The planner owns all continuation search after this single proposal.
const MAX_POLICY_WITNESS_PROPOSALS: usize = 1;

impl OracleCombatWitnessSession {
    pub fn new(root: CombatDecisionRoot, config: OracleCombatWitnessConfig) -> Self {
        Self::with_policy(root, config, uniform_policy())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: OracleCombatWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let exact_key = combat_exact_state_key(&root.position().engine, &root.position().combat);
        let path = PathRank {
            atomic_depth: 0,
            negative_log_policy: 0.0,
        };
        let state = SearchState {
            exact_key: exact_key.clone(),
            path,
            guide_ranks: policy.state_guide_ranks(root.position()),
            queue_revision: 0,
            actions: Vec::new(),
            generator: TurnOptionGeneratorSession::with_policy(
                root.clone(),
                config.generator,
                policy.clone(),
            ),
            synced_options: 0,
            synced_gaps: 0,
        };
        let mut session = Self {
            root,
            config,
            policy,
            states: vec![Some(state)],
            anchor_frontier: BinaryHeap::new(),
            guided_frontiers: Vec::new(),
            next_scheduler_lane: 0,
            best_paths: HashMap::from([(exact_key, path)]),
            next_sequence_id: 0,
            used: OracleCombatWitnessCounters {
                exact_states: 1,
                ..OracleCombatWitnessCounters::default()
            },
            granted_agenda_pops: 0,
            granted_generation_work: 0,
            granted_engine_steps: 0,
            gaps: Vec::new(),
            pending_witness: None,
            witness: None,
            replay_failure: None,
        };
        session.queue_state(0);
        session
    }

    pub fn witness(&self) -> Option<&OracleCombatWitness> {
        self.witness.as_ref()
    }

    pub fn restore_verified_witness(&mut self, witness: OracleCombatWitness) -> Result<(), String> {
        if sts_core::sim::combat::combat_terminal(
            &witness.final_position.engine,
            &witness.final_position.combat,
        ) != CombatTerminal::Win
        {
            return Err("restored oracle combat witness is not terminal victory".to_string());
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

    pub fn counters(&self) -> OracleCombatWitnessCounters {
        self.used
    }

    pub fn retained_state_work(&self) -> usize {
        self.states.iter().filter(|state| state.is_some()).count()
    }

    pub fn progress_snapshot(&self) -> OracleCombatWitnessProgressSnapshot {
        let mut survival_by_turn =
            std::collections::BTreeMap::<u32, OracleCombatDeepStateSnapshot>::new();
        let mut snapshot = OracleCombatWitnessProgressSnapshot {
            retained_states: self.retained_state_work(),
            queued_anchor_entries: self.anchor_frontier.len(),
            queued_guided_entries: self.guided_frontiers.iter().map(BinaryHeap::len).collect(),
            generation_gap_count: self.gaps.len(),
            pending_witness_replay: self.pending_witness.is_some(),
            root_state: self
                .states
                .first()
                .and_then(Option::as_ref)
                .map(state_progress_snapshot),
            ..OracleCombatWitnessProgressSnapshot::default()
        };
        for state in self.states.iter().flatten() {
            let deep_state = deep_state_snapshot(state);
            let replace_turn_survival =
                survival_by_turn
                    .get(&deep_state.player_turn)
                    .is_none_or(|current| {
                        (
                            deep_state.player_hp,
                            -deep_state.enemy_total_hp,
                            -i32::try_from(deep_state.alive_enemy_count).unwrap_or(i32::MAX),
                        ) > (
                            current.player_hp,
                            -current.enemy_total_hp,
                            -i32::try_from(current.alive_enemy_count).unwrap_or(i32::MAX),
                        )
                    });
            if replace_turn_survival {
                survival_by_turn.insert(deep_state.player_turn, deep_state.clone());
            }
            if deep_state.player_turn > snapshot.max_player_turn {
                snapshot.max_player_turn = deep_state.player_turn;
                snapshot.deepest_survival_state = None;
                snapshot.deepest_progress_state = None;
                snapshot.deepest_survival_actions.clear();
                snapshot.deepest_progress_actions.clear();
            }
            if deep_state.player_turn == snapshot.max_player_turn {
                let replace_survival =
                    snapshot
                        .deepest_survival_state
                        .as_ref()
                        .is_none_or(|current| {
                            (
                                deep_state.player_hp,
                                deep_state.player_block,
                                -deep_state.enemy_total_hp,
                            ) > (
                                current.player_hp,
                                current.player_block,
                                -current.enemy_total_hp,
                            )
                        });
                if replace_survival {
                    snapshot.deepest_survival_state = Some(deep_state.clone());
                    snapshot.deepest_survival_actions = state.actions.clone();
                }
                let replace_progress =
                    snapshot
                        .deepest_progress_state
                        .as_ref()
                        .is_none_or(|current| {
                            (
                                deep_state.enemy_total_hp,
                                -deep_state.player_hp,
                                -deep_state.player_block,
                            ) < (
                                current.enemy_total_hp,
                                -current.player_hp,
                                -current.player_block,
                            )
                        });
                if replace_progress {
                    snapshot.deepest_progress_state = Some(deep_state);
                    snapshot.deepest_progress_actions = state.actions.clone();
                }
            }
            snapshot.max_path_atomic_depth =
                snapshot.max_path_atomic_depth.max(state.path.atomic_depth);
            snapshot.max_completed_turn_options_at_state = snapshot
                .max_completed_turn_options_at_state
                .max(state.generator.completed_options().len());
        }
        snapshot.recent_turn_survival_envelope = survival_by_turn
            .into_values()
            .rev()
            .take(32)
            .collect::<Vec<_>>();
        snapshot.recent_turn_survival_envelope.reverse();
        snapshot
    }

    pub fn state_progress_by_exact_hash(
        &self,
        exact_state_hash: &str,
    ) -> Option<OracleCombatWitnessStateProgressSnapshot> {
        self.states
            .iter()
            .flatten()
            .find(|state| exact_hash(state.generator.root().position()) == exact_state_hash)
            .map(|state| self.state_progress_snapshot_with_ranks(state))
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: OracleCombatWitnessQuantum,
    ) -> OracleCombatWitnessReport {
        let before = self.used;
        self.granted_agenda_pops = self
            .granted_agenda_pops
            .saturating_add(quantum.additional_agenda_pops);
        self.granted_generation_work = self
            .granted_generation_work
            .saturating_add(quantum.additional_generation_work);
        self.granted_engine_steps = self
            .granted_engine_steps
            .saturating_add(quantum.additional_engine_steps);

        let status = loop {
            if let Some(status) = self.advance_pending_witness(stepper, quantum.deadline) {
                break status;
            }
            if self.witness_satisfies() {
                break OracleCombatWitnessStatus::WitnessFound;
            }
            if let Some(error) = self.replay_failure.clone() {
                break OracleCombatWitnessStatus::ReplayMismatch(error);
            }
            if deadline_reached(quantum.deadline) {
                break OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::Deadline,
                );
            }
            if self.used.agenda_pops >= self.granted_agenda_pops {
                break OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::AgendaBudget,
                );
            }
            if self.used.generation_work >= self.granted_generation_work {
                break OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::GenerationWorkBudget,
                );
            }
            let Some((state_id, mut state)) = self.pop_scheduled_state() else {
                break if self.gaps.is_empty() {
                    OracleCombatWitnessStatus::FrontierExhausted
                } else {
                    OracleCombatWitnessStatus::MechanicsGap
                };
            };

            self.used.agenda_pops = self.used.agenda_pops.saturating_add(1);
            if self.used.policy_witness_proposals < MAX_POLICY_WITNESS_PROPOSALS {
                self.used.policy_witness_proposals =
                    self.used.policy_witness_proposals.saturating_add(1);
                if let Some(proposal) = self
                    .policy
                    .witness_proposal(state.generator.root().position(), quantum.deadline)
                {
                    let mut actions = state.actions.clone();
                    actions.extend(proposal.actions);
                    let candidate = PendingWitnessReplay {
                        actions,
                        negative_log_policy: state.path.negative_log_policy,
                        position: self.root.position().clone(),
                        next_action: 0,
                        engine_steps: 0,
                        final_hp_hint: proposal.final_hp_hint,
                    };
                    let replace = self
                        .pending_witness
                        .as_ref()
                        .is_none_or(|pending| pending_witness_better(&candidate, pending));
                    if replace {
                        self.pending_witness = Some(candidate);
                    }
                    self.states[state_id] = Some(state);
                    self.queue_state(state_id);
                    continue;
                }
            }
            let generation_grant = self.config.generation_work_per_agenda_pop.max(1).min(
                self.granted_generation_work
                    .saturating_sub(self.used.generation_work),
            );
            // A generator work batch may contain more than one exact action
            // transition. Reserve one transition allowance per granted work
            // item; otherwise every agenda pop is forced to yield after its
            // first action even when `generation_work_per_agenda_pop` asks
            // for a larger coherent slice.
            let engine_grant = self
                .config
                .generator
                .max_engine_steps_per_transition
                .saturating_mul(generation_grant)
                .min(
                    self.granted_engine_steps
                        .saturating_sub(self.used.engine_steps),
                );
            let generation = state.generator.advance(
                stepper,
                CombatPlanningQuantum {
                    additional_generation_work: generation_grant,
                    additional_engine_steps: engine_grant,
                    deadline: quantum.deadline,
                },
            );
            self.used.generation_work = self.used.generation_work.saturating_add(
                generation
                    .after
                    .generation_work
                    .saturating_sub(generation.before.generation_work),
            );
            self.used.engine_steps = self.used.engine_steps.saturating_add(
                generation
                    .after
                    .engine_steps
                    .saturating_sub(generation.before.engine_steps),
            );
            self.used.applied_action_transitions =
                self.used.applied_action_transitions.saturating_add(
                    generation
                        .after_diagnostics
                        .applied_action_transitions
                        .saturating_sub(generation.before_diagnostics.applied_action_transitions),
                );
            self.used.unique_successor_states = self.used.unique_successor_states.saturating_add(
                generation
                    .after_diagnostics
                    .unique_successor_states
                    .saturating_sub(generation.before_diagnostics.unique_successor_states),
            );
            self.used.duplicate_exact_successors =
                self.used.duplicate_exact_successors.saturating_add(
                    generation
                        .after_diagnostics
                        .duplicate_exact_successors
                        .saturating_sub(generation.before_diagnostics.duplicate_exact_successors),
                );
            self.used.completed_turn_options = self
                .used
                .completed_turn_options
                .saturating_add(generation.newly_completed_options);
            state.generator.release_unused_grant();
            self.gaps
                .extend(generation.gaps[state.synced_gaps..].iter().cloned());
            state.synced_gaps = generation.gaps.len();

            let new_options = state.generator.completed_options()[state.synced_options..].to_vec();
            state.synced_options = state.generator.completed_options().len();
            for option in new_options {
                self.accept_option(&state, option);
            }

            if !state.generator.is_finished() {
                self.states[state_id] = Some(state);
                self.queue_state(state_id);
            }
        };

        OracleCombatWitnessReport {
            before,
            after: self.used,
            retained_state_work: self.retained_state_work(),
            generation_gaps: self.gaps.clone(),
            status,
            witness: self.witness.clone(),
        }
    }

    fn accept_option(&mut self, state: &SearchState, option: super::CompleteTurnOption) {
        let mut actions = state.actions.clone();
        actions.extend(option.actions().iter().cloned());
        let path = PathRank {
            atomic_depth: state
                .path
                .atomic_depth
                .saturating_add(option.actions().len()),
            negative_log_policy: state.path.negative_log_policy + option.negative_log_policy(),
        };
        match option.boundary() {
            CompleteTurnOptionBoundary::TerminalWin => {
                let candidate = PendingWitnessReplay {
                    actions,
                    negative_log_policy: path.negative_log_policy,
                    position: self.root.position().clone(),
                    next_action: 0,
                    engine_steps: 0,
                    final_hp_hint: option.exact_successor().combat.entities.player.current_hp,
                };
                let replace = self
                    .pending_witness
                    .as_ref()
                    .is_none_or(|pending| pending_witness_better(&candidate, pending));
                if replace {
                    self.pending_witness = Some(candidate);
                }
            }
            CompleteTurnOptionBoundary::NextPlayerTurn => {
                let exact_key = combat_exact_state_key(
                    &option.exact_successor().engine,
                    &option.exact_successor().combat,
                );
                let should_insert = self
                    .best_paths
                    .get(&exact_key)
                    .is_none_or(|known| path.better_than(*known));
                if !should_insert {
                    return;
                }
                let Ok(root) = CombatDecisionRoot::new(option.exact_successor().clone()) else {
                    return;
                };
                self.best_paths.insert(exact_key.clone(), path);
                self.used.exact_states = self.best_paths.len();
                let state_id = self.states.len();
                self.states.push(Some(SearchState {
                    exact_key,
                    path,
                    guide_ranks: self.policy.state_guide_ranks(root.position()),
                    queue_revision: 0,
                    actions,
                    generator: TurnOptionGeneratorSession::with_policy(
                        root,
                        self.config.generator,
                        self.policy.clone(),
                    ),
                    synced_options: 0,
                    synced_gaps: 0,
                }));
                self.queue_state(state_id);
            }
            CompleteTurnOptionBoundary::TerminalLoss | CompleteTurnOptionBoundary::Escape => {}
        }
    }

    fn queue_state(&mut self, state_id: usize) {
        let Some(state) = self.states.get_mut(state_id).and_then(Option::as_mut) else {
            return;
        };
        let Some((local_depth, local_negative_log_policy)) =
            state.generator.best_retained_path_bound()
        else {
            return;
        };
        state.queue_revision = state.queue_revision.saturating_add(1);
        let revision = state.queue_revision;
        let path_priority = PathRank {
            atomic_depth: state.path.atomic_depth.saturating_add(local_depth),
            negative_log_policy: state.path.negative_log_policy + local_negative_log_policy,
        }
        .levin_log_priority();
        let priority = service_aware_anchor_priority(
            path_priority,
            state.generator.counters().generation_work,
        );
        let sequence_id = self.next_sequence_id;
        self.anchor_frontier.push(StateQueueEntry {
            state_id,
            revision,
            sequence_id,
            priority,
        });
        if self.guided_frontiers.len() < state.guide_ranks.len() {
            self.guided_frontiers
                .resize_with(state.guide_ranks.len(), BinaryHeap::new);
        }
        for (guide_index, guide_rank) in state.guide_ranks.iter().cloned().enumerate() {
            self.guided_frontiers[guide_index].push(GuidedStateQueueEntry {
                guide_index,
                state_id,
                revision,
                sequence_id,
                guide_rank,
                anchor_priority: priority,
            });
        }
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
    }

    fn pop_scheduled_state(&mut self) -> Option<(usize, SearchState)> {
        let lane_count = self.guided_frontiers.len().saturating_add(1);
        for offset in 0..lane_count {
            let lane = (self.next_scheduler_lane + offset) % lane_count;
            let state = if lane == 0 {
                self.pop_anchor_state()
            } else {
                self.pop_guided_state(lane - 1)
            };
            if state.is_some() {
                self.next_scheduler_lane = (lane + 1) % lane_count;
                return state;
            }
        }
        None
    }

    fn pop_anchor_state(&mut self) -> Option<(usize, SearchState)> {
        while let Some(entry) = self.anchor_frontier.pop() {
            if self.entry_is_current(entry.state_id, entry.revision) {
                let state = self.states[entry.state_id]
                    .take()
                    .expect("current queue entry owns a live state");
                return Some((entry.state_id, state));
            }
        }
        None
    }

    fn pop_guided_state(&mut self, guide_index: usize) -> Option<(usize, SearchState)> {
        while let Some(entry) = self.guided_frontiers.get_mut(guide_index)?.pop() {
            if self.entry_is_current(entry.state_id, entry.revision) {
                let state = self.states[entry.state_id]
                    .take()
                    .expect("current queue entry owns a live state");
                return Some((entry.state_id, state));
            }
        }
        None
    }

    fn entry_is_current(&self, state_id: usize, revision: u64) -> bool {
        self.states
            .get(state_id)
            .and_then(Option::as_ref)
            .is_some_and(|state| {
                state.queue_revision == revision
                    && self
                        .best_paths
                        .get(&state.exact_key)
                        .is_some_and(|rank| rank.same_as(state.path))
            })
    }

    fn state_progress_snapshot_with_ranks(
        &self,
        state: &SearchState,
    ) -> OracleCombatWitnessStateProgressSnapshot {
        let mut snapshot = state_progress_snapshot(state);
        let target_anchor = combined_anchor_priority(state);
        let mut anchor_states_ahead = 0usize;
        let mut guided_states_ahead = vec![0usize; state.guide_ranks.len()];
        for other in self.states.iter().flatten() {
            let other_anchor = combined_anchor_priority(other);
            if other_anchor.total_cmp(&target_anchor) == Ordering::Less {
                anchor_states_ahead = anchor_states_ahead.saturating_add(1);
            }
            for (guide_index, target_rank) in state.guide_ranks.iter().enumerate() {
                let Some(other_rank) = other.guide_ranks.get(guide_index) else {
                    continue;
                };
                if other_rank > target_rank
                    || (other_rank == target_rank
                        && other_anchor.total_cmp(&target_anchor) == Ordering::Less)
                {
                    guided_states_ahead[guide_index] =
                        guided_states_ahead[guide_index].saturating_add(1);
                }
            }
        }
        snapshot.anchor_states_ahead = Some(anchor_states_ahead);
        snapshot.guided_states_ahead = Some(guided_states_ahead);
        snapshot
    }

    fn advance_pending_witness(
        &mut self,
        stepper: &dyn CombatStepper,
        deadline: Option<Instant>,
    ) -> Option<OracleCombatWitnessStatus> {
        let Some(replay) = self.pending_witness.as_mut() else {
            return None;
        };
        while replay.next_action < replay.actions.len() {
            if deadline_reached(deadline) {
                return Some(OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::Deadline,
                ));
            }
            let action = &replay.actions[replay.next_action];
            let required = action.engine_steps.max(1);
            if self
                .granted_engine_steps
                .saturating_sub(self.used.engine_steps)
                < required
            {
                return Some(OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::EngineStepBudget,
                ));
            }
            if stepper
                .choice_for_legal_input(&replay.position, &action.input)
                .is_none()
            {
                let error = OracleCombatWitnessReplayError::IllegalInput {
                    action_index: replay.next_action,
                };
                self.replay_failure = Some(error.clone());
                self.pending_witness = None;
                return Some(OracleCombatWitnessStatus::ReplayMismatch(error));
            }
            let result = stepper.apply_to_stable(
                &replay.position,
                action.input.clone(),
                CombatStepLimits {
                    max_engine_steps: required,
                    deadline,
                },
            );
            self.used.engine_steps = self.used.engine_steps.saturating_add(result.engine_steps);
            replay.engine_steps = replay.engine_steps.saturating_add(result.engine_steps);
            if result.timed_out {
                return Some(OracleCombatWitnessStatus::Partial(
                    OracleCombatWitnessInterruption::Deadline,
                ));
            }
            if result.truncated {
                let error = OracleCombatWitnessReplayError::TransitionStepLimit {
                    action_index: replay.next_action,
                };
                self.replay_failure = Some(error.clone());
                self.pending_witness = None;
                return Some(OracleCombatWitnessStatus::ReplayMismatch(error));
            }
            if exact_hash(&result.position) != action.expected_successor_hash {
                let error = OracleCombatWitnessReplayError::SuccessorMismatch {
                    action_index: replay.next_action,
                };
                self.replay_failure = Some(error.clone());
                self.pending_witness = None;
                return Some(OracleCombatWitnessStatus::ReplayMismatch(error));
            }
            replay.position = result.position;
            replay.next_action = replay.next_action.saturating_add(1);
        }

        if stepper.terminal(&replay.position) != CombatTerminal::Win {
            let error = OracleCombatWitnessReplayError::FinalStateIsNotWin;
            self.replay_failure = Some(error.clone());
            self.pending_witness = None;
            return Some(OracleCombatWitnessStatus::ReplayMismatch(error));
        }
        let replay = self
            .pending_witness
            .take()
            .expect("checked pending witness");
        let candidate = OracleCombatWitness {
            actions: replay.actions,
            final_position: replay.position,
            negative_log_policy: replay.negative_log_policy,
            replay_engine_steps: replay.engine_steps,
        };
        let replace = self
            .witness
            .as_ref()
            .is_none_or(|current| witness_better(&candidate, current));
        if replace {
            self.witness = Some(candidate);
        }
        self.witness_satisfies()
            .then_some(OracleCombatWitnessStatus::WitnessFound)
    }

    fn witness_satisfies(&self) -> bool {
        let Some(witness) = self.witness.as_ref() else {
            return false;
        };
        match self.config.satisfaction {
            OracleCombatWitnessSatisfaction::FirstWitness => true,
            OracleCombatWitnessSatisfaction::HpLossAtMost(limit) => {
                let initial_hp = self.root.position().combat.entities.player.current_hp;
                let final_hp = witness.final_position.combat.entities.player.current_hp;
                initial_hp.saturating_sub(final_hp).max(0) as u32 <= limit
            }
            OracleCombatWitnessSatisfaction::BudgetOrExhaustion => false,
        }
    }
}

fn combined_anchor_priority(state: &SearchState) -> f64 {
    let Some((local_depth, local_negative_log_policy)) =
        state.generator.best_retained_path_bound_snapshot()
    else {
        return f64::INFINITY;
    };
    let path_priority = PathRank {
        atomic_depth: state.path.atomic_depth.saturating_add(local_depth),
        negative_log_policy: state.path.negative_log_policy + local_negative_log_policy,
    }
    .levin_log_priority();
    service_aware_anchor_priority(path_priority, state.generator.counters().generation_work)
}

/// The anchor is the liveness lane for resumable state generators.  A pure
/// path rank can repeatedly select one attractive but very wide generator,
/// leaving already materialized later-turn states with zero service.  Charging
/// consumed generator work preserves the policy prior while making continued
/// service progressively earn its budget.
fn service_aware_anchor_priority(path_priority: f64, generation_work: usize) -> f64 {
    path_priority + (generation_work.saturating_add(1) as f64).ln()
}

fn state_progress_snapshot(state: &SearchState) -> OracleCombatWitnessStateProgressSnapshot {
    let counters = state.generator.counters();
    OracleCombatWitnessStateProgressSnapshot {
        exact_state_hash: exact_hash(state.generator.root().position()),
        path_atomic_depth: state.path.atomic_depth,
        path_negative_log_policy: state.path.negative_log_policy,
        generator_work: counters.generation_work,
        generator_engine_steps: counters.engine_steps,
        completed_turn_options: state.generator.completed_options().len(),
        retained_generator_work_items: state.generator.retained_work_items(),
        synced_options: state.synced_options,
        anchor_states_ahead: None,
        guided_states_ahead: None,
    }
}

#[cfg(test)]
mod anchor_priority_tests {
    use super::service_aware_anchor_priority;

    #[test]
    fn consumed_generator_work_makes_anchor_service_progressively_less_preferred() {
        let fresh = service_aware_anchor_priority(3.0, 0);
        let once_served = service_aware_anchor_priority(3.0, 4);
        let repeatedly_served = service_aware_anchor_priority(3.0, 64);

        assert!(fresh < once_served);
        assert!(once_served < repeatedly_served);
    }
}

fn deep_state_snapshot(state: &SearchState) -> OracleCombatDeepStateSnapshot {
    let combat = &state.generator.root().position().combat;
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
        path_atomic_depth: state.path.atomic_depth,
    }
}

fn pending_witness_better(left: &PendingWitnessReplay, right: &PendingWitnessReplay) -> bool {
    left.final_hp_hint
        .cmp(&right.final_hp_hint)
        .then_with(|| right.actions.len().cmp(&left.actions.len()))
        .then_with(|| {
            right
                .negative_log_policy
                .total_cmp(&left.negative_log_policy)
        })
        == Ordering::Greater
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
        == Ordering::Greater
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}
