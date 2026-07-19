use serde::{Deserialize, Serialize};

use super::context::RouteDecisionContextV1;
use super::features::{
    MapRouteTargetV1, NodeFeaturesV1, PathSurvivalEnvelopeV1, RouteCandidateViabilityV1,
    RoutePathSummaryV1, RouteSafetyFlagV1,
};
use super::score::{NeedVectorV1, RouteScoreTermsV1, RouteValueFactorsV1};

pub const ROUTE_DECISION_TRACE_SCHEMA_NAME: &str = "RouteDecisionTraceV1";
pub const ROUTE_DECISION_TRACE_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteDecisionTraceV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub objective: RouteObjectiveV1,
    pub selection_mode: RouteSelectionModeV1,
    pub label_role: String,
    pub context: RouteDecisionContextV1,
    #[serde(default)]
    pub path_budget: usize,
    pub candidates: Vec<RouteCandidateTraceV1>,
    pub selected_index: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteObjectiveV1 {
    DataCollectionSurvivalV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteSelectionModeV1 {
    DeterministicArgmax,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteCandidateTraceV1 {
    pub target: MapRouteTargetV1,
    pub features: NodeFeaturesV1,
    pub path_summary: RoutePathSummaryV1,
    #[serde(default)]
    pub viability: RouteCandidateViabilityV1,
    #[serde(default)]
    pub survival_envelope: PathSurvivalEnvelopeV1,
    pub needs: NeedVectorV1,
    #[serde(default)]
    pub value_factors: RouteValueFactorsV1,
    pub score_terms: RouteScoreTermsV1,
    pub total_score: f32,
    pub safety: RouteSafetyFlagV1,
    pub reasons: Vec<String>,
    pub cautions: Vec<String>,
    pub suggested_command: Option<String>,
}
