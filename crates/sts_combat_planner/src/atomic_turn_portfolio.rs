use std::time::Instant;

use sts_core::sim::combat::{CombatPosition, CombatStepper, CombatTerminal};

use crate::atomic_levin_search::{
    replay_atomic_actions, AtomicLevinSearchHorizon, AtomicLevinWitness, AtomicLevinWitnessConfig,
    AtomicLevinWitnessCounters, AtomicLevinWitnessInterruption, AtomicLevinWitnessQuantum,
    AtomicLevinWitnessReplayError, AtomicLevinWitnessSession, AtomicLevinWitnessStatus,
};
use crate::policy::{uniform_policy, SharedCombatActionPolicy};
use crate::types::{exact_hash, CombatDecisionRoot, TurnOptionAction};

/// A one-layer structural control for independent next-turn service.
///
/// One atomic Levin session enumerates exact successors at the next player
/// turn. Every emitted successor owns a separate, resumable terminal-search
/// session. The generator is serviced periodically; suffix sessions receive
/// the remaining bounded quanta by independent Levin task cost. Descendant
/// work is never inserted back into the generator heap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicTurnPortfolioConfig {
    pub boundary_search: AtomicLevinWitnessConfig,
    pub suffix_search: AtomicLevinWitnessConfig,
    pub boundary_service_transitions: usize,
    pub suffix_service_transitions: usize,
    /// One boundary-generator service per this many coarse services. Suffix
    /// tasks receive the remaining services by their independent Levin cost.
    pub boundary_service_period: usize,
}

impl Default for AtomicTurnPortfolioConfig {
    fn default() -> Self {
        Self {
            boundary_search: AtomicLevinWitnessConfig::default(),
            suffix_search: AtomicLevinWitnessConfig::default(),
            boundary_service_transitions: 64,
            suffix_service_transitions: 512,
            boundary_service_period: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AtomicTurnPortfolioCounters {
    pub services: usize,
    pub boundary_services: usize,
    pub suffix_services: usize,
    pub applied_action_transitions: usize,
    pub engine_steps: usize,
    pub turn_boundaries_found: usize,
    pub suffix_sessions_started: usize,
    pub suffix_sessions_exhausted: usize,
    pub invalid_boundary_roots: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicTurnPortfolioStatus {
    WitnessFound,
    Partial(AtomicLevinWitnessInterruption),
    FrontierExhausted,
    ReplayMismatch(AtomicLevinWitnessReplayError),
}

#[derive(Clone, Debug)]
pub struct AtomicTurnPortfolioReport {
    pub before: AtomicTurnPortfolioCounters,
    pub after: AtomicTurnPortfolioCounters,
    pub active_suffix_sessions: usize,
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
    pub engine_steps: usize,
}

struct SuffixWork {
    boundary_id: u64,
    exact_state_hash: String,
    prefix_actions: Vec<TurnOptionAction>,
    prefix_negative_log_policy: f64,
    session: AtomicLevinWitnessSession,
}

pub struct AtomicTurnPortfolioSession {
    root: CombatPosition,
    config: AtomicTurnPortfolioConfig,
    suffix_policy: SharedCombatActionPolicy,
    boundary_generator: Option<AtomicLevinWitnessSession>,
    suffixes: Vec<SuffixWork>,
    suffix_services_since_boundary: usize,
    next_boundary_id: u64,
    used: AtomicTurnPortfolioCounters,
    granted_applied_transitions: usize,
    granted_engine_steps: usize,
    witness: Option<AtomicLevinWitness>,
    replay_failure: Option<AtomicLevinWitnessReplayError>,
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
        config.boundary_search.horizon = AtomicLevinSearchHorizon::NextPlayerTurn;
        config.suffix_search.horizon = AtomicLevinSearchHorizon::CombatTerminal;
        let position = root.position().clone();
        let boundary_generator =
            AtomicLevinWitnessSession::with_policy(root, config.boundary_search, boundary_policy);
        let boundary_service_period = config.boundary_service_period.max(1);
        Self {
            root: position,
            config,
            suffix_policy,
            boundary_generator: Some(boundary_generator),
            suffixes: Vec::new(),
            suffix_services_since_boundary: boundary_service_period,
            next_boundary_id: 0,
            used: AtomicTurnPortfolioCounters::default(),
            granted_applied_transitions: 0,
            granted_engine_steps: 0,
            witness: None,
            replay_failure: None,
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
        quantum: AtomicLevinWitnessQuantum,
    ) -> AtomicTurnPortfolioReport {
        let before = self.used;
        self.granted_applied_transitions = self
            .granted_applied_transitions
            .saturating_add(quantum.additional_applied_transitions);
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
            if quantum
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                break AtomicTurnPortfolioStatus::Partial(AtomicLevinWitnessInterruption::Deadline);
            }
            let remaining_transitions = self
                .granted_applied_transitions
                .saturating_sub(self.used.applied_action_transitions);
            if remaining_transitions == 0 {
                break AtomicTurnPortfolioStatus::Partial(
                    AtomicLevinWitnessInterruption::AppliedTransitionBudget,
                );
            }
            let remaining_engine_steps = self
                .granted_engine_steps
                .saturating_sub(self.used.engine_steps);
            if remaining_engine_steps == 0 {
                break AtomicTurnPortfolioStatus::Partial(
                    AtomicLevinWitnessInterruption::EngineStepBudget,
                );
            }
            if self.boundary_generator.is_none() && self.suffixes.is_empty() {
                break AtomicTurnPortfolioStatus::FrontierExhausted;
            }

            let service_boundary = self.boundary_generator.is_some()
                && (self.suffixes.is_empty()
                    || self.suffix_services_since_boundary
                        >= self.config.boundary_service_period.max(1).saturating_sub(1));
            if service_boundary {
                self.suffix_services_since_boundary = 0;
                self.service_boundary_generator(
                    stepper,
                    remaining_transitions,
                    remaining_engine_steps,
                    quantum.deadline,
                );
            } else {
                self.suffix_services_since_boundary =
                    self.suffix_services_since_boundary.saturating_add(1);
                self.service_one_suffix(
                    stepper,
                    remaining_transitions,
                    remaining_engine_steps,
                    quantum.deadline,
                );
            }
        };

        AtomicTurnPortfolioReport {
            before,
            after: self.used,
            active_suffix_sessions: self.suffixes.len(),
            boundary_generator_active: self.boundary_generator.is_some(),
            suffix_entries: self
                .suffixes
                .iter()
                .map(|suffix| {
                    let counters = suffix.session.counters();
                    AtomicTurnPortfolioEntryReport {
                        boundary_id: suffix.boundary_id,
                        exact_state_hash: suffix.exact_state_hash.clone(),
                        prefix_action_count: suffix.prefix_actions.len(),
                        prefix_negative_log_policy: suffix.prefix_negative_log_policy,
                        applied_action_transitions: counters.applied_action_transitions,
                        engine_steps: counters.engine_steps,
                    }
                })
                .collect(),
            winning_boundary_id: self.winning_boundary_id,
            winning_boundary_exact_state_hash: self.winning_boundary_exact_state_hash.clone(),
            status,
            witness: self.witness.clone(),
        }
    }

    fn service_boundary_generator(
        &mut self,
        stepper: &dyn CombatStepper,
        remaining_transitions: usize,
        remaining_engine_steps: usize,
        deadline: Option<Instant>,
    ) {
        let transition_quantum = self
            .config
            .boundary_service_transitions
            .max(1)
            .min(remaining_transitions);
        let engine_quantum = transition_quantum
            .saturating_mul(
                self.config
                    .boundary_search
                    .max_engine_steps_per_transition
                    .max(1),
            )
            .min(remaining_engine_steps);
        let report = self
            .boundary_generator
            .as_mut()
            .expect("checked boundary generator")
            .advance(
                stepper,
                AtomicLevinWitnessQuantum {
                    additional_applied_transitions: transition_quantum,
                    additional_engine_steps: engine_quantum,
                    deadline,
                },
            );
        self.used.services = self.used.services.saturating_add(1);
        self.used.boundary_services = self.used.boundary_services.saturating_add(1);
        self.absorb_atomic_work(report.before, report.after);

        match report.status {
            AtomicLevinWitnessStatus::WitnessFound => {
                self.witness = report.witness;
                self.boundary_generator = None;
            }
            AtomicLevinWitnessStatus::TurnBoundaryFound => {
                let boundary = report
                    .turn_boundary
                    .expect("boundary status carries an exact boundary");
                self.used.turn_boundaries_found = self.used.turn_boundaries_found.saturating_add(1);
                let exact_state_hash = exact_hash(&boundary.position);
                match CombatDecisionRoot::new(boundary.position) {
                    Ok(root) => {
                        let boundary_id = self.next_boundary_id;
                        self.next_boundary_id = self.next_boundary_id.saturating_add(1);
                        self.suffixes.push(SuffixWork {
                            boundary_id,
                            exact_state_hash,
                            prefix_actions: boundary.actions,
                            prefix_negative_log_policy: boundary.negative_log_policy,
                            session: AtomicLevinWitnessSession::with_policy(
                                root,
                                self.config.suffix_search,
                                self.suffix_policy.clone(),
                            ),
                        });
                        self.used.suffix_sessions_started =
                            self.used.suffix_sessions_started.saturating_add(1);
                    }
                    Err(_) => {
                        self.used.invalid_boundary_roots =
                            self.used.invalid_boundary_roots.saturating_add(1);
                    }
                }
            }
            AtomicLevinWitnessStatus::FrontierExhausted => {
                self.boundary_generator = None;
            }
            AtomicLevinWitnessStatus::ReplayMismatch(error) => {
                self.replay_failure = Some(error);
                self.boundary_generator = None;
            }
            AtomicLevinWitnessStatus::Partial(_) => {}
        }
    }

    fn service_one_suffix(
        &mut self,
        stepper: &dyn CombatStepper,
        remaining_transitions: usize,
        remaining_engine_steps: usize,
        deadline: Option<Instant>,
    ) {
        let Some(suffix_index) = self.next_suffix_index() else {
            return;
        };
        let mut suffix = self.suffixes.remove(suffix_index);
        let transition_quantum = self
            .config
            .suffix_service_transitions
            .max(1)
            .min(remaining_transitions);
        let engine_quantum = transition_quantum
            .saturating_mul(
                self.config
                    .suffix_search
                    .max_engine_steps_per_transition
                    .max(1),
            )
            .min(remaining_engine_steps);
        let report = suffix.session.advance(
            stepper,
            AtomicLevinWitnessQuantum {
                additional_applied_transitions: transition_quantum,
                additional_engine_steps: engine_quantum,
                deadline,
            },
        );
        self.used.services = self.used.services.saturating_add(1);
        self.used.suffix_services = self.used.suffix_services.saturating_add(1);
        self.absorb_atomic_work(report.before, report.after);

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
                unreachable!("suffix sessions always target a combat terminal")
            }
        }
    }

    fn next_suffix_index(&self) -> Option<usize> {
        let next_quantum = self.config.suffix_service_transitions.max(1);
        self.suffixes
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_work = left
                    .session
                    .counters()
                    .applied_action_transitions
                    .saturating_add(next_quantum)
                    .max(1);
                let right_work = right
                    .session
                    .counters()
                    .applied_action_transitions
                    .saturating_add(next_quantum)
                    .max(1);
                let left_key = left.prefix_negative_log_policy + (left_work as f64).ln();
                let right_key = right.prefix_negative_log_policy + (right_work as f64).ln();
                left_key
                    .total_cmp(&right_key)
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
                .max(self.config.suffix_search.max_engine_steps_per_transition),
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
