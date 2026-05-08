use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DECISION_ENV_CONTRACT_VERSION: &str = "decision_env_contract_v0";
pub const DECISION_RECORD_SCHEMA_VERSION: &str = "decision_record_v0";
pub const REWARD_EVENT_SCHEMA_VERSION: &str = "reward_event_v0";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunSeed(pub u64);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub usize);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionId {
    pub episode_id: String,
    pub step_index: usize,
    pub decision_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnvConfig {
    pub seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: String,
    pub max_steps: usize,
    pub reward_shaping_profile: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationVisibility {
    Public,
    Oracle,
    Debug,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservationPayload {
    pub schema_version: String,
    pub visibility: ObservationVisibility,
    pub decision_type: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActionCandidate {
    pub id: ActionId,
    pub action_schema_version: String,
    pub action_index: usize,
    pub action_key: String,
    pub action_kind: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RewardEvent {
    pub schema_version: String,
    pub scalar_reward: f32,
    pub components: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StepInfo {
    pub state_hash: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeStep {
    pub contract_version: String,
    pub decision_id: DecisionId,
    pub observation: ObservationPayload,
    pub candidates: Vec<ActionCandidate>,
    pub reward: RewardEvent,
    pub terminated: bool,
    pub truncated: bool,
    pub info: StepInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CandidateLabel {
    pub action_id: ActionId,
    pub mean_return: Option<f32>,
    pub stderr: Option<f32>,
    pub sample_count: u32,
    pub dominance: Option<String>,
    pub confidence: Option<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PairwisePreference {
    pub preferred: ActionId,
    pub other: ActionId,
    pub margin: Option<f32>,
    pub confidence: Option<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TeacherDecisionLabel {
    pub teacher_spec_version: String,
    pub return_spec_version: String,
    pub labels: Vec<CandidateLabel>,
    pub pairwise_preferences: Vec<PairwisePreference>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecord {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub parent_decision_id: Option<DecisionId>,
    pub sim_version: String,
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub return_spec_version: String,
    pub seed: u64,
    pub state_hash_before: String,
    pub observation: ObservationPayload,
    pub candidates: Vec<ActionCandidate>,
    pub behavior_action: Option<ActionId>,
    pub teacher_label: Option<TeacherDecisionLabel>,
    pub reward_since_prev: RewardEvent,
    pub terminated: bool,
    pub truncated: bool,
    pub state_hash_after: Option<String>,
    pub info: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecordContext {
    pub parent_decision_id: Option<DecisionId>,
    pub sim_version: String,
    pub return_spec_version: String,
    pub seed: u64,
    pub behavior_action: Option<ActionId>,
    pub teacher_label: Option<TeacherDecisionLabel>,
    pub state_hash_after: Option<String>,
    pub info: Value,
}

impl DecisionRecordContext {
    pub fn new(
        sim_version: impl Into<String>,
        return_spec_version: impl Into<String>,
        seed: u64,
    ) -> Self {
        Self {
            parent_decision_id: None,
            sim_version: sim_version.into(),
            return_spec_version: return_spec_version.into(),
            seed,
            behavior_action: None,
            teacher_label: None,
            state_hash_after: None,
            info: Value::Null,
        }
    }
}

impl DecisionRecord {
    pub fn from_timestep(timestep: &TimeStep, context: DecisionRecordContext) -> Self {
        Self {
            schema_version: DECISION_RECORD_SCHEMA_VERSION.to_string(),
            decision_id: timestep.decision_id.clone(),
            parent_decision_id: context.parent_decision_id,
            sim_version: context.sim_version,
            observation_schema_version: timestep.observation.schema_version.clone(),
            action_schema_version: action_schema_version(timestep),
            return_spec_version: context.return_spec_version,
            seed: context.seed,
            state_hash_before: timestep.info.state_hash.clone(),
            observation: timestep.observation.clone(),
            candidates: timestep.candidates.clone(),
            behavior_action: context.behavior_action,
            teacher_label: context.teacher_label,
            reward_since_prev: timestep.reward.clone(),
            terminated: timestep.terminated,
            truncated: timestep.truncated,
            state_hash_after: context.state_hash_after,
            info: serde_json::json!({
                "timestep_info": timestep.info.payload,
                "record_context": context.info,
            }),
        }
    }

    pub fn from_decision_and_outcome(
        decision: &TimeStep,
        outcome: &TimeStep,
        mut context: DecisionRecordContext,
    ) -> Self {
        let state_hash_after = context
            .state_hash_after
            .take()
            .unwrap_or_else(|| outcome.info.state_hash.clone());
        Self {
            schema_version: DECISION_RECORD_SCHEMA_VERSION.to_string(),
            decision_id: decision.decision_id.clone(),
            parent_decision_id: context.parent_decision_id,
            sim_version: context.sim_version,
            observation_schema_version: decision.observation.schema_version.clone(),
            action_schema_version: action_schema_version(decision),
            return_spec_version: context.return_spec_version,
            seed: context.seed,
            state_hash_before: decision.info.state_hash.clone(),
            observation: decision.observation.clone(),
            candidates: decision.candidates.clone(),
            behavior_action: context.behavior_action,
            teacher_label: context.teacher_label,
            reward_since_prev: outcome.reward.clone(),
            terminated: outcome.terminated,
            truncated: outcome.truncated,
            state_hash_after: Some(state_hash_after),
            info: serde_json::json!({
                "decision_timestep_info": decision.info.payload,
                "outcome_timestep_info": outcome.info.payload,
                "record_context": context.info,
            }),
        }
    }
}

fn action_schema_version(timestep: &TimeStep) -> String {
    timestep
        .candidates
        .first()
        .map(|candidate| candidate.action_schema_version.clone())
        .unwrap_or_default()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionEnvError {
    pub message: String,
}

impl DecisionEnvError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DecisionEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DecisionEnvError {}

pub trait DecisionEnv {
    type Snapshot: Clone;

    fn reset(&mut self, seed: RunSeed, config: EnvConfig) -> Result<TimeStep, DecisionEnvError>;
    fn current_timestep(&mut self) -> Result<TimeStep, DecisionEnvError>;
    fn step(&mut self, action: ActionId) -> Result<TimeStep, DecisionEnvError>;
    fn snapshot(&self) -> Result<Self::Snapshot, DecisionEnvError>;
    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), DecisionEnvError>;
}
