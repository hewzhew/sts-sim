use std::sync::Arc;
use std::time::{Duration, Instant};

const MIN_USABLE_WALL_ALLOWANCE: Duration = Duration::from_millis(1);

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    CombatDecisionRoot, LocalTurnGraphRootActionFamilySnapshot, LocalTurnGraphWitnessConfig,
    LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession, LocalTurnGraphWitnessStatus,
    OracleCombatDeepStateSnapshot, OracleCombatWitness, OracleCombatWitnessConfig,
    OracleCombatWitnessDiscoverySource, OracleCombatWitnessQuantum,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessSession,
    OracleCombatWitnessStateProgressSnapshot, OracleCombatWitnessStatus, TurnOptionAction,
    TurnOptionGeneratorConfig,
};

use super::combat_line_executor::apply_oracle_combat_witness;
use super::combat_search::RunControlCombatWorkAdvanceV1;
use super::combat_search_setup::prepare_search_combat;
use super::oracle_combat_policy::ExistingCombatKnowledgePolicy;
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlCombatSearchRejection, RunControlSession, RunProgressOutcome};
use super::trace_annotation::CombatAutomationTrajectorySource;
use crate::state::core::ClientInput;

pub(super) struct OracleRunCombatWorkV1 {
    start: crate::sim::combat::CombatPosition,
    local_search: LocalTurnGraphWitnessSession,
    global_search: OracleCombatWitnessSession,
    next_portfolio_member: PortfolioMemberV1,
    local_complete: bool,
    global_complete: bool,
    remaining_work: usize,
    remaining_engine_steps: usize,
    max_transition_steps: usize,
    remaining_wall_time: Option<Duration>,
    quantum_count: usize,
    prior_generation_work: u64,
    restart_count: usize,
    last_status: Option<PortfolioStatusV1>,
    local_status: Option<LocalTurnGraphWitnessStatus>,
    global_status: Option<OracleCombatWitnessStatus>,
    incumbent_revision: u64,
    quanta_since_incumbent_improvement: usize,
    last_quantum_generation_work: usize,
    last_quantum_engine_steps: usize,
    search_resume_exact: bool,
    witness_source: CombatAutomationTrajectorySource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortfolioMemberV1 {
    LocalTurnGraph,
    GlobalAgenda,
}

impl PortfolioMemberV1 {
    fn other(self) -> Self {
        match self {
            Self::LocalTurnGraph => Self::GlobalAgenda,
            Self::GlobalAgenda => Self::LocalTurnGraph,
        }
    }
}

#[derive(Clone, Debug)]
enum PortfolioStatusV1 {
    Local(LocalTurnGraphWitnessStatus),
    Global(OracleCombatWitnessStatus),
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
    pub(super) fn root_action_families(&self) -> Vec<LocalTurnGraphRootActionFamilySnapshot> {
        self.local_search.root_action_families()
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
        let policy = Arc::new(ExistingCombatKnowledgePolicy::default());
        let local_search = LocalTurnGraphWitnessSession::with_policy(
            root.clone(),
            LocalTurnGraphWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition: max_transition_steps,
                    ..TurnOptionGeneratorConfig::default()
                },
                generation_quantum_work: 4,
                max_turn_depth: 32,
                satisfaction,
            },
            policy.clone(),
        );
        let global_search = OracleCombatWitnessSession::with_policy(
            root,
            OracleCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition: max_transition_steps,
                    ..TurnOptionGeneratorConfig::default()
                },
                generation_work_per_agenda_pop: 4,
                satisfaction,
            },
            policy,
        );
        Ok(Self {
            start: prepared.start,
            local_search,
            global_search,
            next_portfolio_member: PortfolioMemberV1::LocalTurnGraph,
            local_complete: false,
            global_complete: false,
            remaining_work: max_work,
            remaining_engine_steps: max_work.saturating_mul(max_transition_steps),
            max_transition_steps,
            remaining_wall_time: prepared.config.wall_time,
            quantum_count: 0,
            prior_generation_work: 0,
            restart_count: 0,
            last_status: None,
            local_status: None,
            global_status: None,
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
        if let Some(incumbent) = checkpoint.incumbent {
            work.local_search
                .restore_verified_witness(incumbent.clone())?;
            work.global_search.restore_verified_witness(incumbent)?;
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
            incumbent: self.best_witness().cloned(),
            // Kept in checkpoint schema so old files still deserialize. New
            // local-graph searches never start the retired V2 advisor.
            advisor_nodes: 0,
            advisor_elapsed_ms: 0,
            advisor_complete: true,
            advisor_failure: None,
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
        let before_incumbent_hp = self
            .best_witness()
            .map(|witness| witness.final_position.combat.entities.player.current_hp);
        let engine_grant = self
            .remaining_engine_steps
            .min(work.saturating_mul(self.max_transition_steps));
        let Some(member) = select_portfolio_member(
            self.next_portfolio_member,
            self.local_complete,
            self.global_complete,
        ) else {
            return RunControlCombatWorkAdvanceV1::ReadyToFinish;
        };
        self.next_portfolio_member = member.other();
        let (consumed_work, consumed_engine, member_complete, witness_found, status) = match member
        {
            PortfolioMemberV1::LocalTurnGraph => {
                let before = self.local_search.counters();
                let report = self.local_search.advance(
                    LocalTurnGraphWitnessQuantum {
                        additional_selections: work,
                        additional_generation_work: work,
                        additional_engine_steps: engine_grant,
                        deadline,
                    },
                    &crate::sim::combat::EngineCombatStepper,
                );
                let after = report.counters;
                let member_complete = !matches!(
                    report.status,
                    LocalTurnGraphWitnessStatus::Partial(_)
                        | LocalTurnGraphWitnessStatus::WitnessFound
                );
                let witness_found =
                    matches!(report.status, LocalTurnGraphWitnessStatus::WitnessFound);
                (
                    after.generation_work.saturating_sub(before.generation_work),
                    after.engine_steps.saturating_sub(before.engine_steps),
                    member_complete,
                    witness_found,
                    PortfolioStatusV1::Local(report.status),
                )
            }
            PortfolioMemberV1::GlobalAgenda => {
                let before = self.global_search.counters();
                let report = self.global_search.advance(
                    &crate::sim::combat::EngineCombatStepper,
                    OracleCombatWitnessQuantum {
                        additional_agenda_pops: work,
                        additional_generation_work: work,
                        additional_engine_steps: engine_grant,
                        deadline,
                    },
                );
                let after = report.after;
                let member_complete = !matches!(
                    report.status,
                    OracleCombatWitnessStatus::Partial(_) | OracleCombatWitnessStatus::WitnessFound
                );
                let witness_found =
                    matches!(report.status, OracleCombatWitnessStatus::WitnessFound);
                (
                    after.generation_work.saturating_sub(before.generation_work),
                    after.engine_steps.saturating_sub(before.engine_steps),
                    member_complete,
                    witness_found,
                    PortfolioStatusV1::Global(report.status),
                )
            }
        };
        match &status {
            PortfolioStatusV1::Local(status) => {
                self.local_complete = member_complete;
                self.local_status = Some(status.clone());
            }
            PortfolioStatusV1::Global(status) => {
                self.global_complete = member_complete;
                self.global_status = Some(status.clone());
            }
        }
        self.last_quantum_generation_work = consumed_work;
        self.last_quantum_engine_steps = consumed_engine;
        let after_incumbent_hp = self
            .best_witness()
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
        self.last_status = Some(status);
        if witness_found || (self.local_complete && self.global_complete) {
            RunControlCombatWorkAdvanceV1::ReadyToFinish
        } else if self.remaining_work == 0
            || self.remaining_engine_steps == 0
            || wall_allowance_exhausted(self.remaining_wall_time)
        {
            RunControlCombatWorkAdvanceV1::AllowanceExhausted
        } else {
            RunControlCombatWorkAdvanceV1::Pending
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
        self.best_witness().is_some()
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
        let witness = OracleCombatWitness {
            actions,
            final_position: position,
            // The sequence is accepted for its exact replay proof. Search
            // may still replace it with an equal-HP, shorter witness later.
            negative_log_policy: inputs.len() as f64,
            replay_engine_steps,
            discovery_source: OracleCombatWitnessDiscoverySource::RestoredExactActions,
        };
        self.local_search
            .restore_verified_witness(witness.clone())?;
        self.global_search.restore_verified_witness(witness)?;
        self.witness_source = CombatAutomationTrajectorySource::OracleExactActions;
        Ok(())
    }

    pub(super) fn nodes_expanded(&self) -> u64 {
        self.prior_generation_work
            .saturating_add(self.current_generation_work())
    }

    fn current_generation_work(&self) -> u64 {
        (self.local_search.counters().generation_work as u64)
            .saturating_add(self.global_search.counters().generation_work as u64)
    }

    fn best_witness(&self) -> Option<&OracleCombatWitness> {
        match (self.local_search.witness(), self.global_search.witness()) {
            (Some(local), Some(global)) => Some(if combat_witness_better(local, global) {
                local
            } else {
                global
            }),
            (Some(local), None) => Some(local),
            (None, Some(global)) => Some(global),
            (None, None) => None,
        }
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
        let local_counters = self.local_search.counters();
        let global_counters = self.global_search.counters();
        let local_progress = self.local_search.progress_snapshot();
        let global_progress = self.global_search.progress_snapshot();
        let search_progress = if local_progress.max_player_turn >= global_progress.max_player_turn {
            &local_progress
        } else {
            &global_progress
        };
        let initial_hp = self.start.combat.entities.player.current_hp;
        let incumbent = self.best_witness();
        let incumbent_final_hp =
            incumbent.map(|witness| witness.final_position.combat.entities.player.current_hp);
        let current_generation_work = self.current_generation_work();
        OracleRunCombatWorkProgressV1 {
            historical_generation_work: self.prior_generation_work,
            current_search_generation_work: current_generation_work,
            generation_work: self
                .prior_generation_work
                .saturating_add(current_generation_work),
            engine_steps: local_counters
                .engine_steps
                .saturating_add(global_counters.engine_steps),
            exact_states: local_counters
                .exact_nodes
                .saturating_add(global_counters.exact_states),
            applied_action_transitions: local_counters
                .applied_action_transitions
                .saturating_add(global_counters.applied_action_transitions),
            unique_successor_states: local_counters
                .unique_successor_states
                .saturating_add(global_counters.unique_successor_states),
            duplicate_exact_successors: local_counters
                .duplicate_exact_successors
                .saturating_add(global_counters.duplicate_exact_successors),
            completed_turn_options: local_counters
                .completed_turn_options
                .saturating_add(global_counters.completed_turn_options),
            retained_state_work: self
                .local_search
                .retained_state_work()
                .saturating_add(self.global_search.retained_state_work()),
            queued_anchor_entries: local_progress
                .queued_anchor_entries
                .saturating_add(global_progress.queued_anchor_entries),
            queued_guided_entries: sum_lane_counts(
                &local_progress.queued_guided_entries,
                &global_progress.queued_guided_entries,
            ),
            root_state: search_progress.root_state.clone(),
            max_player_turn: local_progress
                .max_player_turn
                .max(global_progress.max_player_turn),
            deepest_survival_state: search_progress.deepest_survival_state.clone(),
            deepest_progress_state: search_progress.deepest_progress_state.clone(),
            deepest_survival_actions: search_progress.deepest_survival_actions.clone(),
            deepest_progress_actions: search_progress.deepest_progress_actions.clone(),
            recent_turn_survival_envelope: search_progress.recent_turn_survival_envelope.clone(),
            max_path_atomic_depth: local_progress
                .max_path_atomic_depth
                .max(global_progress.max_path_atomic_depth),
            max_completed_turn_options_at_state: local_progress
                .max_completed_turn_options_at_state
                .max(global_progress.max_completed_turn_options_at_state),
            generation_gap_count: local_progress
                .generation_gap_count
                .saturating_add(global_progress.generation_gap_count),
            pending_witness_replay: local_progress.pending_witness_replay
                || global_progress.pending_witness_replay,
            policy_witness_proposals: global_counters.policy_witness_proposals,
            advisor_nodes: 0,
            advisor_elapsed_ms: 0,
            advisor_active: false,
            advisor_failure: None,
            incumbent_discovery_source: incumbent.map(|witness| witness.discovery_source),
            incumbent_final_hp,
            incumbent_hp_loss: incumbent_final_hp
                .map(|final_hp| initial_hp.saturating_sub(final_hp).max(0)),
            incumbent_action_count: incumbent.map(|witness| witness.actions.len()),
            incumbent_revision: self.incumbent_revision,
            quanta_since_incumbent_improvement: self.quanta_since_incumbent_improvement,
            last_quantum_generation_work: self.last_quantum_generation_work,
            last_quantum_engine_steps: self.last_quantum_engine_steps,
            last_status: overall_status_label(
                self.best_witness().is_some(),
                self.local_status.as_ref(),
                self.global_status.as_ref(),
                self.last_status.as_ref(),
            ),
        }
    }

    pub(super) fn finish_and_apply(
        self,
        session: &mut RunControlSession,
    ) -> Result<RunProgressOutcome, String> {
        if session.current_active_combat_position()? != self.start {
            return Err("oracle combat parent changed before search commit".to_string());
        }
        if let Some(witness) = self.best_witness() {
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
        let default_status = PortfolioStatusV1::Local(LocalTurnGraphWitnessStatus::Partial(
            sts_combat_planner::LocalTurnGraphWitnessInterruption::SelectionBudget,
        ));
        let status = self.last_status.as_ref().unwrap_or(&default_status);
        Ok(RunProgressOutcome::message(format!(
            "Combat-search portfolio did not modify state. status={status:?} generation_work={} local_work={} global_work={} exact_states={} retained_work={}",
            self.prior_generation_work
                .saturating_add(self.current_generation_work()),
            self.local_search.counters().generation_work,
            self.global_search.counters().generation_work,
            self.local_search
                .counters()
                .exact_nodes
                .saturating_add(self.global_search.counters().exact_states),
            self.local_search
                .retained_state_work()
                .saturating_add(self.global_search.retained_state_work()),
        ))
        .with_combat_search_rejection(
            RunControlCombatSearchRejection::NoCompleteWinningCandidate,
        ))
    }
}

fn wall_allowance_exhausted(remaining: Option<Duration>) -> bool {
    remaining.is_some_and(|duration| duration < MIN_USABLE_WALL_ALLOWANCE)
}

fn select_portfolio_member(
    next: PortfolioMemberV1,
    local_complete: bool,
    global_complete: bool,
) -> Option<PortfolioMemberV1> {
    match (next, local_complete, global_complete) {
        (_, true, true) => None,
        (PortfolioMemberV1::LocalTurnGraph, false, _)
        | (PortfolioMemberV1::GlobalAgenda, false, true) => Some(PortfolioMemberV1::LocalTurnGraph),
        (PortfolioMemberV1::GlobalAgenda, _, false)
        | (PortfolioMemberV1::LocalTurnGraph, true, false) => Some(PortfolioMemberV1::GlobalAgenda),
    }
}

fn combat_witness_better(candidate: &OracleCombatWitness, incumbent: &OracleCombatWitness) -> bool {
    let candidate_hp = candidate.final_position.combat.entities.player.current_hp;
    let incumbent_hp = incumbent.final_position.combat.entities.player.current_hp;
    candidate_hp > incumbent_hp
        || (candidate_hp == incumbent_hp
            && (candidate.actions.len() < incumbent.actions.len()
                || (candidate.actions.len() == incumbent.actions.len()
                    && candidate
                        .negative_log_policy
                        .total_cmp(&incumbent.negative_log_policy)
                        .is_lt())))
}

fn sum_lane_counts(left: &[usize], right: &[usize]) -> Vec<usize> {
    let lane_count = left.len().max(right.len());
    (0..lane_count)
        .map(|index| {
            left.get(index)
                .copied()
                .unwrap_or(0)
                .saturating_add(right.get(index).copied().unwrap_or(0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hallway_combat_session() -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(
            crate::content::monsters::EnemyId::JawWorm,
        )];
        let mut session =
            RunControlSession::new(crate::eval::run_control::RunControlConfig::default());
        session.active_combat = Some(crate::state::core::ActiveCombat::new(
            crate::state::core::EngineState::CombatPlayerTurn,
            combat,
            crate::state::core::CombatContext::Room(crate::state::core::RoomCombatContext {
                room_type: crate::state::map::node::RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn sub_millisecond_wall_tail_is_not_treated_as_usable_allowance() {
        assert!(wall_allowance_exhausted(Some(Duration::from_micros(999))));
        assert!(!wall_allowance_exhausted(Some(Duration::from_millis(1))));
        assert!(!wall_allowance_exhausted(None));
    }

    #[test]
    fn lane_counters_are_aggregated_without_dropping_longer_portfolio() {
        assert_eq!(sum_lane_counts(&[1, 2], &[4, 8, 16]), vec![5, 10, 16]);
    }

    #[test]
    fn portfolio_alternates_live_members_and_skips_completed_members() {
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::LocalTurnGraph, false, false),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::GlobalAgenda, false, false),
            Some(PortfolioMemberV1::GlobalAgenda)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::LocalTurnGraph, true, false),
            Some(PortfolioMemberV1::GlobalAgenda)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::GlobalAgenda, false, true),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::LocalTurnGraph, true, true),
            None
        );
    }

    #[test]
    fn portfolio_charges_each_live_search_from_one_shared_allowance() {
        let session = hallway_combat_session();
        let mut work = OracleRunCombatWorkV1::new(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
        )
        .expect("portfolio should accept an active combat");
        let quantum = RunControlCombatSearchQuantum {
            label: "portfolio_contract",
            additional_nodes: 1,
            soft_wall_ms: None,
        };

        assert_eq!(
            work.advance(&quantum, None),
            RunControlCombatWorkAdvanceV1::Pending
        );
        assert_eq!(work.local_search.counters().generation_work, 1);
        assert_eq!(work.global_search.counters().generation_work, 0);
        assert_eq!(work.remaining_work, 15);

        assert_eq!(
            work.advance(&quantum, None),
            RunControlCombatWorkAdvanceV1::Pending
        );
        assert_eq!(work.local_search.counters().generation_work, 1);
        assert_eq!(work.global_search.counters().generation_work, 1);
        assert_eq!(work.remaining_work, 14);
    }

    #[test]
    fn portfolio_preserves_stable_overall_status_semantics() {
        assert_eq!(
            overall_status_label(
                false,
                Some(&LocalTurnGraphWitnessStatus::FrontierExhausted),
                Some(&OracleCombatWitnessStatus::FrontierExhausted),
                None,
            ),
            Some("frontier_exhausted")
        );
        assert_eq!(
            overall_status_label(
                false,
                Some(&LocalTurnGraphWitnessStatus::MechanicsGap),
                Some(&OracleCombatWitnessStatus::FrontierExhausted),
                None,
            ),
            Some("mechanics_gap")
        );
        assert_eq!(
            overall_status_label(
                true,
                Some(&LocalTurnGraphWitnessStatus::Partial(
                    sts_combat_planner::LocalTurnGraphWitnessInterruption::SelectionBudget,
                )),
                None,
                None,
            ),
            Some("witness_found")
        );
    }
}

fn local_witness_status_label(status: &LocalTurnGraphWitnessStatus) -> &'static str {
    match status {
        LocalTurnGraphWitnessStatus::WitnessFound => "witness_found",
        LocalTurnGraphWitnessStatus::Partial(_) => "partial",
        LocalTurnGraphWitnessStatus::FrontierExhausted => "frontier_exhausted",
        LocalTurnGraphWitnessStatus::MechanicsGap => "mechanics_gap",
        LocalTurnGraphWitnessStatus::ReplayMismatch(_) => "replay_mismatch",
    }
}

fn global_witness_status_label(status: &OracleCombatWitnessStatus) -> &'static str {
    match status {
        OracleCombatWitnessStatus::WitnessFound => "witness_found",
        OracleCombatWitnessStatus::Partial(_) => "partial",
        OracleCombatWitnessStatus::FrontierExhausted => "frontier_exhausted",
        OracleCombatWitnessStatus::MechanicsGap => "mechanics_gap",
        OracleCombatWitnessStatus::ReplayMismatch(_) => "replay_mismatch",
    }
}

fn portfolio_status_label(status: &PortfolioStatusV1) -> &'static str {
    match status {
        PortfolioStatusV1::Local(status) => local_witness_status_label(status),
        PortfolioStatusV1::Global(status) => global_witness_status_label(status),
    }
}

fn overall_status_label(
    has_witness: bool,
    local: Option<&LocalTurnGraphWitnessStatus>,
    global: Option<&OracleCombatWitnessStatus>,
    last: Option<&PortfolioStatusV1>,
) -> Option<&'static str> {
    if has_witness {
        return Some("witness_found");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::FrontierExhausted))
        && matches!(global, Some(OracleCombatWitnessStatus::FrontierExhausted))
    {
        return Some("frontier_exhausted");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::ReplayMismatch(_)))
        || matches!(global, Some(OracleCombatWitnessStatus::ReplayMismatch(_)))
    {
        return Some("replay_mismatch");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::MechanicsGap))
        || matches!(global, Some(OracleCombatWitnessStatus::MechanicsGap))
    {
        return Some("mechanics_gap");
    }
    last.map(portfolio_status_label)
}
