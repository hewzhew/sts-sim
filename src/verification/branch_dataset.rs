use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::decision_env::{
    ActionCandidate, ActionId, DecisionId, ObservationPayload, RewardEvent, TimeStep,
};

pub const BRANCH_TRACE_SCHEMA_VERSION: &str = "branch_trace_v1";
pub const BRANCH_COMPARISON_SCHEMA_VERSION: &str = "branch_comparison_v1";
pub const BRANCH_DATASET_VALIDATION_SCHEMA_VERSION: &str = "branch_dataset_validation_v1";
pub const PUBLIC_TRANSITION_SUMMARY_SCHEMA_VERSION: &str = "public_transition_summary_v1";
pub const PAIRED_SCENARIO_SCHEMA_VERSION: &str = "paired_scenario_v0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HorizonSpecV1 {
    pub horizon_mode: String,
    pub horizon_decisions: usize,
    pub continuation_policy: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PublicTransitionSummaryV1 {
    pub schema_version: String,
    pub step_index: usize,
    pub decision_id: DecisionId,
    pub chosen_action: Option<ActionId>,
    pub chosen_action_key: Option<String>,
    pub reward: RewardEvent,
    pub terminated: bool,
    pub truncated: bool,
    pub state_hash: String,
    pub info: Value,
}

impl PublicTransitionSummaryV1 {
    pub fn from_timestep(
        step_index: usize,
        timestep: &TimeStep,
        chosen_action: Option<ActionId>,
        chosen_action_key: Option<String>,
    ) -> Self {
        Self {
            schema_version: PUBLIC_TRANSITION_SUMMARY_SCHEMA_VERSION.to_string(),
            step_index,
            decision_id: timestep.decision_id.clone(),
            chosen_action,
            chosen_action_key,
            reward: timestep.reward.clone(),
            terminated: timestep.terminated,
            truncated: timestep.truncated,
            state_hash: timestep.info.state_hash.clone(),
            info: timestep.info.payload.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchOutcomeV1 {
    pub total_reward: f32,
    pub step_count: usize,
    pub boundary_requested: String,
    pub boundary_reached: bool,
    pub stop_reason: String,
    pub horizon_stop_reason: String,
    pub truncation_reason: Option<String>,
    pub outcome_censored: bool,
    pub terminated: bool,
    pub truncated: bool,
    pub result: String,
    pub terminal_reason: String,
    pub hp_delta: i32,
    pub floor_delta: i32,
    pub combat_win_delta: i32,
    pub floor: i32,
    pub act: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub combat_win_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RedactionReportV1 {
    pub redaction_policy_id: String,
    pub observation_visibility: String,
    pub public_transition_summary_visibility: String,
    pub model_input_uses_public_observation: bool,
    pub hidden_state_in_observation: bool,
    pub hidden_future_in_public_summary: bool,
    pub debug_info_in_trace: bool,
    pub notes: Vec<String>,
}

impl Default for RedactionReportV1 {
    fn default() -> Self {
        Self {
            redaction_policy_id: "branch_trace_public_observation_debug_info_v1".to_string(),
            observation_visibility: "public".to_string(),
            public_transition_summary_visibility: "public_safe_with_debug_info".to_string(),
            model_input_uses_public_observation: true,
            hidden_state_in_observation: false,
            hidden_future_in_public_summary: false,
            debug_info_in_trace: true,
            notes: vec![
                "branch trace stores public observation/candidates plus debug info for audit"
                    .to_string(),
                "debug info is not model input".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchTraceV1 {
    pub schema_version: String,
    pub branch_id: String,
    pub episode_id: String,
    pub decision_id: DecisionId,
    pub sim_version: String,
    pub content_version: String,
    pub env_config: Value,
    pub seed: u64,
    pub scenario_seed_id: String,
    pub state_hash_before: String,
    pub rng_state_before_hash: String,
    pub rng_state_after_hash: String,
    pub rng_consumed: bool,
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub observation: ObservationPayload,
    pub candidates: Vec<ActionCandidate>,
    pub forced_prefix: Vec<ActionId>,
    pub forced_action_keys: Vec<String>,
    pub continuation_policy: String,
    pub horizon: HorizonSpecV1,
    pub public_summaries: Vec<PublicTransitionSummaryV1>,
    pub reward_events: Vec<RewardEvent>,
    pub terminal: bool,
    pub truncated: bool,
    pub outcome: BranchOutcomeV1,
    pub redaction_report: RedactionReportV1,
    pub trainable_as_action_label: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutcomeDiffV1 {
    pub total_reward_left_minus_right: f32,
    pub hp_left_minus_right: i32,
    pub floor_left_minus_right: i32,
    pub combat_wins_left_minus_right: i32,
    pub left_dead_right_alive: bool,
    pub left_alive_right_dead: bool,
    pub left_progresses_farther: bool,
    pub right_progresses_farther: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchComparisonV1 {
    pub schema_version: String,
    pub pairing_schema_version: String,
    pub decision_id: DecisionId,
    pub left_branch_id: String,
    pub right_branch_id: String,
    pub pairing_mode: String,
    pub paired_seed_id: String,
    pub scenario_seed_id: String,
    pub common_random_policy: String,
    pub pairing_valid: bool,
    pub paired_validity_status: String,
    pub pairing_invalid_reason: Option<String>,
    pub rng_diverged: Option<bool>,
    pub rng_divergence_reason: Option<String>,
    pub rng_before_hash: String,
    pub left_rng_after_hash: String,
    pub right_rng_after_hash: String,
    pub comparison_scope: String,
    pub trainable_role: String,
    pub outcome_diff: OutcomeDiffV1,
    pub uncertainty: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchDatasetValidationIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub branch_id: Option<String>,
    pub comparison_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchDatasetValidationReport {
    pub schema_version: String,
    pub trace_count: usize,
    pub comparison_count: usize,
    pub issue_count: usize,
    pub valid: bool,
    pub issues: Vec<BranchDatasetValidationIssue>,
}

impl BranchComparisonV1 {
    pub fn from_traces(left: &BranchTraceV1, right: &BranchTraceV1) -> Self {
        let left_dead = left.outcome.result == "defeat";
        let right_dead = right.outcome.result == "defeat";
        let any_censored = left.outcome.outcome_censored || right.outcome.outcome_censored;
        let same_scenario = left.scenario_seed_id == right.scenario_seed_id;
        let same_rng_before = left.rng_state_before_hash == right.rng_state_before_hash;
        let pairing_valid = same_scenario && same_rng_before;
        let rng_diverged = left.rng_state_after_hash != right.rng_state_after_hash;
        Self {
            schema_version: BRANCH_COMPARISON_SCHEMA_VERSION.to_string(),
            pairing_schema_version: PAIRED_SCENARIO_SCHEMA_VERSION.to_string(),
            decision_id: left.decision_id.clone(),
            left_branch_id: left.branch_id.clone(),
            right_branch_id: right.branch_id.clone(),
            pairing_mode: "same_initial_env_seed_single_scenario_v0".to_string(),
            paired_seed_id: format!("seed:{}", left.seed),
            scenario_seed_id: left.scenario_seed_id.clone(),
            common_random_policy: "shared_initial_rng_no_realignment_v0".to_string(),
            pairing_valid,
            paired_validity_status: if pairing_valid {
                "valid_shared_initial_scenario".to_string()
            } else {
                "invalid_unpaired_initial_scenario".to_string()
            },
            pairing_invalid_reason: if pairing_valid {
                None
            } else if !same_scenario {
                Some("scenario_seed_id_mismatch".to_string())
            } else {
                Some("rng_state_before_mismatch".to_string())
            },
            rng_diverged: Some(rng_diverged),
            rng_divergence_reason: if rng_diverged {
                Some("branch_actions_or_continuation_consumed_rng_differently".to_string())
            } else {
                None
            },
            rng_before_hash: left.rng_state_before_hash.clone(),
            left_rng_after_hash: left.rng_state_after_hash.clone(),
            right_rng_after_hash: right.rng_state_after_hash.clone(),
            comparison_scope: if any_censored {
                "same_decision_same_horizon_censored".to_string()
            } else {
                "same_decision_same_horizon".to_string()
            },
            trainable_role: if any_censored {
                "branch_outcome_comparison_censored".to_string()
            } else {
                "branch_outcome_comparison".to_string()
            },
            outcome_diff: OutcomeDiffV1 {
                total_reward_left_minus_right: left.outcome.total_reward
                    - right.outcome.total_reward,
                hp_left_minus_right: left.outcome.hp - right.outcome.hp,
                floor_left_minus_right: left.outcome.floor - right.outcome.floor,
                combat_wins_left_minus_right: left.outcome.combat_win_count as i32
                    - right.outcome.combat_win_count as i32,
                left_dead_right_alive: left_dead && !right_dead,
                left_alive_right_dead: !left_dead && right_dead,
                left_progresses_farther: left.outcome.floor > right.outcome.floor,
                right_progresses_farther: right.outcome.floor > left.outcome.floor,
            },
            uncertainty: serde_json::json!({
                "paired_seed_count": 1,
                "stderr_available": false,
                "common_random_policy": "shared initial RNG only; no post-branch RNG stream realignment",
                "note": "single deterministic branch comparison; aggregate externally"
            }),
        }
    }
}

pub fn validate_branch_dataset(
    traces: &[BranchTraceV1],
    comparisons: &[BranchComparisonV1],
) -> BranchDatasetValidationReport {
    let mut issues = Vec::new();
    for trace in traces {
        validate_trace(trace, &mut issues);
    }
    for (index, comparison) in comparisons.iter().enumerate() {
        validate_comparison(index, comparison, traces, &mut issues);
    }
    BranchDatasetValidationReport {
        schema_version: BRANCH_DATASET_VALIDATION_SCHEMA_VERSION.to_string(),
        trace_count: traces.len(),
        comparison_count: comparisons.len(),
        issue_count: issues.len(),
        valid: issues.is_empty(),
        issues,
    }
}

fn validate_trace(trace: &BranchTraceV1, issues: &mut Vec<BranchDatasetValidationIssue>) {
    let mut push = |code: &str, message: String| {
        issues.push(BranchDatasetValidationIssue {
            code: code.to_string(),
            severity: "error".to_string(),
            message,
            branch_id: Some(trace.branch_id.clone()),
            comparison_index: None,
        });
    };
    if trace.schema_version != BRANCH_TRACE_SCHEMA_VERSION {
        push(
            "trace_schema_version_mismatch",
            format!(
                "expected {BRANCH_TRACE_SCHEMA_VERSION}, got {}",
                trace.schema_version
            ),
        );
    }
    if trace.state_hash_before.is_empty() {
        push(
            "missing_state_hash_before",
            "state_hash_before is empty".to_string(),
        );
    }
    if trace.scenario_seed_id.is_empty() {
        push(
            "missing_scenario_seed_id",
            "scenario_seed_id is empty".to_string(),
        );
    }
    if trace.rng_state_before_hash.is_empty() {
        push(
            "missing_rng_state_before_hash",
            "rng_state_before_hash is empty".to_string(),
        );
    }
    if trace.rng_state_after_hash.is_empty() {
        push(
            "missing_rng_state_after_hash",
            "rng_state_after_hash is empty".to_string(),
        );
    }
    if trace.forced_prefix.is_empty() {
        push(
            "missing_forced_prefix",
            "forced_prefix is empty".to_string(),
        );
    }
    if trace.forced_action_keys.len() != trace.forced_prefix.len() {
        push(
            "forced_prefix_key_mismatch",
            "forced_action_keys length differs from forced_prefix length".to_string(),
        );
    }
    if trace.trainable_as_action_label {
        push(
            "trace_marked_as_action_label",
            "BranchTraceV1 must not be trainable as an action label".to_string(),
        );
    }
    if trace.redaction_report.observation_visibility != "public" {
        push(
            "non_public_observation_visibility",
            format!(
                "expected public observation visibility, got {}",
                trace.redaction_report.observation_visibility
            ),
        );
    }
    if trace.redaction_report.redaction_policy_id.is_empty() {
        push(
            "missing_redaction_policy_id",
            "redaction_policy_id is empty".to_string(),
        );
    }
    if trace
        .redaction_report
        .public_transition_summary_visibility
        .is_empty()
    {
        push(
            "missing_public_transition_summary_visibility",
            "public_transition_summary_visibility is empty".to_string(),
        );
    }
    if !trace.redaction_report.model_input_uses_public_observation {
        push(
            "model_input_not_public_observation",
            "model_input_uses_public_observation must be true".to_string(),
        );
    }
    if trace.redaction_report.hidden_state_in_observation {
        push(
            "hidden_state_in_observation",
            "hidden_state_in_observation must be false".to_string(),
        );
    }
    if trace.redaction_report.hidden_future_in_public_summary {
        push(
            "hidden_future_in_public_summary",
            "hidden_future_in_public_summary must be false".to_string(),
        );
    }
    if trace.horizon.horizon_mode.is_empty() {
        push("missing_horizon_mode", "horizon_mode is empty".to_string());
    }
    if trace.outcome.stop_reason != trace.outcome.horizon_stop_reason {
        push(
            "stop_reason_mismatch",
            "stop_reason must match horizon_stop_reason compatibility field".to_string(),
        );
    }
    if trace.outcome.boundary_requested.is_empty() {
        push(
            "missing_boundary_requested",
            "boundary_requested is empty".to_string(),
        );
    }
    if trace.outcome.outcome_censored && trace.outcome.truncation_reason.is_none() {
        push(
            "censored_without_truncation_reason",
            "outcome_censored=true requires truncation_reason".to_string(),
        );
    }
    if trace.outcome.boundary_requested == "combat_end"
        && trace.outcome.stop_reason == "horizon_decision_cap_before_combat_end"
    {
        if trace.outcome.boundary_reached {
            push(
                "combat_end_cap_marked_reached",
                "combat_end_v1 cap before combat end must not mark boundary_reached=true"
                    .to_string(),
            );
        }
        if !trace.outcome.outcome_censored {
            push(
                "combat_end_cap_not_censored",
                "combat_end_v1 cap before combat end must mark outcome_censored=true".to_string(),
            );
        }
        if !trace.truncated || !trace.outcome.truncated {
            push(
                "combat_end_cap_not_truncated",
                "combat_end_v1 cap before combat end must mark branch trace truncated".to_string(),
            );
        }
    }
    if trace.outcome.boundary_requested == "fixed_decisions"
        && trace.outcome.stop_reason == "horizon_decision_cap"
    {
        if !trace.outcome.boundary_reached {
            push(
                "fixed_cap_not_boundary_reached",
                "fixed_decisions horizon cap is the requested boundary".to_string(),
            );
        }
        if trace.outcome.outcome_censored {
            push(
                "fixed_cap_marked_censored",
                "fixed_decisions cap should not be censored for fixed-horizon labels".to_string(),
            );
        }
    }
    if trace.continuation_policy != trace.horizon.continuation_policy {
        push(
            "continuation_policy_mismatch",
            "trace continuation_policy differs from horizon continuation_policy".to_string(),
        );
    }
}

fn validate_comparison(
    index: usize,
    comparison: &BranchComparisonV1,
    traces: &[BranchTraceV1],
    issues: &mut Vec<BranchDatasetValidationIssue>,
) {
    let mut push = |code: &str, message: String| {
        issues.push(BranchDatasetValidationIssue {
            code: code.to_string(),
            severity: "error".to_string(),
            message,
            branch_id: None,
            comparison_index: Some(index),
        });
    };
    if comparison.schema_version != BRANCH_COMPARISON_SCHEMA_VERSION {
        push(
            "comparison_schema_version_mismatch",
            format!(
                "expected {BRANCH_COMPARISON_SCHEMA_VERSION}, got {}",
                comparison.schema_version
            ),
        );
    }
    if comparison.pairing_schema_version != PAIRED_SCENARIO_SCHEMA_VERSION {
        push(
            "comparison_pairing_schema_version_mismatch",
            format!(
                "expected {PAIRED_SCENARIO_SCHEMA_VERSION}, got {}",
                comparison.pairing_schema_version
            ),
        );
    }
    if comparison.trainable_role.contains("action") {
        push(
            "comparison_action_label_role",
            format!(
                "comparison trainable_role must not be action-like: {}",
                comparison.trainable_role
            ),
        );
    }
    if comparison.pairing_mode.is_empty() {
        push(
            "comparison_missing_pairing_mode",
            "comparison pairing_mode is empty".to_string(),
        );
    }
    if comparison.paired_seed_id.is_empty() {
        push(
            "comparison_missing_paired_seed_id",
            "comparison paired_seed_id is empty".to_string(),
        );
    }
    if comparison.scenario_seed_id.is_empty() {
        push(
            "comparison_missing_scenario_seed_id",
            "comparison scenario_seed_id is empty".to_string(),
        );
    }
    if comparison.common_random_policy.is_empty() {
        push(
            "comparison_missing_common_random_policy",
            "comparison common_random_policy is empty".to_string(),
        );
    }
    if comparison.paired_validity_status.is_empty() {
        push(
            "comparison_missing_paired_validity_status",
            "comparison paired_validity_status is empty".to_string(),
        );
    }
    if !comparison.pairing_valid && comparison.pairing_invalid_reason.is_none() {
        push(
            "comparison_invalid_pairing_without_reason",
            "pairing_valid=false requires pairing_invalid_reason".to_string(),
        );
    }
    let Some(left) = traces
        .iter()
        .find(|trace| trace.branch_id == comparison.left_branch_id)
    else {
        push(
            "comparison_left_branch_missing",
            format!("left branch {} not found", comparison.left_branch_id),
        );
        return;
    };
    let Some(right) = traces
        .iter()
        .find(|trace| trace.branch_id == comparison.right_branch_id)
    else {
        push(
            "comparison_right_branch_missing",
            format!("right branch {} not found", comparison.right_branch_id),
        );
        return;
    };
    if left.decision_id != right.decision_id || left.decision_id != comparison.decision_id {
        push(
            "comparison_decision_mismatch",
            "comparison branches must share the same decision_id".to_string(),
        );
    }
    if left.state_hash_before != right.state_hash_before {
        push(
            "comparison_state_hash_mismatch",
            "comparison branches must share the same state_hash_before".to_string(),
        );
    }
    if left.scenario_seed_id != right.scenario_seed_id {
        push(
            "comparison_scenario_seed_id_mismatch",
            "comparison branches must share scenario_seed_id".to_string(),
        );
    }
    if comparison.scenario_seed_id != left.scenario_seed_id {
        push(
            "comparison_scenario_seed_id_field_mismatch",
            format!(
                "comparison scenario_seed_id must be {}, got {}",
                left.scenario_seed_id, comparison.scenario_seed_id
            ),
        );
    }
    if left.rng_state_before_hash != right.rng_state_before_hash {
        push(
            "comparison_rng_state_before_mismatch",
            "comparison branches must share initial rng_state_before_hash".to_string(),
        );
    }
    if comparison.rng_before_hash != left.rng_state_before_hash {
        push(
            "comparison_rng_before_hash_mismatch",
            format!(
                "comparison rng_before_hash must be {}, got {}",
                left.rng_state_before_hash, comparison.rng_before_hash
            ),
        );
    }
    if comparison.left_rng_after_hash != left.rng_state_after_hash {
        push(
            "comparison_left_rng_after_hash_mismatch",
            "comparison left_rng_after_hash does not match left branch".to_string(),
        );
    }
    if comparison.right_rng_after_hash != right.rng_state_after_hash {
        push(
            "comparison_right_rng_after_hash_mismatch",
            "comparison right_rng_after_hash does not match right branch".to_string(),
        );
    }
    let expected_rng_diverged = left.rng_state_after_hash != right.rng_state_after_hash;
    if comparison.rng_diverged != Some(expected_rng_diverged) {
        push(
            "comparison_rng_diverged_mismatch",
            format!(
                "comparison rng_diverged must be {:?}",
                Some(expected_rng_diverged)
            ),
        );
    }
    if expected_rng_diverged && comparison.rng_divergence_reason.is_none() {
        push(
            "comparison_rng_diverged_without_reason",
            "rng_diverged=true requires rng_divergence_reason".to_string(),
        );
    }
    if !expected_rng_diverged && comparison.rng_divergence_reason.is_some() {
        push(
            "comparison_rng_not_diverged_with_reason",
            "rng_diverged=false must not include rng_divergence_reason".to_string(),
        );
    }
    if comparison.pairing_valid
        && (comparison.pairing_invalid_reason.is_some()
            || left.scenario_seed_id != right.scenario_seed_id
            || left.rng_state_before_hash != right.rng_state_before_hash)
    {
        push(
            "comparison_pairing_validity_inconsistent",
            "pairing_valid=true requires matching scenario, matching initial RNG, and no invalid reason"
                .to_string(),
        );
    }
    if left.horizon != right.horizon {
        push(
            "comparison_horizon_mismatch",
            "comparison branches must share the same horizon spec".to_string(),
        );
    }
    if left.continuation_policy != right.continuation_policy {
        push(
            "comparison_continuation_policy_mismatch",
            "comparison branches must share the same continuation policy".to_string(),
        );
    }
    if left.sim_version != right.sim_version
        || left.content_version != right.content_version
        || left.seed != right.seed
    {
        push(
            "comparison_version_or_seed_mismatch",
            "comparison branches must share sim/content versions and seed".to_string(),
        );
    }
    let expected_paired_seed_id = format!("seed:{}", left.seed);
    if comparison.paired_seed_id != expected_paired_seed_id {
        push(
            "comparison_paired_seed_id_mismatch",
            format!(
                "comparison paired_seed_id must be {expected_paired_seed_id}, got {}",
                comparison.paired_seed_id
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::decision_env::{
        ActionCandidate, DecisionId, ObservationPayload, ObservationVisibility, RewardEvent,
    };
    use serde_json::json;

    fn decision_id() -> DecisionId {
        DecisionId {
            episode_id: "seed:1".to_string(),
            step_index: 10,
            decision_type: "combat".to_string(),
        }
    }

    fn reward() -> RewardEvent {
        RewardEvent {
            schema_version: "reward_event_v0".to_string(),
            scalar_reward: 0.0,
            components: json!({}),
        }
    }

    fn candidate(index: usize) -> ActionCandidate {
        ActionCandidate {
            id: ActionId(index),
            action_schema_version: "test_action_schema".to_string(),
            action_index: index,
            action_key: format!("combat/test/{index}"),
            action_kind: "play_card".to_string(),
            payload: json!({"index": index}),
        }
    }

    fn trace(branch_id: &str, action_index: usize) -> BranchTraceV1 {
        BranchTraceV1 {
            schema_version: BRANCH_TRACE_SCHEMA_VERSION.to_string(),
            branch_id: branch_id.to_string(),
            episode_id: "seed:1".to_string(),
            decision_id: decision_id(),
            sim_version: "sim_test".to_string(),
            content_version: "content_test".to_string(),
            env_config: json!({"seed": 1}),
            seed: 1,
            scenario_seed_id: "initial_env_seed:1:scenario:0".to_string(),
            state_hash_before: "state_hash".to_string(),
            rng_state_before_hash: "rng_before".to_string(),
            rng_state_after_hash: format!("rng_after_{action_index}"),
            rng_consumed: true,
            observation_schema_version: "obs_schema".to_string(),
            action_schema_version: "test_action_schema".to_string(),
            observation: ObservationPayload {
                schema_version: "obs_schema".to_string(),
                visibility: ObservationVisibility::Public,
                decision_type: "combat".to_string(),
                payload: json!({}),
            },
            candidates: vec![candidate(0), candidate(1)],
            forced_prefix: vec![ActionId(action_index)],
            forced_action_keys: vec![format!("combat/test/{action_index}")],
            continuation_policy: "rule_baseline_v0".to_string(),
            horizon: HorizonSpecV1 {
                horizon_mode: "fixed_decisions".to_string(),
                horizon_decisions: 1,
                continuation_policy: "rule_baseline_v0".to_string(),
            },
            public_summaries: Vec::new(),
            reward_events: vec![reward()],
            terminal: false,
            truncated: false,
            outcome: BranchOutcomeV1 {
                total_reward: 0.0,
                step_count: 1,
                boundary_requested: "fixed_decisions".to_string(),
                boundary_reached: true,
                stop_reason: "horizon_decision_cap".to_string(),
                horizon_stop_reason: "horizon_decision_cap".to_string(),
                truncation_reason: None,
                outcome_censored: false,
                terminated: false,
                truncated: false,
                result: "ongoing".to_string(),
                terminal_reason: "running".to_string(),
                hp_delta: 0,
                floor_delta: 0,
                combat_win_delta: 0,
                floor: 1,
                act: 1,
                hp: 80,
                max_hp: 80,
                gold: 99,
                combat_win_count: 0,
            },
            redaction_report: RedactionReportV1::default(),
            trainable_as_action_label: false,
        }
    }

    fn issue_codes(report: &BranchDatasetValidationReport) -> Vec<String> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect()
    }

    #[test]
    fn validator_accepts_valid_branch_dataset() {
        let left = trace("left", 0);
        let right = trace("right", 1);
        let comparison = BranchComparisonV1::from_traces(&left, &right);
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(report.valid, "{:?}", report.issues);
    }

    #[test]
    fn validator_rejects_action_label_trace() {
        let mut left = trace("left", 0);
        left.trainable_as_action_label = true;
        let report = validate_branch_dataset(&[left], &[]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"trace_marked_as_action_label".to_string()));
    }

    #[test]
    fn validator_rejects_hidden_observation_redaction_violation() {
        let mut left = trace("left", 0);
        left.redaction_report.hidden_state_in_observation = true;
        let report = validate_branch_dataset(&[left], &[]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"hidden_state_in_observation".to_string()));
    }

    #[test]
    fn validator_rejects_hidden_future_public_summary_redaction_violation() {
        let mut left = trace("left", 0);
        left.redaction_report.hidden_future_in_public_summary = true;
        let report = validate_branch_dataset(&[left], &[]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"hidden_future_in_public_summary".to_string()));
    }

    #[test]
    fn validator_rejects_comparison_pairing_mismatch() {
        let left = trace("left", 0);
        let mut right = trace("right", 1);
        right.state_hash_before = "different_state".to_string();
        let comparison = BranchComparisonV1::from_traces(&left, &right);
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"comparison_state_hash_mismatch".to_string()));
    }

    #[test]
    fn validator_rejects_comparison_horizon_mismatch() {
        let left = trace("left", 0);
        let mut right = trace("right", 1);
        right.horizon.horizon_decisions = 2;
        let comparison = BranchComparisonV1::from_traces(&left, &right);
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"comparison_horizon_mismatch".to_string()));
    }

    #[test]
    fn validator_rejects_action_like_comparison_role() {
        let left = trace("left", 0);
        let right = trace("right", 1);
        let mut comparison = BranchComparisonV1::from_traces(&left, &right);
        comparison.trainable_role = "action_preference".to_string();
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"comparison_action_label_role".to_string()));
    }

    #[test]
    fn validator_rejects_invalid_pairing_without_reason() {
        let left = trace("left", 0);
        let right = trace("right", 1);
        let mut comparison = BranchComparisonV1::from_traces(&left, &right);
        comparison.pairing_valid = false;
        comparison.pairing_invalid_reason = None;
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(
            issue_codes(&report).contains(&"comparison_invalid_pairing_without_reason".to_string())
        );
    }

    #[test]
    fn validator_rejects_trace_missing_rng_hashes() {
        let mut left = trace("left", 0);
        left.rng_state_before_hash.clear();
        let report = validate_branch_dataset(&[left], &[]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"missing_rng_state_before_hash".to_string()));
    }

    #[test]
    fn validator_rejects_comparison_rng_before_mismatch() {
        let left = trace("left", 0);
        let mut right = trace("right", 1);
        right.rng_state_before_hash = "different_rng_before".to_string();
        let comparison = BranchComparisonV1::from_traces(&left, &right);
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"comparison_rng_state_before_mismatch".to_string()));
    }

    #[test]
    fn validator_rejects_comparison_rng_divergence_mismatch() {
        let left = trace("left", 0);
        let right = trace("right", 1);
        let mut comparison = BranchComparisonV1::from_traces(&left, &right);
        comparison.rng_diverged = Some(false);
        let report = validate_branch_dataset(&[left, right], &[comparison]);
        assert!(!report.valid);
        assert!(issue_codes(&report).contains(&"comparison_rng_diverged_mismatch".to_string()));
    }
}
