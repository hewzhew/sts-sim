use std::sync::Arc;
use std::time::{Duration, Instant};

const MIN_USABLE_WALL_ALLOWANCE: Duration = Duration::from_millis(1);

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    CombatDecisionRoot, OracleCombatDeepStateSnapshot, OracleCombatRootActionFamilySnapshot,
    OracleCombatWitness, OracleCombatWitnessConfig, OracleCombatWitnessDiscoverySource,
    OracleCombatWitnessQuantum, OracleCombatWitnessSatisfaction, OracleCombatWitnessSession,
    OracleCombatWitnessStateProgressSnapshot, OracleCombatWitnessStatus, TurnOptionAction,
    TurnOptionGeneratorConfig,
};

use super::combat_line_executor::apply_oracle_combat_witness;
use super::combat_search::RunControlCombatWorkAdvanceV1;
use super::combat_search_setup::prepare_search_combat;
use super::oracle_combat_policy::{
    ExistingCombatKnowledgeAdvisorAdvanceV1, ExistingCombatKnowledgeAdvisorV1,
    ExistingCombatKnowledgePolicy,
};
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlCombatSearchRejection, RunControlSession, RunProgressOutcome};
use super::trace_annotation::CombatAutomationTrajectorySource;
use crate::state::core::ClientInput;

pub(super) struct OracleRunCombatWorkV1 {
    start: crate::sim::combat::CombatPosition,
    search: OracleCombatWitnessSession,
    advisor: Option<ExistingCombatKnowledgeAdvisorV1>,
    advisor_nodes: u64,
    advisor_elapsed: Duration,
    advisor_complete: bool,
    advisor_failure: Option<String>,
    remaining_work: usize,
    remaining_engine_steps: usize,
    max_transition_steps: usize,
    remaining_wall_time: Option<Duration>,
    quantum_count: usize,
    prior_generation_work: u64,
    restart_count: usize,
    last_status: Option<OracleCombatWitnessStatus>,
    incumbent_revision: u64,
    quanta_since_incumbent_improvement: usize,
    last_quantum_generation_work: usize,
    last_quantum_engine_steps: usize,
    search_resume_exact: bool,
    witness_source: CombatAutomationTrajectorySource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunCombatWorkCheckpointV1 {
    pub consumed_nodes: u64,
    pub remaining_nodes: usize,
    pub remaining_engine_steps: usize,
    pub remaining_wall_ms: Option<u64>,
    pub quantum_count: usize,
    pub restart_count: usize,
    #[serde(default)]
    pub incumbent_revision: u64,
    #[serde(default)]
    pub quanta_since_incumbent_improvement: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incumbent: Option<OracleCombatWitness>,
    #[serde(default)]
    pub advisor_nodes: u64,
    #[serde(default)]
    pub advisor_elapsed_ms: u64,
    #[serde(default)]
    pub advisor_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisor_failure: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct OracleRunCombatWorkProgressV1 {
    /// Work charged by earlier search attempts whose frontier was not
    /// serialized and therefore is not present in the current session.
    pub historical_generation_work: u64,
    /// Work represented by the currently resident search frontier.
    pub current_search_generation_work: u64,
    /// Historical plus current work. This is accounting, not resumable depth.
    pub generation_work: u64,
    pub engine_steps: usize,
    pub exact_states: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub completed_turn_options: usize,
    pub retained_state_work: usize,
    pub queued_anchor_entries: usize,
    pub queued_guided_entries: Vec<usize>,
    pub root_state: Option<OracleCombatWitnessStateProgressSnapshot>,
    pub max_player_turn: u32,
    pub deepest_survival_state: Option<OracleCombatDeepStateSnapshot>,
    pub deepest_progress_state: Option<OracleCombatDeepStateSnapshot>,
    pub deepest_survival_actions: Vec<TurnOptionAction>,
    pub deepest_progress_actions: Vec<TurnOptionAction>,
    pub recent_turn_survival_envelope: Vec<OracleCombatDeepStateSnapshot>,
    pub max_path_atomic_depth: usize,
    pub max_completed_turn_options_at_state: usize,
    pub generation_gap_count: usize,
    pub pending_witness_replay: bool,
    pub policy_witness_proposals: usize,
    pub advisor_nodes: u64,
    pub advisor_elapsed_ms: u64,
    pub advisor_active: bool,
    pub advisor_failure: Option<String>,
    pub incumbent_discovery_source: Option<OracleCombatWitnessDiscoverySource>,
    pub incumbent_final_hp: Option<i32>,
    pub incumbent_hp_loss: Option<i32>,
    pub incumbent_action_count: Option<usize>,
    pub incumbent_revision: u64,
    pub quanta_since_incumbent_improvement: usize,
    pub last_quantum_generation_work: usize,
    pub last_quantum_engine_steps: usize,
    pub last_status: Option<&'static str>,
}

impl OracleRunCombatWorkV1 {
    pub(super) fn root_action_families(&self) -> Vec<OracleCombatRootActionFamilySnapshot> {
        self.search.root_action_families()
    }

    pub(super) fn new(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
    ) -> Result<Self, String> {
        let prepared = prepare_search_combat(session, options)?;
        let max_transition_steps = prepared.config.max_engine_steps_per_action.max(1);
        let max_work = prepared.config.max_nodes;
        let satisfaction = match prepared.config.satisfaction {
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::BudgetOrExhaustion => {
                OracleCombatWitnessSatisfaction::BudgetOrExhaustion
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::ZeroLossOrBudget => {
                OracleCombatWitnessSatisfaction::HpLossAtMost(0)
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin => {
                OracleCombatWitnessSatisfaction::FirstWitness
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(limit) => {
                OracleCombatWitnessSatisfaction::HpLossAtMost(limit)
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden
            | crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(_) => {
                return Err("oracle witness search does not yet own external-burden acceptance"
                    .to_string());
            }
        };
        let root = CombatDecisionRoot::new(prepared.start.clone())
            .map_err(|error| format!("invalid oracle combat root: {error:?}"))?;
        let search = OracleCombatWitnessSession::with_policy(
            root,
            OracleCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition: max_transition_steps,
                    ..TurnOptionGeneratorConfig::default()
                },
                // Keep a selected exact state long enough to make a small,
                // coherent amount of turn-generation progress before the
                // outer state scheduler preempts it. A one-work slice made
                // early turn choices compete with every newly discovered
                // turn-boundary state after each atomic prefix.
                generation_work_per_agenda_pop: 4,
                satisfaction,
            },
            Arc::new(ExistingCombatKnowledgePolicy::default()),
        );
        let advisor = Some(ExistingCombatKnowledgeAdvisorV1::new(
            &prepared.start,
            max_transition_steps,
        ));
        Ok(Self {
            start: prepared.start,
            search,
            advisor,
            advisor_nodes: 0,
            advisor_elapsed: Duration::ZERO,
            advisor_complete: false,
            advisor_failure: None,
            remaining_work: max_work,
            remaining_engine_steps: max_work.saturating_mul(max_transition_steps),
            max_transition_steps,
            remaining_wall_time: prepared.config.wall_time,
            quantum_count: 0,
            prior_generation_work: 0,
            restart_count: 0,
            last_status: None,
            incumbent_revision: 0,
            quanta_since_incumbent_improvement: 0,
            last_quantum_generation_work: 0,
            last_quantum_engine_steps: 0,
            search_resume_exact: false,
            witness_source: CombatAutomationTrajectorySource::SearchCombat,
        })
    }

    pub(super) fn restart_from_checkpoint(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        checkpoint: OracleRunCombatWorkCheckpointV1,
    ) -> Result<Self, String> {
        let mut work = Self::new(session, options)?;
        work.remaining_work = work.remaining_work.min(checkpoint.remaining_nodes);
        work.remaining_engine_steps = work
            .remaining_engine_steps
            .min(checkpoint.remaining_engine_steps);
        work.remaining_wall_time = match (work.remaining_wall_time, checkpoint.remaining_wall_ms) {
            (Some(configured), Some(saved_ms)) => {
                Some(configured.min(Duration::from_millis(saved_ms)))
            }
            (None, Some(saved_ms)) => Some(Duration::from_millis(saved_ms)),
            (configured, None) => configured,
        };
        work.quantum_count = checkpoint.quantum_count;
        work.prior_generation_work = checkpoint.consumed_nodes;
        work.restart_count = checkpoint.restart_count.saturating_add(1);
        work.incumbent_revision = checkpoint.incumbent_revision;
        work.quanta_since_incumbent_improvement = checkpoint.quanta_since_incumbent_improvement;
        work.advisor_nodes = checkpoint.advisor_nodes;
        work.advisor_elapsed = Duration::from_millis(checkpoint.advisor_elapsed_ms);
        work.advisor_complete = checkpoint.advisor_complete;
        work.advisor_failure = checkpoint.advisor_failure;
        if work.advisor_complete {
            work.advisor = None;
        } else if let Some(advisor) = &mut work.advisor {
            advisor.restore_charged_usage(work.advisor_nodes, work.advisor_elapsed);
        }
        if let Some(incumbent) = checkpoint.incumbent {
            work.search.restore_verified_witness(incumbent)?;
            work.advisor = None;
            work.advisor_complete = true;
        }
        Ok(work)
    }

    /// Restores a legacy exact combat state whose checkpoint did not preserve
    /// tactical allowance or incumbent information.  It must be reported as a
    /// search restart even though its allowance necessarily starts fresh.
    pub(super) fn restart_from_exact_state(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
    ) -> Result<Self, String> {
        let mut work = Self::new(session, options)?;
        work.restart_count = 1;
        Ok(work)
    }

    pub(super) fn checkpoint(&self) -> OracleRunCombatWorkCheckpointV1 {
        OracleRunCombatWorkCheckpointV1 {
            consumed_nodes: self.nodes_expanded(),
            remaining_nodes: self.remaining_work,
            remaining_engine_steps: self.remaining_engine_steps,
            remaining_wall_ms: self.remaining_wall_ms(),
            quantum_count: self.quantum_count,
            restart_count: self.restart_count,
            incumbent_revision: self.incumbent_revision,
            quanta_since_incumbent_improvement: self.quanta_since_incumbent_improvement,
            incumbent: self.search.witness().cloned(),
            advisor_nodes: self.advisor_nodes,
            advisor_elapsed_ms: self.advisor_elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            advisor_complete: self.advisor_complete,
            advisor_failure: self.advisor_failure.clone(),
        }
    }

    pub(super) fn advance(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        global_deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        let now = Instant::now();
        let global_remaining =
            global_deadline.map(|deadline| deadline.saturating_duration_since(now));
        if global_remaining == Some(Duration::ZERO) {
            return RunControlCombatWorkAdvanceV1::GlobalDeadlineReached;
        }
        let work = quantum.additional_nodes.min(self.remaining_work);
        if work == 0 || wall_allowance_exhausted(self.remaining_wall_time) {
            return RunControlCombatWorkAdvanceV1::AllowanceExhausted;
        }
        let requested_wall = quantum.soft_wall_ms.map(Duration::from_millis);
        let soft_wall = [requested_wall, self.remaining_wall_time, global_remaining]
            .into_iter()
            .flatten()
            .min();
        if soft_wall == Some(Duration::ZERO) {
            return if global_remaining == Some(Duration::ZERO) {
                RunControlCombatWorkAdvanceV1::GlobalDeadlineReached
            } else {
                RunControlCombatWorkAdvanceV1::AllowanceExhausted
            };
        }
        let deadline = soft_wall.and_then(|duration| now.checked_add(duration));
        self.last_quantum_generation_work = 0;
        self.last_quantum_engine_steps = 0;
        if let Some(advisor) = &mut self.advisor {
            let hard_wall = [self.remaining_wall_time, global_remaining]
                .into_iter()
                .flatten()
                .min();
            let advisor_status = advisor.advance(soft_wall, hard_wall);
            self.advisor_nodes = advisor.total_nodes();
            self.advisor_elapsed = advisor.total_elapsed();
            match advisor_status {
                Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Pending) => {
                    if let Some(remaining) = &mut self.remaining_wall_time {
                        *remaining = remaining.saturating_sub(now.elapsed());
                    }
                    self.quantum_count = self.quantum_count.saturating_add(1);
                    self.quanta_since_incumbent_improvement =
                        self.quanta_since_incumbent_improvement.saturating_add(1);
                    return if wall_allowance_exhausted(self.remaining_wall_time) {
                        RunControlCombatWorkAdvanceV1::AllowanceExhausted
                    } else {
                        RunControlCombatWorkAdvanceV1::Pending
                    };
                }
                Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Proposal(proposal)) => {
                    self.search.offer_witness_proposal(proposal);
                    self.advisor = None;
                    self.advisor_complete = true;
                }
                Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted) => {
                    self.advisor = None;
                    self.advisor_complete = true;
                }
                Err(error) => {
                    self.advisor = None;
                    self.advisor_complete = true;
                    self.advisor_failure = Some(error);
                }
            }
        }
        let before = self.search.counters();
        let before_incumbent_hp = self
            .search
            .witness()
            .map(|witness| witness.final_position.combat.entities.player.current_hp);
        let engine_grant = self
            .remaining_engine_steps
            .min(work.saturating_mul(self.max_transition_steps));
        let report = self.search.advance(
            &crate::sim::combat::EngineCombatStepper,
            OracleCombatWitnessQuantum {
                additional_agenda_pops: work,
                additional_generation_work: work,
                additional_engine_steps: engine_grant,
                deadline,
            },
        );
        let after = report.after;
        let consumed_work = after.generation_work.saturating_sub(before.generation_work);
        let consumed_engine = after.engine_steps.saturating_sub(before.engine_steps);
        self.last_quantum_generation_work = consumed_work;
        self.last_quantum_engine_steps = consumed_engine;
        let after_incumbent_hp = self
            .search
            .witness()
            .map(|witness| witness.final_position.combat.entities.player.current_hp);
        if after_incumbent_hp.is_some()
            && (before_incumbent_hp.is_none() || after_incumbent_hp > before_incumbent_hp)
        {
            self.incumbent_revision = self.incumbent_revision.saturating_add(1);
            self.quanta_since_incumbent_improvement = 0;
        } else {
            self.quanta_since_incumbent_improvement =
                self.quanta_since_incumbent_improvement.saturating_add(1);
        }
        self.remaining_work = self.remaining_work.saturating_sub(consumed_work);
        self.remaining_engine_steps = self.remaining_engine_steps.saturating_sub(consumed_engine);
        if let Some(remaining) = &mut self.remaining_wall_time {
            *remaining = remaining.saturating_sub(now.elapsed());
        }
        self.quantum_count = self.quantum_count.saturating_add(1);
        self.last_status = Some(report.status.clone());
        match report.status {
            OracleCombatWitnessStatus::WitnessFound
            | OracleCombatWitnessStatus::FrontierExhausted
            | OracleCombatWitnessStatus::MechanicsGap
            | OracleCombatWitnessStatus::ReplayMismatch(_) => {
                RunControlCombatWorkAdvanceV1::ReadyToFinish
            }
            OracleCombatWitnessStatus::Partial(_) => {
                if self.remaining_work == 0
                    || self.remaining_engine_steps == 0
                    || wall_allowance_exhausted(self.remaining_wall_time)
                {
                    RunControlCombatWorkAdvanceV1::AllowanceExhausted
                } else {
                    RunControlCombatWorkAdvanceV1::Pending
                }
            }
        }
    }

    /// Extends only an exhausted allowance dimension. The tactical frontier,
    /// transposition table, generators, and incumbent remain resident.
    /// Ensures an explicit analysis request receives the allowance it asked
    /// for without discarding an existing tactical frontier. In particular,
    /// a two-second tail from the previous request must not consume a whole
    /// autosave cycle before a requested thirty-second continuation begins.
    pub(super) fn ensure_requested_allowance(
        &mut self,
        requested_nodes: usize,
        requested_wall_time: Option<Duration>,
    ) {
        self.remaining_work = self.remaining_work.max(requested_nodes);
        self.remaining_engine_steps = self
            .remaining_engine_steps
            .max(requested_nodes.saturating_mul(self.max_transition_steps));
        if let (Some(remaining), Some(requested)) =
            (&mut self.remaining_wall_time, requested_wall_time)
        {
            *remaining = (*remaining).max(requested);
        }
    }

    pub(super) fn mark_search_resume_exact(&mut self) {
        if self.quantum_count > 0 {
            self.search_resume_exact = true;
        }
    }

    pub(super) fn search_resume_exact(&self) -> bool {
        self.search_resume_exact
    }

    pub(super) fn has_verified_witness(&self) -> bool {
        self.search.witness().is_some()
    }

    /// Replays an analyst-supplied exact action sequence from this job's
    /// unchanged combat root and installs it only when every action is legal
    /// and the simulator reaches a terminal victory. This is an explicit
    /// oracle-analysis operation, not a search claim or heuristic shortcut.
    pub(super) fn verify_and_restore_action_witness(
        &mut self,
        inputs: &[ClientInput],
    ) -> Result<(), String> {
        let stepper = crate::sim::combat::EngineCombatStepper;
        let mut position = self.start.clone();
        let mut actions = Vec::with_capacity(inputs.len());
        let mut replay_engine_steps = 0usize;
        for (index, input) in inputs.iter().enumerate() {
            use crate::sim::combat::CombatStepper;

            if stepper.choice_for_legal_input(&position, input).is_none() {
                return Err(format!(
                    "oracle combat witness action {index} is not legal at its exact state: {input:?}"
                ));
            }
            let result = stepper.apply_to_stable(
                &position,
                input.clone(),
                crate::sim::combat::CombatStepLimits {
                    max_engine_steps: self.max_transition_steps,
                    deadline: None,
                },
            );
            if result.truncated {
                return Err(format!(
                    "oracle combat witness action {index} exceeded the transition limit"
                ));
            }
            replay_engine_steps = replay_engine_steps.saturating_add(result.engine_steps);
            actions.push(TurnOptionAction {
                input: input.clone(),
                expected_successor_hash: crate::ai::combat_state_key::combat_exact_state_hash_v1(
                    &result.position.engine,
                    &result.position.combat,
                ),
                engine_steps: result.engine_steps,
            });
            position = result.position;
        }
        if crate::sim::combat::combat_terminal(&position.engine, &position.combat)
            != crate::sim::combat::CombatTerminal::Win
        {
            return Err("oracle combat witness actions did not reach terminal victory".to_string());
        }
        self.search.restore_verified_witness(OracleCombatWitness {
            actions,
            final_position: position,
            // The sequence is accepted for its exact replay proof. Search
            // may still replace it with an equal-HP, shorter witness later.
            negative_log_policy: inputs.len() as f64,
            replay_engine_steps,
            discovery_source: OracleCombatWitnessDiscoverySource::RestoredExactActions,
        })?;
        self.witness_source = CombatAutomationTrajectorySource::OracleExactActions;
        Ok(())
    }

    pub(super) fn nodes_expanded(&self) -> u64 {
        self.prior_generation_work
            .saturating_add(self.search.counters().generation_work as u64)
    }

    pub(super) fn quantum_count(&self) -> usize {
        self.quantum_count
    }

    pub(super) fn remaining_nodes(&self) -> usize {
        self.remaining_work
    }

    pub(super) fn remaining_wall_ms(&self) -> Option<u64> {
        self.remaining_wall_time
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
    }

    pub(super) fn restart_count(&self) -> usize {
        self.restart_count
    }

    pub(super) fn progress(&self) -> OracleRunCombatWorkProgressV1 {
        let counters = self.search.counters();
        let search_progress = self.search.progress_snapshot();
        let initial_hp = self.start.combat.entities.player.current_hp;
        let incumbent = self.search.witness();
        let incumbent_final_hp =
            incumbent.map(|witness| witness.final_position.combat.entities.player.current_hp);
        OracleRunCombatWorkProgressV1 {
            historical_generation_work: self.prior_generation_work,
            current_search_generation_work: counters.generation_work as u64,
            generation_work: self
                .prior_generation_work
                .saturating_add(counters.generation_work as u64),
            engine_steps: counters.engine_steps,
            exact_states: counters.exact_states,
            applied_action_transitions: counters.applied_action_transitions,
            unique_successor_states: counters.unique_successor_states,
            duplicate_exact_successors: counters.duplicate_exact_successors,
            completed_turn_options: counters.completed_turn_options,
            retained_state_work: self.search.retained_state_work(),
            queued_anchor_entries: search_progress.queued_anchor_entries,
            queued_guided_entries: search_progress.queued_guided_entries,
            root_state: search_progress.root_state,
            max_player_turn: search_progress.max_player_turn,
            deepest_survival_state: search_progress.deepest_survival_state,
            deepest_progress_state: search_progress.deepest_progress_state,
            deepest_survival_actions: search_progress.deepest_survival_actions,
            deepest_progress_actions: search_progress.deepest_progress_actions,
            recent_turn_survival_envelope: search_progress.recent_turn_survival_envelope,
            max_path_atomic_depth: search_progress.max_path_atomic_depth,
            max_completed_turn_options_at_state: search_progress
                .max_completed_turn_options_at_state,
            generation_gap_count: search_progress.generation_gap_count,
            pending_witness_replay: search_progress.pending_witness_replay,
            policy_witness_proposals: counters.policy_witness_proposals,
            advisor_nodes: self.advisor_nodes,
            advisor_elapsed_ms: self.advisor_elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            advisor_active: self.advisor.is_some(),
            advisor_failure: self.advisor_failure.clone(),
            incumbent_discovery_source: incumbent.map(|witness| witness.discovery_source),
            incumbent_final_hp,
            incumbent_hp_loss: incumbent_final_hp
                .map(|final_hp| initial_hp.saturating_sub(final_hp).max(0)),
            incumbent_action_count: incumbent.map(|witness| witness.actions.len()),
            incumbent_revision: self.incumbent_revision,
            quanta_since_incumbent_improvement: self.quanta_since_incumbent_improvement,
            last_quantum_generation_work: self.last_quantum_generation_work,
            last_quantum_engine_steps: self.last_quantum_engine_steps,
            last_status: self.last_status.as_ref().map(oracle_witness_status_label),
        }
    }

    pub(super) fn finish_and_apply(
        self,
        session: &mut RunControlSession,
    ) -> Result<RunProgressOutcome, String> {
        if session.current_active_combat_position()? != self.start {
            return Err("oracle combat parent changed before search commit".to_string());
        }
        if let Some(witness) = self.search.witness() {
            let source = match witness.discovery_source {
                OracleCombatWitnessDiscoverySource::PolicyProposal => {
                    CombatAutomationTrajectorySource::V2Donor
                }
                OracleCombatWitnessDiscoverySource::PlannerSearch => {
                    CombatAutomationTrajectorySource::SearchCombat
                }
                OracleCombatWitnessDiscoverySource::SolvedSuffixComposition => {
                    CombatAutomationTrajectorySource::SearchCombat
                }
                OracleCombatWitnessDiscoverySource::RestoredExactActions => {
                    CombatAutomationTrajectorySource::OracleExactActions
                }
                OracleCombatWitnessDiscoverySource::LegacyUnattributed => self.witness_source,
            };
            return apply_oracle_combat_witness(session, &self.start, witness, source);
        }
        let status = self
            .last_status
            .unwrap_or(OracleCombatWitnessStatus::Partial(
                sts_combat_planner::OracleCombatWitnessInterruption::AgendaBudget,
            ));
        Ok(RunProgressOutcome::message(format!(
            "Oracle combat search did not modify state. status={status:?} generation_work={} exact_states={} retained_work={}",
            self.prior_generation_work
                .saturating_add(self.search.counters().generation_work as u64),
            self.search.counters().exact_states,
            self.search.retained_state_work(),
        ))
        .with_combat_search_rejection(
            RunControlCombatSearchRejection::NoCompleteWinningCandidate,
        ))
    }
}

fn wall_allowance_exhausted(remaining: Option<Duration>) -> bool {
    remaining.is_some_and(|duration| duration < MIN_USABLE_WALL_ALLOWANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_millisecond_wall_tail_is_not_treated_as_usable_allowance() {
        assert!(wall_allowance_exhausted(Some(Duration::from_micros(999))));
        assert!(!wall_allowance_exhausted(Some(Duration::from_millis(1))));
        assert!(!wall_allowance_exhausted(None));
    }
}

fn oracle_witness_status_label(status: &OracleCombatWitnessStatus) -> &'static str {
    match status {
        OracleCombatWitnessStatus::WitnessFound => "witness_found",
        OracleCombatWitnessStatus::Partial(_) => "partial",
        OracleCombatWitnessStatus::FrontierExhausted => "frontier_exhausted",
        OracleCombatWitnessStatus::MechanicsGap => "mechanics_gap",
        OracleCombatWitnessStatus::ReplayMismatch(_) => "replay_mismatch",
    }
}
