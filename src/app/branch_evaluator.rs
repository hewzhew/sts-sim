use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::cli::full_run_smoke::{FullRunEnv, FullRunEnvInfo, RunPolicyKind};
use crate::verification::branch_dataset::{
    BranchComparisonV1, BranchOutcomeV1, BranchTraceV1, HorizonSpecV1, PublicTransitionSummaryV1,
    RedactionReportV1, BRANCH_TRACE_SCHEMA_VERSION,
};
use crate::verification::decision_env::{ActionId, DecisionEnv, TimeStep};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchCandidateScope {
    All,
    ControlledV0,
    ControlledV1,
}

impl BranchCandidateScope {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("controlled_v1").to_ascii_lowercase().as_str() {
            "" | "all" => Ok(Self::All),
            "controlled_v0" => Ok(Self::ControlledV0),
            "controlled_v1" => Ok(Self::ControlledV1),
            other => Err(format!(
                "unsupported branch candidate_scope '{other}'; expected all, controlled_v0, or controlled_v1"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ControlledV0 => "controlled_v0",
            Self::ControlledV1 => "controlled_v1",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchHorizonMode {
    FixedDecisions,
    CombatEndV1,
}

impl BranchHorizonMode {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("fixed_decisions").to_ascii_lowercase().as_str() {
            "" | "fixed" | "fixed_decisions" => Ok(Self::FixedDecisions),
            "combat_end_v1" | "combat_end" => Ok(Self::CombatEndV1),
            other => Err(format!(
                "unsupported branch horizon_mode '{other}'; expected fixed_decisions or combat_end_v1"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FixedDecisions => "fixed_decisions",
            Self::CombatEndV1 => "combat_end_v1",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BranchStopClassification {
    boundary_requested: &'static str,
    boundary_reached: bool,
    outcome_censored: bool,
    branch_truncated: bool,
    truncation_reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct BranchEvaluatorConfig {
    pub action_indices: Vec<usize>,
    pub candidate_scope: BranchCandidateScope,
    pub continuation_policy: RunPolicyKind,
    pub horizon_decisions: usize,
    pub horizon_mode: BranchHorizonMode,
    pub sim_version: String,
    pub content_version: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BranchEvaluatorOutput {
    pub action_indices: Vec<usize>,
    pub sampling_summary: BranchSamplingSummary,
    pub traces: Vec<BranchTraceV1>,
    pub comparisons: Vec<BranchComparisonV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BranchSamplingSummary {
    pub requested_action_count: usize,
    pub legal_candidate_count: usize,
    pub included_candidate_count: usize,
    pub invalid_index_count: usize,
    pub scope_filtered_count: usize,
}

struct ScopedActionSelection {
    action_indices: Vec<usize>,
    summary: BranchSamplingSummary,
}

pub struct BranchEvaluator;

impl BranchEvaluator {
    pub fn evaluate_current(
        current: &FullRunEnv,
        decision: &TimeStep,
        config: &BranchEvaluatorConfig,
        include_comparisons: bool,
    ) -> Result<BranchEvaluatorOutput, String> {
        let selection =
            scoped_action_indices(decision, &config.action_indices, config.candidate_scope);
        let traces =
            build_branch_traces_from_current(current, decision, &selection.action_indices, config)?;
        let comparisons = if include_comparisons {
            branch_trace_pairwise_comparisons(&traces)
        } else {
            Vec::new()
        };
        Ok(BranchEvaluatorOutput {
            action_indices: selection.action_indices,
            sampling_summary: selection.summary,
            traces,
            comparisons,
        })
    }
}

fn scoped_action_indices(
    decision: &TimeStep,
    requested_action_indices: &[usize],
    candidate_scope: BranchCandidateScope,
) -> ScopedActionSelection {
    let indices: Vec<usize> = if requested_action_indices.is_empty() {
        (0..decision.candidates.len()).collect()
    } else {
        requested_action_indices.to_vec()
    };
    let mut scoped = Vec::new();
    let mut invalid_index_count = 0usize;
    let mut scope_filtered_count = 0usize;
    for index in &indices {
        let Some(candidate) = decision.candidates.get(*index) else {
            invalid_index_count += 1;
            continue;
        };
        if candidate_in_scope(candidate.action_kind.as_str(), candidate_scope) {
            scoped.push(*index);
        } else {
            scope_filtered_count += 1;
        }
    }
    let summary = BranchSamplingSummary {
        requested_action_count: indices.len(),
        legal_candidate_count: decision.candidates.len(),
        included_candidate_count: scoped.len(),
        invalid_index_count,
        scope_filtered_count,
    };
    ScopedActionSelection {
        action_indices: scoped,
        summary,
    }
}

fn candidate_in_scope(action_kind: &str, candidate_scope: BranchCandidateScope) -> bool {
    match candidate_scope {
        BranchCandidateScope::All => true,
        BranchCandidateScope::ControlledV0 => matches!(action_kind, "play_card" | "end_turn"),
        BranchCandidateScope::ControlledV1 => matches!(
            action_kind,
            "play_card" | "end_turn" | "card_choice" | "discover_choice" | "selection" | "scry"
        ),
    }
}

fn build_branch_traces_from_current(
    current: &FullRunEnv,
    decision: &TimeStep,
    action_indices: &[usize],
    config: &BranchEvaluatorConfig,
) -> Result<Vec<BranchTraceV1>, String> {
    let mut traces = Vec::with_capacity(action_indices.len());
    let rng_state_before_hash = branch_rng_state_hash(current);
    let scenario_seed_id = {
        let info = current.info();
        format!(
            "initial_env_seed:{}:decision:{}:{}",
            info.seed, decision.decision_id.episode_id, decision.decision_id.step_index
        )
    };
    for action_index in action_indices {
        let action_id = ActionId(*action_index);
        let candidate = decision.candidates.get(*action_index).ok_or_else(|| {
            format!(
                "branch trace action index {} out of range for {} candidates",
                action_index,
                decision.candidates.len()
            )
        })?;
        let mut branch_env = current.clone();
        let mut public_summaries = Vec::new();
        let mut reward_events = Vec::new();
        let mut total_reward = 0.0f32;
        let mut forced_action_keys = Vec::new();
        let start_info = current.info();

        let forced = DecisionEnv::step(&mut branch_env, action_id)
            .map_err(|err| format!("force branch action {} failed: {err}", action_index))?;
        total_reward += forced.reward.scalar_reward;
        reward_events.push(forced.reward.clone());
        forced_action_keys.push(candidate.action_key.clone());
        public_summaries.push(PublicTransitionSummaryV1::from_timestep(
            0,
            &forced,
            Some(action_id),
            Some(candidate.action_key.clone()),
        ));

        let mut last = forced;
        let mut continuation_steps = 0usize;
        let horizon_stop_reason;
        loop {
            if let Some(reason) = branch_trace_stop_reason(
                config.horizon_mode,
                &start_info,
                &branch_env,
                &last,
                continuation_steps,
                config.horizon_decisions,
            ) {
                horizon_stop_reason = reason.to_string();
                break;
            }
            let (Some(next_action), next_key) = branch_env
                .preview_policy_action_index(config.continuation_policy)
                .map_err(|err| format!("preview continuation policy failed: {err}"))?
            else {
                horizon_stop_reason = "no_continuation_action".to_string();
                break;
            };
            let next_id = ActionId(next_action);
            let step = DecisionEnv::step(&mut branch_env, next_id)
                .map_err(|err| format!("continuation branch step failed: {err}"))?;
            continuation_steps += 1;
            total_reward += step.reward.scalar_reward;
            reward_events.push(step.reward.clone());
            public_summaries.push(PublicTransitionSummaryV1::from_timestep(
                public_summaries.len(),
                &step,
                Some(next_id),
                next_key,
            ));
            last = step;
        }

        let info = branch_env.info();
        let rng_state_after_hash = branch_rng_state_hash(&branch_env);
        let stop = classify_branch_stop(config.horizon_mode, horizon_stop_reason.as_str());
        let horizon = HorizonSpecV1 {
            horizon_mode: config.horizon_mode.as_str().to_string(),
            horizon_decisions: config.horizon_decisions,
            continuation_policy: policy_name(config.continuation_policy).to_string(),
        };
        traces.push(BranchTraceV1 {
            schema_version: BRANCH_TRACE_SCHEMA_VERSION.to_string(),
            branch_id: format!(
                "{}:{}:branch:{}",
                decision.decision_id.episode_id, decision.decision_id.step_index, action_index
            ),
            episode_id: decision.decision_id.episode_id.clone(),
            decision_id: decision.decision_id.clone(),
            sim_version: config.sim_version.clone(),
            content_version: config.content_version.clone(),
            env_config: json!({
                "source": "full_run_env_driver_current_session",
                "seed": start_info.seed,
                "floor": start_info.floor,
                "act": start_info.act,
                "reward_shaping_profile": "session_current",
            }),
            seed: start_info.seed,
            scenario_seed_id: scenario_seed_id.clone(),
            state_hash_before: decision.info.state_hash.clone(),
            rng_state_before_hash: rng_state_before_hash.clone(),
            rng_state_after_hash: rng_state_after_hash.clone(),
            rng_consumed: rng_state_after_hash != rng_state_before_hash,
            observation_schema_version: decision.observation.schema_version.clone(),
            action_schema_version: decision
                .candidates
                .first()
                .map(|candidate| candidate.action_schema_version.clone())
                .unwrap_or_default(),
            observation: decision.observation.clone(),
            candidates: decision.candidates.clone(),
            forced_prefix: vec![action_id],
            forced_action_keys,
            continuation_policy: policy_name(config.continuation_policy).to_string(),
            horizon,
            public_summaries,
            reward_events,
            terminal: last.terminated,
            truncated: stop.branch_truncated,
            outcome: BranchOutcomeV1 {
                total_reward,
                step_count: info.step.saturating_sub(start_info.step),
                boundary_requested: stop.boundary_requested.to_string(),
                boundary_reached: stop.boundary_reached,
                stop_reason: horizon_stop_reason.clone(),
                horizon_stop_reason,
                truncation_reason: stop.truncation_reason.map(str::to_string),
                outcome_censored: stop.outcome_censored,
                terminated: last.terminated,
                truncated: stop.branch_truncated,
                result: info.result,
                terminal_reason: info.terminal_reason,
                hp_delta: info.hp - start_info.hp,
                floor_delta: info.floor - start_info.floor,
                combat_win_delta: info.combat_win_count as i32 - start_info.combat_win_count as i32,
                floor: info.floor,
                act: info.act,
                hp: info.hp,
                max_hp: info.max_hp,
                gold: info.gold,
                combat_win_count: info.combat_win_count,
            },
            redaction_report: RedactionReportV1::default(),
            trainable_as_action_label: false,
        });
    }
    Ok(traces)
}

pub fn branch_rng_state_hash(env: &FullRunEnv) -> String {
    let mut hasher = DefaultHasher::new();
    "run_rng_pool".hash(&mut hasher);
    format!("{:?}", env.ctx.run_state.rng_pool).hash(&mut hasher);
    "combat_rng_pool".hash(&mut hasher);
    if let Some(combat) = &env.ctx.combat_state {
        format!("{:?}", combat.rng.pool).hash(&mut hasher);
    } else {
        "none".hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn classify_branch_stop(
    horizon_mode: BranchHorizonMode,
    stop_reason: &str,
) -> BranchStopClassification {
    match horizon_mode {
        BranchHorizonMode::FixedDecisions => match stop_reason {
            "horizon_decision_cap" | "terminated" | "non_ongoing_result" => {
                BranchStopClassification {
                    boundary_requested: "fixed_decisions",
                    boundary_reached: true,
                    outcome_censored: false,
                    branch_truncated: false,
                    truncation_reason: None,
                }
            }
            "truncated" => BranchStopClassification {
                boundary_requested: "fixed_decisions",
                boundary_reached: false,
                outcome_censored: true,
                branch_truncated: true,
                truncation_reason: Some("engine_truncated"),
            },
            other => BranchStopClassification {
                boundary_requested: "fixed_decisions",
                boundary_reached: false,
                outcome_censored: true,
                branch_truncated: true,
                truncation_reason: Some(if other == "no_continuation_action" {
                    "no_continuation_action"
                } else {
                    "unexpected_stop_before_fixed_boundary"
                }),
            },
        },
        BranchHorizonMode::CombatEndV1 => match stop_reason {
            "combat_end" | "terminated" | "non_ongoing_result" => BranchStopClassification {
                boundary_requested: "combat_end",
                boundary_reached: true,
                outcome_censored: false,
                branch_truncated: false,
                truncation_reason: None,
            },
            "truncated" => BranchStopClassification {
                boundary_requested: "combat_end",
                boundary_reached: false,
                outcome_censored: true,
                branch_truncated: true,
                truncation_reason: Some("engine_truncated"),
            },
            "horizon_decision_cap_before_combat_end" => BranchStopClassification {
                boundary_requested: "combat_end",
                boundary_reached: false,
                outcome_censored: true,
                branch_truncated: true,
                truncation_reason: Some("horizon_cap_before_combat_end"),
            },
            other => BranchStopClassification {
                boundary_requested: "combat_end",
                boundary_reached: false,
                outcome_censored: true,
                branch_truncated: true,
                truncation_reason: Some(if other == "no_continuation_action" {
                    "no_continuation_action"
                } else {
                    "unexpected_stop_before_combat_end"
                }),
            },
        },
    }
}

fn branch_trace_stop_reason(
    horizon_mode: BranchHorizonMode,
    start_info: &FullRunEnvInfo,
    branch_env: &FullRunEnv,
    last: &TimeStep,
    continuation_steps: usize,
    horizon_decisions: usize,
) -> Option<&'static str> {
    if last.terminated {
        return Some("terminated");
    }
    if last.truncated {
        return Some("truncated");
    }
    let info = branch_env.info();
    if info.result != "ongoing" {
        return Some("non_ongoing_result");
    }
    if matches!(horizon_mode, BranchHorizonMode::CombatEndV1)
        && info.combat_win_count > start_info.combat_win_count
    {
        return Some("combat_end");
    }
    if continuation_steps >= horizon_decisions {
        return Some(match horizon_mode {
            BranchHorizonMode::FixedDecisions => "horizon_decision_cap",
            BranchHorizonMode::CombatEndV1 => "horizon_decision_cap_before_combat_end",
        });
    }
    None
}

fn branch_trace_pairwise_comparisons(traces: &[BranchTraceV1]) -> Vec<BranchComparisonV1> {
    let mut comparisons = Vec::new();
    for left_index in 0..traces.len() {
        for right_index in (left_index + 1)..traces.len() {
            comparisons.push(BranchComparisonV1::from_traces(
                &traces[left_index],
                &traces[right_index],
            ));
        }
    }
    comparisons
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
