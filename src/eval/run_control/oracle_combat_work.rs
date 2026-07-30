use std::sync::Arc;
use std::time::{Duration, Instant};

const MIN_USABLE_WALL_ALLOWANCE: Duration = Duration::from_millis(1);

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    combat_plan_state_guide_policy_v1, CombatDecisionRoot, LocalTurnGraphRootActionFamilySnapshot,
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession,
    LocalTurnGraphWitnessStatus, OracleCombatDeepStateSnapshot, OracleCombatWitness,
    OracleCombatWitnessDiscoverySource, OracleCombatWitnessSatisfaction,
    OracleCombatWitnessStateProgressSnapshot, PolicyDiscrepancyConfig, PolicyDiscrepancyQuantum,
    PolicyDiscrepancySession, PolicyDiscrepancyStatus, PolicyDiscrepancyTurnMacroConfig,
    TurnOptionAction, TurnOptionGeneratorConfig,
};

use super::combat_line_executor::apply_oracle_combat_witness;
use super::combat_search::RunControlCombatWorkAdvanceV1;
use super::combat_search_setup::prepare_search_combat;
use super::oracle_combat_policy::{
    existing_combat_rollout_witness_v1, ExistingCombatKnowledgePolicy,
};
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlCombatSearchRejection, RunControlSession, RunProgressOutcome};
use super::trace_annotation::CombatAutomationTrajectorySource;
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::state::core::ClientInput;

pub(super) struct OracleRunCombatWorkV1 {
    start: crate::sim::combat::CombatPosition,
    local_search: LocalTurnGraphWitnessSession,
    discrepancy_search: PolicyDiscrepancySession,
    next_portfolio_member: PortfolioMemberV1,
    local_complete: bool,
    discrepancy_complete: bool,
    remaining_work: usize,
    remaining_engine_steps: usize,
    max_transition_steps: usize,
    max_potions_used: Option<u32>,
    satisfaction: PortfolioWitnessSatisfactionV1,
    remaining_wall_time: Option<Duration>,
    quantum_count: usize,
    prior_generation_work: u64,
    prior_policy_witness_proposals: usize,
    policy_witness_proposals: usize,
    policy_witness_replay_engine_steps: usize,
    policy_witness_proposal_rejections: usize,
    policy_witness: Option<OracleCombatWitness>,
    discrepancy_witness: Option<OracleCombatWitness>,
    restart_count: usize,
    last_status: Option<PortfolioStatusV1>,
    local_status: Option<LocalTurnGraphWitnessStatus>,
    discrepancy_status: Option<PolicyDiscrepancyStatus>,
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
    PolicyDiscrepancy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortfolioWitnessSatisfactionV1 {
    FirstWitness,
    HpLossAtMost(u32),
    PersistentRunValueGain,
    BudgetOrExhaustion,
}

impl PortfolioMemberV1 {
    fn other(self) -> Self {
        match self {
            Self::LocalTurnGraph => Self::PolicyDiscrepancy,
            Self::PolicyDiscrepancy => Self::LocalTurnGraph,
        }
    }
}

#[derive(Clone, Debug)]
enum PortfolioStatusV1 {
    Local(LocalTurnGraphWitnessStatus),
    PolicyDiscrepancy(PolicyDiscrepancyStatus),
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
    pub policy_witness_proposals: usize,
    #[serde(default)]
    pub policy_witness_proposal_rejections: usize,
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
    pub local_generation_work: u64,
    pub discrepancy_generation_work: u64,
    pub lookahead_evaluations: usize,
    pub lookahead_work: usize,
    pub engine_steps: usize,
    pub exact_states: usize,
    pub local_exact_states: usize,
    pub discrepancy_exact_states: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub completed_turn_options: usize,
    pub retained_state_work: usize,
    pub local_retained_state_work: usize,
    pub discrepancy_retained_state_work: usize,
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
    pub policy_witness_proposal_rejections: usize,
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

    pub(super) fn new_with_guidance(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        Self::new_with_policy_proposal(session, options, true, guidance)
    }

    fn new_with_policy_proposal(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        offer_policy_proposal: bool,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        let prepared = prepare_search_combat(session, options)?;
        let max_transition_steps = prepared.config.max_engine_steps_per_action.max(1);
        let max_work = prepared.config.max_nodes;
        let (satisfaction, planner_satisfaction) = match prepared.config.satisfaction {
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::BudgetOrExhaustion => {
                (
                    PortfolioWitnessSatisfactionV1::BudgetOrExhaustion,
                    OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
                )
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::ZeroLossOrBudget => {
                (
                    PortfolioWitnessSatisfactionV1::HpLossAtMost(0),
                    OracleCombatWitnessSatisfaction::HpLossAtMost(0),
                )
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin => {
                (
                    PortfolioWitnessSatisfactionV1::FirstWitness,
                    OracleCombatWitnessSatisfaction::FirstWitness,
                )
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(limit) => {
                (
                    PortfolioWitnessSatisfactionV1::HpLossAtMost(limit),
                    OracleCombatWitnessSatisfaction::HpLossAtMost(limit),
                )
            }
            crate::ai::combat_search_v2::CombatSearchV2Satisfaction::PersistentRunValueGain => {
                (
                    PortfolioWitnessSatisfactionV1::PersistentRunValueGain,
                    OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
                )
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
        let policy = if let Some(guidance) = guidance {
            guidance.policy(policy)?
        } else {
            policy
        };
        let policy = combat_plan_state_guide_policy_v1(policy);
        let local_search = LocalTurnGraphWitnessSession::with_policy(
            root.clone(),
            LocalTurnGraphWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition: max_transition_steps,
                    allow_potion_expenditure: prepared.config.max_potions_used != Some(0),
                    ..TurnOptionGeneratorConfig::default()
                },
                generation_quantum_work: 4,
                backed_generation_quantum_work: 256,
                initial_expansion_work: 64,
                root_initial_expansion_work: 2_048,
                lookahead_max_evaluations: 384,
                lookahead_work_per_evaluation: 24,
                max_turn_depth: 32,
                satisfaction: planner_satisfaction,
                max_potions_used: prepared.config.max_potions_used,
            },
            policy.clone(),
        );
        let discrepancy_search = PolicyDiscrepancySession::with_policy(
            root,
            PolicyDiscrepancyConfig {
                max_engine_steps_per_transition: max_transition_steps,
                turn_macro: Some(PolicyDiscrepancyTurnMacroConfig {
                    max_applied_transitions: 4_096,
                    proposals_per_view: 8,
                    ..PolicyDiscrepancyTurnMacroConfig::default()
                }),
                max_potions_used: prepared.config.max_potions_used,
                ..PolicyDiscrepancyConfig::default()
            },
            policy,
        );
        let mut work = Self {
            start: prepared.start,
            local_search,
            discrepancy_search,
            next_portfolio_member: PortfolioMemberV1::LocalTurnGraph,
            local_complete: false,
            discrepancy_complete: false,
            remaining_work: max_work,
            remaining_engine_steps: max_work.saturating_mul(max_transition_steps),
            max_transition_steps,
            max_potions_used: prepared.config.max_potions_used,
            satisfaction,
            remaining_wall_time: prepared.config.wall_time,
            quantum_count: 0,
            prior_generation_work: 0,
            prior_policy_witness_proposals: 0,
            policy_witness_proposals: 0,
            policy_witness_replay_engine_steps: 0,
            policy_witness_proposal_rejections: 0,
            policy_witness: None,
            discrepancy_witness: None,
            restart_count: 0,
            last_status: None,
            local_status: None,
            discrepancy_status: None,
            incumbent_revision: 0,
            quanta_since_incumbent_improvement: 0,
            last_quantum_generation_work: 0,
            last_quantum_engine_steps: 0,
            search_resume_exact: false,
            witness_source: CombatAutomationTrajectorySource::SearchCombat,
        };
        if offer_policy_proposal {
            work.offer_initial_rollout_policy_proposal();
        }
        Ok(work)
    }

    pub(super) fn restart_from_checkpoint_with_guidance(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        checkpoint: OracleRunCombatWorkCheckpointV1,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        // A process restart restores the already charged incumbent and
        // allowance. Re-running a non-serialized policy proposal here would
        // repeatedly pay the same startup computation and obscure accounting.
        let mut work = Self::new_with_policy_proposal(session, options, false, guidance)?;
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
        work.prior_policy_witness_proposals = checkpoint.policy_witness_proposals;
        work.policy_witness_proposal_rejections = checkpoint.policy_witness_proposal_rejections;
        work.restart_count = checkpoint.restart_count.saturating_add(1);
        work.incumbent_revision = checkpoint.incumbent_revision;
        work.quanta_since_incumbent_improvement = checkpoint.quanta_since_incumbent_improvement;
        if let Some(incumbent) = checkpoint.incumbent {
            work.restore_checkpoint_incumbent(incumbent)?;
        }
        Ok(work)
    }

    /// Restarts an exact combat at a higher configured fidelity while
    /// preserving all charged work and any already verified incumbent. The
    /// tactical frontier itself is intentionally not serialized, so the
    /// restart is explicit in both accounting and diagnostics.
    pub(super) fn restart_for_higher_fidelity_with_guidance(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        prior: OracleRunCombatWorkCheckpointV1,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        let mut work = Self::new_with_policy_proposal(session, options, false, guidance)?;
        work.quantum_count = prior.quantum_count;
        work.prior_generation_work = prior.consumed_nodes;
        work.prior_policy_witness_proposals = prior.policy_witness_proposals;
        work.policy_witness_proposal_rejections = prior.policy_witness_proposal_rejections;
        work.restart_count = prior.restart_count.saturating_add(1);
        work.incumbent_revision = prior.incumbent_revision;
        work.quanta_since_incumbent_improvement = prior.quanta_since_incumbent_improvement;
        if let Some(incumbent) = prior.incumbent {
            work.restore_checkpoint_incumbent(incumbent)?;
        }
        Ok(work)
    }

    fn restore_checkpoint_incumbent(
        &mut self,
        incumbent: OracleCombatWitness,
    ) -> Result<(), String> {
        if incumbent.discovery_source == OracleCombatWitnessDiscoverySource::PolicyProposal {
            self.policy_witness = Some(incumbent);
            Ok(())
        } else if incumbent.discovery_source
            == OracleCombatWitnessDiscoverySource::PolicyDiscrepancySearch
        {
            self.discrepancy_witness = Some(incumbent);
            Ok(())
        } else {
            self.local_search.restore_verified_witness(incumbent)
        }
    }

    fn offer_initial_rollout_policy_proposal(&mut self) {
        const MAX_POLICY_ACTIONS: usize = 256;
        const POLICY_WALL_LIMIT: Duration = Duration::from_millis(100);

        let allowance = self
            .remaining_wall_time
            .map(|remaining| remaining.min(POLICY_WALL_LIMIT))
            .unwrap_or(POLICY_WALL_LIMIT);
        if allowance.is_zero() {
            return;
        }
        let started = Instant::now();
        let deadline = started.checked_add(allowance);
        let proposal_result = existing_combat_rollout_witness_v1(
            &self.start,
            MAX_POLICY_ACTIONS,
            self.max_transition_steps,
            deadline,
        );
        if let Some(remaining) = &mut self.remaining_wall_time {
            *remaining = remaining.saturating_sub(started.elapsed());
        }
        let Some(proposal) = (match proposal_result {
            Ok(proposal) => proposal,
            Err(_) => {
                self.policy_witness_proposal_rejections =
                    self.policy_witness_proposal_rejections.saturating_add(1);
                None
            }
        }) else {
            return;
        };
        if !combat_witness_within_potion_budget(&proposal, self.max_potions_used) {
            self.policy_witness_proposal_rejections =
                self.policy_witness_proposal_rejections.saturating_add(1);
            return;
        }
        self.policy_witness_proposals = self.policy_witness_proposals.saturating_add(1);
        let replay_steps = proposal.replay_engine_steps;
        self.policy_witness_replay_engine_steps = self
            .policy_witness_replay_engine_steps
            .saturating_add(replay_steps);
        self.remaining_engine_steps = self.remaining_engine_steps.saturating_sub(replay_steps);
        self.policy_witness = Some(proposal);
    }

    /// Restores a legacy exact combat state whose checkpoint did not preserve
    /// tactical allowance or incumbent information.  It must be reported as a
    /// search restart even though its allowance necessarily starts fresh.
    pub(super) fn restart_from_exact_state_with_guidance(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        let mut work = Self::new_with_guidance(session, options, guidance)?;
        work.restart_count = 1;
        Ok(work)
    }

    pub(super) fn for_exact_action_witness_with_guidance(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        // Exact analyst actions need simulator verification, not an implicit
        // rollout proposal that could replace the explicitly supplied line.
        Self::new_with_policy_proposal(session, options, false, guidance)
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
            policy_witness_proposals: self
                .prior_policy_witness_proposals
                .saturating_add(self.policy_witness_proposals),
            policy_witness_proposal_rejections: self.policy_witness_proposal_rejections,
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
        self.advance_with_witness_policy(quantum, global_deadline, true)
    }

    /// Continues serving the portfolio past an insufficient first witness, but
    /// honors the configured quality satisfaction instead of mechanically
    /// consuming the whole allowance. `BudgetOrExhaustion` remains available
    /// for an analyst who explicitly requests exhaustive bounded improvement.
    pub(super) fn advance_improving_incumbent(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        global_deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        self.advance_with_witness_policy(quantum, global_deadline, false)
    }

    fn advance_with_witness_policy(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        global_deadline: Option<Instant>,
        stop_on_first_witness: bool,
    ) -> RunControlCombatWorkAdvanceV1 {
        let now = Instant::now();
        let global_remaining =
            global_deadline.map(|deadline| deadline.saturating_duration_since(now));
        if global_remaining == Some(Duration::ZERO) {
            return RunControlCombatWorkAdvanceV1::GlobalDeadlineReached;
        }
        let work = quantum.additional_nodes.min(self.remaining_work);
        if work == 0 || wall_allowance_exhausted(self.remaining_wall_time) {
            return if self.best_witness().is_some() {
                RunControlCombatWorkAdvanceV1::ReadyToFinish
            } else {
                RunControlCombatWorkAdvanceV1::AllowanceExhausted
            };
        }
        let requested_wall = quantum.soft_wall_ms.map(Duration::from_millis);
        let soft_wall = [requested_wall, self.remaining_wall_time, global_remaining]
            .into_iter()
            .flatten()
            .min();
        if soft_wall == Some(Duration::ZERO) {
            return if global_remaining == Some(Duration::ZERO) {
                RunControlCombatWorkAdvanceV1::GlobalDeadlineReached
            } else if self.best_witness().is_some() {
                RunControlCombatWorkAdvanceV1::ReadyToFinish
            } else {
                RunControlCombatWorkAdvanceV1::AllowanceExhausted
            };
        }
        let deadline = soft_wall.and_then(|duration| now.checked_add(duration));
        self.last_quantum_generation_work = 0;
        self.last_quantum_engine_steps = 0;
        let before_incumbent_quality = self.best_witness().map(combat_witness_quality);
        let engine_grant = self
            .remaining_engine_steps
            .min(work.saturating_mul(self.max_transition_steps));
        let Some(member) = select_portfolio_member(
            self.next_portfolio_member,
            self.local_complete,
            self.discrepancy_complete,
        ) else {
            return RunControlCombatWorkAdvanceV1::ReadyToFinish;
        };
        self.next_portfolio_member = member.other();
        let (consumed_work, consumed_engine, member_complete, status) = match member {
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
                let before_work = before.generation_work.saturating_add(before.lookahead_work);
                let after_work = after.generation_work.saturating_add(after.lookahead_work);
                let member_complete =
                    !matches!(&report.status, LocalTurnGraphWitnessStatus::Partial(_));
                (
                    after_work.saturating_sub(before_work),
                    after.engine_steps.saturating_sub(before.engine_steps),
                    member_complete,
                    PortfolioStatusV1::Local(report.status),
                )
            }
            PortfolioMemberV1::PolicyDiscrepancy => {
                let before = self.discrepancy_search.counters();
                let report = self.discrepancy_search.advance(
                    &crate::sim::combat::EngineCombatStepper,
                    PolicyDiscrepancyQuantum {
                        additional_applied_transitions: work,
                        additional_engine_steps: engine_grant,
                        deadline,
                    },
                );
                let after = report.after;
                let status = report.status;
                if let Some(witness) = report.witness {
                    let witness = OracleCombatWitness {
                        actions: witness.actions,
                        final_position: witness.final_position,
                        negative_log_policy: witness.negative_log_policy,
                        replay_engine_steps: witness.replay_engine_steps,
                        discovery_source:
                            OracleCombatWitnessDiscoverySource::PolicyDiscrepancySearch,
                    };
                    if self
                        .discrepancy_witness
                        .as_ref()
                        .is_none_or(|current| combat_witness_better(&witness, current))
                    {
                        self.discrepancy_witness = Some(witness);
                    }
                }
                let member_complete = !matches!(status, PolicyDiscrepancyStatus::Partial(_));
                (
                    after
                        .applied_action_transitions
                        .saturating_sub(before.applied_action_transitions),
                    after.engine_steps.saturating_sub(before.engine_steps),
                    member_complete,
                    PortfolioStatusV1::PolicyDiscrepancy(status),
                )
            }
        };
        match &status {
            PortfolioStatusV1::Local(status) => {
                self.local_complete = member_complete;
                self.local_status = Some(status.clone());
            }
            PortfolioStatusV1::PolicyDiscrepancy(status) => {
                self.discrepancy_complete = member_complete;
                self.discrepancy_status = Some(status.clone());
            }
        }
        self.last_quantum_generation_work = consumed_work;
        self.last_quantum_engine_steps = consumed_engine;
        let after_incumbent_quality = self.best_witness().map(combat_witness_quality);
        let incumbent_improved = match (before_incumbent_quality, after_incumbent_quality) {
            (None, Some(_)) => true,
            (Some(before), Some(after)) => combat_witness_quality_better(after, before),
            _ => false,
        };
        let acceptance_improved =
            combat_witness_acceptance_improved(before_incumbent_quality, after_incumbent_quality);
        if incumbent_improved {
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
        // A verified policy line is a fallback, not an instant terminal
        // signal. Give the independent local graph one complete caller-sized
        // work quantum to challenge it. If that bounded challenge cannot
        // improve HP, commit the exact fallback instead of spending the
        // encounter's entire wall allowance proving that no improvement
        // exists.
        let fallback_challenge_complete = stop_on_first_witness
            && self.policy_witness.is_some()
            && self.current_local_search_work() >= quantum.additional_nodes;
        let quality_satisfied = !stop_on_first_witness
            && self.best_witness().is_some_and(|witness| {
                combat_witness_satisfies(self.satisfaction, &self.start, witness)
            });
        let quality_challenge_complete = self.best_witness().is_none_or(|witness| {
            witness.discovery_source != OracleCombatWitnessDiscoverySource::PolicyProposal
        }) || self.current_local_search_work()
            >= quantum.additional_nodes
            || self.local_complete;
        if (stop_on_first_witness && acceptance_improved)
            || fallback_challenge_complete
            || (quality_satisfied && quality_challenge_complete)
            || (self.local_complete && self.discrepancy_complete)
        {
            RunControlCombatWorkAdvanceV1::ReadyToFinish
        } else if self.remaining_work == 0
            || self.remaining_engine_steps == 0
            || wall_allowance_exhausted(self.remaining_wall_time)
        {
            if self.best_witness().is_some() {
                RunControlCombatWorkAdvanceV1::ReadyToFinish
            } else {
                RunControlCombatWorkAdvanceV1::AllowanceExhausted
            }
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

    pub(super) fn has_quality_satisfying_witness(&self) -> bool {
        self.best_witness().is_some_and(|witness| {
            combat_witness_satisfies(self.satisfaction, &self.start, witness)
        })
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
                expected_successor_hash: crate::ai::combat_state_key::combat_exact_state_hash_v2(
                    &result.position.engine,
                    &result.position.combat,
                )
                .into(),
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
        if !combat_witness_within_potion_budget(&witness, self.max_potions_used) {
            return Err(format!(
                "oracle combat witness uses {} potion(s), exceeding configured limit {:?}",
                combat_witness_potion_expenditures(&witness),
                self.max_potions_used
            ));
        }
        self.local_search.restore_verified_witness(witness)?;
        self.witness_source = CombatAutomationTrajectorySource::OracleExactActions;
        Ok(())
    }

    pub(super) fn nodes_expanded(&self) -> u64 {
        self.prior_generation_work
            .saturating_add(self.current_generation_work())
    }

    fn current_generation_work(&self) -> u64 {
        (self.current_local_search_work() as u64).saturating_add(
            self.discrepancy_search
                .counters()
                .applied_action_transitions as u64,
        )
    }

    fn current_local_search_work(&self) -> usize {
        let local = self.local_search.counters();
        local.generation_work.saturating_add(local.lookahead_work)
    }

    fn best_witness(&self) -> Option<&OracleCombatWitness> {
        [
            self.local_search.witness(),
            self.discrepancy_witness.as_ref(),
            self.policy_witness.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|witness| combat_witness_within_potion_budget(witness, self.max_potions_used))
        .reduce(|best, candidate| {
            if combat_witness_better(candidate, best) {
                candidate
            } else {
                best
            }
        })
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

    pub(super) fn max_potions_used(&self) -> Option<u32> {
        self.max_potions_used
    }

    pub(super) fn restart_count(&self) -> usize {
        self.restart_count
    }

    pub(super) fn progress(&self) -> OracleRunCombatWorkProgressV1 {
        let local_counters = self.local_search.counters();
        let discrepancy_counters = self.discrepancy_search.counters();
        let local_progress = self.local_search.progress_snapshot();
        let search_progress = &local_progress;
        let initial_hp = self.start.combat.entities.player.current_hp;
        let incumbent = self.best_witness();
        let incumbent_final_hp =
            incumbent.map(|witness| witness.final_position.combat.entities.player.current_hp);
        let local_generation_work = self.current_local_search_work() as u64;
        let discrepancy_generation_work = discrepancy_counters.applied_action_transitions as u64;
        let current_generation_work =
            local_generation_work.saturating_add(discrepancy_generation_work);
        let local_retained_state_work = self.local_search.retained_state_work();
        let discrepancy_retained_state_work = self.discrepancy_search.retained_state_work();
        OracleRunCombatWorkProgressV1 {
            historical_generation_work: self.prior_generation_work,
            current_search_generation_work: current_generation_work,
            generation_work: self
                .prior_generation_work
                .saturating_add(current_generation_work),
            local_generation_work,
            discrepancy_generation_work,
            lookahead_evaluations: local_counters.lookahead_evaluations,
            lookahead_work: local_counters.lookahead_work,
            engine_steps: local_counters
                .engine_steps
                .saturating_add(discrepancy_counters.engine_steps)
                .saturating_add(self.policy_witness_replay_engine_steps),
            exact_states: local_counters
                .exact_nodes
                .saturating_add(discrepancy_counters.exact_states),
            local_exact_states: local_counters.exact_nodes,
            discrepancy_exact_states: discrepancy_counters.exact_states,
            applied_action_transitions: local_counters
                .applied_action_transitions
                .saturating_add(discrepancy_counters.applied_action_transitions),
            unique_successor_states: local_counters.unique_successor_states,
            duplicate_exact_successors: local_counters
                .duplicate_exact_successors
                .saturating_add(discrepancy_counters.duplicate_or_dominated_states),
            completed_turn_options: local_counters
                .completed_turn_options
                .saturating_add(discrepancy_counters.turn_macro_options_generated),
            retained_state_work: local_retained_state_work
                .saturating_add(discrepancy_retained_state_work),
            local_retained_state_work,
            discrepancy_retained_state_work,
            queued_anchor_entries: local_progress
                .queued_anchor_entries
                .saturating_add(self.discrepancy_search.frontier_entries()),
            queued_guided_entries: local_progress.queued_guided_entries.clone(),
            root_state: search_progress.root_state.clone(),
            max_player_turn: local_progress.max_player_turn,
            deepest_survival_state: search_progress.deepest_survival_state.clone(),
            deepest_progress_state: search_progress.deepest_progress_state.clone(),
            deepest_survival_actions: search_progress.deepest_survival_actions.clone(),
            deepest_progress_actions: search_progress.deepest_progress_actions.clone(),
            recent_turn_survival_envelope: search_progress.recent_turn_survival_envelope.clone(),
            max_path_atomic_depth: local_progress.max_path_atomic_depth,
            max_completed_turn_options_at_state: local_progress.max_completed_turn_options_at_state,
            generation_gap_count: local_progress
                .generation_gap_count
                .saturating_add(discrepancy_counters.transition_step_limit_gaps),
            pending_witness_replay: local_progress.pending_witness_replay,
            policy_witness_proposals: self
                .policy_witness_proposals
                .saturating_add(self.prior_policy_witness_proposals),
            policy_witness_proposal_rejections: self.policy_witness_proposal_rejections,
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
                self.discrepancy_status.as_ref(),
                self.last_status.as_ref(),
            ),
        }
    }

    pub(super) fn finish_and_apply(
        &self,
        session: &mut RunControlSession,
    ) -> Result<RunProgressOutcome, String> {
        if session.current_active_combat_position()? != self.start {
            return Err("oracle combat parent changed before search commit".to_string());
        }
        if let Some(witness) = self.best_witness() {
            let source = match witness.discovery_source {
                OracleCombatWitnessDiscoverySource::PolicyProposal => {
                    CombatAutomationTrajectorySource::MaturePolicyProposal
                }
                OracleCombatWitnessDiscoverySource::PlannerSearch => {
                    CombatAutomationTrajectorySource::SearchCombat
                }
                OracleCombatWitnessDiscoverySource::PolicyDiscrepancySearch => {
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
            "Combat-search portfolio did not modify state. status={status:?} generation_work={} local_work={} discrepancy_work={} exact_states={} retained_work={}",
            self.prior_generation_work
                .saturating_add(self.current_generation_work()),
            self.local_search.counters().generation_work,
            self.discrepancy_search
                .counters()
                .applied_action_transitions,
            self.local_search
                .counters()
                .exact_nodes
                .saturating_add(self.discrepancy_search.counters().exact_states),
            self.local_search
                .retained_state_work()
                .saturating_add(self.discrepancy_search.retained_state_work()),
        ))
        .with_combat_search_rejection(
            RunControlCombatSearchRejection::NoCompleteWinningCandidate,
        ))
    }
}

#[derive(Clone, Copy)]
struct CombatWitnessQualityV1 {
    persistent_adjusted_hp: i32,
    final_hp: i32,
    persistent_run_value: i32,
    potions_used: u32,
    action_count: usize,
    negative_log_policy: f64,
}

fn combat_witness_quality(witness: &OracleCombatWitness) -> CombatWitnessQualityV1 {
    let final_hp = witness.final_position.combat.entities.player.current_hp;
    let persistent_run_value =
        crate::ai::combat_search_v2::persistent_run_value(&witness.final_position.combat);
    CombatWitnessQualityV1 {
        persistent_adjusted_hp: final_hp.saturating_add(persistent_run_value),
        final_hp,
        persistent_run_value,
        potions_used: combat_witness_potion_expenditures(witness),
        action_count: witness.actions.len(),
        negative_log_policy: witness.negative_log_policy,
    }
}

fn combat_witness_quality_better(
    left: CombatWitnessQualityV1,
    right: CombatWitnessQualityV1,
) -> bool {
    left.persistent_adjusted_hp
        .cmp(&right.persistent_adjusted_hp)
        .then_with(|| left.final_hp.cmp(&right.final_hp))
        .then_with(|| left.persistent_run_value.cmp(&right.persistent_run_value))
        .then_with(|| right.potions_used.cmp(&left.potions_used))
        .then_with(|| right.action_count.cmp(&left.action_count))
        .then_with(|| {
            right
                .negative_log_policy
                .total_cmp(&left.negative_log_policy)
        })
        == std::cmp::Ordering::Greater
}

fn combat_witness_acceptance_improved(
    before: Option<CombatWitnessQualityV1>,
    after: Option<CombatWitnessQualityV1>,
) -> bool {
    match (before, after) {
        (None, Some(_)) => true,
        (Some(before), Some(after)) => {
            after.persistent_adjusted_hp > before.persistent_adjusted_hp
                || (after.persistent_adjusted_hp == before.persistent_adjusted_hp
                    && (after.final_hp > before.final_hp
                        || (after.final_hp == before.final_hp
                            && after.potions_used < before.potions_used)))
        }
        _ => false,
    }
}

fn combat_witness_better(left: &OracleCombatWitness, right: &OracleCombatWitness) -> bool {
    combat_witness_quality_better(combat_witness_quality(left), combat_witness_quality(right))
}

fn combat_witness_potion_expenditures(witness: &OracleCombatWitness) -> u32 {
    witness
        .actions
        .iter()
        .filter(|action| {
            matches!(
                action.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            )
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn combat_witness_within_potion_budget(
    witness: &OracleCombatWitness,
    max_potions_used: Option<u32>,
) -> bool {
    max_potions_used.is_none_or(|limit| combat_witness_potion_expenditures(witness) <= limit)
}

fn combat_witness_satisfies(
    satisfaction: PortfolioWitnessSatisfactionV1,
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
) -> bool {
    match satisfaction {
        PortfolioWitnessSatisfactionV1::FirstWitness => true,
        PortfolioWitnessSatisfactionV1::HpLossAtMost(limit) => {
            let initial_hp = start.combat.entities.player.current_hp;
            let final_hp = witness.final_position.combat.entities.player.current_hp;
            initial_hp.saturating_sub(final_hp).max(0) as u32 <= limit
        }
        PortfolioWitnessSatisfactionV1::PersistentRunValueGain => {
            crate::ai::combat_search_v2::persistent_run_value(&witness.final_position.combat)
                > crate::ai::combat_search_v2::persistent_run_value(&start.combat)
        }
        PortfolioWitnessSatisfactionV1::BudgetOrExhaustion => false,
    }
}

fn wall_allowance_exhausted(remaining: Option<Duration>) -> bool {
    remaining.is_some_and(|duration| duration < MIN_USABLE_WALL_ALLOWANCE)
}

fn select_portfolio_member(
    next: PortfolioMemberV1,
    local_complete: bool,
    discrepancy_complete: bool,
) -> Option<PortfolioMemberV1> {
    match (next, local_complete, discrepancy_complete) {
        (_, true, true) => None,
        (PortfolioMemberV1::LocalTurnGraph, false, _)
        | (PortfolioMemberV1::PolicyDiscrepancy, false, true) => {
            Some(PortfolioMemberV1::LocalTurnGraph)
        }
        (PortfolioMemberV1::PolicyDiscrepancy, _, false)
        | (PortfolioMemberV1::LocalTurnGraph, true, false) => {
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hallway_combat_session() -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        let plan = crate::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &jaw_worm,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&jaw_worm),
            &[],
        );
        jaw_worm.set_planned_move_id(plan.move_id);
        jaw_worm.set_planned_steps(plan.steps);
        jaw_worm.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters = vec![jaw_worm];
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

    fn one_strike_win_session() -> RunControlSession {
        let mut session = hallway_combat_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.turn.energy = 1;
        combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Strike,
            1,
        )];
        combat.entities.monsters[0].current_hp = 1;
        combat.entities.monsters[0].max_hp = 1;
        session
    }

    #[test]
    fn exact_restored_witness_cannot_bypass_zero_potion_contract() {
        let mut session = one_strike_win_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        let target = combat.entities.monsters[0].id;
        combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FirePotion,
            7,
        ))];
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                max_potions_used: Some(0),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("zero-potion portfolio should accept an active combat");

        let error = work
            .verify_and_restore_action_witness(&[ClientInput::UsePotion {
                potion_index: 0,
                target: Some(target),
            }])
            .expect_err("an exact potion win must not bypass the configured resource contract");
        assert!(error.contains("exceeding configured limit"));
        assert!(work
            .best_witness()
            .is_none_or(|witness| { combat_witness_potion_expenditures(witness) == 0 }));
    }

    #[test]
    fn sub_millisecond_wall_tail_is_not_treated_as_usable_allowance() {
        assert!(wall_allowance_exhausted(Some(Duration::from_micros(999))));
        assert!(!wall_allowance_exhausted(Some(Duration::from_millis(1))));
        assert!(!wall_allowance_exhausted(None));
    }

    #[test]
    fn portfolio_alternates_live_members_and_skips_completed_members() {
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::LocalTurnGraph, false, false),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::PolicyDiscrepancy, false, false),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::LocalTurnGraph, true, false),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
        assert_eq!(
            select_portfolio_member(PortfolioMemberV1::PolicyDiscrepancy, false, true),
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
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
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
        assert_eq!(
            work.discrepancy_search
                .counters()
                .applied_action_transitions,
            0
        );
        assert_eq!(work.remaining_work, 15);

        assert_eq!(
            work.advance(&quantum, None),
            RunControlCombatWorkAdvanceV1::Pending
        );
        assert_eq!(work.local_search.counters().generation_work, 1);
        assert_eq!(
            work.discrepancy_search
                .counters()
                .applied_action_transitions,
            1
        );
        assert_eq!(work.remaining_work, 14);
    }

    #[test]
    fn policy_fallback_does_not_precomplete_the_independent_local_search() {
        let session = one_strike_win_session();
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("one-strike combat should create a portfolio");

        assert!(work.policy_witness.is_some());
        assert_eq!(work.policy_witness_proposals, 1);
        assert!(
            work.local_search.witness().is_none(),
            "a verified fallback must not masquerade as a local-search result"
        );

        let result = work.advance(
            &RunControlCombatSearchQuantum {
                label: "independent_local_search_contract",
                additional_nodes: 1,
                soft_wall_ms: None,
            },
            None,
        );
        assert!(
            work.local_search.counters().generation_work > 0,
            "the local graph must receive real work despite the fallback witness"
        );
        assert_eq!(
            result,
            RunControlCombatWorkAdvanceV1::ReadyToFinish,
            "after one complete caller-sized challenge, the exact fallback may finish"
        );
    }

    #[test]
    fn quality_mode_honors_satisfaction_after_an_independent_challenge() {
        let session = one_strike_win_session();
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(0),
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("one-strike combat should create a portfolio");

        assert!(work.policy_witness.is_some());
        let result = work.advance_improving_incumbent(
            &RunControlCombatSearchQuantum {
                label: "quality_satisfaction_contract",
                additional_nodes: 1,
                soft_wall_ms: None,
            },
            None,
        );

        assert!(
            work.local_search.counters().generation_work > 0,
            "quality acceptance must not let a policy proposal bypass independent search"
        );
        assert_eq!(result, RunControlCombatWorkAdvanceV1::ReadyToFinish);
    }

    #[test]
    fn explicit_budget_quality_mode_does_not_invent_an_early_threshold() {
        let session = one_strike_win_session();
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::BudgetOrExhaustion,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("one-strike combat should create a portfolio");

        let result = work.advance_improving_incumbent(
            &RunControlCombatSearchQuantum {
                label: "explicit_budget_contract",
                additional_nodes: 1,
                soft_wall_ms: None,
            },
            None,
        );

        assert_eq!(result, RunControlCombatWorkAdvanceV1::Pending);
    }

    #[test]
    fn persistent_payoff_satisfaction_requires_materialized_run_value() {
        let session = one_strike_win_session();
        let work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::PersistentRunValueGain,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("one-strike combat should create a portfolio");
        let plain = work
            .policy_witness
            .clone()
            .expect("policy should provide an exact winning line");
        let mut profitable = plain.clone();
        profitable
            .final_position
            .combat
            .entities
            .player
            .gold_delta_this_combat = 20;

        assert!(!combat_witness_satisfies(
            PortfolioWitnessSatisfactionV1::PersistentRunValueGain,
            &work.start,
            &plain
        ));
        assert!(combat_witness_satisfies(
            PortfolioWitnessSatisfactionV1::PersistentRunValueGain,
            &work.start,
            &profitable
        ));
    }

    #[test]
    fn quality_satisfaction_rejects_a_verified_but_over_budget_win() {
        let session = one_strike_win_session();
        let work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(8),
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("one-strike combat should create a portfolio");
        let mut poor = work
            .policy_witness
            .clone()
            .expect("policy should provide an exact winning line");
        poor.final_position.combat.entities.player.current_hp = work
            .start
            .combat
            .entities
            .player
            .current_hp
            .saturating_sub(20);

        assert!(!combat_witness_satisfies(
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            &poor
        ));
    }

    #[test]
    fn equal_hp_search_result_does_not_end_fallback_improvement() {
        let fallback = CombatWitnessQualityV1 {
            persistent_adjusted_hp: 128,
            final_hp: 56,
            persistent_run_value: 72,
            potions_used: 0,
            action_count: 33,
            negative_log_policy: 33.0,
        };
        let equal_hp_search_result = CombatWitnessQualityV1 {
            persistent_adjusted_hp: 128,
            final_hp: 56,
            persistent_run_value: 72,
            potions_used: 0,
            action_count: 30,
            negative_log_policy: 5.0,
        };

        assert!(combat_witness_quality_better(
            equal_hp_search_result,
            fallback
        ));
        assert!(!combat_witness_acceptance_improved(
            Some(fallback),
            Some(equal_hp_search_result)
        ));
    }

    #[test]
    fn portfolio_compares_persistent_payoff_without_ignoring_combat_hp() {
        let ordinary_lethal = CombatWitnessQualityV1 {
            persistent_adjusted_hp: 85,
            final_hp: 13,
            persistent_run_value: 72,
            potions_used: 0,
            action_count: 30,
            negative_log_policy: 3.0,
        };
        let profitable_lethal = CombatWitnessQualityV1 {
            persistent_adjusted_hp: 87,
            final_hp: 11,
            persistent_run_value: 76,
            potions_used: 0,
            action_count: 31,
            negative_log_policy: 4.0,
        };
        let reckless_profitable_lethal = CombatWitnessQualityV1 {
            persistent_adjusted_hp: 81,
            final_hp: 5,
            persistent_run_value: 76,
            potions_used: 0,
            action_count: 29,
            negative_log_policy: 2.0,
        };

        assert!(combat_witness_quality_better(
            profitable_lethal,
            ordinary_lethal
        ));
        assert!(combat_witness_acceptance_improved(
            Some(ordinary_lethal),
            Some(profitable_lethal)
        ));
        assert!(
            !combat_witness_quality_better(reckless_profitable_lethal, ordinary_lethal),
            "persistent payoff is run value, not permission to discard more HP than it gains"
        );
    }

    #[test]
    fn production_local_graph_does_not_run_rollout_lookahead() {
        let session = hallway_combat_session();
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(4_096),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::BudgetOrExhaustion,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("portfolio should accept an active combat");
        let quantum = RunControlCombatSearchQuantum {
            label: "lookahead_accounting_contract",
            additional_nodes: 4_096,
            soft_wall_ms: None,
        };

        let before_remaining = work.remaining_work;
        let _ = work.advance(&quantum, None);
        let counters = work.local_search.counters();
        assert_eq!(
            counters.lookahead_work, 0,
            "rollout lookahead remains a laboratory control rather than hidden production work"
        );
        assert_eq!(
            work.discrepancy_search
                .counters()
                .applied_action_transitions,
            0
        );
        assert_eq!(
            before_remaining.saturating_sub(work.remaining_work),
            counters.generation_work
        );
    }

    #[test]
    fn portfolio_preserves_stable_overall_status_semantics() {
        assert_eq!(
            overall_status_label(
                false,
                Some(&LocalTurnGraphWitnessStatus::FrontierExhausted),
                Some(&PolicyDiscrepancyStatus::FrontierExhausted),
                None,
            ),
            Some("frontier_exhausted")
        );
        assert_eq!(
            overall_status_label(
                false,
                Some(&LocalTurnGraphWitnessStatus::MechanicsGap),
                Some(&PolicyDiscrepancyStatus::FrontierExhausted),
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

fn discrepancy_status_label(status: &PolicyDiscrepancyStatus) -> &'static str {
    match status {
        PolicyDiscrepancyStatus::WitnessFound => "witness_found",
        PolicyDiscrepancyStatus::Partial(_) => "partial",
        PolicyDiscrepancyStatus::FrontierExhausted => "frontier_exhausted",
        PolicyDiscrepancyStatus::ReplayMismatch => "replay_mismatch",
    }
}

fn portfolio_status_label(status: &PortfolioStatusV1) -> &'static str {
    match status {
        PortfolioStatusV1::Local(status) => local_witness_status_label(status),
        PortfolioStatusV1::PolicyDiscrepancy(status) => discrepancy_status_label(status),
    }
}

fn overall_status_label(
    has_witness: bool,
    local: Option<&LocalTurnGraphWitnessStatus>,
    discrepancy: Option<&PolicyDiscrepancyStatus>,
    last: Option<&PortfolioStatusV1>,
) -> Option<&'static str> {
    if has_witness {
        return Some("witness_found");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::FrontierExhausted))
        && matches!(
            discrepancy,
            Some(PolicyDiscrepancyStatus::FrontierExhausted)
        )
    {
        return Some("frontier_exhausted");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::ReplayMismatch(_)))
        || matches!(discrepancy, Some(PolicyDiscrepancyStatus::ReplayMismatch))
    {
        return Some("replay_mismatch");
    }
    if matches!(local, Some(LocalTurnGraphWitnessStatus::MechanicsGap)) {
        return Some("mechanics_gap");
    }
    last.map(portfolio_status_label)
}
