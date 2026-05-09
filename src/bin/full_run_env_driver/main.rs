use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::app::branch_evaluator::{
    branch_rng_state_hash, BranchCandidateScope, BranchEvaluator, BranchEvaluatorConfig,
    BranchHorizonMode,
};
use sts_simulator::app::policy_runner::NeutralProbeEvaluator;
use sts_simulator::cli::full_run_smoke::{
    FullRunEnv, FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState, RewardShapingProfile,
    RunActionCandidate, RunPolicyKind,
};
use sts_simulator::verification::branch_dataset::{
    validate_branch_dataset, BRANCH_TRACE_SCHEMA_VERSION, PAIRED_SCENARIO_SCHEMA_VERSION,
};
use sts_simulator::verification::decision_env::{
    ActionId, CandidateLabel, DecisionEnv, DecisionRecord, DecisionRecordContext,
    PairwisePreference, PolicyInput, TeacherDecisionLabel, TimeStep,
};
use sts_simulator::verification::neutral_engine_query::SearchExecutionContext;
use sts_simulator::verification::search_policy::DeliberationTrace;

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum DriverRequest {
    Ping,
    Reset {
        seed: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
        reward_shaping_profile: Option<String>,
    },
    Observation,
    DecisionEnvObservation,
    PolicyInput {
        time_budget_ms: Option<u32>,
    },
    BranchTraceCacheIdentity,
    NeutralPolicyTrace {
        time_budget_ms: Option<u32>,
        max_branch_depth: Option<u8>,
        max_candidates: Option<usize>,
        reference_action_id: Option<usize>,
    },
    Step {
        action_index: usize,
    },
    DecisionEnvStep {
        action_id: usize,
    },
    DecisionRecordStep {
        action_id: usize,
        sim_version: Option<String>,
        return_spec_version: Option<String>,
        context: Option<Value>,
        teacher_continuation_policy: Option<String>,
        teacher_horizon_decisions: Option<usize>,
        teacher_horizon_mode: Option<String>,
        teacher_gamma: Option<f32>,
        teacher_evaluation_mode: Option<String>,
        teacher_value_cache_scope: Option<String>,
        teacher_value_cache_max_entries: Option<usize>,
        teacher_parallelism: Option<usize>,
        teacher_exact_root_dedup: Option<bool>,
    },
    BranchTrace {
        action_indices: Vec<usize>,
        candidate_scope: Option<String>,
        candidate_sampling_spec_id: Option<String>,
        candidate_cap: Option<usize>,
        behavior_action_id: Option<usize>,
        sampling_seed: Option<u64>,
        continuation_policy: Option<String>,
        horizon_decisions: usize,
        horizon_mode: Option<String>,
        sim_version: Option<String>,
        content_version: Option<String>,
        max_steps: Option<usize>,
        include_comparisons: Option<bool>,
    },
    StepPolicy {
        policy: String,
    },
    PreviewPolicyAction {
        policy: String,
        include_state: Option<bool>,
        include_next_state: Option<bool>,
        check_live_env_unchanged: Option<bool>,
    },
    EvaluateCandidates {
        action_indices: Vec<usize>,
        continuation_policy: String,
        horizon_decisions: usize,
        horizon_mode: Option<String>,
        gamma: f32,
        evaluation_mode: Option<String>,
        value_cache_scope: Option<String>,
        value_cache_max_entries: Option<usize>,
        parallelism: Option<usize>,
        exact_root_dedup: Option<bool>,
        include_state: Option<bool>,
        include_next_state: Option<bool>,
        include_continuation_trace: Option<bool>,
        check_live_env_unchanged: Option<bool>,
    },
    RunVerifiedAdvOverrideEpisode {
        seed: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
        reward_shaping_profile: Option<String>,
        candidate_scope: Option<String>,
        continuation_policy: Option<String>,
        horizon_decisions: usize,
        horizon_mode: Option<String>,
        oracle_margin: f32,
        gamma: f32,
        evaluation_mode: Option<String>,
        value_cache_scope: Option<String>,
        value_cache_max_entries: Option<usize>,
        parallelism: Option<usize>,
        exact_root_dedup: Option<bool>,
        verifier_strategy: Option<String>,
        prefilter_horizon_decisions: Option<usize>,
        prefilter_horizon_mode: Option<String>,
        prefilter_margin: Option<f32>,
        prefilter_top_k: Option<usize>,
        proposer_model_path: Option<String>,
        proposer_top_k: Option<usize>,
        proposer_threshold: Option<f32>,
        evidence_gate: Option<String>,
        low_evidence_margin: Option<f32>,
        confirm_low_evidence_horizon_decisions: Option<usize>,
        confirm_low_evidence_horizon_mode: Option<String>,
        confirm_low_evidence_margin: Option<f32>,
    },
    RunVerifiedAdvOverrideBatch {
        episodes: usize,
        seed_start: Option<u64>,
        seed_step: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
        reward_shaping_profile: Option<String>,
        candidate_scope: Option<String>,
        continuation_policy: Option<String>,
        horizon_decisions: usize,
        horizon_mode: Option<String>,
        oracle_margin: f32,
        gamma: f32,
        evaluation_mode: Option<String>,
        value_cache_scope: Option<String>,
        value_cache_max_entries: Option<usize>,
        parallelism: Option<usize>,
        exact_root_dedup: Option<bool>,
        verifier_strategy: Option<String>,
        prefilter_horizon_decisions: Option<usize>,
        prefilter_horizon_mode: Option<String>,
        prefilter_margin: Option<f32>,
        prefilter_top_k: Option<usize>,
        proposer_model_path: Option<String>,
        proposer_top_k: Option<usize>,
        proposer_threshold: Option<f32>,
        evidence_gate: Option<String>,
        low_evidence_margin: Option<f32>,
        confirm_low_evidence_horizon_decisions: Option<usize>,
        confirm_low_evidence_horizon_mode: Option<String>,
        confirm_low_evidence_margin: Option<f32>,
        summary_only: Option<bool>,
    },
    InspectCounterfactualPending {
        candidate_scope: Option<String>,
        continuation_policy: Option<String>,
        horizon_decisions: usize,
        horizon_mode: Option<String>,
        oracle_margin: f32,
        gamma: f32,
        max_roots: Option<usize>,
        max_groups: Option<usize>,
        parallelism: Option<usize>,
        include_observation: Option<bool>,
    },
    Close,
}

#[derive(Debug, Serialize)]
struct DriverResponse {
    ok: bool,
    error: Option<String>,
    payload: Option<Value>,
    reward: Option<f32>,
    done: Option<bool>,
    chosen_action_key: Option<String>,
    info: Option<FullRunEnvInfo>,
}

#[derive(Debug, Serialize)]
struct CandidateEvaluationPayload {
    schema_version: String,
    continuation_policy: String,
    horizon_decisions: usize,
    horizon_mode: String,
    gamma: f32,
    evaluation_mode: String,
    value_cache_scope: String,
    root_candidate_count: usize,
    root_exact_dedup_count: usize,
    root_rule_equivalent_prune_count: usize,
    value_cache_hit_count: usize,
    value_cache_miss_count: usize,
    policy_step_eval_count: usize,
    cache_entry_count: usize,
    parallelism_requested: usize,
    parallelism_used: usize,
    candidate_eval_wall_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    live_env_unchanged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_before: Option<FullRunEnvState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_after: Option<FullRunEnvState>,
    evaluations: Vec<CandidateEvaluation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateScope {
    All,
    ControlledV0,
    ControlledV1,
}

impl CandidateScope {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("controlled_v1").to_ascii_lowercase().as_str() {
            "" | "all" => Ok(Self::All),
            "controlled_v0" => Ok(Self::ControlledV0),
            "controlled_v1" => Ok(Self::ControlledV1),
            other => Err(format!(
                "unsupported candidate_scope '{other}'; expected all, controlled_v0, or controlled_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ControlledV0 => "controlled_v0",
            Self::ControlledV1 => "controlled_v1",
        }
    }
}

#[derive(Debug, Serialize)]
struct VerifiedAdvOverrideBatchPayload {
    schema_version: String,
    config: VerifiedAdvOverrideRunConfigPayload,
    policy_summary: BTreeMap<String, VerifiedAdvOverridePolicySummary>,
    episodes: Vec<VerifiedAdvOverrideEpisodeSummary>,
}

#[derive(Debug, Serialize)]
struct VerifiedAdvOverrideRunConfigPayload {
    episodes: usize,
    seed_start: u64,
    seed_step: u64,
    ascension: u8,
    final_act: bool,
    class: String,
    max_steps: usize,
    reward_shaping_profile: String,
    candidate_scope: String,
    continuation_policy: String,
    horizon_decisions: usize,
    horizon_mode: String,
    oracle_margin: f32,
    verifier_strategy: String,
    prefilter_horizon_decisions: Option<usize>,
    prefilter_horizon_mode: Option<String>,
    prefilter_margin: Option<f32>,
    prefilter_top_k: Option<usize>,
    proposer_model_path: Option<String>,
    proposer_top_k: Option<usize>,
    proposer_threshold: Option<f32>,
    gamma: f32,
    evidence_gate: String,
    low_evidence_margin: Option<f32>,
    confirm_low_evidence_horizon_decisions: Option<usize>,
    confirm_low_evidence_horizon_mode: Option<String>,
    confirm_low_evidence_margin: Option<f32>,
    evaluation_mode: String,
    value_cache_scope: String,
    value_cache_max_entries: usize,
    parallelism: usize,
    exact_root_dedup: bool,
}

#[derive(Debug, Serialize)]
struct VerifiedAdvOverridePolicySummary {
    episodes: usize,
    crash_count: usize,
    result_counts: BTreeMap<String, usize>,
    terminal_reason_counts: BTreeMap<String, usize>,
    death_floor_counts: BTreeMap<String, usize>,
    average_total_reward: f32,
    reward_stderr: Option<f32>,
    average_combat_win_count: f32,
    average_final_floor: f32,
    average_final_hp: f32,
    average_steps: f32,
    verified_decision_count: usize,
    verified_override_count: usize,
    verified_override_rate: f32,
    verified_candidate_evaluation_count: usize,
    verified_prefilter_candidate_evaluation_count: usize,
    verified_final_candidate_evaluation_count: usize,
    verified_prefilter_policy_step_eval_count: usize,
    verified_final_policy_step_eval_count: usize,
    verified_prefilter_decision_count: usize,
    verified_prefilter_kept_decision_count: usize,
    verified_prefilter_kept_candidate_count: usize,
    verified_prefilter_kept_rate: Option<f32>,
    verified_prefilter_average_kept_candidate_count: Option<f32>,
    verified_proposer_decision_count: usize,
    verified_proposer_non_rule_candidate_count: usize,
    verified_proposer_kept_candidate_count: usize,
    verified_proposer_keep_rate: Option<f32>,
    verified_average_scoped_candidate_count: f32,
    verified_adv_mean_on_overrides: Option<f32>,
    verified_harmful_override_count: usize,
    verified_harmful_override_rate: Option<f32>,
    verified_low_evidence_reject_count: usize,
    verified_confirm_decision_count: usize,
    verified_confirm_accept_count: usize,
    verified_confirm_reject_count: usize,
    verified_confirm_candidate_evaluation_count: usize,
    verified_confirm_policy_step_eval_count: usize,
    verified_artifact_confirm_decision_count: usize,
    verified_artifact_confirm_accept_count: usize,
    verified_artifact_confirm_reject_count: usize,
    verified_artifact_confirm_candidate_evaluation_count: usize,
    verified_artifact_confirm_policy_step_eval_count: usize,
    verified_decision_type_counts: BTreeMap<String, usize>,
    verified_override_decision_type_counts: BTreeMap<String, usize>,
    verified_decision_context_counts: BTreeMap<String, usize>,
    verified_override_context_counts: BTreeMap<String, usize>,
    verified_best_adv_bucket_counts: BTreeMap<String, usize>,
    verified_horizon_stop_reason_counts: BTreeMap<String, usize>,
    verified_payoff_reason_counts: BTreeMap<String, usize>,
    verified_override_payoff_reason_counts: BTreeMap<String, usize>,
    verified_horizon_artifact_reason_counts: BTreeMap<String, usize>,
    verified_missing_counts: BTreeMap<String, usize>,
    verified_cached_root_candidate_count: usize,
    verified_cached_root_exact_dedup_count: usize,
    verified_cached_root_exact_dedup_rate: Option<f32>,
    verified_root_rule_equivalent_prune_count: usize,
    verified_root_rule_equivalent_prune_rate: Option<f32>,
    verified_cached_value_hit_count: usize,
    verified_cached_value_miss_count: usize,
    verified_cached_value_hit_rate: Option<f32>,
    verified_cached_policy_step_eval_count: usize,
    verified_cached_cache_entry_count_max: usize,
    verified_parallelism_used_max: usize,
    verified_candidate_eval_wall_ms: u64,
}

#[derive(Debug, Serialize)]
struct VerifiedAdvOverrideEpisodeSummary {
    policy: String,
    seed: u64,
    steps: usize,
    done: bool,
    crash: Option<String>,
    result: String,
    terminal_reason: String,
    final_floor: i32,
    final_act: u8,
    final_hp: i32,
    final_max_hp: i32,
    final_deck_size: usize,
    final_relic_count: usize,
    combat_win_count: usize,
    total_reward: f32,
    learned_decisions: usize,
    #[serde(flatten)]
    stats: VerifiedAdvOverrideStatsPayload,
}

#[derive(Clone, Debug, Default, Serialize)]
struct VerifiedAdvOverrideStatsPayload {
    verified_decision_count: usize,
    verified_override_count: usize,
    verified_reject_count: usize,
    verified_override_rate: f32,
    verified_candidate_evaluation_count: usize,
    verified_prefilter_candidate_evaluation_count: usize,
    verified_final_candidate_evaluation_count: usize,
    verified_prefilter_policy_step_eval_count: usize,
    verified_final_policy_step_eval_count: usize,
    verified_prefilter_decision_count: usize,
    verified_prefilter_kept_decision_count: usize,
    verified_prefilter_kept_candidate_count: usize,
    verified_prefilter_kept_rate: Option<f32>,
    verified_prefilter_average_kept_candidate_count: Option<f32>,
    verified_proposer_decision_count: usize,
    verified_proposer_non_rule_candidate_count: usize,
    verified_proposer_kept_candidate_count: usize,
    verified_proposer_keep_rate: Option<f32>,
    verified_average_scoped_candidate_count: f32,
    verified_adv_mean_on_overrides: Option<f32>,
    verified_harmful_override_count: usize,
    verified_harmful_override_rate: Option<f32>,
    verified_low_evidence_reject_count: usize,
    verified_confirm_decision_count: usize,
    verified_confirm_accept_count: usize,
    verified_confirm_reject_count: usize,
    verified_confirm_candidate_evaluation_count: usize,
    verified_confirm_policy_step_eval_count: usize,
    verified_artifact_confirm_decision_count: usize,
    verified_artifact_confirm_accept_count: usize,
    verified_artifact_confirm_reject_count: usize,
    verified_artifact_confirm_candidate_evaluation_count: usize,
    verified_artifact_confirm_policy_step_eval_count: usize,
    verified_min_adv_on_overrides: Option<f32>,
    verified_max_adv_on_overrides: Option<f32>,
    verified_decision_type_counts: BTreeMap<String, usize>,
    verified_override_decision_type_counts: BTreeMap<String, usize>,
    verified_decision_context_counts: BTreeMap<String, usize>,
    verified_override_context_counts: BTreeMap<String, usize>,
    verified_best_adv_bucket_counts: BTreeMap<String, usize>,
    verified_horizon_stop_reason_counts: BTreeMap<String, usize>,
    verified_payoff_reason_counts: BTreeMap<String, usize>,
    verified_override_payoff_reason_counts: BTreeMap<String, usize>,
    verified_horizon_artifact_reason_counts: BTreeMap<String, usize>,
    verified_missing_counts: BTreeMap<String, usize>,
    verified_cached_root_candidate_count: usize,
    verified_cached_root_exact_dedup_count: usize,
    verified_root_rule_equivalent_prune_count: usize,
    verified_cached_value_hit_count: usize,
    verified_cached_value_miss_count: usize,
    verified_cached_policy_step_eval_count: usize,
    verified_cached_cache_entry_count_max: usize,
    verified_parallelism_used_max: usize,
    verified_candidate_eval_wall_ms: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    verified_override_events: Vec<VerifiedOverrideEvent>,
}

#[derive(Clone, Debug, Serialize)]
struct VerifiedOverrideEvent {
    step: usize,
    decision_type: String,
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    context_keys: Vec<String>,
    rule_index: usize,
    selected_index: usize,
    rule_action_key: Option<String>,
    selected_action_key: Option<String>,
    rule_return: f32,
    selected_return: f32,
    adv_vs_rule: f32,
    oracle_margin: f32,
    horizon_decisions: usize,
    horizon_mode: String,
    horizon_stop_reason: Option<String>,
    payoff_reasons: Vec<String>,
    confirmation_kind: Option<String>,
    artifact_reasons: Vec<String>,
    scoped_candidate_count: usize,
    evaluated_candidate_count: usize,
    policy_step_eval_count: usize,
}

#[derive(Default)]
struct VerifiedAdvOverrideStats {
    decision_count: usize,
    scoped_candidate_count_sum: usize,
    override_count: usize,
    reject_count: usize,
    evaluated_candidate_count: usize,
    prefilter_candidate_evaluation_count: usize,
    final_candidate_evaluation_count: usize,
    prefilter_policy_step_eval_count: usize,
    final_policy_step_eval_count: usize,
    prefilter_decision_count: usize,
    prefilter_kept_decision_count: usize,
    prefilter_kept_candidate_count: usize,
    proposer_decision_count: usize,
    proposer_non_rule_candidate_count: usize,
    proposer_kept_candidate_count: usize,
    verified_adv_sum: f32,
    harmful_override_count: usize,
    low_evidence_reject_count: usize,
    confirm_decision_count: usize,
    confirm_accept_count: usize,
    confirm_reject_count: usize,
    confirm_candidate_evaluation_count: usize,
    confirm_policy_step_eval_count: usize,
    artifact_confirm_decision_count: usize,
    artifact_confirm_accept_count: usize,
    artifact_confirm_reject_count: usize,
    artifact_confirm_candidate_evaluation_count: usize,
    artifact_confirm_policy_step_eval_count: usize,
    decision_type_counts: BTreeMap<String, usize>,
    override_decision_type_counts: BTreeMap<String, usize>,
    decision_context_counts: BTreeMap<String, usize>,
    override_context_counts: BTreeMap<String, usize>,
    best_adv_bucket_counts: BTreeMap<String, usize>,
    horizon_stop_reason_counts: BTreeMap<String, usize>,
    payoff_reason_counts: BTreeMap<String, usize>,
    override_payoff_reason_counts: BTreeMap<String, usize>,
    horizon_artifact_reason_counts: BTreeMap<String, usize>,
    missing_counts: BTreeMap<String, usize>,
    cached_root_candidate_count: usize,
    cached_root_exact_dedup_count: usize,
    root_rule_equivalent_prune_count: usize,
    cached_value_hit_count: usize,
    cached_value_miss_count: usize,
    cached_policy_step_eval_count: usize,
    cached_cache_entry_count_max: usize,
    parallelism_used_max: usize,
    candidate_eval_wall_ms: u64,
    max_verified_adv: Option<f32>,
    min_verified_adv: Option<f32>,
    override_events: Vec<VerifiedOverrideEvent>,
}

#[derive(Debug, Serialize)]
struct PolicyPreviewPayload {
    schema_version: String,
    policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    live_env_unchanged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_before: Option<FullRunEnvState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_after: Option<FullRunEnvState>,
    chosen_action_index: Option<usize>,
    chosen_action_key: Option<String>,
    reward: f32,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_state: Option<FullRunEnvState>,
    info: FullRunEnvInfo,
}

#[derive(Clone, Copy)]
struct EvaluationOutputOptions {
    include_state: bool,
    include_next_state: bool,
    include_continuation_trace: bool,
    check_live_env_unchanged: bool,
}

impl Default for EvaluationOutputOptions {
    fn default() -> Self {
        Self {
            include_state: true,
            include_next_state: true,
            include_continuation_trace: true,
            check_live_env_unchanged: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvaluationMode {
    Independent,
    BellmanCachedV1,
}

impl EvaluationMode {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("independent").to_ascii_lowercase().as_str() {
            "independent" => Ok(Self::Independent),
            "bellman_cached_v1" => Ok(Self::BellmanCachedV1),
            other => Err(format!(
                "unsupported evaluation_mode '{other}'; expected independent or bellman_cached_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Independent => "independent",
            Self::BellmanCachedV1 => "bellman_cached_v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueCacheScope {
    Request,
    Episode,
}

impl ValueCacheScope {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("request").to_ascii_lowercase().as_str() {
            "request" => Ok(Self::Request),
            "episode" => Ok(Self::Episode),
            other => Err(format!(
                "unsupported value_cache_scope '{other}'; expected request or episode"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Episode => "episode",
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum HorizonMode {
    FixedDecisions,
    AdaptiveNextPlayerTurnV1,
    AdaptivePayoffWindowV1,
    CombatEndV1,
}

impl HorizonMode {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("fixed_decisions").to_ascii_lowercase().as_str() {
            "" | "fixed" | "fixed_decisions" => Ok(Self::FixedDecisions),
            "adaptive_next_player_turn_v1" | "next_player_turn_v1" => {
                Ok(Self::AdaptiveNextPlayerTurnV1)
            }
            "adaptive_payoff_window_v1" | "payoff_window_v1" => {
                Ok(Self::AdaptivePayoffWindowV1)
            }
            "combat_end_v1" | "combat_end" => Ok(Self::CombatEndV1),
            other => Err(format!(
                "unsupported horizon_mode '{other}'; expected fixed_decisions, adaptive_next_player_turn_v1, adaptive_payoff_window_v1, or combat_end_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FixedDecisions => "fixed_decisions",
            Self::AdaptiveNextPlayerTurnV1 => "adaptive_next_player_turn_v1",
            Self::AdaptivePayoffWindowV1 => "adaptive_payoff_window_v1",
            Self::CombatEndV1 => "combat_end_v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VerifiedStrategy {
    SingleStage,
    TwoStagePrefilterV1,
    ModelProposerV1,
}

impl VerifiedStrategy {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("single_stage").to_ascii_lowercase().as_str() {
            "" | "single" | "single_stage" => Ok(Self::SingleStage),
            "two_stage_prefilter_v1" | "two_stage" => Ok(Self::TwoStagePrefilterV1),
            "model_proposer_v1" | "model_proposer" => Ok(Self::ModelProposerV1),
            other => Err(format!(
                "unsupported verifier_strategy '{other}'; expected single_stage, two_stage_prefilter_v1, or model_proposer_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SingleStage => "single_stage",
            Self::TwoStagePrefilterV1 => "two_stage_prefilter_v1",
            Self::ModelProposerV1 => "model_proposer_v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceGate {
    None,
    HorizonCapNoPayoffV1,
    HorizonCapAnyV1,
}

impl EvidenceGate {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("horizon_cap_no_payoff_v1").to_ascii_lowercase().as_str() {
            "" | "none" => Ok(Self::None),
            "horizon_cap_no_payoff_v1" | "no_payoff" => Ok(Self::HorizonCapNoPayoffV1),
            "horizon_cap_any_v1" | "horizon_cap_any" => Ok(Self::HorizonCapAnyV1),
            other => Err(format!(
                "unsupported evidence_gate '{other}'; expected none, horizon_cap_no_payoff_v1, or horizon_cap_any_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::HorizonCapNoPayoffV1 => "horizon_cap_no_payoff_v1",
            Self::HorizonCapAnyV1 => "horizon_cap_any_v1",
        }
    }
}

#[derive(Clone, Copy)]
struct EvaluationRuntimeOptions {
    mode: EvaluationMode,
    cache_scope: ValueCacheScope,
    cache_max_entries: usize,
    parallelism: usize,
    exact_root_dedup: bool,
}

#[derive(Clone)]
struct VerifiedAdvOverrideOptions {
    candidate_scope: CandidateScope,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    oracle_margin: f32,
    strategy: VerifiedStrategy,
    prefilter_horizon_decisions: usize,
    prefilter_horizon_mode: HorizonMode,
    prefilter_margin: f32,
    prefilter_top_k: usize,
    proposer: Option<VerifiedProposerOptions>,
    gamma: f32,
    evidence_gate: EvidenceGate,
    low_evidence_margin: Option<f32>,
    confirm_low_evidence: Option<LowEvidenceConfirmationOptions>,
    runtime: EvaluationRuntimeOptions,
}

#[derive(Clone, Copy)]
struct LowEvidenceConfirmationOptions {
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    margin: f32,
}

#[derive(Clone)]
struct VerifiedProposerOptions {
    model_path: String,
    top_k: usize,
    threshold: f32,
    model: Arc<VerifiedProposerModel>,
}

#[derive(Debug, Deserialize)]
struct PortableMlpProposerJson {
    schema_version: String,
    model_type: String,
    feature_set: String,
    feature_dim: usize,
    activation: String,
    out_activation: String,
    input_weights: Vec<Vec<f32>>,
    hidden_bias: Vec<f32>,
    output_weights: Vec<f32>,
    output_bias: f32,
}

#[derive(Debug)]
struct VerifiedProposerModel {
    feature_dim: usize,
    input_weights: Vec<Vec<f32>>,
    hidden_bias: Vec<f32>,
    output_weights: Vec<f32>,
    output_bias: f32,
}

impl VerifiedProposerModel {
    fn load_json(path: &str) -> Result<Self, String> {
        let path_buf = PathBuf::from(path);
        let bytes = fs::read(&path_buf)
            .map_err(|err| format!("failed to read proposer model '{}': {err}", path))?;
        let payload: PortableMlpProposerJson = serde_json::from_slice(&bytes)
            .map_err(|err| format!("failed to parse proposer model '{}': {err}", path))?;
        if payload.schema_version != "verified_proposer_mlp_json_v0"
            || payload.model_type != "verified_proposer_mlp_json_v0"
        {
            return Err(format!(
                "unsupported proposer model schema/model_type: {}/{}",
                payload.schema_version, payload.model_type
            ));
        }
        if payload.feature_set != "candidate_only" {
            return Err(format!(
                "Rust verified proposer currently supports feature_set=candidate_only, got {}",
                payload.feature_set
            ));
        }
        if payload.activation != "relu" || payload.out_activation != "logistic" {
            return Err(format!(
                "unsupported proposer activations: hidden={}, output={}",
                payload.activation, payload.out_activation
            ));
        }
        let hidden_dim = payload.hidden_bias.len();
        if hidden_dim == 0 {
            return Err("proposer hidden layer is empty".to_string());
        }
        if payload.output_weights.len() != hidden_dim {
            return Err("proposer output_weights length does not match hidden_bias".to_string());
        }
        if payload.input_weights.len() != payload.feature_dim {
            return Err(format!(
                "proposer input_weights length {} does not match feature_dim {}",
                payload.input_weights.len(),
                payload.feature_dim
            ));
        }
        if payload
            .input_weights
            .iter()
            .any(|row| row.len() != hidden_dim)
        {
            return Err("proposer input weight row has wrong hidden dimension".to_string());
        }
        Ok(Self {
            feature_dim: payload.feature_dim,
            input_weights: payload.input_weights,
            hidden_bias: payload.hidden_bias,
            output_weights: payload.output_weights,
            output_bias: payload.output_bias,
        })
    }

    fn predict_candidate_only(
        &self,
        candidate: &RunActionCandidate,
        rule_candidate: &RunActionCandidate,
    ) -> f32 {
        let sparse = candidate_only_sparse_features(candidate, rule_candidate, self.feature_dim);
        let mut hidden = self.hidden_bias.clone();
        for (feature_index, value) in sparse {
            if let Some(row) = self.input_weights.get(feature_index) {
                for (slot, weight) in hidden.iter_mut().zip(row.iter()) {
                    *slot += value * *weight;
                }
            }
        }
        let mut output = self.output_bias;
        for (activation, weight) in hidden.iter().zip(self.output_weights.iter()) {
            output += activation.max(0.0) * *weight;
        }
        sigmoid_f32(output)
    }
}

fn candidate_only_sparse_features(
    candidate: &RunActionCandidate,
    rule_candidate: &RunActionCandidate,
    feature_dim: usize,
) -> HashMap<usize, f32> {
    let mut sparse = HashMap::new();
    add_compact_candidate_features(&mut sparse, "", candidate, feature_dim);
    add_action_only_features(&mut sparse, "", candidate, feature_dim);
    add_compact_candidate_features(&mut sparse, "rule.", rule_candidate, feature_dim);
    add_action_only_features(&mut sparse, "rule.", rule_candidate, feature_dim);
    sparse
}

fn add_feature(sparse: &mut HashMap<usize, f32>, token: String, value: f32, feature_dim: usize) {
    if value == 0.0 || feature_dim == 0 {
        return;
    }
    let index = hash_feature_blake2b(&token, feature_dim);
    *sparse.entry(index).or_insert(0.0) += value;
}

fn add_cat(sparse: &mut HashMap<usize, f32>, prefix: &str, token: String, feature_dim: usize) {
    add_feature(sparse, format!("{prefix}{token}"), 1.0, feature_dim);
}

fn add_num(
    sparse: &mut HashMap<usize, f32>,
    prefix: &str,
    name: &str,
    value: f32,
    width: f32,
    feature_dim: usize,
) {
    if !value.is_finite() {
        return;
    }
    let safe_width = width.max(1.0);
    add_feature(
        sparse,
        format!("{prefix}candidate.{name}.num"),
        (value / (safe_width * 10.0)).tanh(),
        feature_dim,
    );
    add_cat(
        sparse,
        prefix,
        format!(
            "candidate.{name}.bucket:{}",
            (value / safe_width).floor() as i32
        ),
        feature_dim,
    );
}

fn add_compact_candidate_features(
    sparse: &mut HashMap<usize, f32>,
    prefix: &str,
    candidate: &RunActionCandidate,
    feature_dim: usize,
) {
    let key = candidate.action_key.as_str();
    add_cat(
        sparse,
        prefix,
        format!(
            "candidate.action_type:{}",
            trace_action_type(&candidate.action, key)
        ),
        feature_dim,
    );
    if let Some(target) = extract_action_key_segment(key, "target") {
        if !target.is_empty() {
            add_cat(
                sparse,
                prefix,
                format!("candidate.target:{target}"),
                feature_dim,
            );
        }
    }
    if candidate.dominated {
        add_cat(
            sparse,
            prefix,
            "candidate.dominated:true".to_string(),
            feature_dim,
        );
    }
    add_num(
        sparse,
        prefix,
        "action_index",
        candidate.action_index as f32,
        4.0,
        feature_dim,
    );

    if let Some(card) = &candidate.card {
        add_cat(
            sparse,
            prefix,
            format!("candidate.card_card_id:{}", card.card_id),
            feature_dim,
        );
        add_cat(
            sparse,
            prefix,
            format!("candidate.card_card_type_id:{}", card.card_type_id),
            feature_dim,
        );
        add_cat(
            sparse,
            prefix,
            format!("candidate.card_rarity_id:{}", card.rarity_id),
            feature_dim,
        );
        add_cat(
            sparse,
            prefix,
            format!("candidate.card_cost:{}", card.cost),
            feature_dim,
        );
        add_cat(
            sparse,
            prefix,
            format!("candidate.card_upgrades:{}", card.upgrades),
            feature_dim,
        );
        for (enabled, name) in [
            (card.starter_basic, "starter_basic"),
            (card.aoe, "aoe"),
            (card.multi_damage, "multi_damage"),
            (card.draws_cards, "draws_cards"),
            (card.gains_energy, "gains_energy"),
            (card.exhaust, "exhaust"),
            (card.ethereal, "ethereal"),
            (card.applies_vulnerable, "applies_vulnerable"),
            (card.applies_weak, "applies_weak"),
            (card.scaling_piece, "scaling_piece"),
        ] {
            if enabled {
                add_cat(
                    sparse,
                    prefix,
                    format!("candidate.card_{name}:true"),
                    feature_dim,
                );
            }
        }
        add_num(
            sparse,
            prefix,
            "card_base_damage",
            card.base_damage as f32,
            4.0,
            feature_dim,
        );
        add_num(
            sparse,
            prefix,
            "card_base_block",
            card.base_block as f32,
            4.0,
            feature_dim,
        );
        add_num(
            sparse,
            prefix,
            "card_base_magic",
            card.base_magic as f32,
            2.0,
            feature_dim,
        );
        add_num(
            sparse,
            prefix,
            "card_deck_copies",
            card.deck_copies as f32,
            1.0,
            feature_dim,
        );
    }
}

fn add_action_only_features(
    sparse: &mut HashMap<usize, f32>,
    prefix: &str,
    candidate: &RunActionCandidate,
    feature_dim: usize,
) {
    let key = candidate.action_key.as_str();
    if key.starts_with("combat/end_turn") {
        add_cat(sparse, prefix, "action:end_turn".to_string(), feature_dim);
    } else if key.starts_with("combat/play_card") {
        add_cat(sparse, prefix, "action:play_card".to_string(), feature_dim);
        if let Some(card) = extract_action_key_segment(key, "card") {
            if !card.is_empty() {
                add_cat(sparse, prefix, format!("card:{card}"), feature_dim);
            }
        }
    } else if key.starts_with("combat/use_potion") {
        add_cat(sparse, prefix, "action:use_potion".to_string(), feature_dim);
    } else {
        let head = key
            .split_once('/')
            .map(|(head, _)| head)
            .unwrap_or("unknown");
        add_cat(sparse, prefix, format!("action:{head}"), feature_dim);
    }
}

fn extract_action_key_segment<'a>(key: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}:");
    key.split('/')
        .find_map(|part| part.strip_prefix(marker.as_str()))
}

fn trace_action_type(
    action: &sts_simulator::cli::full_run_smoke::TraceClientInput,
    _key: &str,
) -> &'static str {
    use sts_simulator::cli::full_run_smoke::TraceClientInput::*;
    match action {
        PlayCard { .. } => "play_card",
        UsePotion { .. } => "use_potion",
        DiscardPotion { .. } => "discard_potion",
        EndTurn => "end_turn",
        SubmitCardChoice { .. } => "submit_card_choice",
        SubmitDiscoverChoice { .. } => "submit_discover_choice",
        SelectMapNode { .. } => "select_map_node",
        FlyToNode { .. } => "fly_to_node",
        SelectEventOption { .. } => "select_event_option",
        CampfireOption { .. } => "campfire_option",
        EventChoice { .. } => "event_choice",
        SubmitScryDiscard { .. } => "submit_scry_discard",
        SubmitSelection { .. } => "submit_selection",
        SubmitHandSelect { .. } => "submit_hand_select",
        SubmitGridSelect { .. } => "submit_grid_select",
        SubmitDeckSelect { .. } => "submit_deck_select",
        ClaimReward { .. } => "claim_reward",
        SelectCard { .. } => "select_card",
        BuyCard { .. } => "buy_card",
        BuyRelic { .. } => "buy_relic",
        BuyPotion { .. } => "buy_potion",
        PurgeCard { .. } => "purge_card",
        SubmitRelicChoice { .. } => "submit_relic_choice",
        Proceed => "proceed",
        Cancel => "cancel",
    }
}

fn hash_feature_blake2b(token: &str, feature_dim: usize) -> usize {
    let mut hasher = Blake2bVar::new(8).expect("8-byte blake2b output is valid");
    hasher.update(token.as_bytes());
    let mut out = [0u8; 8];
    hasher
        .finalize_variable(&mut out)
        .expect("fixed blake2b output buffer has correct length");
    (u64::from_be_bytes(out) as usize) % feature_dim
}

fn sigmoid_f32(value: f32) -> f32 {
    if value >= 0.0 {
        let z = (-value).exp();
        1.0 / (1.0 + z)
    } else {
        let z = value.exp();
        z / (1.0 + z)
    }
}

impl Default for EvaluationRuntimeOptions {
    fn default() -> Self {
        Self {
            mode: EvaluationMode::Independent,
            cache_scope: ValueCacheScope::Request,
            cache_max_entries: 4096,
            parallelism: 1,
            exact_root_dedup: false,
        }
    }
}

#[derive(Default)]
struct DriverSession {
    env: Option<FullRunEnv>,
    episode_value_cache: ValueCache,
}

#[derive(Clone)]
struct SuffixValue {
    discounted_return: f32,
    continuation_steps: usize,
    continuation_action_keys: Vec<String>,
    rollout_done: bool,
    rollout_terminal_reason: String,
    horizon_stop_reason: String,
    final_info: FullRunEnvInfo,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
struct PayoffHorizonProfile {
    post_turn_normal_decision_budget: usize,
    reasons: Vec<&'static str>,
}

struct ValueCacheEntry {
    env: FullRunEnv,
    value: SuffixValue,
}

#[derive(Default)]
struct ValueCache {
    buckets: HashMap<u64, Vec<ValueCacheEntry>>,
    entry_count: usize,
}

impl ValueCache {
    fn clear(&mut self) {
        self.buckets.clear();
        self.entry_count = 0;
    }

    fn get(
        &self,
        env: &FullRunEnv,
        continuation_policy: RunPolicyKind,
        horizon_decisions: usize,
        horizon_mode: HorizonMode,
        gamma: f32,
        include_trace: bool,
    ) -> Option<SuffixValue> {
        let bucket = cache_bucket(
            env,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            include_trace,
        );
        self.buckets.get(&bucket).and_then(|entries| {
            entries
                .iter()
                .find(|entry| &entry.env == env)
                .map(|entry| entry.value.clone())
        })
    }

    fn insert(
        &mut self,
        env: FullRunEnv,
        continuation_policy: RunPolicyKind,
        horizon_decisions: usize,
        horizon_mode: HorizonMode,
        gamma: f32,
        include_trace: bool,
        value: SuffixValue,
        max_entries: usize,
    ) {
        if max_entries == 0 || self.entry_count >= max_entries {
            return;
        }
        let bucket = cache_bucket(
            &env,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            include_trace,
        );
        self.buckets
            .entry(bucket)
            .or_default()
            .push(ValueCacheEntry { env, value });
        self.entry_count += 1;
    }
}

#[derive(Default)]
struct EvaluationStats {
    root_candidate_count: usize,
    root_exact_dedup_count: usize,
    root_rule_equivalent_prune_count: usize,
    value_cache_hit_count: usize,
    value_cache_miss_count: usize,
    policy_step_eval_count: usize,
    parallelism_used: usize,
}

impl EvaluationStats {
    fn merge(&mut self, other: EvaluationStats) {
        self.root_candidate_count += other.root_candidate_count;
        self.root_exact_dedup_count += other.root_exact_dedup_count;
        self.root_rule_equivalent_prune_count += other.root_rule_equivalent_prune_count;
        self.value_cache_hit_count += other.value_cache_hit_count;
        self.value_cache_miss_count += other.value_cache_miss_count;
        self.policy_step_eval_count += other.policy_step_eval_count;
        self.parallelism_used = self.parallelism_used.max(other.parallelism_used);
    }
}

#[derive(Clone, Copy)]
struct PreviewOutputOptions {
    include_state: bool,
    include_next_state: bool,
    check_live_env_unchanged: bool,
}

impl Default for PreviewOutputOptions {
    fn default() -> Self {
        Self {
            include_state: true,
            include_next_state: true,
            check_live_env_unchanged: true,
        }
    }
}

#[derive(Debug, Serialize)]
struct CandidateEvaluation {
    action_index: usize,
    candidate: Option<RunActionCandidate>,
    ok: bool,
    error: Option<String>,
    chosen_action_key: Option<String>,
    one_step_reward: f32,
    discounted_return: f32,
    next_state: Option<FullRunEnvState>,
    done: bool,
    terminal_reason: String,
    continuation_steps: usize,
    continuation_action_keys: Vec<String>,
    rollout_done: bool,
    rollout_terminal_reason: String,
    horizon_stop_reason: String,
    payoff_reasons: Vec<String>,
    final_info: Option<FullRunEnvInfo>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = DriverSession::default();
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<DriverRequest>(&line);
        let should_close = matches!(request.as_ref(), Ok(DriverRequest::Close));
        let response = match request {
            Ok(request) => handle_request(&mut session, request),
            Err(err) => error_response(format!("invalid request: {err}")),
        };
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        if should_close {
            break;
        }
    }

    Ok(())
}

fn handle_request(session: &mut DriverSession, request: DriverRequest) -> DriverResponse {
    match request {
        DriverRequest::Ping => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            done: None,
            chosen_action_key: None,
            info: None,
        },
        DriverRequest::Close => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            done: None,
            chosen_action_key: None,
            info: session.env.as_ref().map(|current| current.info()),
        },
        DriverRequest::Reset {
            seed,
            ascension,
            final_act,
            class,
            max_steps,
            reward_shaping_profile,
        } => {
            let player_class = match normalize_player_class(class.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let config = FullRunEnvConfig {
                seed: seed.unwrap_or(1),
                ascension: ascension.unwrap_or(0),
                final_act: final_act.unwrap_or(false),
                player_class,
                max_steps: max_steps.unwrap_or(5000),
                reward_shaping_profile: match reward_shaping_profile {
                    Some(value) => match RewardShapingProfile::parse(&value) {
                        Ok(profile) => profile,
                        Err(err) => return error_response(err),
                    },
                    None => RewardShapingProfile::Baseline,
                },
            };
            match FullRunEnv::new(config) {
                Ok(mut next_env) => match next_env.state() {
                    Ok(state) => {
                        let info = next_env.info();
                        let done = info.result != "ongoing";
                        session.episode_value_cache.clear();
                        session.env = Some(next_env);
                        DriverResponse {
                            ok: true,
                            error: None,
                            payload: Some(state_payload(state)),
                            reward: Some(0.0),
                            done: Some(done),
                            chosen_action_key: None,
                            info: session.env.as_ref().map(|current| current.info()),
                        }
                    }
                    Err(err) => error_response(err),
                },
                Err(err) => error_response(err),
            }
        }
        DriverRequest::Observation => match session.env.as_mut() {
            Some(current) => match current.state() {
                Ok(state) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(state_payload(state)),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::DecisionEnvObservation => match session.env.as_mut() {
            Some(current) => match DecisionEnv::current_timestep(current) {
                Ok(timestep) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(
                        serde_json::to_value(timestep)
                            .expect("decision env timestep should serialize"),
                    ),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err.to_string()),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::PolicyInput { time_budget_ms } => match session.env.as_mut() {
            Some(current) => match DecisionEnv::current_timestep(current).and_then(|timestep| {
                PolicyInput::from_timestep(&timestep, time_budget_ms.unwrap_or(25))
            }) {
                Ok(policy_input) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(
                        serde_json::to_value(policy_input).expect("policy input should serialize"),
                    ),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err.to_string()),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::BranchTraceCacheIdentity => match session.env.as_mut() {
            Some(current) => match DecisionEnv::current_timestep(current) {
                Ok(timestep) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(json!({
                        "schema_version": "branch_trace_cache_identity_v1",
                        "decision_id": timestep.decision_id,
                        "state_hash": timestep.info.state_hash,
                        "rng_state_hash": branch_rng_state_hash(current),
                        "candidate_count": timestep.candidates.len(),
                        "candidate_action_keys": timestep
                            .candidates
                            .iter()
                            .map(|candidate| candidate.action_key.clone())
                            .collect::<Vec<_>>(),
                    })),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err.to_string()),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::NeutralPolicyTrace {
            time_budget_ms,
            max_branch_depth,
            max_candidates,
            reference_action_id,
        } => match session.env.as_mut() {
            Some(current) => {
                let timestep = match DecisionEnv::current_timestep(current) {
                    Ok(timestep) => timestep,
                    Err(err) => return error_response(err.to_string()),
                };
                let policy_input =
                    match PolicyInput::from_timestep(&timestep, time_budget_ms.unwrap_or(25)) {
                        Ok(policy_input) => policy_input,
                        Err(err) => return error_response(err.to_string()),
                    };
                let context_parts = match current.current_combat_decision_context_parts() {
                    Ok(parts) => parts,
                    Err(err) => return error_response(err),
                };
                let Some((engine, combat, candidates)) = context_parts else {
                    return DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(json!({
                            "schema_version": "neutral_policy_trace_driver_v0",
                            "supported": false,
                            "reason": "current_decision_is_not_combat_engine_decision",
                            "policy_input": policy_input,
                        })),
                        reward: None,
                        done: Some(current.info().result != "ongoing"),
                        chosen_action_key: None,
                        info: Some(current.info()),
                    };
                };
                let mut runner = NeutralProbeEvaluator::default();
                if let Some(max_branch_depth) = max_branch_depth {
                    runner.config.max_branch_depth = max_branch_depth;
                }
                if let Some(max_candidates) = max_candidates {
                    runner.config.max_candidates = max_candidates;
                }
                let execution_context = SearchExecutionContext::from_policy_input(
                    &policy_input,
                    engine,
                    combat,
                    candidates,
                );
                let trace = runner.deliberate(&policy_input, &execution_context);
                let signal_candidate_id = short_horizon_signal_candidate_id(&trace);
                let paired_compare_vs_reference = signal_candidate_id.and_then(|selected| {
                    let reference = ActionId(reference_action_id?);
                    (selected != reference)
                        .then(|| {
                            runner
                                .query
                                .paired_compare(&execution_context, selected, reference)
                        })
                        .flatten()
                });
                let commutation_probe_vs_reference = signal_candidate_id.and_then(|selected| {
                    let reference = ActionId(reference_action_id?);
                    (selected != reference)
                        .then(|| {
                            runner
                                .query
                                .commutation_probe(&execution_context, selected, reference)
                        })
                        .flatten()
                });
                let isolated_enemy_response_public_probe_vs_reference = signal_candidate_id
                    .and_then(|selected| {
                        let reference = ActionId(reference_action_id?);
                        (selected != reference)
                            .then(|| {
                                runner.query.enemy_response_public_probe(
                                    &execution_context,
                                    selected,
                                    reference,
                                )
                            })
                            .flatten()
                    });
                let aligned_enemy_response_public_probe_vs_reference = signal_candidate_id
                    .and_then(|selected| {
                        let reference = ActionId(reference_action_id?);
                        (selected != reference)
                            .then(|| {
                                runner.query.aligned_enemy_response_public_probe(
                                    &execution_context,
                                    selected,
                                    reference,
                                )
                            })
                            .flatten()
                    });
                let paired_compare_value = paired_compare_vs_reference
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok());
                let commutation_probe_value = commutation_probe_vs_reference
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok());
                let reference_suffix_replay_probe = commutation_probe_value
                    .as_ref()
                    .map(reference_suffix_replay_probe_from_commutation);
                let isolated_enemy_response_public_probe_value =
                    isolated_enemy_response_public_probe_vs_reference
                        .as_ref()
                        .and_then(|value| serde_json::to_value(value).ok());
                let aligned_enemy_response_public_probe_value =
                    aligned_enemy_response_public_probe_vs_reference
                        .as_ref()
                        .and_then(|value| serde_json::to_value(value).ok());
                let disagreement_audit = neutral_disagreement_audit(
                    &policy_input,
                    &trace,
                    reference_action_id.map(ActionId),
                    paired_compare_value.clone(),
                    commutation_probe_value.clone(),
                    reference_suffix_replay_probe.clone(),
                    isolated_enemy_response_public_probe_value.clone(),
                    aligned_enemy_response_public_probe_value.clone(),
                );
                DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(json!({
                        "schema_version": "neutral_policy_trace_driver_v0",
                        "supported": true,
                        "policy_input": policy_input,
                        "trace": trace,
                        "summary": neutral_policy_trace_summary(&trace),
                        "paired_compare_vs_reference": paired_compare_vs_reference,
                        "commutation_probe_vs_reference": commutation_probe_vs_reference,
                        "reference_suffix_replay_probe": reference_suffix_replay_probe,
                        "isolated_enemy_response_public_probe_vs_reference": isolated_enemy_response_public_probe_vs_reference,
                        "aligned_enemy_response_public_probe_vs_reference": aligned_enemy_response_public_probe_vs_reference,
                        "disagreement_audit": disagreement_audit,
                    })),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                }
            }
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::Step { action_index } => match session.env.as_mut() {
            Some(current) => match current.step(action_index) {
                Ok(step) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(state_payload(step.state)),
                    reward: Some(step.reward),
                    done: Some(step.done),
                    chosen_action_key: step.chosen_action_key,
                    info: Some(step.info),
                },
                Err(err) => error_response(err),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::DecisionEnvStep { action_id } => match session.env.as_mut() {
            Some(current) => match DecisionEnv::step(current, ActionId(action_id)) {
                Ok(timestep) => {
                    let reward = timestep.reward.scalar_reward;
                    let done = timestep.terminated || timestep.truncated;
                    let chosen_action_key = timestep
                        .reward
                        .components
                        .get("chosen_action_key")
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                    DriverResponse {
                        ok: true,
                        error: None,
                        reward: Some(reward),
                        done: Some(done),
                        chosen_action_key,
                        info: Some(current.info()),
                        payload: Some(
                            serde_json::to_value(timestep)
                                .expect("decision env timestep should serialize"),
                        ),
                    }
                }
                Err(err) => error_response(err.to_string()),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::DecisionRecordStep {
            action_id,
            sim_version,
            return_spec_version,
            context,
            teacher_continuation_policy,
            teacher_horizon_decisions,
            teacher_horizon_mode,
            teacher_gamma,
            teacher_evaluation_mode,
            teacher_value_cache_scope,
            teacher_value_cache_max_entries,
            teacher_parallelism,
            teacher_exact_root_dedup,
        } => match session.env.as_mut() {
            Some(current) => {
                let seed = current.info().seed;
                let decision = match DecisionEnv::current_timestep(current) {
                    Ok(timestep) => timestep,
                    Err(err) => return error_response(err.to_string()),
                };
                let teacher_label = match build_teacher_label_for_decision_record(
                    current,
                    &mut session.episode_value_cache,
                    &decision,
                    teacher_continuation_policy,
                    teacher_horizon_decisions,
                    teacher_horizon_mode,
                    teacher_gamma,
                    teacher_evaluation_mode,
                    teacher_value_cache_scope,
                    teacher_value_cache_max_entries,
                    teacher_parallelism,
                    teacher_exact_root_dedup,
                    return_spec_version.as_deref().unwrap_or("driver_reward_v0"),
                ) {
                    Ok(label) => label,
                    Err(err) => return error_response(err),
                };
                let outcome = match DecisionEnv::step(current, ActionId(action_id)) {
                    Ok(timestep) => timestep,
                    Err(err) => return error_response(err.to_string()),
                };
                let mut record_context = DecisionRecordContext::new(
                    sim_version.unwrap_or_else(|| "full_run_env".to_string()),
                    return_spec_version.unwrap_or_else(|| "driver_reward_v0".to_string()),
                    seed,
                );
                record_context.behavior_action = Some(ActionId(action_id));
                record_context.teacher_label = teacher_label;
                record_context.info =
                    context.unwrap_or_else(|| json!({"source": "full_run_env_driver"}));
                let record =
                    DecisionRecord::from_decision_and_outcome(&decision, &outcome, record_context);
                DriverResponse {
                    ok: true,
                    error: None,
                    reward: Some(record.reward_since_prev.scalar_reward),
                    done: Some(record.terminated || record.truncated),
                    chosen_action_key: record
                        .reward_since_prev
                        .components
                        .get("chosen_action_key")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    info: Some(current.info()),
                    payload: Some(
                        serde_json::to_value(record).expect("decision record should serialize"),
                    ),
                }
            }
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::StepPolicy { policy } => {
            let policy_kind = match normalize_policy(&policy) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            match session.env.as_mut() {
                Some(current) => match current.step_policy(policy_kind) {
                    Ok(step) => DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(state_payload(step.state)),
                        reward: Some(step.reward),
                        done: Some(step.done),
                        chosen_action_key: step.chosen_action_key,
                        info: Some(step.info),
                    },
                    Err(err) => error_response(err),
                },
                None => {
                    error_response("full-run env not initialized; send reset first".to_string())
                }
            }
        }
        DriverRequest::BranchTrace {
            action_indices,
            candidate_scope,
            candidate_sampling_spec_id,
            candidate_cap,
            behavior_action_id,
            sampling_seed,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            sim_version,
            content_version,
            max_steps,
            include_comparisons,
        } => {
            let continuation_policy = match normalize_policy(
                continuation_policy.as_deref().unwrap_or("rule_baseline_v0"),
            ) {
                Ok(policy) => policy,
                Err(err) => return error_response(err),
            };
            let candidate_scope = match BranchCandidateScope::parse(candidate_scope.as_deref()) {
                Ok(scope) => scope,
                Err(err) => return error_response(err),
            };
            let horizon_mode = match BranchHorizonMode::parse(horizon_mode.as_deref()) {
                Ok(mode) => mode,
                Err(err) => return error_response(err),
            };
            match session.env.as_mut() {
                Some(current) => {
                    let decision = match DecisionEnv::current_timestep(current) {
                        Ok(decision) => decision,
                        Err(err) => return error_response(err.to_string()),
                    };
                    let stable_live = current.clone();
                    let horizon_decisions = max_steps
                        .map(|cap| horizon_decisions.min(cap))
                        .unwrap_or(horizon_decisions);
                    let config = BranchEvaluatorConfig {
                        action_indices,
                        candidate_scope,
                        continuation_policy,
                        horizon_decisions,
                        horizon_mode,
                        sim_version: sim_version.as_deref().unwrap_or("sim_current").to_string(),
                        content_version: content_version
                            .as_deref()
                            .unwrap_or("content_current")
                            .to_string(),
                    };
                    let output = match BranchEvaluator::evaluate_current(
                        &stable_live,
                        &decision,
                        &config,
                        include_comparisons.unwrap_or(true),
                    ) {
                        Ok(output) => output,
                        Err(err) => return error_response(err),
                    };
                    let live_env_unchanged = *current == stable_live;
                    let validation_report =
                        validate_branch_dataset(&output.traces, &output.comparisons);
                    let behavior_action_included = behavior_action_id
                        .is_some_and(|action_id| output.action_indices.contains(&action_id));
                    let candidate_sampling_spec = json!({
                        "schema_version": "candidate_sampling_spec_v1",
                        "candidate_sampling_spec_id": candidate_sampling_spec_id
                            .as_deref()
                            .unwrap_or("explicit_action_indices_then_scope_filter_v1"),
                        "scope": candidate_scope.as_str(),
                        "source": "explicit_action_indices_then_scope_filter",
                        "candidate_cap": candidate_cap,
                        "sampling_seed": sampling_seed,
                        "behavior_action_id": behavior_action_id,
                        "include_behavior_action": behavior_action_included,
                        "requested_action_count": output.sampling_summary.requested_action_count,
                        "legal_candidate_count": output.sampling_summary.legal_candidate_count,
                        "included_candidate_count": output.sampling_summary.included_candidate_count,
                        "excluded_candidate_count": output.sampling_summary.invalid_index_count + output.sampling_summary.scope_filtered_count,
                        "excluded_by_reason": {
                            "invalid_index": output.sampling_summary.invalid_index_count,
                            "scope_filter": output.sampling_summary.scope_filtered_count
                        },
                        "uses_neutral_signal": false,
                        "uses_legacy_best_move": false,
                        "uses_exact_turn_best_line": false,
                        "uses_frontier_eval_score": false
                    });
                    let paired_scenario_spec = json!({
                        "schema_version": PAIRED_SCENARIO_SCHEMA_VERSION,
                        "pairing_mode": "same_initial_env_seed_single_scenario_v0",
                        "common_random_policy": "shared_initial_rng_no_realignment_v0",
                        "scenario_seed_id": output.traces.first().map(|trace| trace.scenario_seed_id.clone()),
                        "paired_seed_id": output.traces.first().map(|trace| format!("seed:{}", trace.seed)),
                        "rng_divergence_recorded": true,
                        "note": "branches share initial exact env and RNG; later RNG divergence is measured, not realigned"
                    });
                    DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(json!({
                            "schema_version": "branch_trace_batch_v1",
                            "branch_trace_schema_version": BRANCH_TRACE_SCHEMA_VERSION,
                            "decision_id": decision.decision_id,
                            "horizon_decisions": horizon_decisions,
                            "horizon_mode": horizon_mode.as_str(),
                            "continuation_policy": policy_name(continuation_policy),
                            "candidate_scope": candidate_scope.as_str(),
                            "candidate_sampling_spec": candidate_sampling_spec,
                            "paired_scenario_spec": paired_scenario_spec,
                            "requested_action_indices": config.action_indices,
                            "action_indices": output.action_indices,
                            "trace_count": output.traces.len(),
                            "comparison_count": output.comparisons.len(),
                            "live_env_unchanged": live_env_unchanged,
                            "validation_report": validation_report,
                            "traces": output.traces,
                            "comparisons": output.comparisons,
                        })),
                        reward: None,
                        done: Some(current.info().result != "ongoing"),
                        chosen_action_key: None,
                        info: Some(current.info()),
                    }
                }
                None => {
                    error_response("full-run env not initialized; send reset first".to_string())
                }
            }
        }
        DriverRequest::EvaluateCandidates {
            action_indices,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            evaluation_mode,
            value_cache_scope,
            value_cache_max_entries,
            parallelism,
            exact_root_dedup,
            include_state,
            include_next_state,
            include_continuation_trace,
            check_live_env_unchanged,
        } => {
            let policy_kind = match normalize_policy(&continuation_policy) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let mode = match EvaluationMode::parse(evaluation_mode.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let cache_scope = match ValueCacheScope::parse(value_cache_scope.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let horizon_mode = match HorizonMode::parse(horizon_mode.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let runtime = EvaluationRuntimeOptions {
                mode,
                cache_scope,
                cache_max_entries: value_cache_max_entries.unwrap_or(4096),
                parallelism: parallelism.unwrap_or(1),
                exact_root_dedup: exact_root_dedup.unwrap_or(false),
            };
            match session.env.as_mut() {
                Some(current) => match evaluate_candidates(
                    current,
                    &mut session.episode_value_cache,
                    action_indices,
                    policy_kind,
                    horizon_decisions,
                    horizon_mode,
                    gamma,
                    runtime,
                    EvaluationOutputOptions {
                        include_state: include_state.unwrap_or(true),
                        include_next_state: include_next_state.unwrap_or(true),
                        include_continuation_trace: include_continuation_trace.unwrap_or(true),
                        check_live_env_unchanged: check_live_env_unchanged.unwrap_or(true),
                    },
                ) {
                    Ok(payload) => DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(
                            serde_json::to_value(payload)
                                .expect("candidate evaluation payload should serialize"),
                        ),
                        reward: None,
                        done: Some(current.info().result != "ongoing"),
                        chosen_action_key: None,
                        info: Some(current.info()),
                    },
                    Err(err) => error_response(err),
                },
                None => {
                    error_response("full-run env not initialized; send reset first".to_string())
                }
            }
        }
        DriverRequest::PreviewPolicyAction {
            policy,
            include_state,
            include_next_state,
            check_live_env_unchanged,
        } => {
            let policy_kind = match normalize_policy(&policy) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            match session.env.as_mut() {
                Some(current) => match preview_policy_action(
                    current,
                    policy_kind,
                    PreviewOutputOptions {
                        include_state: include_state.unwrap_or(true),
                        include_next_state: include_next_state.unwrap_or(true),
                        check_live_env_unchanged: check_live_env_unchanged.unwrap_or(true),
                    },
                ) {
                    Ok(payload) => DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(
                            serde_json::to_value(payload)
                                .expect("policy preview payload should serialize"),
                        ),
                        reward: None,
                        done: Some(current.info().result != "ongoing"),
                        chosen_action_key: None,
                        info: Some(current.info()),
                    },
                    Err(err) => error_response(err),
                },
                None => {
                    error_response("full-run env not initialized; send reset first".to_string())
                }
            }
        }
        DriverRequest::RunVerifiedAdvOverrideEpisode {
            seed,
            ascension,
            final_act,
            class,
            max_steps,
            reward_shaping_profile,
            candidate_scope,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            oracle_margin,
            gamma,
            evaluation_mode,
            value_cache_scope,
            value_cache_max_entries,
            parallelism,
            exact_root_dedup,
            verifier_strategy,
            prefilter_horizon_decisions,
            prefilter_horizon_mode,
            prefilter_margin,
            prefilter_top_k,
            proposer_model_path,
            proposer_top_k,
            proposer_threshold,
            evidence_gate,
            low_evidence_margin,
            confirm_low_evidence_horizon_decisions,
            confirm_low_evidence_horizon_mode,
            confirm_low_evidence_margin,
        } => {
            let config = match build_env_config(
                seed.unwrap_or(1),
                ascension,
                final_act,
                class.as_deref(),
                max_steps,
                reward_shaping_profile,
            ) {
                Ok(config) => config,
                Err(err) => return error_response(err),
            };
            let options = match build_verified_options(
                candidate_scope.as_deref(),
                continuation_policy.as_deref(),
                horizon_decisions,
                horizon_mode.as_deref(),
                oracle_margin,
                verifier_strategy.as_deref(),
                prefilter_horizon_decisions,
                prefilter_horizon_mode.as_deref(),
                prefilter_margin,
                prefilter_top_k,
                proposer_model_path.as_deref(),
                proposer_top_k,
                proposer_threshold,
                evidence_gate.as_deref(),
                low_evidence_margin,
                confirm_low_evidence_horizon_decisions,
                confirm_low_evidence_horizon_mode.as_deref(),
                confirm_low_evidence_margin,
                gamma,
                evaluation_mode.as_deref(),
                value_cache_scope.as_deref(),
                value_cache_max_entries,
                parallelism,
                exact_root_dedup,
            ) {
                Ok(options) => options,
                Err(err) => return error_response(err),
            };
            match run_verified_adv_override_episode(config.seed, config, options) {
                Ok(summary) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(
                        serde_json::to_value(summary)
                            .expect("verified episode summary should serialize"),
                    ),
                    reward: None,
                    done: Some(true),
                    chosen_action_key: None,
                    info: None,
                },
                Err(err) => error_response(err),
            }
        }
        DriverRequest::RunVerifiedAdvOverrideBatch {
            episodes,
            seed_start,
            seed_step,
            ascension,
            final_act,
            class,
            max_steps,
            reward_shaping_profile,
            candidate_scope,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            oracle_margin,
            gamma,
            evaluation_mode,
            value_cache_scope,
            value_cache_max_entries,
            parallelism,
            exact_root_dedup,
            verifier_strategy,
            prefilter_horizon_decisions,
            prefilter_horizon_mode,
            prefilter_margin,
            prefilter_top_k,
            proposer_model_path,
            proposer_top_k,
            proposer_threshold,
            evidence_gate,
            low_evidence_margin,
            confirm_low_evidence_horizon_decisions,
            confirm_low_evidence_horizon_mode,
            confirm_low_evidence_margin,
            summary_only,
        } => {
            let seed_start = seed_start.unwrap_or(1);
            let seed_step = seed_step.unwrap_or(1);
            let options = match build_verified_options(
                candidate_scope.as_deref(),
                continuation_policy.as_deref(),
                horizon_decisions,
                horizon_mode.as_deref(),
                oracle_margin,
                verifier_strategy.as_deref(),
                prefilter_horizon_decisions,
                prefilter_horizon_mode.as_deref(),
                prefilter_margin,
                prefilter_top_k,
                proposer_model_path.as_deref(),
                proposer_top_k,
                proposer_threshold,
                evidence_gate.as_deref(),
                low_evidence_margin,
                confirm_low_evidence_horizon_decisions,
                confirm_low_evidence_horizon_mode.as_deref(),
                confirm_low_evidence_margin,
                gamma,
                evaluation_mode.as_deref(),
                value_cache_scope.as_deref(),
                value_cache_max_entries,
                parallelism,
                exact_root_dedup,
            ) {
                Ok(options) => options,
                Err(err) => return error_response(err),
            };
            let base_config = match build_env_config(
                seed_start,
                ascension,
                final_act,
                class.as_deref(),
                max_steps,
                reward_shaping_profile,
            ) {
                Ok(config) => config,
                Err(err) => return error_response(err),
            };
            match run_verified_adv_override_batch(
                episodes,
                seed_start,
                seed_step,
                base_config,
                options,
                summary_only.unwrap_or(false),
            ) {
                Ok(payload) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(
                        serde_json::to_value(payload)
                            .expect("verified batch payload should serialize"),
                    ),
                    reward: None,
                    done: Some(true),
                    chosen_action_key: None,
                    info: None,
                },
                Err(err) => error_response(err),
            }
        }
        DriverRequest::InspectCounterfactualPending {
            candidate_scope,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            oracle_margin,
            gamma,
            max_roots,
            max_groups,
            parallelism,
            include_observation,
        } => {
            let Some(current) = session.env.as_mut() else {
                return error_response("env not initialized; call reset first".to_string());
            };
            let scope = match CandidateScope::parse(candidate_scope.as_deref()) {
                Ok(scope) => scope,
                Err(err) => return error_response(err),
            };
            let continuation_policy = match normalize_policy(
                continuation_policy.as_deref().unwrap_or("rule_baseline_v0"),
            ) {
                Ok(policy) => policy,
                Err(err) => return error_response(err),
            };
            let horizon_mode = match HorizonMode::parse(horizon_mode.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            match inspect_counterfactual_pending_groups(
                current,
                scope,
                continuation_policy,
                horizon_decisions,
                horizon_mode,
                oracle_margin,
                gamma,
                max_roots.unwrap_or(usize::MAX),
                max_groups.unwrap_or(usize::MAX),
                parallelism.unwrap_or(1),
                include_observation.unwrap_or(false),
            ) {
                Ok(payload) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(payload),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err),
            }
        }
    }
}

fn error_response(error: String) -> DriverResponse {
    DriverResponse {
        ok: false,
        error: Some(error),
        payload: None,
        reward: None,
        done: None,
        chosen_action_key: None,
        info: None,
    }
}

fn neutral_policy_trace_summary(trace: &DeliberationTrace) -> Value {
    let evaluation = &trace.decision.payload;
    let expanded_group_count = evaluation
        .get("expanded_branch_groups")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let unexpanded_group_count = evaluation
        .get("unexpanded_branch_groups")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let candidate_evaluations = evaluation
        .get("candidate_evaluations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let truncated_candidate_count = candidate_evaluations
        .iter()
        .filter(|item| {
            item.get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let dead_candidate_count = candidate_evaluations
        .iter()
        .filter(|item| {
            item.get("player_dead")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let selected_action_id = trace
        .decision
        .selected_action_id
        .map(|action_id| action_id.0);
    let short_horizon_signal_candidate_id =
        short_horizon_signal_candidate_id(trace).map(|action| action.0);
    let controller_decision = evaluation
        .get("controller_decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let signal_label_role = evaluation
        .get("signal_label_role")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    json!({
        "schema_version": "neutral_policy_trace_summary_v0",
        "policy_id": &trace.decision.policy_id,
        "mode": &trace.decision.mode,
        "controller_decision": controller_decision,
        "short_horizon_signal_candidate_id": short_horizon_signal_candidate_id,
        "signal_label_role": signal_label_role,
        "selected_action_id": selected_action_id,
        "fallback": selected_action_id.is_none(),
        "fallback_reason": &trace.decision.fallback_reason,
        "candidate_count": candidate_evaluations.len(),
        "evidence_count": trace.evidence.len(),
        "request_count": trace.search_plan.requests.len(),
        "expanded_group_count": expanded_group_count,
        "unexpanded_group_count": unexpanded_group_count,
        "group_count": expanded_group_count + unexpanded_group_count,
        "truncated_candidate_count": truncated_candidate_count,
        "dead_candidate_count": dead_candidate_count,
    })
}

fn neutral_disagreement_audit(
    policy_input: &PolicyInput,
    trace: &DeliberationTrace,
    reference_action_id: Option<ActionId>,
    paired_compare: Option<Value>,
    commutation_probe: Option<Value>,
    reference_suffix_replay_probe: Option<Value>,
    isolated_enemy_response_public_probe: Option<Value>,
    aligned_enemy_response_public_probe: Option<Value>,
) -> Option<Value> {
    let selected_action_id = short_horizon_signal_candidate_id(trace)?;
    let reference_action_id = reference_action_id?;
    if selected_action_id == reference_action_id {
        return None;
    }
    let selected = policy_input.candidates.get(selected_action_id.0)?;
    let behavior = policy_input.candidates.get(reference_action_id.0)?;
    let selected_eval = candidate_evaluation_by_action(&trace.decision.payload, selected_action_id);
    let reason_code = selected_eval
        .as_ref()
        .and_then(|value| value.get("reason_code"))
        .and_then(Value::as_str)
        .unwrap_or("insufficient");
    let evidence_scope = selected_eval
        .as_ref()
        .and_then(|value| value.get("evidence_scope"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let hypothesis_class = selected_eval
        .as_ref()
        .and_then(|value| value.get("hypothesis_class"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let label_role = selected_eval
        .as_ref()
        .and_then(|value| value.get("label_role"))
        .and_then(Value::as_str)
        .unwrap_or("AuditOnly");
    let risk_buckets = selected_eval
        .as_ref()
        .and_then(|value| value.get("risk_buckets"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let irreversible_resource_ledger = irreversible_resource_ledger(&risk_buckets, reason_code);
    let route = route_disagreement(
        reason_code,
        paired_compare.as_ref(),
        commutation_probe.as_ref(),
    );
    let typed_comparability = typed_comparability_contract(
        reason_code,
        &risk_buckets,
        behavior,
        selected,
        paired_compare.as_ref(),
        commutation_probe.as_ref(),
        aligned_enemy_response_public_probe.as_ref(),
    );
    Some(json!({
        "schema_version": "neutral_disagreement_audit_v3",
        "decision_id": &policy_input.decision_id,
        "behavior": action_descriptor(reference_action_id, behavior),
        "short_horizon_signal": action_descriptor(selected_action_id, selected),
        "action_kind_pair": format!("{}->{}", behavior.action_kind, selected.action_kind),
        "reason_code": reason_code,
        "evidence_scope": evidence_scope,
        "hypothesis_class": hypothesis_class,
        "label_role": label_role,
        "risk_buckets": risk_buckets,
        "irreversible_resource_ledger": irreversible_resource_ledger,
        "route": route.get("route").cloned().unwrap_or(Value::Null),
        "route_status": route.get("status").cloned().unwrap_or(Value::Null),
        "action_label": route.get("action_label").cloned().unwrap_or(Value::Null),
        "router": route,
        "typed_comparability": typed_comparability,
        "paired_compare_deltas": paired_compare.as_ref().map(paired_compare_delta_summary),
        "paired_compare": paired_compare,
        "commutation_result": commutation_probe.as_ref().map(commutation_summary),
        "commutation_probe": commutation_probe,
        "reference_suffix_replay_probe": reference_suffix_replay_probe,
        "isolated_enemy_response_public_probe": isolated_enemy_response_public_probe,
        "aligned_enemy_response_public_probe": aligned_enemy_response_public_probe,
        "trainable_as_action_label": false,
    }))
}

fn typed_comparability_contract(
    reason_code: &str,
    risk_buckets: &Value,
    behavior: &sts_simulator::verification::decision_env::PublicActionCandidateView,
    signal: &sts_simulator::verification::decision_env::PublicActionCandidateView,
    paired_compare: Option<&Value>,
    commutation_probe: Option<&Value>,
    aligned_enemy_response_public_probe: Option<&Value>,
) -> Value {
    let has_risk = |bucket: &str| {
        risk_buckets
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(bucket)))
    };
    let order_only = commutation_probe
        .and_then(|value| value.get("order_only_equivalent"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let paired = PairedRouteEvidence::from_value(paired_compare);
    let aligned = PairedRouteEvidence::from_value(aligned_enemy_response_public_probe);
    let aligned_summary_equal = aligned_enemy_response_public_probe
        .and_then(|value| value.get("summary_equal"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let same_action_domain = behavior.action_kind == signal.action_kind
        && !has_risk("resource")
        && !has_risk("draw")
        && !has_risk("exhaust")
        && !has_risk("setup")
        && !has_risk("power")
        && !has_risk("debuff")
        && !has_risk("pending_choice")
        && !has_risk("hp_cost");

    let (comparability, certificate, requires_horizon_or_value, notes) = if order_only {
        (
            "OrderEquivalent",
            "order_equivalence",
            false,
            vec!["commutation_probe_summary_equal"],
        )
    } else if aligned.neutral_dead_behavior_alive || paired.neutral_dead_behavior_alive {
        (
            "RefutedByAlignedBoundary",
            "refutation",
            false,
            vec!["signal_dead_behavior_alive"],
        )
    } else if aligned.behavior_clears_neutral_not || paired.behavior_clears_neutral_not {
        (
            "RefutedByAlignedBoundary",
            "refutation",
            false,
            vec!["behavior_clears_signal_not"],
        )
    } else if aligned.neutral_alive_behavior_dead || paired.neutral_alive_behavior_dead {
        (
            "SurvivalComparable",
            "survival_flip_candidate",
            false,
            vec!["signal_alive_behavior_dead"],
        )
    } else if aligned.neutral_clears_behavior_not
        || paired.neutral_clears_behavior_not
        || reason_code == "terminal_clear"
    {
        (
            "TerminalComparable",
            "terminal_clear_candidate",
            false,
            vec!["terminal_engine_outcome"],
        )
    } else if has_risk("resource") {
        (
            "IncomparableResourceBoundary",
            "none",
            true,
            vec!["cross_combat_resource_domain"],
        )
    } else if has_risk("pending_choice") {
        (
            "IncomparablePendingChoice",
            "none",
            true,
            vec!["pending_choice_requires_decision_state"],
        )
    } else if has_risk("draw") {
        (
            "IncomparableDrawOrHiddenSample",
            "none",
            true,
            vec!["draw_or_hidden_sample"],
        )
    } else if has_risk("exhaust") {
        (
            "IncomparableIrreversibleCost",
            "none",
            true,
            vec!["irreversible_exhaust_or_state_cost"],
        )
    } else if has_risk("setup") || has_risk("power") || has_risk("debuff") || has_risk("block") {
        (
            "IncomparableNeedsHorizon",
            "none",
            true,
            vec!["delayed_or_defensive_value"],
        )
    } else if reason_code == "damage_delta_only" {
        (
            "DiagnosticOnlyDamageDelta",
            "none",
            true,
            vec!["damage_delta_not_action_preference"],
        )
    } else if same_action_domain && aligned_summary_equal {
        (
            "SameDomainEquivalentAtAlignedBoundary",
            "same_domain_equivalence",
            false,
            vec!["aligned_public_summary_equal"],
        )
    } else if same_action_domain {
        (
            "SameDomainComparableNeedsValue",
            "none",
            true,
            vec!["same_action_kind_without_certificate"],
        )
    } else {
        (
            "IncomparableNeedsHorizon",
            "none",
            true,
            vec!["no_comparable_certificate"],
        )
    };

    json!({
        "schema_version": "typed_comparability_contract_v0",
        "comparability": comparability,
        "certificate_gate": {
            "certificate": certificate,
            "action_label": "none",
            "trainable_as_action_label": false,
            "requires_horizon_or_value": requires_horizon_or_value,
            "notes": notes,
        },
        "domains": {
            "behavior_action_kind": behavior.action_kind.clone(),
            "signal_action_kind": signal.action_kind.clone(),
            "same_action_domain": same_action_domain,
        },
        "boundaries": {
            "uses_aligned_enemy_response": aligned_enemy_response_public_probe.is_some(),
            "aligned_summary_equal": aligned_summary_equal,
            "order_only": order_only,
        },
    })
}

fn short_horizon_signal_candidate_id(trace: &DeliberationTrace) -> Option<ActionId> {
    trace
        .decision
        .payload
        .get("short_horizon_signal_candidate_id")
        .and_then(Value::as_u64)
        .map(|value| ActionId(value as usize))
}

fn reference_suffix_replay_probe_from_commutation(value: &Value) -> Value {
    json!({
        "schema_version": "reference_suffix_replay_probe_v0",
        "suffix_source": "reference_action_only",
        "attempted_suffix_len": 1,
        "minimal_suffix_probe": true,
        "signal_then_reference_legal": value
            .get("left_then_right_legal")
            .cloned()
            .unwrap_or(Value::Null),
        "reference_then_signal_legal": value
            .get("right_then_left_legal")
            .cloned()
            .unwrap_or(Value::Null),
        "both_orders_reached_boundary": value
            .get("both_orders_reached_boundary")
            .cloned()
            .unwrap_or(Value::Null),
        "summary_equal": value.get("summary_equal").cloned().unwrap_or(Value::Null),
        "terminal_diff": value.get("terminal_diff").cloned().unwrap_or(Value::Null),
        "signal_replay_break_reason": value
            .pointer("/left_then_right/failure_reason")
            .cloned()
            .unwrap_or(Value::Null),
        "reference_replay_break_reason": value
            .pointer("/right_then_left/failure_reason")
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn irreversible_resource_ledger(risk_buckets: &Value, reason_code: &str) -> Value {
    let has = |bucket: &str| {
        risk_buckets
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(bucket)))
    };
    let delayed_or_defensive = matches!(
        reason_code,
        "draw_sample_uncertain"
            | "exhaust_cost_unmodeled"
            | "setup_value_unmodeled"
            | "delayed_debuff_horizon_missing"
            | "defense_horizon_missing"
    );
    json!({
        "schema_version": "irreversible_resource_ledger_v0",
        "draw_changed_or_sampled": has("draw"),
        "exhaust_cost_or_state_changed": has("exhaust"),
        "setup_or_power_value_unmodeled": has("setup") || has("power"),
        "debuff_value_delayed": has("debuff"),
        "defense_value_requires_enemy_response": has("block"),
        "hp_cost_observed": has("hp_cost"),
        "pending_choice_created": has("pending_choice"),
        "cross_combat_resource": has("resource"),
        "requires_horizon_or_value": delayed_or_defensive,
    })
}

fn route_disagreement(
    reason_code: &str,
    paired_compare: Option<&Value>,
    commutation_probe: Option<&Value>,
) -> Value {
    let order_only = commutation_probe
        .and_then(|value| value.get("order_only_equivalent"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if order_only {
        return json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "equivalent_order_only",
            "status": "equivalent",
            "action_label": "none",
            "requires_confirmation": false,
            "required_confirmation_boundary": Value::Null,
            "notes": ["commutation_probe_summary_equal"],
        });
    }

    let paired = PairedRouteEvidence::from_value(paired_compare);
    if paired.neutral_dead_behavior_alive || paired.behavior_clears_neutral_not {
        return json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "refuted_by_stable_boundary",
            "status": "refuted",
            "action_label": "none",
            "requires_confirmation": false,
            "required_confirmation_boundary": Value::Null,
            "notes": ["stable_boundary_bad_flip"],
        });
    }
    if paired.neutral_alive_behavior_dead || paired.neutral_clears_behavior_not {
        return json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "terminal_or_survival_certificate",
            "status": "confirmed_positive",
            "action_label": "none",
            "requires_confirmation": false,
            "required_confirmation_boundary": Value::Null,
            "notes": ["stable_boundary_terminal_or_survival_flip"],
        });
    }

    match reason_code {
        "terminal_clear" | "survival_flip" => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "terminal_or_survival_certificate",
            "status": "confirmed_positive",
            "action_label": "none",
            "requires_confirmation": false,
            "required_confirmation_boundary": Value::Null,
            "notes": ["candidate_reason_certificate"],
        }),
        "typed_immediate_dominance" => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "typed_immediate_dominance",
            "status": "needs_aligned_confirmation",
            "action_label": "none",
            "requires_confirmation": true,
            "required_confirmation_boundary": "next_aligned_combat_boundary",
            "notes": ["typed_immediate_not_training_label"],
        }),
        "damage_delta_only" => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "needs_aligned_confirmation",
            "status": "needs_aligned_confirmation",
            "action_label": "none",
            "requires_confirmation": true,
            "required_confirmation_boundary": "next_aligned_combat_boundary",
            "notes": ["damage_delta_only_forbidden_as_direct_label"],
        }),
        "draw_sample_uncertain"
        | "exhaust_cost_unmodeled"
        | "setup_value_unmodeled"
        | "delayed_debuff_horizon_missing"
        | "defense_horizon_missing" => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "needs_horizon_or_value",
            "status": "needs_horizon_or_value",
            "action_label": "none",
            "requires_confirmation": true,
            "required_confirmation_boundary": "horizon_or_value_model",
            "notes": ["unmodeled_delayed_or_defensive_value"],
        }),
        "resource_ineligible" => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "audit_only_resource",
            "status": "audit_only",
            "action_label": "none",
            "requires_confirmation": false,
            "required_confirmation_boundary": Value::Null,
            "notes": ["resource_action_ineligible"],
        }),
        _ => json!({
            "schema_version": "neutral_disagreement_router_v3",
            "route": "insufficient",
            "status": "insufficient",
            "action_label": "none",
            "requires_confirmation": true,
            "required_confirmation_boundary": "manual_or_stronger_evidence",
            "notes": ["unclassified_or_insufficient_evidence"],
        }),
    }
}

#[derive(Default)]
struct PairedRouteEvidence {
    neutral_dead_behavior_alive: bool,
    neutral_alive_behavior_dead: bool,
    neutral_clears_behavior_not: bool,
    behavior_clears_neutral_not: bool,
}

impl PairedRouteEvidence {
    fn from_value(value: Option<&Value>) -> Self {
        Self {
            neutral_dead_behavior_alive: bool_field(value, "left_dead_right_alive"),
            neutral_alive_behavior_dead: bool_field(value, "left_alive_right_dead"),
            neutral_clears_behavior_not: bool_field(value, "left_clears_right_not"),
            behavior_clears_neutral_not: bool_field(value, "right_clears_left_not"),
        }
    }
}

fn bool_field(value: Option<&Value>, key: &str) -> bool {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn candidate_evaluation_by_action(payload: &Value, action_id: ActionId) -> Option<Value> {
    payload
        .get("candidate_evaluations")
        .and_then(Value::as_array)?
        .iter()
        .find(|candidate| action_value_eq(candidate.get("action_id"), action_id))
        .cloned()
}

fn action_value_eq(value: Option<&Value>, action_id: ActionId) -> bool {
    value
        .and_then(Value::as_u64)
        .is_some_and(|value| value as usize == action_id.0)
}

fn action_descriptor(
    action_id: ActionId,
    candidate: &sts_simulator::verification::decision_env::PublicActionCandidateView,
) -> Value {
    json!({
        "action_id": action_id.0,
        "action_index": candidate.action_index,
        "action_kind": candidate.action_kind.clone(),
        "action_key": candidate.action_key.clone(),
        "card": candidate.payload.get("card").cloned().unwrap_or(Value::Null),
        "payload": candidate.payload.clone(),
    })
}

fn paired_compare_delta_summary(value: &Value) -> Value {
    json!({
        "hp_lost_diff_left_minus_behavior": value
            .get("hp_lost_diff_left_minus_right")
            .cloned()
            .unwrap_or(Value::Null),
        "enemy_removed_diff_left_minus_behavior": value
            .get("enemy_removed_diff_left_minus_right")
            .cloned()
            .unwrap_or(Value::Null),
        "kill_diff_left_minus_behavior": value
            .get("kill_diff_left_minus_right")
            .cloned()
            .unwrap_or(Value::Null),
        "neutral_dead_behavior_alive": value
            .get("left_dead_right_alive")
            .cloned()
            .unwrap_or(Value::Null),
        "neutral_alive_behavior_dead": value
            .get("left_alive_right_dead")
            .cloned()
            .unwrap_or(Value::Null),
        "neutral_clears_behavior_not": value
            .get("left_clears_right_not")
            .cloned()
            .unwrap_or(Value::Null),
        "behavior_clears_neutral_not": value
            .get("right_clears_left_not")
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn commutation_summary(value: &Value) -> Value {
    json!({
        "left_then_right_legal": value
            .get("left_then_right_legal")
            .cloned()
            .unwrap_or(Value::Null),
        "right_then_left_legal": value
            .get("right_then_left_legal")
            .cloned()
            .unwrap_or(Value::Null),
        "both_orders_reached_boundary": value
            .get("both_orders_reached_boundary")
            .cloned()
            .unwrap_or(Value::Null),
        "summary_equal": value.get("summary_equal").cloned().unwrap_or(Value::Null),
        "order_only_equivalent": value
            .get("order_only_equivalent")
            .cloned()
            .unwrap_or(Value::Null),
        "hp_loss_diff": value.get("hp_loss_diff").cloned().unwrap_or(Value::Null),
        "enemy_removed_diff": value
            .get("enemy_removed_diff")
            .cloned()
            .unwrap_or(Value::Null),
        "kill_diff": value.get("kill_diff").cloned().unwrap_or(Value::Null),
        "terminal_diff": value.get("terminal_diff").cloned().unwrap_or(Value::Null),
    })
}

fn build_teacher_label_for_decision_record(
    env: &mut FullRunEnv,
    episode_cache: &mut ValueCache,
    decision: &TimeStep,
    continuation_policy: Option<String>,
    horizon_decisions: Option<usize>,
    horizon_mode: Option<String>,
    gamma: Option<f32>,
    evaluation_mode: Option<String>,
    value_cache_scope: Option<String>,
    value_cache_max_entries: Option<usize>,
    parallelism: Option<usize>,
    exact_root_dedup: Option<bool>,
    return_spec_version: &str,
) -> Result<Option<TeacherDecisionLabel>, String> {
    let requested = continuation_policy.is_some()
        || horizon_decisions.is_some()
        || horizon_mode.is_some()
        || gamma.is_some()
        || evaluation_mode.is_some()
        || value_cache_scope.is_some()
        || value_cache_max_entries.is_some()
        || parallelism.is_some()
        || exact_root_dedup.is_some();
    if !requested {
        return Ok(None);
    }

    let continuation_policy =
        normalize_policy(continuation_policy.as_deref().unwrap_or("rule_baseline_v0"))?;
    let horizon_decisions = horizon_decisions.unwrap_or(8);
    let horizon_mode = HorizonMode::parse(horizon_mode.as_deref())?;
    let gamma = gamma.unwrap_or(0.99);
    if !gamma.is_finite() {
        return Err("teacher_gamma must be finite".to_string());
    }
    let evaluation_mode =
        EvaluationMode::parse(evaluation_mode.as_deref().or(Some("bellman_cached_v1")))?;
    let value_cache_scope =
        ValueCacheScope::parse(value_cache_scope.as_deref().or(Some("episode")))?;
    let action_indices = decision
        .candidates
        .iter()
        .map(|candidate| candidate.action_index)
        .collect::<Vec<_>>();
    let payload = evaluate_candidates(
        env,
        episode_cache,
        action_indices,
        continuation_policy,
        horizon_decisions,
        horizon_mode,
        gamma,
        EvaluationRuntimeOptions {
            mode: evaluation_mode,
            cache_scope: value_cache_scope,
            cache_max_entries: value_cache_max_entries.unwrap_or(4096),
            parallelism: parallelism.unwrap_or(1),
            exact_root_dedup: exact_root_dedup.unwrap_or(true),
        },
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: true,
        },
    )?;
    Ok(Some(teacher_label_from_candidate_evaluation(
        payload,
        return_spec_version,
    )))
}

fn teacher_label_from_candidate_evaluation(
    payload: CandidateEvaluationPayload,
    return_spec_version: &str,
) -> TeacherDecisionLabel {
    let best_return = payload
        .evaluations
        .iter()
        .filter(|evaluation| evaluation.ok)
        .map(|evaluation| evaluation.discounted_return)
        .fold(None, |acc: Option<f32>, value| {
            Some(acc.map_or(value, |best| best.max(value)))
        });
    let labels = payload
        .evaluations
        .iter()
        .map(|evaluation| {
            let action_id = ActionId(evaluation.action_index);
            let dominance = if !evaluation.ok {
                Some("error".to_string())
            } else if best_return
                .is_some_and(|best| (best - evaluation.discounted_return).abs() <= f32::EPSILON)
            {
                Some("best_or_tied".to_string())
            } else {
                Some("evaluated".to_string())
            };
            CandidateLabel {
                action_id,
                mean_return: evaluation.ok.then_some(evaluation.discounted_return),
                stderr: None,
                sample_count: u32::from(evaluation.ok),
                dominance,
                confidence: evaluation.ok.then_some("single_rollout".to_string()),
                payload: json!({
                    "action_index": evaluation.action_index,
                    "ok": evaluation.ok,
                    "error": evaluation.error,
                    "chosen_action_key": evaluation.chosen_action_key,
                    "one_step_reward": evaluation.one_step_reward,
                    "discounted_return": evaluation.discounted_return,
                    "done": evaluation.done,
                    "terminal_reason": evaluation.terminal_reason,
                    "continuation_steps": evaluation.continuation_steps,
                    "rollout_done": evaluation.rollout_done,
                    "rollout_terminal_reason": evaluation.rollout_terminal_reason,
                    "horizon_stop_reason": evaluation.horizon_stop_reason,
                    "payoff_reasons": evaluation.payoff_reasons,
                    "final_info": evaluation.final_info,
                }),
            }
        })
        .collect::<Vec<_>>();
    let pairwise_preferences = pairwise_preferences_from_evaluations(&payload.evaluations);
    let training_eligibility = teacher_training_eligibility(&payload, pairwise_preferences.len());
    TeacherDecisionLabel {
        teacher_spec_version: "candidate_evaluation_teacher_v0".to_string(),
        return_spec_version: return_spec_version.to_string(),
        labels,
        pairwise_preferences,
        payload: json!({
            "source_schema_version": payload.schema_version,
            "continuation_policy": payload.continuation_policy,
            "horizon_decisions": payload.horizon_decisions,
            "horizon_mode": payload.horizon_mode,
            "gamma": payload.gamma,
            "evaluation_mode": payload.evaluation_mode,
            "value_cache_scope": payload.value_cache_scope,
            "root_candidate_count": payload.root_candidate_count,
            "root_exact_dedup_count": payload.root_exact_dedup_count,
            "root_rule_equivalent_prune_count": payload.root_rule_equivalent_prune_count,
            "value_cache_hit_count": payload.value_cache_hit_count,
            "value_cache_miss_count": payload.value_cache_miss_count,
            "policy_step_eval_count": payload.policy_step_eval_count,
            "cache_entry_count": payload.cache_entry_count,
            "parallelism_requested": payload.parallelism_requested,
            "parallelism_used": payload.parallelism_used,
            "candidate_eval_wall_ms": payload.candidate_eval_wall_ms,
            "live_env_unchanged": payload.live_env_unchanged,
            "training_eligibility": training_eligibility,
        }),
    }
}

fn teacher_training_eligibility(
    payload: &CandidateEvaluationPayload,
    pairwise_count: usize,
) -> Value {
    let mut reasons = Vec::<String>::new();
    if payload.live_env_unchanged != Some(true) {
        reasons.push("live_env_changed_or_unchecked".to_string());
    }
    if payload.evaluations.len() < 2 {
        reasons.push("fewer_than_two_candidates".to_string());
    }
    if pairwise_count == 0 {
        reasons.push("no_strict_pairwise_preferences".to_string());
    }
    if payload
        .evaluations
        .iter()
        .any(|evaluation| !evaluation.ok || evaluation.error.is_some())
    {
        reasons.push("candidate_evaluation_error".to_string());
    }
    if payload
        .evaluations
        .iter()
        .any(|evaluation| !evaluation.discounted_return.is_finite())
    {
        reasons.push("non_finite_return".to_string());
    }
    if payload.horizon_mode == "fixed_decisions" {
        reasons.push("fixed_decision_horizon_audit_only".to_string());
    }
    if payload
        .evaluations
        .iter()
        .any(|evaluation| evaluation.horizon_stop_reason == "horizon_decision_cap")
    {
        reasons.push("horizon_decision_cap_hit".to_string());
    }
    if payload.evaluations.iter().any(|evaluation| {
        evaluation
            .final_info
            .as_ref()
            .is_some_and(|info| matches!(info.result.as_str(), "truncated" | "crash"))
    }) {
        reasons.push("truncated_or_crash_final_info".to_string());
    }
    let strict_modes = ["combat_end_v1"];
    if !strict_modes.contains(&payload.horizon_mode.as_str()) {
        reasons.push(format!(
            "horizon_mode_not_strict_trainable:{}",
            payload.horizon_mode
        ));
    }
    reasons.sort();
    reasons.dedup();
    let eligible_for_training = reasons.is_empty();
    let label_use = if eligible_for_training {
        "trainable_pairwise"
    } else if payload
        .evaluations
        .iter()
        .any(|evaluation| evaluation.ok && evaluation.discounted_return.is_finite())
    {
        "audit_or_screening_only"
    } else {
        "unusable"
    };
    json!({
        "eligible_for_training": eligible_for_training,
        "label_use": label_use,
        "ineligibility_reasons": reasons,
        "candidate_count": payload.evaluations.len(),
        "pairwise_count": pairwise_count,
        "strict_trainable_horizon_modes": strict_modes,
    })
}

fn pairwise_preferences_from_evaluations(
    evaluations: &[CandidateEvaluation],
) -> Vec<PairwisePreference> {
    let Some(best) = evaluations
        .iter()
        .filter(|evaluation| evaluation.ok)
        .max_by(|left, right| left.discounted_return.total_cmp(&right.discounted_return))
    else {
        return Vec::new();
    };
    evaluations
        .iter()
        .filter(|other| {
            other.ok
                && best.action_index != other.action_index
                && best.discounted_return > other.discounted_return + f32::EPSILON
        })
        .map(|other| PairwisePreference {
            preferred: ActionId(best.action_index),
            other: ActionId(other.action_index),
            margin: Some(best.discounted_return - other.discounted_return),
            confidence: Some("best_vs_other_single_rollout".to_string()),
            payload: json!({
                "preferred_return": best.discounted_return,
                "other_return": other.discounted_return,
            }),
        })
        .collect()
}

fn state_payload(state: FullRunEnvState) -> Value {
    serde_json::to_value(state).expect("full-run state should serialize")
}

fn build_env_config(
    seed: u64,
    ascension: Option<u8>,
    final_act: Option<bool>,
    class: Option<&str>,
    max_steps: Option<usize>,
    reward_shaping_profile: Option<String>,
) -> Result<FullRunEnvConfig, String> {
    Ok(FullRunEnvConfig {
        seed,
        ascension: ascension.unwrap_or(0),
        final_act: final_act.unwrap_or(false),
        player_class: normalize_player_class(class)?,
        max_steps: max_steps.unwrap_or(5000),
        reward_shaping_profile: match reward_shaping_profile {
            Some(value) => RewardShapingProfile::parse(&value)?,
            None => RewardShapingProfile::Baseline,
        },
    })
}

fn build_verified_options(
    candidate_scope: Option<&str>,
    continuation_policy: Option<&str>,
    horizon_decisions: usize,
    horizon_mode: Option<&str>,
    oracle_margin: f32,
    verifier_strategy: Option<&str>,
    prefilter_horizon_decisions: Option<usize>,
    prefilter_horizon_mode: Option<&str>,
    prefilter_margin: Option<f32>,
    prefilter_top_k: Option<usize>,
    proposer_model_path: Option<&str>,
    proposer_top_k: Option<usize>,
    proposer_threshold: Option<f32>,
    evidence_gate: Option<&str>,
    low_evidence_margin: Option<f32>,
    confirm_low_evidence_horizon_decisions: Option<usize>,
    confirm_low_evidence_horizon_mode: Option<&str>,
    confirm_low_evidence_margin: Option<f32>,
    gamma: f32,
    evaluation_mode: Option<&str>,
    value_cache_scope: Option<&str>,
    value_cache_max_entries: Option<usize>,
    parallelism: Option<usize>,
    exact_root_dedup: Option<bool>,
) -> Result<VerifiedAdvOverrideOptions, String> {
    if !oracle_margin.is_finite() {
        return Err("oracle_margin must be finite".to_string());
    }
    if prefilter_margin.is_some_and(|value| !value.is_finite()) {
        return Err("prefilter_margin must be finite".to_string());
    }
    if proposer_threshold.is_some_and(|value| !value.is_finite()) {
        return Err("proposer_threshold must be finite".to_string());
    }
    if low_evidence_margin.is_some_and(|value| !value.is_finite()) {
        return Err("low_evidence_margin must be finite".to_string());
    }
    if confirm_low_evidence_margin.is_some_and(|value| !value.is_finite()) {
        return Err("confirm_low_evidence_margin must be finite".to_string());
    }
    if !gamma.is_finite() {
        return Err("gamma must be finite".to_string());
    }
    let strategy = VerifiedStrategy::parse(verifier_strategy)?;
    let evidence_gate = EvidenceGate::parse(evidence_gate)?;
    let proposer = match proposer_model_path {
        Some(path) if !path.trim().is_empty() => Some(VerifiedProposerOptions {
            model_path: path.to_string(),
            top_k: proposer_top_k.unwrap_or(0),
            threshold: proposer_threshold.unwrap_or(-1.0),
            model: Arc::new(VerifiedProposerModel::load_json(path)?),
        }),
        _ => None,
    };
    if strategy == VerifiedStrategy::ModelProposerV1 && proposer.is_none() {
        return Err("model_proposer_v1 requires proposer_model_path".to_string());
    }
    let parsed_horizon_mode = HorizonMode::parse(horizon_mode)?;
    let confirm_low_evidence = match confirm_low_evidence_horizon_decisions {
        Some(horizon_decisions) => Some(LowEvidenceConfirmationOptions {
            horizon_decisions,
            horizon_mode: HorizonMode::parse(confirm_low_evidence_horizon_mode.or(horizon_mode))?,
            margin: confirm_low_evidence_margin
                .unwrap_or(oracle_margin)
                .max(oracle_margin),
        }),
        None => None,
    };
    Ok(VerifiedAdvOverrideOptions {
        candidate_scope: CandidateScope::parse(candidate_scope)?,
        continuation_policy: normalize_policy(continuation_policy.unwrap_or("rule_baseline_v0"))?,
        horizon_decisions,
        horizon_mode: parsed_horizon_mode,
        oracle_margin,
        strategy,
        prefilter_horizon_decisions: prefilter_horizon_decisions.unwrap_or(horizon_decisions),
        prefilter_horizon_mode: HorizonMode::parse(prefilter_horizon_mode.or(horizon_mode))?,
        prefilter_margin: prefilter_margin.unwrap_or(oracle_margin),
        prefilter_top_k: prefilter_top_k.unwrap_or(0),
        proposer,
        gamma,
        evidence_gate,
        low_evidence_margin: low_evidence_margin.filter(|value| *value > oracle_margin),
        confirm_low_evidence,
        runtime: EvaluationRuntimeOptions {
            mode: EvaluationMode::parse(evaluation_mode)?,
            cache_scope: ValueCacheScope::parse(value_cache_scope)?,
            cache_max_entries: value_cache_max_entries.unwrap_or(4096),
            parallelism: parallelism.unwrap_or(1),
            exact_root_dedup: exact_root_dedup.unwrap_or(false),
        },
    })
}

include!("candidate_evaluation_impl.rs");

fn normalize_player_class(value: Option<&str>) -> Result<&'static str, String> {
    match value.unwrap_or("ironclad").to_ascii_lowercase().as_str() {
        "ironclad" | "red" => Ok("Ironclad"),
        "silent" | "green" => Ok("Silent"),
        "defect" | "blue" => Ok("Defect"),
        "watcher" | "purple" => Ok("Watcher"),
        other => Err(format!(
            "unsupported class '{other}'; expected ironclad, silent, defect, or watcher"
        )),
    }
}

fn normalize_policy(value: &str) -> Result<RunPolicyKind, String> {
    match value.to_ascii_lowercase().as_str() {
        "rule_baseline_v0" => Ok(RunPolicyKind::RuleBaselineV0),
        "rule_baseline_v0_control" => Ok(RunPolicyKind::RuleBaselineV0Control),
        "rule_baseline_v1_candidate" => Ok(RunPolicyKind::RuleBaselineV1Candidate),
        "plan_query_v0" => Ok(RunPolicyKind::PlanQueryV0),
        other => Err(format!(
            "unsupported policy '{other}'; expected rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0"
        )),
    }
}

fn policy_name(policy: RunPolicyKind) -> &'static str {
    match policy {
        RunPolicyKind::RuleBaselineV0 => "rule_baseline_v0",
        RunPolicyKind::RuleBaselineV0Control => "rule_baseline_v0_control",
        RunPolicyKind::RuleBaselineV1Candidate => "rule_baseline_v1_candidate",
        RunPolicyKind::PlanQueryV0 => "plan_query_v0",
        RunPolicyKind::RandomMasked => "random_masked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env() -> FullRunEnv {
        FullRunEnv::new(FullRunEnvConfig {
            seed: 1,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 200,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        })
        .expect("test env should initialize")
    }

    fn advance_to_combat(env: &mut FullRunEnv) {
        for _ in 0..50 {
            let state = env.state().expect("state should build");
            if state.observation.decision_type.starts_with("combat") {
                return;
            }
            env.step_policy(RunPolicyKind::RuleBaselineV0)
                .expect("rule step should advance");
        }
        panic!("test env should reach a combat decision");
    }

    #[test]
    fn evaluate_candidates_does_not_mutate_live_env() {
        let mut env = test_env();
        let before = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");
        let mut cache = ValueCache::default();
        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            Vec::new(),
            RunPolicyKind::RuleBaselineV0,
            1,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions::default(),
            EvaluationOutputOptions::default(),
        )
        .expect("evaluation should run");
        let after = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        assert_eq!(payload.live_env_unchanged, Some(true));
        assert_eq!(before, after);
        assert!(!payload.evaluations.is_empty());
    }

    #[test]
    fn evaluate_candidate_horizon_zero_matches_clone_step_reward() {
        let mut env = test_env();
        let state = env.state().expect("state should build");
        let action_index = state
            .action_mask
            .iter()
            .position(|legal| *legal)
            .expect("at least one legal action");
        let mut clone = env.clone();
        let expected = clone.step(action_index).expect("clone step should run");
        let mut cache = ValueCache::default();

        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![action_index],
            RunPolicyKind::RuleBaselineV0,
            0,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions::default(),
            EvaluationOutputOptions::default(),
        )
        .expect("evaluation should run");
        let evaluation = payload
            .evaluations
            .first()
            .expect("single evaluation should exist");

        assert!(evaluation.ok);
        assert_eq!(evaluation.continuation_steps, 0);
        assert!((evaluation.one_step_reward - expected.reward).abs() < f32::EPSILON);
        assert!((evaluation.discounted_return - expected.reward).abs() < f32::EPSILON);
        assert_eq!(evaluation.done, expected.done);
    }

    #[test]
    fn evaluate_candidates_minimal_payload_omits_states() {
        let mut env = test_env();
        let action_index = env
            .state()
            .expect("state should build")
            .action_mask
            .iter()
            .position(|legal| *legal)
            .expect("at least one legal action");
        let mut cache = ValueCache::default();

        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![action_index],
            RunPolicyKind::RuleBaselineV0,
            1,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions::default(),
            EvaluationOutputOptions {
                include_state: false,
                include_next_state: false,
                include_continuation_trace: false,
                check_live_env_unchanged: false,
            },
        )
        .expect("minimal evaluation should run");
        let evaluation = payload
            .evaluations
            .first()
            .expect("single evaluation should exist");

        assert_eq!(payload.live_env_unchanged, None);
        assert!(payload.state_before.is_none());
        assert!(payload.state_after.is_none());
        assert!(evaluation.next_state.is_none());
        assert!(evaluation.continuation_action_keys.is_empty());
    }

    #[test]
    fn counterfactual_pending_inspect_does_not_mutate_live_env() {
        let mut env = test_env();
        advance_to_combat(&mut env);
        let before = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        let payload = inspect_counterfactual_pending_groups(
            &mut env,
            CandidateScope::ControlledV1,
            RunPolicyKind::RuleBaselineV0,
            1,
            HorizonMode::FixedDecisions,
            1.0,
            0.99,
            8,
            8,
            1,
            false,
        )
        .expect("counterfactual pending inspect should run");
        let after = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        assert_eq!(
            payload
                .get("schema_version")
                .and_then(|value| value.as_str()),
            Some("verified_teacher_counterfactual_pending_inspect_v0")
        );
        assert_eq!(before, after);
    }

    #[test]
    fn cached_evaluation_does_not_mutate_live_env() {
        let mut env = test_env();
        let before = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");
        let mut cache = ValueCache::default();
        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            Vec::new(),
            RunPolicyKind::RuleBaselineV0,
            1,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions {
                mode: EvaluationMode::BellmanCachedV1,
                cache_scope: ValueCacheScope::Episode,
                cache_max_entries: 4096,
                ..EvaluationRuntimeOptions::default()
            },
            EvaluationOutputOptions::default(),
        )
        .expect("cached evaluation should run");
        let after = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        assert_eq!(payload.evaluation_mode, "bellman_cached_v1");
        assert_eq!(payload.live_env_unchanged, Some(true));
        assert_eq!(before, after);
        assert!(!payload.evaluations.is_empty());
    }

    #[test]
    fn cached_h0_and_h1_match_independent_returns() {
        for horizon in [0usize, 1] {
            let mut independent_env = test_env();
            let action_indices = vec![0];
            let mut independent_cache = ValueCache::default();
            let independent = evaluate_candidates(
                &mut independent_env,
                &mut independent_cache,
                action_indices.clone(),
                RunPolicyKind::RuleBaselineV0,
                horizon,
                HorizonMode::FixedDecisions,
                0.99,
                EvaluationRuntimeOptions::default(),
                EvaluationOutputOptions {
                    include_state: false,
                    include_next_state: false,
                    include_continuation_trace: false,
                    check_live_env_unchanged: false,
                },
            )
            .expect("independent evaluation should run");

            let mut cached_env = test_env();
            let mut cached_cache = ValueCache::default();
            let cached = evaluate_candidates(
                &mut cached_env,
                &mut cached_cache,
                action_indices,
                RunPolicyKind::RuleBaselineV0,
                horizon,
                HorizonMode::FixedDecisions,
                0.99,
                EvaluationRuntimeOptions {
                    mode: EvaluationMode::BellmanCachedV1,
                    cache_scope: ValueCacheScope::Episode,
                    cache_max_entries: 4096,
                    ..EvaluationRuntimeOptions::default()
                },
                EvaluationOutputOptions {
                    include_state: false,
                    include_next_state: false,
                    include_continuation_trace: false,
                    check_live_env_unchanged: false,
                },
            )
            .expect("cached evaluation should run");

            let left = independent.evaluations.first().expect("independent row");
            let right = cached.evaluations.first().expect("cached row");
            assert_eq!(left.ok, right.ok);
            assert_eq!(left.action_index, right.action_index);
            assert!((left.discounted_return - right.discounted_return).abs() < f32::EPSILON);
            assert_eq!(left.continuation_steps, right.continuation_steps);
        }
    }

    #[test]
    fn independent_parallel_matches_serial_returns() {
        let options = EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        };
        let mut serial_env = test_env();
        let mut serial_cache = ValueCache::default();
        let serial = evaluate_candidates(
            &mut serial_env,
            &mut serial_cache,
            vec![0, 1, 2],
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions::default(),
            options,
        )
        .expect("serial independent evaluation should run");

        let mut parallel_env = test_env();
        let mut parallel_cache = ValueCache::default();
        let parallel = evaluate_candidates(
            &mut parallel_env,
            &mut parallel_cache,
            vec![0, 1, 2],
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions {
                parallelism: 2,
                ..EvaluationRuntimeOptions::default()
            },
            options,
        )
        .expect("parallel independent evaluation should run");

        assert_eq!(parallel.parallelism_used, 2);
        assert_eq!(serial.evaluations.len(), parallel.evaluations.len());
        for (left, right) in serial.evaluations.iter().zip(parallel.evaluations.iter()) {
            assert_eq!(left.ok, right.ok);
            assert_eq!(left.action_index, right.action_index);
            assert_eq!(left.chosen_action_key, right.chosen_action_key);
            assert!((left.discounted_return - right.discounted_return).abs() < f32::EPSILON);
            assert_eq!(left.continuation_steps, right.continuation_steps);
        }
    }

    #[test]
    fn independent_exact_root_dedup_reuses_duplicate_suffix() {
        let options = EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        };
        let mut env = test_env();
        let mut cache = ValueCache::default();
        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![0, 0],
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions {
                exact_root_dedup: true,
                ..EvaluationRuntimeOptions::default()
            },
            options,
        )
        .expect("dedup independent evaluation should run");

        assert_eq!(payload.evaluations.len(), 2);
        assert_eq!(payload.root_candidate_count, 2);
        assert_eq!(payload.root_exact_dedup_count, 1);
        assert_eq!(payload.policy_step_eval_count, 2);
        assert_eq!(payload.evaluations[0].action_index, 0);
        assert_eq!(payload.evaluations[1].action_index, 0);
        assert!(
            (payload.evaluations[0].discounted_return - payload.evaluations[1].discounted_return)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn cached_repeated_state_hits_episode_cache() {
        let mut env = test_env();
        let mut cache = ValueCache::default();
        let options = EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        };
        let runtime = EvaluationRuntimeOptions {
            mode: EvaluationMode::BellmanCachedV1,
            cache_scope: ValueCacheScope::Episode,
            cache_max_entries: 4096,
            ..EvaluationRuntimeOptions::default()
        };
        let first = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![0],
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            runtime,
            options,
        )
        .expect("first cached evaluation should run");
        let second = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![0],
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            runtime,
            options,
        )
        .expect("second cached evaluation should run");

        assert!(first.value_cache_miss_count > 0);
        assert!(second.value_cache_hit_count > 0);
        assert_eq!(first.cache_entry_count, second.cache_entry_count);
    }

    #[test]
    fn exact_unequal_env_does_not_hit_cache() {
        let env_one = test_env();
        let mut env_two = test_env();
        let _ = env_two.step(0).expect("env two should advance");
        let mut cache = ValueCache::default();
        let value = base_suffix_value(&env_one, false);
        cache.insert(
            env_one,
            RunPolicyKind::RuleBaselineV0,
            2,
            HorizonMode::FixedDecisions,
            0.99,
            false,
            value,
            4096,
        );

        assert!(cache
            .get(
                &env_two,
                RunPolicyKind::RuleBaselineV0,
                2,
                HorizonMode::FixedDecisions,
                0.99,
                false,
            )
            .is_none());
    }

    #[test]
    fn reset_clears_episode_value_cache() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(1),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(200),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);
        let evaluate = DriverRequest::EvaluateCandidates {
            action_indices: vec![0],
            continuation_policy: "rule_baseline_v0".to_string(),
            horizon_decisions: 2,
            horizon_mode: Some("fixed_decisions".to_string()),
            gamma: 0.99,
            evaluation_mode: Some("bellman_cached_v1".to_string()),
            value_cache_scope: Some("episode".to_string()),
            value_cache_max_entries: Some(4096),
            parallelism: None,
            exact_root_dedup: None,
            include_state: Some(false),
            include_next_state: Some(false),
            include_continuation_trace: Some(false),
            check_live_env_unchanged: Some(false),
        };
        assert!(handle_request(&mut session, evaluate).ok);
        assert!(session.episode_value_cache.entry_count > 0);
        let reset_again = DriverRequest::Reset {
            seed: Some(2),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(200),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset_again).ok);
        assert_eq!(session.episode_value_cache.entry_count, 0);
    }

    #[test]
    fn exact_root_dedup_keeps_duplicate_candidate_rows() {
        let mut env = test_env();
        let mut cache = ValueCache::default();
        let payload = evaluate_candidates(
            &mut env,
            &mut cache,
            vec![0, 0],
            RunPolicyKind::RuleBaselineV0,
            1,
            HorizonMode::FixedDecisions,
            0.99,
            EvaluationRuntimeOptions {
                mode: EvaluationMode::BellmanCachedV1,
                cache_scope: ValueCacheScope::Request,
                cache_max_entries: 4096,
                ..EvaluationRuntimeOptions::default()
            },
            EvaluationOutputOptions {
                include_state: false,
                include_next_state: false,
                include_continuation_trace: false,
                check_live_env_unchanged: false,
            },
        )
        .expect("cached evaluation should run");

        assert_eq!(payload.evaluations.len(), 2);
        assert_eq!(payload.root_candidate_count, 2);
        assert_eq!(payload.root_exact_dedup_count, 1);
        assert_eq!(payload.evaluations[0].action_index, 0);
        assert_eq!(payload.evaluations[1].action_index, 0);
    }

    #[test]
    fn preview_policy_action_does_not_mutate_live_env() {
        let mut env = test_env();
        let before = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");
        let payload = preview_policy_action(
            &mut env,
            RunPolicyKind::RuleBaselineV0,
            PreviewOutputOptions::default(),
        )
        .expect("preview should run");
        let after = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        assert_eq!(payload.live_env_unchanged, Some(true));
        assert_eq!(before, after);
        assert!(payload.chosen_action_key.is_some());
        let chosen_index = payload
            .chosen_action_index
            .expect("preview should map chosen action to current candidate index");
        assert_eq!(
            payload
                .state_before
                .as_ref()
                .expect("default preview should include state")
                .action_candidates[chosen_index]
                .action_key,
            payload.chosen_action_key.unwrap()
        );
    }

    #[test]
    fn preview_policy_action_minimal_payload_omits_states() {
        let mut env = test_env();
        let before = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");
        let payload = preview_policy_action(
            &mut env,
            RunPolicyKind::RuleBaselineV0,
            PreviewOutputOptions {
                include_state: false,
                include_next_state: false,
                check_live_env_unchanged: false,
            },
        )
        .expect("minimal preview should run");
        let after = serde_json::to_value(env.state().expect("state should build"))
            .expect("state should serialize");

        assert_eq!(before, after);
        assert_eq!(payload.live_env_unchanged, None);
        assert!(payload.state_before.is_none());
        assert!(payload.state_after.is_none());
        assert!(payload.next_state.is_none());
        assert!(payload.chosen_action_index.is_some());
        assert!(payload.chosen_action_key.is_some());
    }

    #[test]
    fn driver_exposes_decision_env_timestep_commands() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(3),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let observation = handle_request(&mut session, DriverRequest::DecisionEnvObservation);
        assert!(observation.ok);
        let observation_payload = observation.payload.expect("timestep payload");
        assert_eq!(
            observation_payload["contract_version"],
            "decision_env_contract_v0"
        );
        assert_eq!(observation_payload["observation"]["visibility"], "public");
        let first_action_id = observation_payload["candidates"][0]["id"]
            .as_u64()
            .expect("candidate action id") as usize;

        let stepped = handle_request(
            &mut session,
            DriverRequest::DecisionEnvStep {
                action_id: first_action_id,
            },
        );
        assert!(stepped.ok);
        let stepped_payload = stepped.payload.expect("stepped timestep payload");
        assert_eq!(
            stepped_payload["contract_version"],
            "decision_env_contract_v0"
        );
        assert_eq!(
            stepped.reward.expect("driver reward"),
            stepped_payload["reward"]["scalar_reward"]
                .as_f64()
                .expect("payload reward") as f32
        );
        assert_eq!(
            stepped.done.expect("driver done"),
            stepped_payload["terminated"].as_bool().unwrap()
                || stepped_payload["truncated"].as_bool().unwrap()
        );
    }

    #[test]
    fn driver_exposes_policy_input_without_debug_info() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(8),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let response = handle_request(
            &mut session,
            DriverRequest::PolicyInput {
                time_budget_ms: Some(11),
            },
        );

        assert!(response.ok);
        let payload = response.payload.expect("policy input payload");
        assert_eq!(payload["schema_version"], "policy_input_v0");
        assert_eq!(payload["time_budget_ms"], 11);
        assert_eq!(payload["observation"]["visibility"], "public");
        let serialized = serde_json::to_string(&payload).expect("serialize policy input");
        assert!(!serialized.contains("state_hash"));
        assert!(!serialized.contains("timestep_info"));
        assert!(!serialized.contains("teacher_label"));
        assert!(!serialized.contains("rule_score"));
    }

    #[test]
    fn driver_exposes_neutral_policy_trace_without_stepping_env() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(8),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let mut supported_trace = None;
        for _ in 0..32 {
            let trace = handle_request(
                &mut session,
                DriverRequest::NeutralPolicyTrace {
                    time_budget_ms: Some(17),
                    max_branch_depth: Some(1),
                    max_candidates: Some(16),
                    reference_action_id: None,
                },
            );
            assert!(trace.ok);
            let payload = trace.payload.expect("neutral trace payload");
            assert_eq!(payload["schema_version"], "neutral_policy_trace_driver_v0");
            if payload["supported"].as_bool() == Some(true) {
                supported_trace = Some(payload);
                break;
            }
            let preview = handle_request(
                &mut session,
                DriverRequest::PreviewPolicyAction {
                    policy: "rule_baseline_v0".to_string(),
                    include_state: Some(false),
                    include_next_state: Some(false),
                    check_live_env_unchanged: Some(false),
                },
            );
            assert!(preview.ok);
            let Some(action_id) = preview
                .payload
                .as_ref()
                .and_then(|payload| payload["chosen_action_index"].as_u64())
            else {
                break;
            };
            let step = handle_request(
                &mut session,
                DriverRequest::DecisionEnvStep {
                    action_id: action_id as usize,
                },
            );
            assert!(step.ok);
            if step.done == Some(true) {
                break;
            }
        }

        let payload = supported_trace.expect("expected to reach a combat decision");
        assert_eq!(
            payload["trace"]["proposal"]["policy_id"],
            "neutral_probe_evaluator_v1"
        );
        assert_eq!(
            payload["trace"]["evidence"][0]["search_kind"]["kind"],
            "neutral_branch_compression"
        );
        assert!(payload["summary"]["candidate_count"].as_u64().unwrap_or(0) > 0);
    }

    #[test]
    fn driver_neutral_policy_trace_can_pair_compare_against_reference_action() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(8),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let mut supported_trace = None;
        for _ in 0..32 {
            let trace = handle_request(
                &mut session,
                DriverRequest::NeutralPolicyTrace {
                    time_budget_ms: Some(17),
                    max_branch_depth: Some(1),
                    max_candidates: Some(16),
                    reference_action_id: None,
                },
            );
            assert!(trace.ok);
            let payload = trace.payload.expect("neutral trace payload");
            if payload["supported"].as_bool() == Some(true) {
                supported_trace = Some(payload);
                break;
            }
            let preview = handle_request(
                &mut session,
                DriverRequest::PreviewPolicyAction {
                    policy: "rule_baseline_v0".to_string(),
                    include_state: Some(false),
                    include_next_state: Some(false),
                    check_live_env_unchanged: Some(false),
                },
            );
            assert!(preview.ok);
            let Some(action_id) = preview
                .payload
                .as_ref()
                .and_then(|payload| payload["chosen_action_index"].as_u64())
            else {
                break;
            };
            let step = handle_request(
                &mut session,
                DriverRequest::DecisionEnvStep {
                    action_id: action_id as usize,
                },
            );
            assert!(step.ok);
            if step.done == Some(true) {
                break;
            }
        }

        let payload = supported_trace.expect("expected to reach a combat decision");
        assert!(payload["summary"]["selected_action_id"].is_null());
        let signal = payload["summary"]["short_horizon_signal_candidate_id"]
            .as_u64()
            .expect("neutral should emit a short-horizon signal in this smoke state")
            as usize;
        let candidate_count = payload["summary"]["candidate_count"]
            .as_u64()
            .expect("candidate count") as usize;
        let reference = if signal == 0 && candidate_count > 1 {
            1
        } else {
            0
        };
        let trace = handle_request(
            &mut session,
            DriverRequest::NeutralPolicyTrace {
                time_budget_ms: Some(17),
                max_branch_depth: Some(1),
                max_candidates: Some(16),
                reference_action_id: Some(reference),
            },
        );
        assert!(trace.ok);
        let payload = trace.payload.expect("neutral trace payload");
        assert!(!payload["paired_compare_vs_reference"].is_null());
        assert!(!payload["commutation_probe_vs_reference"].is_null());
        assert!(!payload["reference_suffix_replay_probe"].is_null());
        assert!(!payload["isolated_enemy_response_public_probe_vs_reference"].is_null());
        assert!(!payload["aligned_enemy_response_public_probe_vs_reference"].is_null());
        assert_eq!(payload["summary"]["controller_decision"], "abstain");
        assert!(payload["summary"]["short_horizon_signal_candidate_id"].is_u64());
        assert_eq!(
            payload["disagreement_audit"]["schema_version"],
            "neutral_disagreement_audit_v3"
        );
        assert_eq!(
            payload["disagreement_audit"]["trainable_as_action_label"],
            false
        );
        assert!(payload["disagreement_audit"]["reason_code"].is_string());
        assert!(payload["disagreement_audit"]["route"].is_string());
        assert_eq!(payload["disagreement_audit"]["action_label"], "none");
        assert!(payload["disagreement_audit"]["label_role"].is_string());
        assert!(payload["disagreement_audit"]["irreversible_resource_ledger"].is_object());
        assert_eq!(
            payload["disagreement_audit"]["typed_comparability"]["schema_version"],
            "typed_comparability_contract_v0"
        );
        assert_eq!(
            payload["disagreement_audit"]["typed_comparability"]["certificate_gate"]
                ["action_label"],
            "none"
        );
        assert_eq!(
            payload["disagreement_audit"]["typed_comparability"]["certificate_gate"]
                ["trainable_as_action_label"],
            false
        );
    }

    #[test]
    fn driver_emits_versioned_decision_record_step() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(5),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let record_response = handle_request(
            &mut session,
            DriverRequest::DecisionRecordStep {
                action_id: 0,
                sim_version: Some("test_sim".to_string()),
                return_spec_version: Some("test_return".to_string()),
                context: Some(json!({"collector": "driver_test"})),
                teacher_continuation_policy: None,
                teacher_horizon_decisions: None,
                teacher_horizon_mode: None,
                teacher_gamma: None,
                teacher_evaluation_mode: None,
                teacher_value_cache_scope: None,
                teacher_value_cache_max_entries: None,
                teacher_parallelism: None,
                teacher_exact_root_dedup: None,
            },
        );

        assert!(record_response.ok);
        let record = record_response.payload.expect("record payload");
        assert_eq!(record["schema_version"], "decision_record_v0");
        assert_eq!(record["sim_version"], "test_sim");
        assert_eq!(record["return_spec_version"], "test_return");
        assert_eq!(record["behavior_action"], 0);
        assert!(record["state_hash_before"].is_string());
        assert!(record["state_hash_after"].is_string());
        assert_eq!(record["info"]["record_context"]["collector"], "driver_test");
        assert_eq!(
            record_response.reward.expect("driver reward"),
            record["reward_since_prev"]["scalar_reward"]
                .as_f64()
                .expect("record reward") as f32
        );
    }

    #[test]
    fn driver_can_attach_teacher_label_to_decision_record_step() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(6),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let record_response = handle_request(
            &mut session,
            DriverRequest::DecisionRecordStep {
                action_id: 0,
                sim_version: Some("test_sim".to_string()),
                return_spec_version: Some("test_return".to_string()),
                context: Some(json!({"collector": "teacher_label_test"})),
                teacher_continuation_policy: Some("rule_baseline_v0".to_string()),
                teacher_horizon_decisions: Some(1),
                teacher_horizon_mode: Some("fixed_decisions".to_string()),
                teacher_gamma: Some(0.99),
                teacher_evaluation_mode: Some("bellman_cached_v1".to_string()),
                teacher_value_cache_scope: Some("request".to_string()),
                teacher_value_cache_max_entries: Some(64),
                teacher_parallelism: Some(1),
                teacher_exact_root_dedup: Some(true),
            },
        );

        assert!(record_response.ok);
        let record = record_response.payload.expect("record payload");
        let label = &record["teacher_label"];
        assert_eq!(
            label["teacher_spec_version"],
            "candidate_evaluation_teacher_v0"
        );
        assert_eq!(label["return_spec_version"], "test_return");
        assert!(label["labels"]
            .as_array()
            .is_some_and(|labels| !labels.is_empty()));
        assert_eq!(
            label["payload"]["source_schema_version"],
            "return_q_candidate_evaluation_v0"
        );
        assert_eq!(label["payload"]["live_env_unchanged"], true);
    }

    #[test]
    fn driver_emits_branch_trace_without_stepping_live_env() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(7),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(80),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let before = handle_request(
            &mut session,
            DriverRequest::PolicyInput {
                time_budget_ms: Some(10),
            },
        )
        .payload
        .expect("policy input before branch trace");
        let response = handle_request(
            &mut session,
            DriverRequest::BranchTrace {
                action_indices: vec![0],
                candidate_scope: Some("all".to_string()),
                candidate_sampling_spec_id: None,
                candidate_cap: None,
                behavior_action_id: None,
                sampling_seed: None,
                continuation_policy: Some("rule_baseline_v0".to_string()),
                horizon_decisions: 0,
                horizon_mode: Some("fixed_decisions".to_string()),
                sim_version: Some("test_sim".to_string()),
                content_version: Some("test_content".to_string()),
                max_steps: None,
                include_comparisons: Some(true),
            },
        );
        assert!(response.ok);
        let payload = response.payload.expect("branch trace payload");
        assert_eq!(payload["schema_version"], "branch_trace_batch_v1");
        assert_eq!(payload["trace_count"], 1);
        assert_eq!(payload["comparison_count"], 0);
        assert_eq!(payload["live_env_unchanged"], true);
        assert_eq!(payload["validation_report"]["valid"], true);
        assert_eq!(payload["validation_report"]["issue_count"], 0);
        assert_eq!(payload["traces"][0]["schema_version"], "branch_trace_v1");
        assert_eq!(payload["traces"][0]["trainable_as_action_label"], false);
        assert_eq!(
            payload["traces"][0]["redaction_report"]["model_input_uses_public_observation"],
            true
        );

        let after = handle_request(
            &mut session,
            DriverRequest::PolicyInput {
                time_budget_ms: Some(10),
            },
        )
        .payload
        .expect("policy input after branch trace");
        assert_eq!(before["decision_id"], after["decision_id"]);
        assert_eq!(before["candidates"], after["candidates"]);
    }

    #[test]
    fn branch_trace_is_deterministic_for_same_branch_spec() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(8),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(120),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);

        let first = handle_request(
            &mut session,
            DriverRequest::BranchTrace {
                action_indices: vec![0],
                candidate_scope: Some("all".to_string()),
                candidate_sampling_spec_id: None,
                candidate_cap: None,
                behavior_action_id: None,
                sampling_seed: None,
                continuation_policy: Some("rule_baseline_v0".to_string()),
                horizon_decisions: 2,
                horizon_mode: Some("fixed_decisions".to_string()),
                sim_version: Some("determinism_test".to_string()),
                content_version: Some("test_content".to_string()),
                max_steps: None,
                include_comparisons: Some(true),
            },
        )
        .payload
        .expect("first branch trace payload");
        let second = handle_request(
            &mut session,
            DriverRequest::BranchTrace {
                action_indices: vec![0],
                candidate_scope: Some("all".to_string()),
                candidate_sampling_spec_id: None,
                candidate_cap: None,
                behavior_action_id: None,
                sampling_seed: None,
                continuation_policy: Some("rule_baseline_v0".to_string()),
                horizon_decisions: 2,
                horizon_mode: Some("fixed_decisions".to_string()),
                sim_version: Some("determinism_test".to_string()),
                content_version: Some("test_content".to_string()),
                max_steps: None,
                include_comparisons: Some(true),
            },
        )
        .payload
        .expect("second branch trace payload");

        assert_eq!(first["validation_report"]["valid"], true);
        assert_eq!(second["validation_report"]["valid"], true);
        assert_eq!(first["traces"], second["traces"]);
        assert_eq!(first["comparisons"], second["comparisons"]);
    }

    #[test]
    fn combat_end_branch_trace_caps_are_censored() {
        let mut session = DriverSession::default();
        let reset = DriverRequest::Reset {
            seed: Some(9),
            ascension: Some(0),
            final_act: Some(false),
            class: Some("ironclad".to_string()),
            max_steps: Some(120),
            reward_shaping_profile: Some("baseline".to_string()),
        };
        assert!(handle_request(&mut session, reset).ok);
        advance_to_combat(session.env.as_mut().expect("env should exist"));

        let response = handle_request(
            &mut session,
            DriverRequest::BranchTrace {
                action_indices: vec![0],
                candidate_scope: Some("all".to_string()),
                candidate_sampling_spec_id: None,
                candidate_cap: None,
                behavior_action_id: None,
                sampling_seed: None,
                continuation_policy: Some("rule_baseline_v0".to_string()),
                horizon_decisions: 0,
                horizon_mode: Some("combat_end_v1".to_string()),
                sim_version: Some("censor_test".to_string()),
                content_version: Some("test_content".to_string()),
                max_steps: None,
                include_comparisons: Some(true),
            },
        );
        assert!(response.ok);
        let payload = response.payload.expect("branch trace payload");
        assert_eq!(payload["validation_report"]["valid"], true);
        let outcome = &payload["traces"][0]["outcome"];
        if outcome["stop_reason"] == "horizon_decision_cap_before_combat_end" {
            assert_eq!(outcome["boundary_requested"], "combat_end");
            assert_eq!(outcome["boundary_reached"], false);
            assert_eq!(outcome["outcome_censored"], true);
            assert_eq!(outcome["truncated"], true);
            assert_eq!(
                outcome["truncation_reason"],
                "horizon_cap_before_combat_end"
            );
            assert_eq!(payload["traces"][0]["truncated"], true);
        }
    }
}
