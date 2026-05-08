use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::decision_env::{ActionId, DecisionId, PolicyInput};

pub const SEARCH_AWARE_POLICY_SCHEMA_VERSION: &str = "search_aware_policy_v0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CandidateScore {
    pub action_id: ActionId,
    pub score: f32,
    pub rank: usize,
    pub source: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyLevel {
    Low,
    Medium,
    High,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CandidateUncertainty {
    pub action_id: ActionId,
    pub level: UncertaintyLevel,
    pub reasons: Vec<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CandidateRiskFlags {
    pub action_id: ActionId,
    pub flags: Vec<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchHint {
    pub candidate_id: Option<ActionId>,
    pub search_kind: SearchKind,
    pub priority: f32,
    pub reason: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PolicyProposal {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub policy_id: String,
    pub prior_scores: Vec<CandidateScore>,
    pub uncertainty: Vec<CandidateUncertainty>,
    pub risk_flags: Vec<CandidateRiskFlags>,
    pub search_hints: Vec<SearchHint>,
    pub fast_path_allowed: bool,
    pub payload: Value,
}

impl PolicyProposal {
    pub fn legacy_fallback(
        input: &PolicyInput,
        policy_id: impl Into<String>,
        selected_action_id: Option<ActionId>,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
            decision_id: input.decision_id.clone(),
            policy_id: policy_id.into(),
            prior_scores: input
                .candidates
                .iter()
                .enumerate()
                .map(|(rank, candidate)| CandidateScore {
                    action_id: candidate.id,
                    score: if Some(candidate.id) == selected_action_id {
                        1.0
                    } else {
                        0.0
                    },
                    rank,
                    source: "legacy_fallback_selection".to_string(),
                    payload: Value::Null,
                })
                .collect(),
            uncertainty: selected_action_id
                .map(|action_id| CandidateUncertainty {
                    action_id,
                    level: UncertaintyLevel::Unknown,
                    reasons: vec!["legacy_fallback_has_no_model_uncertainty".to_string()],
                    payload: Value::Null,
                })
                .into_iter()
                .collect(),
            risk_flags: Vec::new(),
            search_hints: selected_action_id
                .map(|action_id| SearchHint {
                    candidate_id: Some(action_id),
                    search_kind: SearchKind::LegacyRootSearch { depth_limit: None },
                    priority: 1.0,
                    reason: "legacy_fallback_candidate_anchor".to_string(),
                    payload: Value::Null,
                })
                .into_iter()
                .collect(),
            fast_path_allowed: false,
            payload,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchKind {
    NeutralOneStepTransition,
    NeutralStableTransition {
        max_engine_steps: u32,
    },
    NeutralBranchCompression {
        max_engine_steps: u32,
    },
    ExactTurn {
        max_nodes: Option<u32>,
        stop_at_end_turn: bool,
    },
    LethalVerifier,
    DeathVerifier,
    PairwiseCompare {
        other: ActionId,
        horizon: HorizonSpec,
    },
    Rollout {
        horizon: HorizonSpec,
        continuation_policy: String,
        num_rollouts: u32,
    },
    DominanceCheck,
    EquivalenceCheck,
    LegacyRootSearch {
        depth_limit: Option<u32>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HorizonSpec {
    Decisions(u32),
    CombatEnd { max_decisions: u32 },
    StableBoundary { max_decisions: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Exactness {
    Exact,
    BoundedExact,
    Sampled,
    HeuristicOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchBudget {
    pub time_budget_ms: u32,
    pub max_requests: usize,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchRequest {
    pub request_id: String,
    pub decision_id: DecisionId,
    pub candidate_id: Option<ActionId>,
    pub search_kind: SearchKind,
    pub reason: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchPlan {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub budget: SearchBudget,
    pub requests: Vec<SearchRequest>,
    pub mandatory_anchor_action_ids: Vec<ActionId>,
    pub payload: Value,
}

impl SearchPlan {
    pub fn from_hints(
        input: &PolicyInput,
        hints: &[SearchHint],
        budget: SearchBudget,
        payload: Value,
    ) -> Self {
        let requests = hints
            .iter()
            .enumerate()
            .map(|(index, hint)| SearchRequest {
                request_id: format!("search_request_{index}"),
                decision_id: input.decision_id.clone(),
                candidate_id: hint.candidate_id,
                search_kind: hint.search_kind.clone(),
                reason: hint.reason.clone(),
                payload: hint.payload.clone(),
            })
            .collect::<Vec<_>>();
        let mandatory_anchor_action_ids = hints
            .iter()
            .filter_map(|hint| hint.candidate_id)
            .collect::<Vec<_>>();
        Self {
            schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
            decision_id: input.decision_id.clone(),
            budget,
            requests,
            mandatory_anchor_action_ids,
            payload,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchEvidence {
    pub evidence_id: String,
    pub decision_id: DecisionId,
    pub candidate_id: Option<ActionId>,
    pub request_id: Option<String>,
    pub search_kind: SearchKind,
    pub exactness: Exactness,
    pub truncated: bool,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionMode {
    FastPriorAccepted,
    ExactTurnResolved,
    RolloutResolved,
    DominanceResolved,
    EvidenceTieBrokenByModel,
    EvidenceTieBrokenByLegacy,
    LegacyFallback,
    TimeoutFallback,
    SafetyFallback,
    NoLegalAction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PolicyDecision {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub policy_id: String,
    pub selected_action_id: Option<ActionId>,
    pub mode: DecisionMode,
    pub confidence: String,
    pub fallback_reason: Option<String>,
    pub evidence_used: Vec<String>,
    pub payload: Value,
}

impl PolicyDecision {
    pub fn legacy_fallback(
        input: &PolicyInput,
        selected_action_id: Option<ActionId>,
        evidence: &[SearchEvidence],
        reason: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
            decision_id: input.decision_id.clone(),
            policy_id: "legacy_frontier_fallback".to_string(),
            selected_action_id,
            mode: if selected_action_id.is_some() {
                DecisionMode::LegacyFallback
            } else {
                DecisionMode::NoLegalAction
            },
            confidence: "fallback".to_string(),
            fallback_reason: Some(reason.into()),
            evidence_used: evidence
                .iter()
                .map(|item| item.evidence_id.clone())
                .collect(),
            payload,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeliberationTrace {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub input_schema_version: String,
    pub proposal: PolicyProposal,
    pub search_plan: SearchPlan,
    pub evidence: Vec<SearchEvidence>,
    pub decision: PolicyDecision,
}

impl DeliberationTrace {
    pub fn new(
        input: &PolicyInput,
        proposal: PolicyProposal,
        search_plan: SearchPlan,
        evidence: Vec<SearchEvidence>,
        decision: PolicyDecision,
    ) -> Self {
        Self {
            schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
            decision_id: input.decision_id.clone(),
            input_schema_version: input.schema_version.clone(),
            proposal,
            search_plan,
            evidence,
            decision,
        }
    }
}

pub trait SearchAwarePolicyRunner {
    fn propose(&self, input: &PolicyInput) -> PolicyProposal;

    fn request_search(
        &self,
        input: &PolicyInput,
        proposal: &PolicyProposal,
        budget: SearchBudget,
    ) -> SearchPlan;

    fn decide(
        &self,
        input: &PolicyInput,
        proposal: &PolicyProposal,
        evidence: &[SearchEvidence],
    ) -> PolicyDecision;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::verification::decision_env::{
        ObservationPayload, ObservationVisibility, PublicActionCandidateView,
        POLICY_INPUT_SCHEMA_VERSION,
    };

    fn sample_policy_input() -> PolicyInput {
        PolicyInput {
            schema_version: POLICY_INPUT_SCHEMA_VERSION.to_string(),
            decision_id: DecisionId {
                episode_id: "test".to_string(),
                step_index: 3,
                decision_type: "combat".to_string(),
            },
            observation: ObservationPayload {
                schema_version: "public_obs_v0".to_string(),
                visibility: ObservationVisibility::Public,
                decision_type: "combat".to_string(),
                payload: json!({"hp": 80}),
            },
            candidates: vec![
                PublicActionCandidateView {
                    id: ActionId(0),
                    action_schema_version: "action_v0".to_string(),
                    action_index: 0,
                    action_key: "end_turn".to_string(),
                    action_kind: "end_turn".to_string(),
                    payload: Value::Null,
                },
                PublicActionCandidateView {
                    id: ActionId(1),
                    action_schema_version: "action_v0".to_string(),
                    action_index: 1,
                    action_key: "play_card/0".to_string(),
                    action_kind: "play_card".to_string(),
                    payload: Value::Null,
                },
            ],
            time_budget_ms: 100,
        }
    }

    #[test]
    fn legacy_fallback_trace_serializes_without_claiming_verified_evidence() {
        let input = sample_policy_input();
        let proposal = PolicyProposal::legacy_fallback(
            &input,
            "legacy_frontier_prior",
            Some(ActionId(1)),
            json!({}),
        );
        assert!(!proposal.fast_path_allowed);
        assert_eq!(proposal.search_hints.len(), 1);
        let budget = SearchBudget {
            time_budget_ms: input.time_budget_ms,
            max_requests: 1,
            payload: Value::Null,
        };
        let plan = SearchPlan::from_hints(&input, &proposal.search_hints, budget, Value::Null);
        let evidence = vec![SearchEvidence {
            evidence_id: "legacy_root_search_0".to_string(),
            decision_id: input.decision_id.clone(),
            candidate_id: Some(ActionId(1)),
            request_id: plan
                .requests
                .first()
                .map(|request| request.request_id.clone()),
            search_kind: SearchKind::LegacyRootSearch {
                depth_limit: Some(2),
            },
            exactness: Exactness::HeuristicOnly,
            truncated: false,
            payload: json!({"score": 1.0}),
        }];
        let decision = PolicyDecision::legacy_fallback(
            &input,
            Some(ActionId(1)),
            &evidence,
            "no_model_policy_available",
            Value::Null,
        );
        let trace = DeliberationTrace::new(&input, proposal, plan, evidence, decision);
        let serialized = serde_json::to_value(trace).unwrap();
        assert_eq!(
            serialized.pointer("/decision/mode").and_then(Value::as_str),
            Some("legacy_fallback")
        );
        assert_eq!(
            serialized
                .pointer("/evidence/0/exactness")
                .and_then(Value::as_str),
            Some("heuristic_only")
        );
    }
}
