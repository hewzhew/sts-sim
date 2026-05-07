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
use sts_simulator::cli::full_run_smoke::{
    FullRunEnv, FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState, RewardShapingProfile,
    RunActionCandidate, RunPolicyKind,
};

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
    Step {
        action_index: usize,
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
    verified_decision_type_counts: BTreeMap<String, usize>,
    verified_override_decision_type_counts: BTreeMap<String, usize>,
    verified_decision_context_counts: BTreeMap<String, usize>,
    verified_override_context_counts: BTreeMap<String, usize>,
    verified_best_adv_bucket_counts: BTreeMap<String, usize>,
    verified_horizon_stop_reason_counts: BTreeMap<String, usize>,
    verified_payoff_reason_counts: BTreeMap<String, usize>,
    verified_override_payoff_reason_counts: BTreeMap<String, usize>,
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
    decision_type_counts: BTreeMap<String, usize>,
    override_decision_type_counts: BTreeMap<String, usize>,
    decision_context_counts: BTreeMap<String, usize>,
    override_context_counts: BTreeMap<String, usize>,
    best_adv_bucket_counts: BTreeMap<String, usize>,
    horizon_stop_reason_counts: BTreeMap<String, usize>,
    payoff_reason_counts: BTreeMap<String, usize>,
    override_payoff_reason_counts: BTreeMap<String, usize>,
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
            other => Err(format!(
                "unsupported horizon_mode '{other}'; expected fixed_decisions, adaptive_next_player_turn_v1, or adaptive_payoff_window_v1"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FixedDecisions => "fixed_decisions",
            Self::AdaptiveNextPlayerTurnV1 => "adaptive_next_player_turn_v1",
            Self::AdaptivePayoffWindowV1 => "adaptive_payoff_window_v1",
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
    Ok(VerifiedAdvOverrideOptions {
        candidate_scope: CandidateScope::parse(candidate_scope)?,
        continuation_policy: normalize_policy(continuation_policy.unwrap_or("rule_baseline_v0"))?,
        horizon_decisions,
        horizon_mode: HorizonMode::parse(horizon_mode)?,
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
        confirm_low_evidence: confirm_low_evidence_horizon_decisions.map(|horizon_decisions| {
            LowEvidenceConfirmationOptions {
                horizon_decisions,
                margin: confirm_low_evidence_margin
                    .unwrap_or(oracle_margin)
                    .max(oracle_margin),
            }
        }),
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
        "plan_query_v0" => Ok(RunPolicyKind::PlanQueryV0),
        other => Err(format!(
            "unsupported policy '{other}'; expected rule_baseline_v0 or plan_query_v0"
        )),
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
}
