use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::time::Instant;

use sts_core::sim::combat::{CombatPosition, CombatStepper, CombatTerminal};

use crate::atomic_levin_search::{
    replay_atomic_actions, AtomicLevinSearchHorizon, AtomicLevinWitness, AtomicLevinWitnessConfig,
    AtomicLevinWitnessCounters, AtomicLevinWitnessQuantum, AtomicLevinWitnessReplayError,
    AtomicLevinWitnessSession, AtomicLevinWitnessStatus,
};
use crate::generator::TurnOptionGeneratorSession;
use crate::local_turn_graph_search::{
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession,
    LocalTurnGraphWitnessStatus,
};
use crate::policy::{uniform_policy, SharedCombatActionPolicy, SharedCombatLookaheadEvaluator};
use crate::policy_discrepancy_search::{
    PolicyDiscrepancyConfig, PolicyDiscrepancyQuantum, PolicyDiscrepancySession,
    PolicyDiscrepancyStatus,
};
use crate::types::{
    exact_hash, CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOption,
    CompleteTurnOptionBoundary, TurnOptionAction, TurnOptionGeneratorConfig,
};
use crate::witness_search::OracleCombatWitnessReplayError;

/// A bounded turn-decomposition control for independent tactical service.
///
/// Boundary tasks enumerate exact successors at the next player turn. After
/// the configured number of boundary layers, every emitted successor owns a
/// separate, resumable terminal-search session. Widening and terminal search
/// have explicit service classes; independent policy and state-guide views
/// choose work only within one class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicTurnPortfolioConfig {
    pub boundary_search: TurnOptionGeneratorConfig,
    pub suffix_search: AtomicLevinWitnessConfig,
    /// Deterministic boundary-generation work in one progressive-widening
    /// batch. Exhaustion may end the first batch earlier.
    pub initial_boundary_work: usize,
    pub boundary_service_work: usize,
    /// Charged search work granted to one terminal task. Atomic backends
    /// charge exact transitions; the local-turn graph charges generation plus
    /// lookahead work. This shared unit is deliberately not wall time.
    pub suffix_service_work: usize,
    /// Number of exact player-turn boundaries to expose before a task switches
    /// to terminal search. `1` reproduces the original one-layer control.
    pub boundary_layers: usize,
    /// Terminal search work required before another boundary batch opens.
    /// This makes widening pay for itself with coherent child evaluation
    /// instead of flooding the portfolio merely because generation is cheap.
    pub terminal_work_per_boundary_batch: usize,
}

impl Default for AtomicTurnPortfolioConfig {
    fn default() -> Self {
        Self {
            boundary_search: TurnOptionGeneratorConfig::default(),
            suffix_search: AtomicLevinWitnessConfig::default(),
            initial_boundary_work: 512,
            boundary_service_work: 64,
            suffix_service_work: 8_192,
            boundary_layers: 1,
            terminal_work_per_boundary_batch: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AtomicTurnPortfolioCounters {
    pub services: usize,
    pub boundary_services: usize,
    pub suffix_services: usize,
    pub applied_action_transitions: usize,
    pub boundary_generation_work: usize,
    /// Backend-independent work used to enforce the portfolio allowance.
    pub charged_search_work: usize,
    /// Search work consumed after a task reaches its terminal-search phase.
    pub terminal_search_work: usize,
    pub engine_steps: usize,
    pub turn_boundaries_found: usize,
    pub suffix_sessions_started: usize,
    pub suffix_sessions_exhausted: usize,
    pub suffix_sessions_mechanics_gap: usize,
    pub invalid_boundary_roots: usize,
    pub duplicate_boundary_successors: usize,
    pub anchor_view_services: usize,
    pub guide_view_services: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicTurnPortfolioInterruption {
    SearchWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicTurnPortfolioStatus {
    WitnessFound,
    Partial(AtomicTurnPortfolioInterruption),
    FrontierExhausted,
    ReplayMismatch(AtomicLevinWitnessReplayError),
    PolicyReplayMismatch,
}

#[derive(Clone, Debug)]
pub struct AtomicTurnPortfolioReport {
    pub before: AtomicTurnPortfolioCounters,
    pub after: AtomicTurnPortfolioCounters,
    pub root_exact_state_hash: String,
    pub active_suffix_sessions: usize,
    pub active_boundary_tasks: usize,
    pub active_terminal_tasks: usize,
    pub recursive_active_tasks: usize,
    pub recursive_boundary_tasks: usize,
    pub recursive_terminal_tasks: usize,
    pub maximum_portfolio_depth: usize,
    pub boundary_generator_active: bool,
    pub suffix_entries: Vec<AtomicTurnPortfolioEntryReport>,
    pub winning_boundary_id: Option<u64>,
    pub winning_boundary_exact_state_hash: Option<String>,
    pub status: AtomicTurnPortfolioStatus,
    pub witness: Option<AtomicLevinWitness>,
}

#[derive(Clone, Debug)]
pub struct AtomicTurnPortfolioEntryReport {
    pub boundary_id: u64,
    pub exact_state_hash: String,
    pub prefix_action_count: usize,
    pub prefix_negative_log_policy: f64,
    pub applied_action_transitions: usize,
    pub boundary_generation_work: usize,
    pub terminal_search_work: usize,
    pub scheduler_work: usize,
    pub services: usize,
    pub engine_steps: usize,
    pub remaining_boundary_layers: usize,
    pub task_kind: AtomicTurnPortfolioTaskKind,
    pub recursive_active_tasks: usize,
    pub maximum_portfolio_depth: usize,
    pub boundary_guides: Vec<AtomicTurnPortfolioGuideRank>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicTurnPortfolioTaskKind {
    Boundary,
    AtomicTerminal,
    PolicyDiscrepancyTerminal,
    LocalTurnGraphTerminal,
    NestedPortfolio,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicTurnPortfolioGuideRank {
    pub lane: u32,
    pub components: Vec<i32>,
}

struct SuffixWork {
    boundary_id: u64,
    exact_state_hash: String,
    prefix_actions: Vec<TurnOptionAction>,
    prefix_negative_log_policy: f64,
    remaining_boundary_layers: usize,
    boundary_guides: Vec<AtomicTurnPortfolioGuideRank>,
    session: PortfolioTaskSession,
    boundary_generation_work: usize,
    terminal_search_work: usize,
    applied_action_transitions: usize,
    engine_steps: usize,
    services: usize,
}

enum PortfolioTaskSession {
    Boundary(Box<TurnOptionGeneratorSession>),
    AtomicTerminal(Box<AtomicLevinWitnessSession>),
    PolicyDiscrepancyTerminal(Box<PolicyDiscrepancySession>),
    LocalTurnGraphTerminal(Box<LocalTurnGraphWitnessSession>),
    NestedPortfolio(Box<AtomicTurnPortfolioSession>),
}

#[derive(Clone)]
enum PortfolioTerminalSearch {
    AtomicLevin,
    PolicyDiscrepancy(PolicyDiscrepancyConfig),
    LocalTurnGraph {
        config: LocalTurnGraphWitnessConfig,
        lookahead: Option<SharedCombatLookaheadEvaluator>,
    },
}

impl SuffixWork {
    fn task_kind(&self) -> AtomicTurnPortfolioTaskKind {
        match &self.session {
            PortfolioTaskSession::Boundary(_) => AtomicTurnPortfolioTaskKind::Boundary,
            PortfolioTaskSession::AtomicTerminal(_) => AtomicTurnPortfolioTaskKind::AtomicTerminal,
            PortfolioTaskSession::PolicyDiscrepancyTerminal(_) => {
                AtomicTurnPortfolioTaskKind::PolicyDiscrepancyTerminal
            }
            PortfolioTaskSession::LocalTurnGraphTerminal(_) => {
                AtomicTurnPortfolioTaskKind::LocalTurnGraphTerminal
            }
            PortfolioTaskSession::NestedPortfolio(_) => {
                AtomicTurnPortfolioTaskKind::NestedPortfolio
            }
        }
    }

    /// Number of schedulable leaf tasks represented by this entry.
    ///
    /// A nested portfolio is a container, not an additional task beside its
    /// children, so it contributes the number of leaves it currently owns.
    fn recursive_active_tasks(&self) -> usize {
        match &self.session {
            PortfolioTaskSession::NestedPortfolio(session) => session.recursive_active_tasks(),
            _ => 1,
        }
    }

    /// Number of portfolio levels from this entry through its deepest leaf.
    fn maximum_portfolio_depth(&self) -> usize {
        match &self.session {
            PortfolioTaskSession::NestedPortfolio(session) => {
                1usize.saturating_add(session.maximum_portfolio_depth())
            }
            _ => 1,
        }
    }

    fn scheduler_work(&self) -> usize {
        if self.remaining_boundary_layers > 0 {
            self.boundary_generation_work
        } else {
            self.terminal_search_work
        }
    }

    fn next_scheduler_work(&self, config: &AtomicTurnPortfolioConfig) -> usize {
        if self.remaining_boundary_layers > 0 {
            config
                .boundary_service_work
                .min(self.boundary_guides.len().saturating_add(1))
                .max(1)
        } else {
            config.suffix_service_work.max(1)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AtomicTurnPortfolioQuantum {
    pub additional_search_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

pub struct AtomicTurnPortfolioSession {
    root: CombatPosition,
    config: AtomicTurnPortfolioConfig,
    boundary_policy: SharedCombatActionPolicy,
    suffix_policy: SharedCombatActionPolicy,
    terminal_search: PortfolioTerminalSearch,
    suffixes: Vec<SuffixWork>,
    next_boundary_id: u64,
    seen_boundary_states: HashSet<(String, usize)>,
    boundary_view_cursor: usize,
    terminal_view_cursor: usize,
    used: AtomicTurnPortfolioCounters,
    granted_search_work: usize,
    granted_engine_steps: usize,
    witness: Option<AtomicLevinWitness>,
    replay_failure: Option<AtomicLevinWitnessReplayError>,
    policy_replay_mismatch: bool,
    winning_boundary_id: Option<u64>,
    winning_boundary_exact_state_hash: Option<String>,
}

impl AtomicTurnPortfolioSession {
    pub fn new(root: CombatDecisionRoot, config: AtomicTurnPortfolioConfig) -> Self {
        Self::with_policies(root, config, uniform_policy(), uniform_policy())
    }

    pub fn with_policies(
        root: CombatDecisionRoot,
        mut config: AtomicTurnPortfolioConfig,
        boundary_policy: SharedCombatActionPolicy,
        suffix_policy: SharedCombatActionPolicy,
    ) -> Self {
        config.boundary_layers = config.boundary_layers.max(1);
        config.suffix_search.horizon = AtomicLevinSearchHorizon::CombatTerminal;
        Self::with_terminal_search(
            root,
            config,
            boundary_policy,
            suffix_policy,
            PortfolioTerminalSearch::AtomicLevin,
        )
    }

    /// Use the exact turn-boundary portfolio only as a rerooting layer, then
    /// give every emitted boundary an independent policy-discrepancy search.
    /// Parent discrepancy still orders boundary service, but it never leaks
    /// into the child search's local path cost.
    pub fn with_policy_discrepancy_suffix(
        root: CombatDecisionRoot,
        mut config: AtomicTurnPortfolioConfig,
        mut suffix_search: PolicyDiscrepancyConfig,
        boundary_policy: SharedCombatActionPolicy,
        suffix_policy: SharedCombatActionPolicy,
    ) -> Self {
        config.boundary_layers = config.boundary_layers.max(1);
        suffix_search.max_engine_steps_per_transition =
            config.suffix_search.max_engine_steps_per_transition;
        Self::with_terminal_search(
            root,
            config,
            boundary_policy,
            suffix_policy,
            PortfolioTerminalSearch::PolicyDiscrepancy(suffix_search),
        )
    }

    /// Enumerate exact next-turn successors once, then give every successor an
    /// independent resumable local-turn graph. The graph keeps a coherent
    /// multi-turn search within one root action while this portfolio supplies
    /// policy-weighted fair service across root actions.
    pub fn with_local_turn_graph_suffix(
        root: CombatDecisionRoot,
        mut config: AtomicTurnPortfolioConfig,
        mut suffix_search: LocalTurnGraphWitnessConfig,
        boundary_policy: SharedCombatActionPolicy,
        suffix_policy: SharedCombatActionPolicy,
        lookahead: Option<SharedCombatLookaheadEvaluator>,
    ) -> Self {
        config.boundary_layers = config.boundary_layers.max(1);
        suffix_search.generator.max_engine_steps_per_transition =
            config.boundary_search.max_engine_steps_per_transition;
        Self::with_terminal_search(
            root,
            config,
            boundary_policy,
            suffix_policy,
            PortfolioTerminalSearch::LocalTurnGraph {
                config: suffix_search,
                lookahead,
            },
        )
    }

    fn with_terminal_search(
        root: CombatDecisionRoot,
        config: AtomicTurnPortfolioConfig,
        boundary_policy: SharedCombatActionPolicy,
        suffix_policy: SharedCombatActionPolicy,
        terminal_search: PortfolioTerminalSearch,
    ) -> Self {
        let position = root.position().clone();
        let root_hash = exact_hash(&position);
        let root_task = SuffixWork {
            boundary_id: 0,
            exact_state_hash: root_hash.clone(),
            prefix_actions: Vec::new(),
            prefix_negative_log_policy: 0.0,
            remaining_boundary_layers: config.boundary_layers,
            boundary_guides: boundary_policy
                .state_guides(&position)
                .into_iter()
                .map(|guide| AtomicTurnPortfolioGuideRank {
                    lane: guide.lane.value(),
                    components: guide.rank.components().to_vec(),
                })
                .collect(),
            session: PortfolioTaskSession::Boundary(Box::new(
                TurnOptionGeneratorSession::with_policy(
                    root,
                    config.boundary_search,
                    boundary_policy.clone(),
                ),
            )),
            boundary_generation_work: 0,
            terminal_search_work: 0,
            applied_action_transitions: 0,
            engine_steps: 0,
            services: 0,
        };
        let boundary_layers = config.boundary_layers;
        let used = AtomicTurnPortfolioCounters {
            suffix_sessions_started: 1,
            ..AtomicTurnPortfolioCounters::default()
        };
        Self {
            root: position,
            config,
            boundary_policy,
            suffix_policy,
            terminal_search,
            suffixes: vec![root_task],
            next_boundary_id: 1,
            seen_boundary_states: HashSet::from([(root_hash, boundary_layers)]),
            boundary_view_cursor: 0,
            terminal_view_cursor: 0,
            used,
            granted_search_work: 0,
            granted_engine_steps: 0,
            witness: None,
            replay_failure: None,
            policy_replay_mismatch: false,
            winning_boundary_id: None,
            winning_boundary_exact_state_hash: None,
        }
    }

    pub fn counters(&self) -> AtomicTurnPortfolioCounters {
        self.used
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: AtomicTurnPortfolioQuantum,
    ) -> AtomicTurnPortfolioReport {
        let before = self.used;
        self.granted_search_work = self
            .granted_search_work
            .saturating_add(quantum.additional_search_work);
        self.granted_engine_steps = self
            .granted_engine_steps
            .saturating_add(quantum.additional_engine_steps);

        let status = loop {
            if self.witness.is_some() {
                break AtomicTurnPortfolioStatus::WitnessFound;
            }
            if let Some(error) = self.replay_failure.clone() {
                break AtomicTurnPortfolioStatus::ReplayMismatch(error);
            }
            if self.policy_replay_mismatch {
                break AtomicTurnPortfolioStatus::PolicyReplayMismatch;
            }
            if quantum
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                break AtomicTurnPortfolioStatus::Partial(
                    AtomicTurnPortfolioInterruption::Deadline,
                );
            }
            let remaining_transitions = self
                .granted_search_work
                .saturating_sub(self.used.charged_search_work);
            if remaining_transitions == 0 {
                break AtomicTurnPortfolioStatus::Partial(
                    AtomicTurnPortfolioInterruption::SearchWorkBudget,
                );
            }
            let remaining_engine_steps = self
                .granted_engine_steps
                .saturating_sub(self.used.engine_steps);
            if remaining_engine_steps == 0 {
                break AtomicTurnPortfolioStatus::Partial(
                    AtomicTurnPortfolioInterruption::EngineStepBudget,
                );
            }
            if self.suffixes.is_empty() {
                break AtomicTurnPortfolioStatus::FrontierExhausted;
            }

            let has_boundary_task = self
                .suffixes
                .iter()
                .any(|task| task.remaining_boundary_layers > 0);
            let has_terminal_task = self
                .suffixes
                .iter()
                .any(|task| task.remaining_boundary_layers == 0);
            let boundary_work_allowance = progressive_boundary_work_allowance(
                self.config.initial_boundary_work,
                self.config.terminal_work_per_boundary_batch,
                self.used.terminal_search_work,
            );
            let service_boundary = has_boundary_task
                && (!has_terminal_task
                    || self.used.boundary_generation_work < boundary_work_allowance);
            if service_boundary {
                self.service_one_task(
                    stepper,
                    true,
                    remaining_transitions,
                    remaining_engine_steps,
                    quantum.deadline,
                );
            } else {
                self.service_one_task(
                    stepper,
                    false,
                    remaining_transitions,
                    remaining_engine_steps,
                    quantum.deadline,
                );
            }
        };

        AtomicTurnPortfolioReport {
            before,
            after: self.used,
            root_exact_state_hash: exact_hash(&self.root),
            active_suffix_sessions: self.suffixes.len(),
            active_boundary_tasks: self
                .suffixes
                .iter()
                .filter(|task| task.remaining_boundary_layers > 0)
                .count(),
            active_terminal_tasks: self
                .suffixes
                .iter()
                .filter(|task| task.remaining_boundary_layers == 0)
                .count(),
            recursive_active_tasks: self.recursive_active_tasks(),
            recursive_boundary_tasks: self.recursive_boundary_tasks(),
            recursive_terminal_tasks: self.recursive_terminal_tasks(),
            maximum_portfolio_depth: self.maximum_portfolio_depth(),
            boundary_generator_active: self
                .suffixes
                .iter()
                .any(|task| task.remaining_boundary_layers > 0),
            suffix_entries: self
                .suffixes
                .iter()
                .map(|suffix| AtomicTurnPortfolioEntryReport {
                    boundary_id: suffix.boundary_id,
                    exact_state_hash: suffix.exact_state_hash.clone(),
                    prefix_action_count: suffix.prefix_actions.len(),
                    prefix_negative_log_policy: suffix.prefix_negative_log_policy,
                    applied_action_transitions: suffix.applied_action_transitions,
                    boundary_generation_work: suffix.boundary_generation_work,
                    terminal_search_work: suffix.terminal_search_work,
                    scheduler_work: suffix.scheduler_work(),
                    services: suffix.services,
                    engine_steps: suffix.engine_steps,
                    remaining_boundary_layers: suffix.remaining_boundary_layers,
                    task_kind: suffix.task_kind(),
                    recursive_active_tasks: suffix.recursive_active_tasks(),
                    maximum_portfolio_depth: suffix.maximum_portfolio_depth(),
                    boundary_guides: suffix.boundary_guides.clone(),
                })
                .collect(),
            winning_boundary_id: self.winning_boundary_id,
            winning_boundary_exact_state_hash: self.winning_boundary_exact_state_hash.clone(),
            status,
            witness: self.witness.clone(),
        }
    }

    fn service_one_task(
        &mut self,
        stepper: &dyn CombatStepper,
        boundary_task: bool,
        remaining_transitions: usize,
        remaining_engine_steps: usize,
        deadline: Option<Instant>,
    ) {
        let Some(suffix_index) = self.next_suffix_index(boundary_task) else {
            return;
        };
        let mut suffix = self.suffixes.remove(suffix_index);
        self.used.services = self.used.services.saturating_add(1);
        suffix.services = suffix.services.saturating_add(1);
        if boundary_task {
            self.used.boundary_services = self.used.boundary_services.saturating_add(1);
            self.service_boundary_task(
                stepper,
                suffix,
                remaining_transitions,
                remaining_engine_steps,
                deadline,
            );
        } else {
            self.used.suffix_services = self.used.suffix_services.saturating_add(1);
            self.service_terminal_task(
                stepper,
                suffix,
                remaining_transitions,
                remaining_engine_steps,
                deadline,
            );
        }
    }

    fn service_boundary_task(
        &mut self,
        stepper: &dyn CombatStepper,
        mut suffix: SuffixWork,
        remaining_transitions: usize,
        remaining_engine_steps: usize,
        deadline: Option<Instant>,
    ) {
        let PortfolioTaskSession::Boundary(session) = &mut suffix.session else {
            unreachable!("a boundary task owns a turn generator")
        };
        let generation_quantum = self
            .config
            .boundary_service_work
            .max(1)
            .min(remaining_transitions);
        let engine_quantum = generation_quantum
            .saturating_mul(
                self.config
                    .boundary_search
                    .max_engine_steps_per_transition
                    .max(1),
            )
            .min(remaining_engine_steps);
        let report = session.advance_one_scheduling_round(
            stepper,
            CombatPlanningQuantum {
                additional_generation_work: generation_quantum,
                additional_engine_steps: engine_quantum,
                deadline,
            },
        );
        session.release_unused_grant();
        let completed = session.take_completed_options();
        let finished = session.is_finished();
        let generation_delta = report
            .after
            .generation_work
            .saturating_sub(report.before.generation_work);
        let transition_delta = report
            .after_diagnostics
            .applied_action_transitions
            .saturating_sub(report.before_diagnostics.applied_action_transitions);
        let engine_delta = report
            .after
            .engine_steps
            .saturating_sub(report.before.engine_steps);
        suffix.boundary_generation_work = suffix
            .boundary_generation_work
            .saturating_add(generation_delta);
        suffix.applied_action_transitions = suffix
            .applied_action_transitions
            .saturating_add(transition_delta);
        suffix.engine_steps = suffix.engine_steps.saturating_add(engine_delta);
        self.used.boundary_generation_work = self
            .used
            .boundary_generation_work
            .saturating_add(generation_delta);
        self.used.charged_search_work = self
            .used
            .charged_search_work
            .saturating_add(generation_delta);
        self.used.applied_action_transitions = self
            .used
            .applied_action_transitions
            .saturating_add(transition_delta);
        self.used.engine_steps = self.used.engine_steps.saturating_add(engine_delta);

        for option in completed {
            self.accept_boundary_option(stepper, &suffix, option);
            if self.witness.is_some() || self.replay_failure.is_some() {
                return;
            }
        }
        if finished {
            self.used.suffix_sessions_exhausted =
                self.used.suffix_sessions_exhausted.saturating_add(1);
        } else {
            self.suffixes.push(suffix);
        }
    }

    fn service_terminal_task(
        &mut self,
        stepper: &dyn CombatStepper,
        mut suffix: SuffixWork,
        remaining_transitions: usize,
        remaining_engine_steps: usize,
        deadline: Option<Instant>,
    ) {
        let transition_quantum = self
            .config
            .suffix_service_work
            .max(1)
            .min(remaining_transitions);
        let max_engine_steps_per_transition = match &self.terminal_search {
            PortfolioTerminalSearch::AtomicLevin => {
                self.config.suffix_search.max_engine_steps_per_transition
            }
            PortfolioTerminalSearch::PolicyDiscrepancy(config) => {
                config.max_engine_steps_per_transition
            }
            PortfolioTerminalSearch::LocalTurnGraph { config, .. } => {
                config.generator.max_engine_steps_per_transition
            }
        };
        let engine_quantum = transition_quantum
            .saturating_mul(max_engine_steps_per_transition.max(1))
            .min(remaining_engine_steps);
        match &mut suffix.session {
            PortfolioTaskSession::AtomicTerminal(session) => {
                let report = session.advance(
                    stepper,
                    AtomicLevinWitnessQuantum {
                        additional_applied_transitions: transition_quantum,
                        additional_engine_steps: engine_quantum,
                        deadline,
                    },
                );
                let transition_delta = report
                    .after
                    .applied_action_transitions
                    .saturating_sub(report.before.applied_action_transitions);
                let engine_delta = report
                    .after
                    .engine_steps
                    .saturating_sub(report.before.engine_steps);
                suffix.applied_action_transitions = suffix
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                suffix.terminal_search_work =
                    suffix.terminal_search_work.saturating_add(transition_delta);
                suffix.engine_steps = suffix.engine_steps.saturating_add(engine_delta);
                self.absorb_atomic_work(report.before, report.after);
                self.charge_terminal_work(transition_delta);

                match report.status {
                    AtomicLevinWitnessStatus::WitnessFound => {
                        if let Some(witness) = report.witness {
                            self.finish_combined_witness(stepper, &suffix, witness);
                        }
                    }
                    AtomicLevinWitnessStatus::FrontierExhausted => {
                        self.used.suffix_sessions_exhausted =
                            self.used.suffix_sessions_exhausted.saturating_add(1);
                    }
                    AtomicLevinWitnessStatus::ReplayMismatch(error) => {
                        self.replay_failure = Some(error);
                    }
                    AtomicLevinWitnessStatus::Partial(_) => self.suffixes.push(suffix),
                    AtomicLevinWitnessStatus::TurnBoundaryFound => {
                        unreachable!("terminal sessions never stream turn boundaries")
                    }
                }
            }
            PortfolioTaskSession::PolicyDiscrepancyTerminal(session) => {
                let report = session.advance(
                    stepper,
                    PolicyDiscrepancyQuantum {
                        additional_applied_transitions: transition_quantum,
                        additional_engine_steps: engine_quantum,
                        deadline,
                    },
                );
                let transition_delta = report
                    .after
                    .applied_action_transitions
                    .saturating_sub(report.before.applied_action_transitions);
                let engine_delta = report
                    .after
                    .engine_steps
                    .saturating_sub(report.before.engine_steps);
                suffix.applied_action_transitions = suffix
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                suffix.terminal_search_work =
                    suffix.terminal_search_work.saturating_add(transition_delta);
                suffix.engine_steps = suffix.engine_steps.saturating_add(engine_delta);
                self.used.applied_action_transitions = self
                    .used
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                self.used.engine_steps = self.used.engine_steps.saturating_add(engine_delta);
                self.charge_terminal_work(transition_delta);

                match report.status {
                    PolicyDiscrepancyStatus::WitnessFound => {
                        if let Some(witness) = report.witness {
                            self.finish_combined_witness(stepper, &suffix, witness);
                        }
                    }
                    PolicyDiscrepancyStatus::FrontierExhausted => {
                        self.used.suffix_sessions_exhausted =
                            self.used.suffix_sessions_exhausted.saturating_add(1);
                    }
                    PolicyDiscrepancyStatus::ReplayMismatch => {
                        self.policy_replay_mismatch = true;
                    }
                    PolicyDiscrepancyStatus::Partial(_) => self.suffixes.push(suffix),
                }
            }
            PortfolioTaskSession::LocalTurnGraphTerminal(session) => {
                let before = session.counters();
                let report = session.advance(
                    LocalTurnGraphWitnessQuantum {
                        additional_selections: transition_quantum,
                        additional_generation_work: transition_quantum,
                        additional_engine_steps: engine_quantum,
                        deadline,
                    },
                    stepper,
                );
                let after = report.counters.clone();
                let transition_delta = after
                    .applied_action_transitions
                    .saturating_sub(before.applied_action_transitions);
                let generation_and_lookahead_delta = after
                    .generation_work
                    .saturating_add(after.lookahead_work)
                    .saturating_sub(before.generation_work.saturating_add(before.lookahead_work));
                let selection_delta = after.selections.saturating_sub(before.selections);
                // Local graph scheduling and generation have independent
                // allowances. Charge whichever dimension advanced farther so
                // a selection-only path cannot consume unbounded portfolio
                // service without reducing the shared allowance.
                let search_work_delta =
                    local_graph_charged_work(selection_delta, generation_and_lookahead_delta);
                let engine_delta = after.engine_steps.saturating_sub(before.engine_steps);
                suffix.applied_action_transitions = suffix
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                suffix.terminal_search_work = suffix
                    .terminal_search_work
                    .saturating_add(search_work_delta);
                suffix.engine_steps = suffix.engine_steps.saturating_add(engine_delta);
                self.used.applied_action_transitions = self
                    .used
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                self.used.engine_steps = self.used.engine_steps.saturating_add(engine_delta);
                self.charge_terminal_work(search_work_delta);

                match report.status {
                    LocalTurnGraphWitnessStatus::WitnessFound => {
                        if let Some(witness) = report.witness {
                            self.finish_combined_witness(
                                stepper,
                                &suffix,
                                AtomicLevinWitness {
                                    actions: witness.actions,
                                    final_position: witness.final_position,
                                    negative_log_policy: witness.negative_log_policy,
                                    replay_engine_steps: witness.replay_engine_steps,
                                },
                            );
                        }
                    }
                    LocalTurnGraphWitnessStatus::FrontierExhausted => {
                        self.used.suffix_sessions_exhausted =
                            self.used.suffix_sessions_exhausted.saturating_add(1);
                    }
                    LocalTurnGraphWitnessStatus::MechanicsGap => {
                        self.used.suffix_sessions_mechanics_gap =
                            self.used.suffix_sessions_mechanics_gap.saturating_add(1);
                    }
                    LocalTurnGraphWitnessStatus::ReplayMismatch(error) => {
                        self.replay_failure = Some(atomic_replay_error(error));
                    }
                    LocalTurnGraphWitnessStatus::Partial(_) => self.suffixes.push(suffix),
                }
            }
            PortfolioTaskSession::NestedPortfolio(session) => {
                let before = session.counters();
                let report = session.advance(
                    stepper,
                    AtomicTurnPortfolioQuantum {
                        additional_search_work: transition_quantum,
                        additional_engine_steps: engine_quantum,
                        deadline,
                    },
                );
                let after = report.after;
                let search_work_delta = after
                    .charged_search_work
                    .saturating_sub(before.charged_search_work);
                let transition_delta = after
                    .applied_action_transitions
                    .saturating_sub(before.applied_action_transitions);
                let engine_delta = after.engine_steps.saturating_sub(before.engine_steps);
                suffix.applied_action_transitions = suffix
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                suffix.terminal_search_work = suffix
                    .terminal_search_work
                    .saturating_add(search_work_delta);
                suffix.engine_steps = suffix.engine_steps.saturating_add(engine_delta);
                self.used.applied_action_transitions = self
                    .used
                    .applied_action_transitions
                    .saturating_add(transition_delta);
                self.used.engine_steps = self.used.engine_steps.saturating_add(engine_delta);
                self.used.suffix_sessions_mechanics_gap =
                    self.used.suffix_sessions_mechanics_gap.saturating_add(
                        after
                            .suffix_sessions_mechanics_gap
                            .saturating_sub(before.suffix_sessions_mechanics_gap),
                    );
                self.charge_terminal_work(search_work_delta);
                suffix.boundary_guides =
                    merge_backed_guides(&suffix.boundary_guides, &session.best_backed_guides());

                match report.status {
                    AtomicTurnPortfolioStatus::WitnessFound => {
                        if let Some(witness) = report.witness {
                            self.finish_combined_witness(stepper, &suffix, witness);
                        }
                    }
                    AtomicTurnPortfolioStatus::FrontierExhausted => {
                        self.used.suffix_sessions_exhausted =
                            self.used.suffix_sessions_exhausted.saturating_add(1);
                    }
                    AtomicTurnPortfolioStatus::ReplayMismatch(error) => {
                        self.replay_failure = Some(error);
                    }
                    AtomicTurnPortfolioStatus::PolicyReplayMismatch => {
                        self.policy_replay_mismatch = true;
                    }
                    AtomicTurnPortfolioStatus::Partial(_) => self.suffixes.push(suffix),
                }
            }
            PortfolioTaskSession::Boundary(_) => {
                unreachable!("a terminal task owns a terminal search session")
            }
        }
    }

    fn accept_boundary_option(
        &mut self,
        stepper: &dyn CombatStepper,
        suffix: &SuffixWork,
        option: CompleteTurnOption,
    ) {
        match option.boundary() {
            CompleteTurnOptionBoundary::NextPlayerTurn => {
                self.used.turn_boundaries_found = self.used.turn_boundaries_found.saturating_add(1);
                let mut prefix_actions = suffix.prefix_actions.clone();
                prefix_actions.extend_from_slice(option.actions());
                self.enqueue_boundary_successor(
                    option.exact_successor().clone(),
                    prefix_actions,
                    suffix.prefix_negative_log_policy + option.negative_log_policy(),
                    suffix.remaining_boundary_layers.saturating_sub(1),
                );
            }
            CompleteTurnOptionBoundary::TerminalWin => {
                self.finish_combined_witness(
                    stepper,
                    suffix,
                    AtomicLevinWitness {
                        actions: option.actions().to_vec(),
                        final_position: option.exact_successor().clone(),
                        negative_log_policy: option.negative_log_policy(),
                        replay_engine_steps: option.engine_steps(),
                    },
                );
            }
            CompleteTurnOptionBoundary::TerminalLoss | CompleteTurnOptionBoundary::Escape => {}
        }
    }

    fn enqueue_boundary_successor(
        &mut self,
        position: CombatPosition,
        prefix_actions: Vec<TurnOptionAction>,
        prefix_negative_log_policy: f64,
        remaining_boundary_layers: usize,
    ) {
        let exact_state_hash = exact_hash(&position);
        if !self
            .seen_boundary_states
            .insert((exact_state_hash.clone(), remaining_boundary_layers))
        {
            self.used.duplicate_boundary_successors =
                self.used.duplicate_boundary_successors.saturating_add(1);
            return;
        }
        let boundary_guides = self
            .boundary_policy
            .state_guides(&position)
            .into_iter()
            .map(|guide| AtomicTurnPortfolioGuideRank {
                lane: guide.lane.value(),
                components: guide.rank.components().to_vec(),
            })
            .collect();
        let Ok(root) = CombatDecisionRoot::new(position) else {
            self.used.invalid_boundary_roots = self.used.invalid_boundary_roots.saturating_add(1);
            return;
        };
        let boundary_id = self.next_boundary_id;
        self.next_boundary_id = self.next_boundary_id.saturating_add(1);
        let session = if remaining_boundary_layers > 0 {
            let mut nested_config = self.config;
            nested_config.boundary_layers = remaining_boundary_layers;
            PortfolioTaskSession::NestedPortfolio(Box::new(Self::with_terminal_search(
                root,
                nested_config,
                self.boundary_policy.clone(),
                self.suffix_policy.clone(),
                self.terminal_search.clone(),
            )))
        } else {
            match &self.terminal_search {
                PortfolioTerminalSearch::AtomicLevin => PortfolioTaskSession::AtomicTerminal(
                    Box::new(AtomicLevinWitnessSession::with_policy(
                        root,
                        self.config.suffix_search,
                        self.suffix_policy.clone(),
                    )),
                ),
                PortfolioTerminalSearch::PolicyDiscrepancy(config) => {
                    PortfolioTaskSession::PolicyDiscrepancyTerminal(Box::new(
                        PolicyDiscrepancySession::with_policy(
                            root,
                            *config,
                            self.suffix_policy.clone(),
                        ),
                    ))
                }
                PortfolioTerminalSearch::LocalTurnGraph { config, lookahead } => {
                    PortfolioTaskSession::LocalTurnGraphTerminal(Box::new(match lookahead {
                        Some(lookahead) => LocalTurnGraphWitnessSession::with_policy_and_lookahead(
                            root,
                            *config,
                            self.suffix_policy.clone(),
                            lookahead.clone(),
                        ),
                        None => LocalTurnGraphWitnessSession::with_policy(
                            root,
                            *config,
                            self.suffix_policy.clone(),
                        ),
                    }))
                }
            }
        };
        self.suffixes.push(SuffixWork {
            boundary_id,
            exact_state_hash,
            prefix_actions,
            prefix_negative_log_policy,
            // Recursive boundary layers are a coherent terminal task from the
            // parent portfolio's perspective. Their own session owns the
            // remaining boundary depth and its widening schedule.
            remaining_boundary_layers: 0,
            boundary_guides,
            session,
            boundary_generation_work: 0,
            terminal_search_work: 0,
            applied_action_transitions: 0,
            engine_steps: 0,
            services: 0,
        });
        self.used.suffix_sessions_started = self.used.suffix_sessions_started.saturating_add(1);
    }

    fn next_suffix_index(&mut self, boundary_task: bool) -> Option<usize> {
        let guide_lanes = self
            .suffixes
            .iter()
            .filter(|task| (task.remaining_boundary_layers > 0) == boundary_task)
            .flat_map(|task| task.boundary_guides.iter().map(|guide| guide.lane))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let view_count = guide_lanes.len().saturating_add(1);
        let cursor = if boundary_task {
            let cursor = self.boundary_view_cursor;
            self.boundary_view_cursor = self.boundary_view_cursor.saturating_add(1);
            cursor
        } else {
            let cursor = self.terminal_view_cursor;
            self.terminal_view_cursor = self.terminal_view_cursor.saturating_add(1);
            cursor
        };
        let selected_guide_lane = (cursor % view_count != 0)
            .then(|| guide_lanes[(cursor % view_count).saturating_sub(1)]);
        if selected_guide_lane.is_some() {
            self.used.guide_view_services = self.used.guide_view_services.saturating_add(1);
        } else {
            self.used.anchor_view_services = self.used.anchor_view_services.saturating_add(1);
        }
        self.suffixes
            .iter()
            .enumerate()
            .filter(|(_, task)| (task.remaining_boundary_layers > 0) == boundary_task)
            .min_by(|(_, left), (_, right)| {
                let left_work = left
                    .scheduler_work()
                    .saturating_add(left.next_scheduler_work(&self.config))
                    .max(1);
                let right_work = right
                    .scheduler_work()
                    .saturating_add(right.next_scheduler_work(&self.config))
                    .max(1);
                let left_key = left.prefix_negative_log_policy + (left_work as f64).ln();
                let right_key = right.prefix_negative_log_policy + (right_work as f64).ln();
                let guide_order = selected_guide_lane
                    .map(|lane| compare_guide_lane(left, right, lane))
                    .unwrap_or(std::cmp::Ordering::Equal);
                // Guide views exploit their strongest materialized evidence;
                // the independent anchor view owns starvation freedom. Work
                // rounds break ties between equal guide evidence.
                let guide_service_order = selected_guide_lane
                    .map(|_| guide_then_service_round(guide_order, left_work, right_work))
                    .unwrap_or(std::cmp::Ordering::Equal);
                guide_service_order
                    .then_with(|| left_key.total_cmp(&right_key))
                    .then_with(|| left.boundary_id.cmp(&right.boundary_id))
            })
            .map(|(index, _)| index)
    }

    fn absorb_atomic_work(
        &mut self,
        before: AtomicLevinWitnessCounters,
        after: AtomicLevinWitnessCounters,
    ) {
        self.used.applied_action_transitions = self.used.applied_action_transitions.saturating_add(
            after
                .applied_action_transitions
                .saturating_sub(before.applied_action_transitions),
        );
        self.used.engine_steps = self
            .used
            .engine_steps
            .saturating_add(after.engine_steps.saturating_sub(before.engine_steps));
    }

    fn charge_terminal_work(&mut self, work: usize) {
        self.used.terminal_search_work = self.used.terminal_search_work.saturating_add(work);
        self.used.charged_search_work = self.used.charged_search_work.saturating_add(work);
    }

    fn recursive_active_tasks(&self) -> usize {
        self.suffixes
            .iter()
            .map(SuffixWork::recursive_active_tasks)
            .sum()
    }

    fn recursive_boundary_tasks(&self) -> usize {
        self.suffixes
            .iter()
            .map(|suffix| match &suffix.session {
                PortfolioTaskSession::Boundary(_) => 1,
                PortfolioTaskSession::NestedPortfolio(session) => {
                    session.recursive_boundary_tasks()
                }
                _ => 0,
            })
            .sum()
    }

    fn recursive_terminal_tasks(&self) -> usize {
        self.suffixes
            .iter()
            .map(|suffix| match &suffix.session {
                PortfolioTaskSession::NestedPortfolio(session) => {
                    session.recursive_terminal_tasks()
                }
                PortfolioTaskSession::Boundary(_) => 0,
                _ => 1,
            })
            .sum()
    }

    fn maximum_portfolio_depth(&self) -> usize {
        self.suffixes
            .iter()
            .map(SuffixWork::maximum_portfolio_depth)
            .max()
            .unwrap_or(1)
    }

    /// Best exact state-guide evidence materialized below this portfolio root.
    /// This is an optimistic backup, but never a prediction: every component
    /// comes from a successor that was actually generated.
    fn best_backed_guides(&self) -> Vec<AtomicTurnPortfolioGuideRank> {
        let mut best = BTreeMap::<u32, Vec<i32>>::new();
        for suffix in &self.suffixes {
            for guide in &suffix.boundary_guides {
                best.entry(guide.lane)
                    .and_modify(|current| {
                        if guide.components > *current {
                            *current = guide.components.clone();
                        }
                    })
                    .or_insert_with(|| guide.components.clone());
            }
        }
        best.into_iter()
            .map(|(lane, components)| AtomicTurnPortfolioGuideRank { lane, components })
            .collect()
    }

    fn finish_combined_witness(
        &mut self,
        stepper: &dyn CombatStepper,
        suffix: &SuffixWork,
        suffix_witness: AtomicLevinWitness,
    ) {
        let mut actions = suffix.prefix_actions.clone();
        actions.extend(suffix_witness.actions);
        match replay_atomic_actions(
            stepper,
            &self.root,
            &actions,
            self.config
                .boundary_search
                .max_engine_steps_per_transition
                .max(match &self.terminal_search {
                    PortfolioTerminalSearch::AtomicLevin => {
                        self.config.suffix_search.max_engine_steps_per_transition
                    }
                    PortfolioTerminalSearch::PolicyDiscrepancy(config) => {
                        config.max_engine_steps_per_transition
                    }
                    PortfolioTerminalSearch::LocalTurnGraph { config, .. } => {
                        config.generator.max_engine_steps_per_transition
                    }
                }),
        ) {
            Ok((final_position, replay_engine_steps))
                if stepper.terminal(&final_position) == CombatTerminal::Win =>
            {
                self.witness = Some(AtomicLevinWitness {
                    actions,
                    final_position,
                    negative_log_policy: suffix.prefix_negative_log_policy
                        + suffix_witness.negative_log_policy,
                    replay_engine_steps,
                });
                self.winning_boundary_id = Some(suffix.boundary_id);
                self.winning_boundary_exact_state_hash = Some(suffix.exact_state_hash.clone());
            }
            Ok(_) => {
                self.replay_failure = Some(AtomicLevinWitnessReplayError::FinalStateIsNotWin);
            }
            Err(error) => self.replay_failure = Some(error),
        }
    }
}

pub(crate) fn guide_then_service_round(
    guide_order: std::cmp::Ordering,
    left_work: usize,
    right_work: usize,
) -> std::cmp::Ordering {
    guide_order.then_with(|| left_work.max(1).ilog2().cmp(&right_work.max(1).ilog2()))
}

pub(crate) fn progressive_boundary_work_allowance(
    batch_work: usize,
    terminal_work_per_batch: usize,
    terminal_work: usize,
) -> usize {
    let completed_terminal_batches = terminal_work / terminal_work_per_batch.max(1);
    batch_work
        .max(1)
        .saturating_mul(completed_terminal_batches.saturating_add(1))
}

pub(crate) fn local_graph_charged_work(
    selection_work: usize,
    generation_and_lookahead_work: usize,
) -> usize {
    selection_work.max(generation_and_lookahead_work)
}

pub(crate) fn merge_backed_guides(
    current: &[AtomicTurnPortfolioGuideRank],
    backed: &[AtomicTurnPortfolioGuideRank],
) -> Vec<AtomicTurnPortfolioGuideRank> {
    let mut merged = current
        .iter()
        .map(|guide| (guide.lane, guide.components.clone()))
        .collect::<BTreeMap<_, _>>();
    for guide in backed {
        merged
            .entry(guide.lane)
            .and_modify(|components| {
                if guide.components > *components {
                    *components = guide.components.clone();
                }
            })
            .or_insert_with(|| guide.components.clone());
    }
    merged
        .into_iter()
        .map(|(lane, components)| AtomicTurnPortfolioGuideRank { lane, components })
        .collect()
}

fn atomic_replay_error(error: OracleCombatWitnessReplayError) -> AtomicLevinWitnessReplayError {
    match error {
        OracleCombatWitnessReplayError::IllegalInput { action_index } => {
            AtomicLevinWitnessReplayError::IllegalInput { action_index }
        }
        OracleCombatWitnessReplayError::TransitionStepLimit { action_index } => {
            AtomicLevinWitnessReplayError::TransitionStepLimit { action_index }
        }
        OracleCombatWitnessReplayError::SuccessorMismatch { action_index } => {
            AtomicLevinWitnessReplayError::SuccessorMismatch { action_index }
        }
        OracleCombatWitnessReplayError::FinalStateIsNotWin => {
            AtomicLevinWitnessReplayError::FinalStateIsNotWin
        }
    }
}

fn compare_guide_lane(left: &SuffixWork, right: &SuffixWork, lane: u32) -> std::cmp::Ordering {
    let left_rank = left
        .boundary_guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| guide.components.as_slice());
    let right_rank = right
        .boundary_guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| guide.components.as_slice());
    match (left_rank, right_rank) {
        (Some(left_rank), Some(right_rank)) => right_rank.cmp(left_rank),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
