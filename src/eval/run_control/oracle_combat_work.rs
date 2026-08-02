use std::sync::Arc;
use std::time::{Duration, Instant};

const MIN_USABLE_WALL_ALLOWANCE: Duration = Duration::from_millis(1);

use super::combat_line_executor::apply_oracle_combat_witness;
use super::combat_search::RunControlCombatWorkAdvanceV1;
use super::combat_search_setup::prepare_search_combat;
use super::oracle_combat_policy::{
    authorized_potion_trial_policy_v1, existing_combat_rollout_witness_v1,
    ExistingCombatKnowledgePolicy,
};
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlCombatSearchRejection, RunControlSession, RunProgressOutcome};
use super::trace_annotation::CombatAutomationTrajectorySource;
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::state::core::ClientInput;
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

pub(super) struct OracleRunCombatWorkV1 {
    start: crate::sim::combat::CombatPosition,
    local_search: LocalTurnGraphWitnessSession,
    discrepancy_search: PolicyDiscrepancySession,
    portfolio_service_order: PortfolioServiceOrderV1,
    next_portfolio_member: PortfolioMemberV1,
    local_complete: bool,
    discrepancy_complete: bool,
    remaining_work: usize,
    remaining_engine_steps: usize,
    max_transition_steps: usize,
    max_potions_used: Option<u32>,
    allowed_potion_slots: Option<u64>,
    allow_potion_discard: bool,
    potion_spend_requires_satisfaction: bool,
    protected_potion_free_incumbent: Option<OracleCombatWitness>,
    prior_stage_incumbent: Option<OracleCombatWitness>,
    stage_entry_incumbent: Option<OracleCombatWitness>,
    satisfaction: PortfolioWitnessSatisfactionV1,
    remaining_wall_time: Option<Duration>,
    quantum_count: usize,
    prior_generation_work: u64,
    prior_policy_witness_proposals: usize,
    policy_witness_proposals: usize,
    policy_witness_replay_engine_steps: usize,
    policy_witness_proposal_rejections: usize,
    plan_prefix_proposals: usize,
    plan_prefix_proposed_turns: usize,
    plan_prefix_proposed_actions: usize,
    plan_prefix_proposal_rejections: usize,
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
enum PortfolioServiceOrderV1 {
    RoundRobin,
    LocalPrimary,
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
    /// Distinguishes a newly written exact potion contract from legacy
    /// checkpoints where absent fields meant "unknown, reconstruct".
    #[serde(default)]
    pub potion_contract_recorded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_potions_used: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_potion_slots: Option<u64>,
    /// When true, a verified potion-free incumbent is protected from a
    /// higher-HP spending line that still misses the configured satisfaction.
    #[serde(default)]
    pub potion_spend_requires_satisfaction: bool,
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
    pub root_exact_state_hash: String,
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
    pub current_policy_witness_proposals: usize,
    pub current_policy_witness_proposal_rejections: usize,
    pub policy_witness_proposals: usize,
    pub policy_witness_proposal_rejections: usize,
    pub plan_prefix_proposals: usize,
    pub plan_prefix_proposed_turns: usize,
    pub plan_prefix_proposed_actions: usize,
    pub plan_prefix_proposal_rejections: usize,
    pub advisor_nodes: u64,
    pub advisor_elapsed_ms: u64,
    pub advisor_active: bool,
    pub advisor_failure: Option<String>,
    pub incumbent_discovery_source: Option<OracleCombatWitnessDiscoverySource>,
    pub incumbent_final_hp: Option<i32>,
    pub incumbent_hp_loss: Option<i32>,
    pub incumbent_action_count: Option<usize>,
    pub incumbent_potions_used: Option<u32>,
    pub incumbent_potion_slots: Option<u64>,
    pub incumbent_satisfies_satisfaction: Option<bool>,
    pub incumbent_ends_quality_refinement: Option<bool>,
    pub potion_spend_requires_satisfaction: bool,
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
        let allow_potion_discard = matches!(
            prepared.config.potion_policy,
            crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::All
        );
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
            | crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(_)
            | crate::ai::combat_search_v2::CombatSearchV2Satisfaction::PotionFreeHpLossAtMostWithoutNewExternalBurden(_) => {
                return Err("oracle witness search does not yet own external-burden acceptance"
                    .to_string());
            }
        };
        let root = CombatDecisionRoot::new(prepared.start.clone())
            .map_err(|error| format!("invalid oracle combat root: {error:?}"))?;
        let policy = Arc::new(ExistingCombatKnowledgePolicy::default());
        let mut policy = if let Some(guidance) = guidance {
            guidance.policy(policy)?
        } else {
            policy
        };
        if let Some(allowed_potion_slots) = prepared
            .options
            .allowed_potion_slots
            .filter(|slots| *slots != 0)
            .filter(|_| prepared.config.max_potions_used != Some(0))
        {
            policy = authorized_potion_trial_policy_v1(
                policy,
                prepared.start.clone(),
                allowed_potion_slots,
            );
        }
        let policy = combat_plan_state_guide_policy_v1(policy);
        let portfolio_service_order = if prepared.start.combat.meta.is_boss_fight {
            PortfolioServiceOrderV1::LocalPrimary
        } else {
            PortfolioServiceOrderV1::RoundRobin
        };
        let local_search = LocalTurnGraphWitnessSession::with_policy(
            root.clone(),
            LocalTurnGraphWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition: max_transition_steps,
                    allow_potion_expenditure: prepared.config.max_potions_used != Some(0),
                    allow_potion_discard,
                    allowed_potion_slots: prepared.options.allowed_potion_slots,
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
                allow_potion_discard,
                allowed_potion_slots: prepared.options.allowed_potion_slots,
                ..PolicyDiscrepancyConfig::default()
            },
            policy,
        );
        let mut work = Self {
            start: prepared.start,
            local_search,
            discrepancy_search,
            portfolio_service_order,
            next_portfolio_member: PortfolioMemberV1::LocalTurnGraph,
            local_complete: false,
            discrepancy_complete: false,
            remaining_work: max_work,
            remaining_engine_steps: max_work.saturating_mul(max_transition_steps),
            max_transition_steps,
            max_potions_used: prepared.config.max_potions_used,
            allowed_potion_slots: prepared.options.allowed_potion_slots,
            allow_potion_discard,
            potion_spend_requires_satisfaction: false,
            protected_potion_free_incumbent: None,
            prior_stage_incumbent: None,
            stage_entry_incumbent: None,
            satisfaction,
            remaining_wall_time: prepared.config.wall_time,
            quantum_count: 0,
            prior_generation_work: 0,
            prior_policy_witness_proposals: 0,
            policy_witness_proposals: 0,
            policy_witness_replay_engine_steps: 0,
            policy_witness_proposal_rejections: 0,
            plan_prefix_proposals: 0,
            plan_prefix_proposed_turns: 0,
            plan_prefix_proposed_actions: 0,
            plan_prefix_proposal_rejections: 0,
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
        work.offer_initial_plan_prefix();
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
        work.potion_spend_requires_satisfaction = checkpoint.potion_spend_requires_satisfaction;
        if work.potion_spend_requires_satisfaction {
            work.protected_potion_free_incumbent = checkpoint
                .incumbent
                .as_ref()
                .filter(|incumbent| combat_witness_potion_expenditures(&work.start, incumbent) == 0)
                .cloned();
        }
        work.stage_entry_incumbent = checkpoint.incumbent.clone();
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
        let expands_potion_contract = prior.max_potions_used == Some(0)
            && work.max_potions_used.is_some_and(|limit| limit > 0);
        let protected_incumbent = prior
            .incumbent
            .as_ref()
            .filter(|incumbent| combat_witness_potion_expenditures(&work.start, incumbent) == 0)
            .cloned();
        work.potion_spend_requires_satisfaction = prior.potion_spend_requires_satisfaction
            || (expands_potion_contract && protected_incumbent.is_some());
        if work.potion_spend_requires_satisfaction {
            work.protected_potion_free_incumbent = protected_incumbent;
        }
        work.stage_entry_incumbent = prior.incumbent.clone();
        if let Some(incumbent) = prior.incumbent {
            work.restore_checkpoint_incumbent(incumbent)?;
        }
        Ok(work)
    }

    fn restore_checkpoint_incumbent(
        &mut self,
        incumbent: OracleCombatWitness,
    ) -> Result<(), String> {
        self.verify_checkpoint_incumbent(&incumbent)?;
        if !combat_witness_within_potion_contract(
            &self.start,
            &incumbent,
            self.max_potions_used,
            self.allowed_potion_slots,
        ) {
            self.prior_stage_incumbent = Some(incumbent);
            return Ok(());
        }
        if incumbent.discovery_source == OracleCombatWitnessDiscoverySource::PolicyProposal {
            self.policy_witness = Some(incumbent);
        } else if incumbent.discovery_source
            == OracleCombatWitnessDiscoverySource::PolicyDiscrepancySearch
        {
            self.discrepancy_witness = Some(incumbent);
        } else {
            self.local_search.restore_verified_witness(incumbent)?;
        }
        self.prior_stage_incumbent = None;
        Ok(())
    }

    fn verify_checkpoint_incumbent(&self, incumbent: &OracleCombatWitness) -> Result<(), String> {
        use crate::sim::combat::CombatStepper;

        if incumbent.actions.is_empty() {
            return Err("checkpoint incumbent contains no combat actions".to_string());
        }
        let stepper = crate::sim::combat::EngineCombatStepper;
        let mut position = self.start.clone();
        for (index, action) in incumbent.actions.iter().enumerate() {
            if stepper
                .choice_for_legal_input(&position, &action.input)
                .is_none()
            {
                return Err(format!(
                    "checkpoint incumbent action {index} is not legal at its exact state"
                ));
            }
            let result = stepper.apply_to_stable(
                &position,
                action.input.clone(),
                crate::sim::combat::CombatStepLimits {
                    max_engine_steps: self.max_transition_steps,
                    deadline: None,
                },
            );
            if result.truncated {
                return Err(format!(
                    "checkpoint incumbent action {index} exceeded the transition limit"
                ));
            }
            position = result.position;
        }
        if position != incumbent.final_position {
            return Err(
                "checkpoint incumbent final position does not match its exact replay".to_string(),
            );
        }
        if position.combat.runtime.combat_smoked {
            return Err(
                "checkpoint incumbent is a Smoke Bomb escape, not a terminal victory".to_string(),
            );
        }
        if crate::sim::combat::combat_terminal(&position.engine, &position.combat)
            != crate::sim::combat::CombatTerminal::Win
        {
            return Err("checkpoint incumbent exact replay is not terminal victory".to_string());
        }
        Ok(())
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
            self.max_potions_used,
            self.allowed_potion_slots,
        );
        if let Some(remaining) = &mut self.remaining_wall_time {
            *remaining = remaining.saturating_sub(started.elapsed());
        }
        let proposal = match proposal_result {
            Ok(proposal) => proposal,
            Err(_) => {
                self.policy_witness_proposal_rejections =
                    self.policy_witness_proposal_rejections.saturating_add(1);
                None
            }
        };
        let Some(proposal) = proposal else {
            return;
        };
        if !combat_witness_within_potion_contract(
            &self.start,
            &proposal,
            self.max_potions_used,
            self.allowed_potion_slots,
        ) {
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

    fn offer_initial_plan_prefix(&mut self) {
        const MAX_PLAN_PREFIX_TURNS: usize = 6;
        const MAX_PLAN_PREFIX_ACTIONS: usize = 64;

        let max_actions = MAX_PLAN_PREFIX_ACTIONS.min(self.remaining_work);
        if max_actions == 0
            || wall_allowance_exhausted(self.remaining_wall_time)
            || !self.local_search.has_supported_initial_plan_prefix()
        {
            return;
        }
        let started = Instant::now();
        let report = self.local_search.offer_plan_compatible_policy_line(
            MAX_PLAN_PREFIX_TURNS,
            max_actions,
            &crate::sim::combat::EngineCombatStepper,
        );
        if let Some(remaining) = &mut self.remaining_wall_time {
            *remaining = remaining.saturating_sub(started.elapsed());
        }
        let report = match report {
            Ok(report) => report,
            Err(_) => {
                self.plan_prefix_proposal_rejections =
                    self.plan_prefix_proposal_rejections.saturating_add(1);
                return;
            }
        };
        if report.proposed_turns == 0 {
            return;
        }

        self.plan_prefix_proposals = self.plan_prefix_proposals.saturating_add(1);
        self.plan_prefix_proposed_turns = self
            .plan_prefix_proposed_turns
            .saturating_add(report.proposed_turns);
        self.plan_prefix_proposed_actions = self
            .plan_prefix_proposed_actions
            .saturating_add(report.proposed_actions.len());
        self.remaining_work = self
            .remaining_work
            .saturating_sub(report.proposed_actions.len());
        self.remaining_engine_steps = self
            .remaining_engine_steps
            .saturating_sub(report.engine_steps);
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
            potion_contract_recorded: true,
            max_potions_used: self.max_potions_used,
            allowed_potion_slots: self.allowed_potion_slots,
            potion_spend_requires_satisfaction: self.potion_spend_requires_satisfaction,
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
        // "First" is relative to the configured owner satisfaction. Survival
        // mode still accepts the first exact win; strategic quality modes keep
        // an insufficient win safe while using only this stage's allowance to
        // seek a quality-reaching replacement.
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
        stop_on_first_satisfying_witness: bool,
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
        let before_incumbent_quality = self
            .best_witness()
            .map(|witness| combat_witness_quality(&self.start, witness));
        let engine_grant = self
            .remaining_engine_steps
            .min(work.saturating_mul(self.max_transition_steps));
        let productive_member = if stop_on_first_satisfying_witness
            && self.satisfaction != PortfolioWitnessSatisfactionV1::BudgetOrExhaustion
        {
            self.best_witness().and_then(|witness| {
                if combat_witness_satisfies(self.satisfaction, &self.start, witness) {
                    return None;
                }
                match witness.discovery_source {
                    OracleCombatWitnessDiscoverySource::PlannerSearch => {
                        Some(PortfolioMemberV1::LocalTurnGraph)
                    }
                    OracleCombatWitnessDiscoverySource::PolicyDiscrepancySearch => {
                        Some(PortfolioMemberV1::PolicyDiscrepancy)
                    }
                    _ => None,
                }
            })
        } else {
            None
        };
        let Some(member) = select_productive_portfolio_member(
            productive_member,
            self.portfolio_service_order,
            self.next_portfolio_member,
            self.local_complete,
            self.discrepancy_complete,
        ) else {
            return RunControlCombatWorkAdvanceV1::ReadyToFinish;
        };
        self.next_portfolio_member = member.other();
        let (consumed_work, consumed_engine, member_complete, status) =
            match member {
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
                        if self.discrepancy_witness.as_ref().is_none_or(|current| {
                            combat_witness_better(&self.start, &witness, current)
                        }) {
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
        let after_incumbent_quality = self
            .best_witness()
            .map(|witness| combat_witness_quality(&self.start, witness));
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
        let fallback_challenge_complete = stop_on_first_satisfying_witness
            && self.policy_witness.is_some()
            && self.current_local_search_work() >= quantum.additional_nodes;
        let inherited_satisfying_challenge_complete = inherited_satisfying_incumbent_challenged(
            stop_on_first_satisfying_witness,
            self.satisfaction,
            &self.start,
            self.stage_entry_incumbent.as_ref(),
            self.current_local_search_work(),
            quantum.additional_nodes,
        );
        let standard_satisfaction_reached = standard_witness_ends_stage(
            stop_on_first_satisfying_witness,
            acceptance_improved,
            self.satisfaction,
            &self.start,
            self.best_witness(),
        );
        let quality_satisfied = !stop_on_first_satisfying_witness
            && self.best_witness().is_some_and(|witness| {
                combat_witness_ends_quality_refinement(
                    &self.start,
                    self.satisfaction,
                    self.potion_spend_requires_satisfaction,
                    witness,
                )
            });
        let quality_challenge_complete = self.best_witness().is_none_or(|witness| {
            witness.discovery_source != OracleCombatWitnessDiscoverySource::PolicyProposal
        }) || self.current_local_search_work()
            >= quantum.additional_nodes
            || self.local_complete;
        if standard_satisfaction_reached
            || fallback_challenge_complete
            || inherited_satisfying_challenge_complete
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

    pub(super) fn incumbent_hp_loss(&self) -> Option<u32> {
        let initial_hp = self.start.combat.entities.player.current_hp;
        self.best_witness().map(|witness| {
            initial_hp
                .saturating_sub(witness.final_position.combat.entities.player.current_hp)
                .max(0) as u32
        })
    }

    pub(super) fn has_refinement_ending_witness(&self) -> bool {
        self.best_witness().is_some_and(|witness| {
            combat_witness_ends_quality_refinement(
                &self.start,
                self.satisfaction,
                self.potion_spend_requires_satisfaction,
                witness,
            )
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
        if !self.allow_potion_discard
            && inputs
                .iter()
                .any(|input| matches!(input, ClientInput::DiscardPotion(_)))
        {
            return Err(
                "oracle combat witness uses potion discard outside an all-legal search policy"
                    .to_string(),
            );
        }
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
        if !combat_witness_within_potion_contract(
            &self.start,
            &witness,
            self.max_potions_used,
            self.allowed_potion_slots,
        ) {
            return Err(format!(
                "oracle combat witness violates potion contract: uses {} potion(s), limit {:?}, allowed slots {:?}",
                combat_witness_potion_expenditures(&self.start, &witness),
                self.max_potions_used,
                self.allowed_potion_slots,
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
        local
            .generation_work
            .saturating_add(local.lookahead_work)
            .saturating_add(self.plan_prefix_proposed_actions)
    }

    fn best_witness(&self) -> Option<&OracleCombatWitness> {
        [
            self.local_search.witness(),
            self.discrepancy_witness.as_ref(),
            self.policy_witness.as_ref(),
            self.protected_potion_free_incumbent.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|witness| {
            combat_witness_within_potion_contract(
                &self.start,
                witness,
                self.max_potions_used,
                self.allowed_potion_slots,
            )
        })
        .chain(self.prior_stage_incumbent.as_ref())
        .filter(|witness| self.allow_potion_discard || !combat_witness_uses_potion_discard(witness))
        .reduce(|best, candidate| {
            if combat_witness_better_with_potion_quality_gate(
                &self.start,
                self.satisfaction,
                self.potion_spend_requires_satisfaction,
                candidate,
                best,
            ) {
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

    pub(super) fn allowed_potion_slots(&self) -> Option<u64> {
        self.allowed_potion_slots
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
            root_exact_state_hash: crate::ai::combat_state_key::combat_exact_state_hash_v2(
                &self.start.engine,
                &self.start.combat,
            ),
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
            current_policy_witness_proposals: self.policy_witness_proposals,
            current_policy_witness_proposal_rejections: self.policy_witness_proposal_rejections,
            policy_witness_proposals: self
                .policy_witness_proposals
                .saturating_add(self.prior_policy_witness_proposals),
            policy_witness_proposal_rejections: self.policy_witness_proposal_rejections,
            plan_prefix_proposals: self.plan_prefix_proposals,
            plan_prefix_proposed_turns: self.plan_prefix_proposed_turns,
            plan_prefix_proposed_actions: self.plan_prefix_proposed_actions,
            plan_prefix_proposal_rejections: self.plan_prefix_proposal_rejections,
            advisor_nodes: 0,
            advisor_elapsed_ms: 0,
            advisor_active: false,
            advisor_failure: None,
            incumbent_discovery_source: incumbent.map(|witness| witness.discovery_source),
            incumbent_final_hp,
            incumbent_hp_loss: incumbent_final_hp
                .map(|final_hp| initial_hp.saturating_sub(final_hp).max(0)),
            incumbent_action_count: incumbent.map(|witness| witness.actions.len()),
            incumbent_potions_used: incumbent
                .map(|witness| combat_witness_potion_expenditures(&self.start, witness)),
            incumbent_potion_slots: incumbent
                .map(|witness| combat_witness_potion_expenditure_slots(&self.start, witness)),
            incumbent_satisfies_satisfaction: incumbent
                .map(|witness| combat_witness_satisfies(self.satisfaction, &self.start, witness)),
            incumbent_ends_quality_refinement: incumbent.map(|witness| {
                combat_witness_ends_quality_refinement(
                    &self.start,
                    self.satisfaction,
                    self.potion_spend_requires_satisfaction,
                    witness,
                )
            }),
            potion_spend_requires_satisfaction: self.potion_spend_requires_satisfaction,
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

fn combat_witness_quality(
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
) -> CombatWitnessQualityV1 {
    let final_hp = witness.final_position.combat.entities.player.current_hp;
    let persistent_run_value =
        crate::ai::combat_search_v2::persistent_run_value(&witness.final_position.combat);
    CombatWitnessQualityV1 {
        persistent_adjusted_hp: final_hp.saturating_add(persistent_run_value),
        final_hp,
        persistent_run_value,
        potions_used: combat_witness_potion_expenditures(start, witness),
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

fn combat_witness_better(
    start: &crate::sim::combat::CombatPosition,
    left: &OracleCombatWitness,
    right: &OracleCombatWitness,
) -> bool {
    combat_witness_quality_better(
        combat_witness_quality(start, left),
        combat_witness_quality(start, right),
    )
}

fn standard_witness_ends_stage(
    stop_on_first_satisfying_witness: bool,
    acceptance_improved: bool,
    satisfaction: PortfolioWitnessSatisfactionV1,
    start: &crate::sim::combat::CombatPosition,
    witness: Option<&OracleCombatWitness>,
) -> bool {
    stop_on_first_satisfying_witness
        && acceptance_improved
        && witness.is_some_and(|witness| combat_witness_satisfies(satisfaction, start, witness))
}

fn inherited_satisfying_incumbent_challenged(
    stop_on_first_satisfying_witness: bool,
    satisfaction: PortfolioWitnessSatisfactionV1,
    start: &crate::sim::combat::CombatPosition,
    inherited: Option<&OracleCombatWitness>,
    local_generation_work: usize,
    challenge_work: usize,
) -> bool {
    stop_on_first_satisfying_witness
        && local_generation_work >= challenge_work
        && inherited.is_some_and(|witness| combat_witness_satisfies(satisfaction, start, witness))
}

fn combat_witness_better_with_potion_quality_gate(
    start: &crate::sim::combat::CombatPosition,
    satisfaction: PortfolioWitnessSatisfactionV1,
    potion_spend_requires_satisfaction: bool,
    left: &OracleCombatWitness,
    right: &OracleCombatWitness,
) -> bool {
    if potion_spend_requires_satisfaction {
        let left_potions = combat_witness_potion_expenditures(start, left);
        let right_potions = combat_witness_potion_expenditures(start, right);
        let left_satisfies = combat_witness_satisfies(satisfaction, start, left);
        let right_satisfies = combat_witness_satisfies(satisfaction, start, right);
        if left_satisfies != right_satisfies {
            return left_satisfies;
        }
        if !left_satisfies && left_potions != right_potions {
            return left_potions < right_potions;
        }
    }
    combat_witness_better(start, left, right)
}

fn combat_witness_ends_quality_refinement(
    start: &crate::sim::combat::CombatPosition,
    satisfaction: PortfolioWitnessSatisfactionV1,
    potion_spend_requires_satisfaction: bool,
    witness: &OracleCombatWitness,
) -> bool {
    combat_witness_satisfies(satisfaction, start, witness)
        && (!potion_spend_requires_satisfaction
            || combat_witness_potion_expenditures(start, witness) == 0)
}

fn combat_witness_potion_expenditures(
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
) -> u32 {
    let explicit_expenditures = witness
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
        .unwrap_or(u32::MAX);
    let remaining_uuids = witness
        .final_position
        .combat
        .entities
        .potions
        .iter()
        .flatten()
        .map(|potion| potion.uuid)
        .collect::<std::collections::BTreeSet<_>>();
    let missing_starting_resources = start
        .combat
        .entities
        .potions
        .iter()
        .flatten()
        .filter(|potion| !remaining_uuids.contains(&potion.uuid))
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    explicit_expenditures.max(missing_starting_resources)
}

fn combat_witness_uses_potion_discard(witness: &OracleCombatWitness) -> bool {
    witness
        .actions
        .iter()
        .any(|action| matches!(action.input, ClientInput::DiscardPotion(_)))
}

fn combat_witness_within_potion_contract(
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
    max_potions_used: Option<u32>,
    allowed_potion_slots: Option<u64>,
) -> bool {
    max_potions_used.is_none_or(|limit| combat_witness_potion_expenditures(start, witness) <= limit)
        && witness.actions.iter().all(|action| {
            let slot = match action.input {
                ClientInput::UsePotion { potion_index, .. } => Some(potion_index),
                ClientInput::DiscardPotion(slot) => Some(slot),
                _ => None,
            };
            slot.is_none_or(|slot| {
                allowed_potion_slots.is_none_or(|allowed_slots| {
                    u32::try_from(slot)
                        .ok()
                        .and_then(|slot| 1_u64.checked_shl(slot))
                        .is_some_and(|slot_mask| allowed_slots & slot_mask != 0)
                })
            })
        })
        && allowed_potion_slots.is_none_or(|allowed_slots| {
            starting_potion_expenditure_slots(start, witness) & !allowed_slots == 0
        })
}

fn starting_potion_expenditure_slots(
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
) -> u64 {
    let remaining_uuids = witness
        .final_position
        .combat
        .entities
        .potions
        .iter()
        .flatten()
        .map(|potion| potion.uuid)
        .collect::<std::collections::BTreeSet<_>>();
    start
        .combat
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(slot, potion)| {
            let potion = potion.as_ref()?;
            if remaining_uuids.contains(&potion.uuid) {
                return None;
            }
            u32::try_from(slot)
                .ok()
                .and_then(|slot| 1_u64.checked_shl(slot))
        })
        .fold(0_u64, |mask, slot| mask | slot)
}

fn combat_witness_potion_expenditure_slots(
    start: &crate::sim::combat::CombatPosition,
    witness: &OracleCombatWitness,
) -> u64 {
    witness
        .actions
        .iter()
        .filter_map(|action| match action.input {
            ClientInput::UsePotion { potion_index, .. } => Some(potion_index),
            ClientInput::DiscardPotion(slot) => Some(slot),
            _ => None,
        })
        .filter_map(|slot| {
            u32::try_from(slot)
                .ok()
                .and_then(|slot| 1_u64.checked_shl(slot))
        })
        .fold(
            starting_potion_expenditure_slots(start, witness),
            |mask, slot| mask | slot,
        )
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

fn select_productive_portfolio_member(
    productive_member: Option<PortfolioMemberV1>,
    service_order: PortfolioServiceOrderV1,
    next: PortfolioMemberV1,
    local_complete: bool,
    discrepancy_complete: bool,
) -> Option<PortfolioMemberV1> {
    match productive_member {
        Some(PortfolioMemberV1::LocalTurnGraph) if !local_complete => {
            Some(PortfolioMemberV1::LocalTurnGraph)
        }
        Some(PortfolioMemberV1::PolicyDiscrepancy) if !discrepancy_complete => {
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        }
        _ => select_portfolio_member(service_order, next, local_complete, discrepancy_complete),
    }
}

fn select_portfolio_member(
    service_order: PortfolioServiceOrderV1,
    next: PortfolioMemberV1,
    local_complete: bool,
    discrepancy_complete: bool,
) -> Option<PortfolioMemberV1> {
    if service_order == PortfolioServiceOrderV1::LocalPrimary {
        return match (local_complete, discrepancy_complete) {
            (false, _) => Some(PortfolioMemberV1::LocalTurnGraph),
            (true, false) => Some(PortfolioMemberV1::PolicyDiscrepancy),
            (true, true) => None,
        };
    }
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

    fn boss_combat_session() -> RunControlSession {
        let mut session = hallway_combat_session();
        let active = session.active_combat.as_mut().expect("active combat");
        active.combat_state.meta.is_boss_fight = true;
        active.context =
            crate::state::core::CombatContext::Room(crate::state::core::RoomCombatContext {
                room_type: crate::state::map::node::RoomType::MonsterRoomBoss,
            });
        session
    }

    fn bronze_automaton_combat_session() -> RunControlSession {
        let mut session = boss_combat_session();
        let active = session.active_combat.as_mut().expect("active combat");
        let combat = &mut active.combat_state;
        let mut automaton =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::BronzeAutomaton);
        automaton.id = 10;
        let plan = crate::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &automaton,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&automaton),
            &[],
        );
        automaton.set_planned_move_id(plan.move_id);
        automaton.set_planned_steps(plan.steps);
        automaton.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters = vec![automaton];
        crate::content::powers::store::set_powers_for(
            combat,
            10,
            vec![crate::runtime::combat::Power {
                power_type: crate::content::powers::PowerId::Artifact,
                instance_id: None,
                amount: 3,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.turn.energy = 3;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Disarm, 1),
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::ThunderClap, 2),
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, 3),
        ];
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

    fn single_potion_slot_options(slot_mask: u64) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            max_nodes: Some(16),
            max_potions_used: Some(1),
            allowed_potion_slots: Some(slot_mask),
            satisfaction: Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
            ),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn explosive_stage_one_work() -> (RunControlSession, OracleRunCombatWorkV1) {
        let mut session = one_strike_win_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.monsters[0].current_hp = 10;
        combat.entities.monsters[0].max_hp = 10;
        combat.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                crate::content::potions::PotionId::ExplosivePotion,
                70,
            )),
            Some(crate::content::potions::Potion::new(
                crate::content::potions::PotionId::BlockPotion,
                71,
            )),
        ];
        let mut work = OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(
            &session,
            single_potion_slot_options(0b01),
            None,
        )
        .expect("slot-zero exact witness work");
        work.verify_and_restore_action_witness(&[ClientInput::UsePotion {
            potion_index: 0,
            target: None,
        }])
        .expect("Explosive Potion should provide an exact slot-zero victory");
        (session, work)
    }

    fn synthetic_witness(
        start: &crate::sim::combat::CombatPosition,
        final_hp: i32,
        spend_potion: bool,
    ) -> OracleCombatWitness {
        let mut final_position = start.clone();
        final_position.combat.entities.player.current_hp = final_hp;
        let actions = if spend_potion {
            final_position.combat.entities.potions[0] = None;
            vec![TurnOptionAction {
                input: ClientInput::UsePotion {
                    potion_index: 0,
                    target: None,
                },
                expected_successor_hash: "synthetic".into(),
                engine_steps: 0,
            }]
        } else {
            Vec::new()
        };
        OracleCombatWitness {
            actions,
            final_position,
            negative_log_policy: 0.0,
            replay_engine_steps: 0,
            discovery_source: OracleCombatWitnessDiscoverySource::RestoredExactActions,
        }
    }

    #[test]
    fn quality_gate_protects_no_potion_incumbent_from_marginal_spend() {
        let mut session = hallway_combat_session();
        session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .entities
            .potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::ColorlessPotion,
            7,
        ))];
        let start = session
            .current_active_combat_position()
            .expect("exact potion rescue root");
        let baseline = synthetic_witness(&start, 50, false);
        let marginal_spend = synthetic_witness(&start, 55, true);
        let quality_spend = synthetic_witness(&start, 65, true);
        let quality_clean = synthetic_witness(&start, 65, false);
        let satisfaction = PortfolioWitnessSatisfactionV1::HpLossAtMost(20);

        assert!(
            combat_witness_better(&start, &marginal_spend, &baseline),
            "raw final-HP comparison demonstrates the regression guard's purpose"
        );
        assert!(!combat_witness_better_with_potion_quality_gate(
            &start,
            satisfaction,
            true,
            &marginal_spend,
            &baseline,
        ));
        assert!(combat_witness_better_with_potion_quality_gate(
            &start,
            satisfaction,
            true,
            &baseline,
            &marginal_spend,
        ));
        assert!(combat_witness_better_with_potion_quality_gate(
            &start,
            satisfaction,
            true,
            &quality_spend,
            &baseline,
        ));
        assert!(!combat_witness_better_with_potion_quality_gate(
            &start,
            satisfaction,
            true,
            &baseline,
            &quality_spend,
        ));
        assert!(!combat_witness_ends_quality_refinement(
            &start,
            satisfaction,
            true,
            &quality_spend,
        ));
        assert!(combat_witness_ends_quality_refinement(
            &start,
            satisfaction,
            true,
            &quality_clean,
        ));
        assert!(combat_witness_ends_quality_refinement(
            &start,
            satisfaction,
            false,
            &quality_spend,
        ));
    }

    #[test]
    fn conserving_rollout_keeps_the_no_potion_baseline_when_a_potion_line_is_better() {
        let mut session = one_strike_win_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.player.current_hp = 20;
        combat.entities.player.max_hp = 20;
        combat.entities.monsters[0].current_hp = 7;
        combat.entities.monsters[0].max_hp = 7;
        combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FirePotion,
            7,
        ))];
        let start = session
            .current_active_combat_position()
            .expect("exact potion rollout root");
        let unconstrained =
            crate::ai::combat_search_v2::oracle_rollout_witness_proposal_v1(&start, 64, None)
                .expect("unconstrained rollout proposal");
        assert!(unconstrained
            .actions
            .iter()
            .any(|input| matches!(input, ClientInput::UsePotion { .. })));

        let conserving =
            existing_combat_rollout_witness_v1(&start, 64, 250, None, Some(0), Some(0))
                .expect("replay conserving proposal")
                .expect("no-potion baseline");

        assert_eq!(combat_witness_potion_expenditures(&start, &conserving), 0);
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
        assert!(error.contains("violates potion contract"));
        assert!(work.best_witness().is_none_or(|witness| {
            combat_witness_potion_expenditures(&work.start, witness) == 0
        }));
    }

    #[test]
    fn exact_restored_discard_requires_an_all_legal_potion_policy() {
        // Exercise the public exact-witness restoration boundary under both
        // policy modes; filtering only generated actions would miss restores.
        let mut session = one_strike_win_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        let target = combat.entities.monsters[0].id;
        combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::EnergyPotion,
            7,
        ))];
        let actions = [
            ClientInput::DiscardPotion(0),
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(target),
            },
        ];
        let options = |potion_policy| RunControlSearchCombatOptions {
            max_nodes: Some(16),
            potion_policy: Some(potion_policy),
            max_potions_used: Some(1),
            allowed_potion_slots: Some(1),
            satisfaction: Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
            ),
            ..RunControlSearchCombatOptions::default()
        };
        let mut semantic = OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(
            &session,
            options(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted),
            None,
        )
        .expect("semantic exact-witness work");

        let error = semantic
            .verify_and_restore_action_witness(&actions)
            .expect_err("semantic victory search must omit explicit discard");
        assert!(error.contains("outside an all-legal search policy"));

        let mut all_legal = OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(
            &session,
            options(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::All),
            None,
        )
        .expect("all-legal exact-witness work");
        all_legal
            .verify_and_restore_action_witness(&actions)
            .expect("all-legal discard witness");
        assert!(all_legal.has_verified_witness());
    }

    #[test]
    fn exact_restored_witness_cannot_bypass_potion_slot_contract() {
        let mut session = one_strike_win_session();
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        let target = combat.entities.monsters[0].id;
        combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FirePotion,
            8,
        ))];
        let mut work = OracleRunCombatWorkV1::new_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                max_potions_used: Some(1),
                allowed_potion_slots: Some(1_u64 << 1),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("slot-constrained portfolio should accept an active combat");

        let error = work
            .verify_and_restore_action_witness(&[ClientInput::UsePotion {
                potion_index: 0,
                target: Some(target),
            }])
            .expect_err("an exact potion win must not bypass the exact slot contract");

        assert!(error.contains("allowed slots Some(2)"), "{error}");
        assert!(work.best_witness().is_none_or(|witness| {
            witness.actions.iter().all(|action| {
                !matches!(
                    action.input,
                    ClientInput::UsePotion {
                        potion_index: 0,
                        ..
                    }
                )
            })
        }));
    }

    #[test]
    fn identity_stage_preserves_verified_prior_incumbent_across_checkpoint_restore() {
        let (session, stage_one) = explosive_stage_one_work();
        let stage_one_checkpoint = stage_one.checkpoint();
        let stage_one_incumbent = stage_one_checkpoint
            .incumbent
            .as_ref()
            .expect("slot-zero stage should checkpoint its exact victory");
        assert_eq!(
            combat_witness_potion_expenditure_slots(&stage_one.start, stage_one_incumbent),
            0b01
        );

        let stage_two_options = single_potion_slot_options(0b10);
        let stage_two = OracleRunCombatWorkV1::restart_for_higher_fidelity_with_guidance(
            &session,
            stage_two_options.clone(),
            stage_one_checkpoint,
            None,
        )
        .expect("slot-one stage should retain the verified slot-zero incumbent");

        assert_eq!(stage_two.allowed_potion_slots(), Some(0b10));
        assert!(stage_two.has_verified_witness());
        let retained = stage_two
            .best_witness()
            .expect("prior-stage incumbent remains the portfolio fallback");
        assert_eq!(
            combat_witness_potion_expenditure_slots(&stage_two.start, retained),
            0b01
        );
        assert!(
            !combat_witness_within_potion_contract(
                &stage_two.start,
                retained,
                stage_two.max_potions_used,
                stage_two.allowed_potion_slots,
            ),
            "the prior incumbent is retained without widening the new stage's generation contract"
        );
        let progress = stage_two.progress();
        assert_eq!(progress.incumbent_potion_slots, Some(0b01));

        let stage_two_checkpoint = stage_two.checkpoint();
        assert_eq!(stage_two_checkpoint.allowed_potion_slots, Some(0b10));
        assert_eq!(
            combat_witness_potion_expenditure_slots(
                &stage_two.start,
                stage_two_checkpoint
                    .incumbent
                    .as_ref()
                    .expect("prior-stage fallback remains checkpointed"),
            ),
            0b01
        );
        let restored = OracleRunCombatWorkV1::restart_from_checkpoint_with_guidance(
            &session,
            stage_two_options,
            stage_two_checkpoint,
            None,
        )
        .expect("same-stage process restore should retain the exact prior-stage fallback");

        assert_eq!(restored.allowed_potion_slots(), Some(0b10));
        assert!(restored.has_verified_witness());
        assert_eq!(
            restored.progress().incumbent_potion_slots,
            Some(0b01),
            "checkpoint restore must not lose the earlier identity's verified victory"
        );
    }

    #[test]
    fn wider_potion_stage_keeps_a_separate_entry_incumbent_challenge() {
        let (session, stage_one) = explosive_stage_one_work();
        let stage_two = OracleRunCombatWorkV1::restart_for_higher_fidelity_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(16),
                max_potions_used: Some(2),
                allowed_potion_slots: Some(0b11),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            stage_one.checkpoint(),
            None,
        )
        .expect("wider stage should retain the exact slot-zero incumbent");

        assert!(stage_two.prior_stage_incumbent.is_none());
        let inherited = stage_two
            .stage_entry_incumbent
            .as_ref()
            .expect("stage entry should remain explicit even inside the wider mask");
        assert!(combat_witness_within_potion_contract(
            &stage_two.start,
            inherited,
            stage_two.max_potions_used,
            stage_two.allowed_potion_slots,
        ));
        assert!(inherited_satisfying_incumbent_challenged(
            true,
            stage_two.satisfaction,
            &stage_two.start,
            Some(inherited),
            16,
            16,
        ));
    }

    #[test]
    fn prior_stage_incumbent_must_replay_exactly_before_restore() {
        let (session, stage_one) = explosive_stage_one_work();
        let mut checkpoint = stage_one.checkpoint();
        checkpoint
            .incumbent
            .as_mut()
            .expect("slot-zero exact incumbent")
            .final_position
            .combat
            .entities
            .player
            .current_hp -= 1;

        let error = match OracleRunCombatWorkV1::restart_for_higher_fidelity_with_guidance(
            &session,
            single_potion_slot_options(0b10),
            checkpoint,
            None,
        ) {
            Ok(_) => {
                panic!("an unreplayable prior-stage incumbent must not enter the fallback channel")
            }
            Err(error) => error,
        };

        assert!(error.contains("final position"), "{error}");
    }

    #[test]
    fn passive_fairy_consumption_is_not_mislabeled_as_zero_potion() {
        let mut session = hallway_combat_session();
        session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .entities
            .potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FairyPotion,
            9,
        ))];
        let start = session
            .current_active_combat_position()
            .expect("exact Fairy root");
        let mut final_position = start.clone();
        final_position.combat.entities.potions[0] = None;
        let witness = OracleCombatWitness {
            actions: Vec::new(),
            final_position,
            negative_log_policy: 0.0,
            replay_engine_steps: 0,
            discovery_source: OracleCombatWitnessDiscoverySource::PolicyProposal,
        };

        assert_eq!(combat_witness_potion_expenditures(&start, &witness), 1);
        assert_eq!(combat_witness_potion_expenditure_slots(&start, &witness), 1);
        assert!(!combat_witness_within_potion_contract(
            &start,
            &witness,
            Some(0),
            None,
        ));
        assert!(!combat_witness_within_potion_contract(
            &start,
            &witness,
            Some(1),
            Some(0),
        ));
        assert!(combat_witness_within_potion_contract(
            &start,
            &witness,
            Some(1),
            Some(1),
        ));
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
            select_portfolio_member(
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::LocalTurnGraph,
                false,
                false,
            ),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::PolicyDiscrepancy,
                false,
                false,
            ),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::LocalTurnGraph,
                true,
                false,
            ),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::PolicyDiscrepancy,
                false,
                true,
            ),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::LocalTurnGraph,
                true,
                true,
            ),
            None
        );
    }

    #[test]
    fn boss_portfolio_keeps_local_primary_until_it_completes() {
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::LocalPrimary,
                PortfolioMemberV1::PolicyDiscrepancy,
                false,
                false,
            ),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_portfolio_member(
                PortfolioServiceOrderV1::LocalPrimary,
                PortfolioMemberV1::LocalTurnGraph,
                true,
                false,
            ),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
    }

    #[test]
    fn insufficient_incumbent_keeps_service_with_its_productive_member() {
        assert_eq!(
            select_productive_portfolio_member(
                Some(PortfolioMemberV1::LocalTurnGraph),
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::PolicyDiscrepancy,
                false,
                false,
            ),
            Some(PortfolioMemberV1::LocalTurnGraph)
        );
        assert_eq!(
            select_productive_portfolio_member(
                Some(PortfolioMemberV1::PolicyDiscrepancy),
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::LocalTurnGraph,
                false,
                false,
            ),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
        );
        assert_eq!(
            select_productive_portfolio_member(
                Some(PortfolioMemberV1::LocalTurnGraph),
                PortfolioServiceOrderV1::RoundRobin,
                PortfolioMemberV1::LocalTurnGraph,
                true,
                false,
            ),
            Some(PortfolioMemberV1::PolicyDiscrepancy)
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
    fn boss_portfolio_serves_local_graph_across_caller_quanta() {
        let session = boss_combat_session();
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
        .expect("boss portfolio should accept an active combat");
        let quantum = RunControlCombatSearchQuantum {
            label: "boss_local_primary_contract",
            additional_nodes: 1,
            soft_wall_ms: None,
        };

        assert_eq!(
            work.advance(&quantum, None),
            RunControlCombatWorkAdvanceV1::Pending
        );
        assert_eq!(
            work.advance(&quantum, None),
            RunControlCombatWorkAdvanceV1::Pending
        );
        assert_eq!(work.local_search.counters().generation_work, 2);
        assert_eq!(
            work.discrepancy_search
                .counters()
                .applied_action_transitions,
            0
        );
        assert_eq!(work.remaining_work, 14);
    }

    #[test]
    fn timed_bronze_plan_materializes_one_bounded_charged_prefix() {
        let session = bronze_automaton_combat_session();
        let max_work = 128;
        let work = OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(max_work),
                satisfaction: Some(
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
                ),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("Bronze Automaton should create a production portfolio");
        let progress = work.progress();

        assert_eq!(progress.plan_prefix_proposals, 1);
        assert!((1..=6).contains(&progress.plan_prefix_proposed_turns));
        assert!((1..=64).contains(&progress.plan_prefix_proposed_actions));
        assert_eq!(progress.plan_prefix_proposal_rejections, 0);
        assert_eq!(
            max_work.saturating_sub(work.remaining_work),
            progress.plan_prefix_proposed_actions,
            "materialized prefix actions must consume the shared generation allowance"
        );
        assert_eq!(
            progress.current_search_generation_work as usize,
            progress.plan_prefix_proposed_actions
        );
        assert_eq!(
            max_work
                .saturating_mul(work.max_transition_steps)
                .saturating_sub(work.remaining_engine_steps),
            progress.engine_steps,
            "exact transition previews must consume the shared engine allowance"
        );
    }

    #[test]
    fn encounter_without_timed_plan_does_not_offer_a_prefix() {
        let session = hallway_combat_session();
        let max_work = 16;
        let work = OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(max_work),
                ..RunControlSearchCombatOptions::default()
            },
            None,
        )
        .expect("ordinary hallway combat should create a production portfolio");
        let progress = work.progress();

        assert_eq!(progress.plan_prefix_proposals, 0);
        assert_eq!(progress.plan_prefix_proposed_turns, 0);
        assert_eq!(progress.plan_prefix_proposed_actions, 0);
        assert_eq!(progress.plan_prefix_proposal_rejections, 0);
        assert_eq!(work.remaining_work, max_work);
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
    fn standard_advance_ends_a_stage_only_on_a_satisfying_new_win() {
        // Keep one witness shape fixed and vary only its final HP so this
        // contract tests stage termination rather than search reachability.
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
        let satisfying = work
            .policy_witness
            .clone()
            .expect("policy should provide an exact winning line");
        let mut insufficient = satisfying.clone();
        insufficient
            .final_position
            .combat
            .entities
            .player
            .current_hp = work
            .start
            .combat
            .entities
            .player
            .current_hp
            .saturating_sub(20);

        assert!(!standard_witness_ends_stage(
            true,
            true,
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            Some(&insufficient)
        ));
        assert!(standard_witness_ends_stage(
            true,
            true,
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            Some(&satisfying)
        ));
        assert!(standard_witness_ends_stage(
            true,
            true,
            PortfolioWitnessSatisfactionV1::FirstWitness,
            &work.start,
            Some(&insufficient)
        ));
        assert!(!standard_witness_ends_stage(
            true,
            false,
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            Some(&satisfying)
        ));
        assert!(inherited_satisfying_incumbent_challenged(
            true,
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            Some(&satisfying),
            64,
            64
        ));
        assert!(!inherited_satisfying_incumbent_challenged(
            true,
            PortfolioWitnessSatisfactionV1::HpLossAtMost(8),
            &work.start,
            Some(&insufficient),
            64,
            64
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
