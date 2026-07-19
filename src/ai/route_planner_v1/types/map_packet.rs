use serde::{Deserialize, Serialize};

use super::context::RouteDecisionContextV1;
use super::features::{
    MapRouteTargetV1, NodeFeaturesV1, PathSurvivalEnvelopeV1, RouteCandidateViabilityV1,
    RouteMoveKindV1, RoutePathSummaryV1, RouteSafetyFlagV1,
};
use super::score::{NeedVectorV1, RouteScoreTermsV1, RouteValueFactorsV1};
use super::trace::{RouteDecisionTraceV1, RouteObjectiveV1, RouteSelectionModeV1};

pub const MAP_DECISION_PACKET_SCHEMA_NAME: &str = "MapDecisionPacketV1";
pub const MAP_DECISION_PACKET_SCHEMA_VERSION: u32 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapDecisionPacketV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub objective: RouteObjectiveV1,
    pub selection_mode: RouteSelectionModeV1,
    pub label_role: String,
    pub context: RouteDecisionContextV1,
    pub selected_index: Option<usize>,
    pub candidate_pool: RouteCandidatePoolProvenanceV1,
    pub candidates: Vec<RouteMoveCandidateV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCandidatePoolProvenanceV1 {
    pub legal_candidate_count: usize,
    pub emitted_candidate_count: usize,
    pub normal_edge_count: usize,
    pub wing_boots_jump_count: usize,
    pub complete_legal_pool: bool,
    pub ordering: RouteCandidateOrderingV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteCandidateOrderingV1 {
    SafetyThenScoreThenX,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMoveCandidateV1 {
    pub candidate_id: String,
    pub rank: usize,
    pub target: MapRouteTargetV1,
    pub action: RouteMapActionV1,
    pub command: String,
    pub features: NodeFeaturesV1,
    pub projection: RouteProjectionFrontierV1,
    pub needs: NeedVectorV1,
    pub evaluation: RouteMoveEvaluationV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteMapActionV1 {
    Go { x: i32 },
    Fly { x: i32, y: i32 },
}

impl RouteMapActionV1 {
    pub fn command(&self) -> String {
        match self {
            Self::Go { x } => format!("go {x}"),
            Self::Fly { x, y } => format!("fly {x} {y}"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteProjectionFrontierV1 {
    pub path_summary: RoutePathSummaryV1,
    #[serde(default)]
    pub viability: RouteCandidateViabilityV1,
    #[serde(default)]
    pub survival_envelope: PathSurvivalEnvelopeV1,
    pub metadata: RouteProjectionMetadataV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteProjectionMetadataV1 {
    pub source: RouteProjectionSourceV1,
    pub path_budget: usize,
    pub observed_path_count: usize,
    pub coverage: RouteProjectionCoverageV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteProjectionSourceV1 {
    VisibleMapDfs,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteProjectionCoverageV1 {
    CompleteWithinBudget,
    PossiblyTruncatedByPathBudget,
    NoVisibleContinuation,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMoveEvaluationV1 {
    pub value_source: RouteEvaluationSourceV1,
    pub calibration_status: RouteEvaluationCalibrationStatusV1,
    pub safety: RouteSafetyFlagV1,
    #[serde(default)]
    pub value_factors: RouteValueFactorsV1,
    pub score_terms: RouteScoreTermsV1,
    pub total_score: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_cautions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteEvaluationSourceV1 {
    HeuristicRoutePlannerV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteEvaluationCalibrationStatusV1 {
    UncalibratedBehaviorEstimate,
}

impl MapDecisionPacketV1 {
    pub fn from_route_decision_trace_v1(trace: &RouteDecisionTraceV1) -> Self {
        Self {
            schema_name: MAP_DECISION_PACKET_SCHEMA_NAME.to_string(),
            schema_version: MAP_DECISION_PACKET_SCHEMA_VERSION,
            objective: trace.objective,
            selection_mode: trace.selection_mode,
            label_role: trace.label_role.clone(),
            context: trace.context.clone(),
            selected_index: trace.selected_index,
            candidate_pool: route_candidate_pool_provenance_v1(trace),
            candidates: trace
                .candidates
                .iter()
                .enumerate()
                .map(|(rank, candidate)| {
                    let action = route_action_from_target_v1(&candidate.target);
                    RouteMoveCandidateV1 {
                        candidate_id: route_move_candidate_id_v1(&candidate.target),
                        rank,
                        target: candidate.target.clone(),
                        command: candidate
                            .suggested_command
                            .clone()
                            .unwrap_or_else(|| action.command()),
                        action,
                        features: candidate.features.clone(),
                        projection: RouteProjectionFrontierV1 {
                            path_summary: candidate.path_summary.clone(),
                            viability: candidate.viability.clone(),
                            survival_envelope: candidate.survival_envelope.clone(),
                            metadata: route_projection_metadata_v1(
                                candidate.path_summary.path_count,
                                trace.path_budget,
                                candidate.path_summary.path_budget_exhausted,
                            ),
                        },
                        needs: candidate.needs.clone(),
                        evaluation: RouteMoveEvaluationV1 {
                            value_source: RouteEvaluationSourceV1::HeuristicRoutePlannerV1,
                            calibration_status:
                                RouteEvaluationCalibrationStatusV1::UncalibratedBehaviorEstimate,
                            safety: candidate.safety,
                            value_factors: candidate.value_factors.clone(),
                            score_terms: candidate.score_terms.clone(),
                            total_score: candidate.total_score,
                            legacy_reasons: candidate.reasons.clone(),
                            legacy_cautions: candidate.cautions.clone(),
                        },
                    }
                })
                .collect(),
            warnings: trace.warnings.clone(),
        }
    }
}

fn route_candidate_pool_provenance_v1(
    trace: &RouteDecisionTraceV1,
) -> RouteCandidatePoolProvenanceV1 {
    let normal_edge_count = trace
        .candidates
        .iter()
        .filter(|candidate| candidate.target.move_kind == RouteMoveKindV1::NormalEdge)
        .count();
    let wing_boots_jump_count = trace
        .candidates
        .iter()
        .filter(|candidate| candidate.target.move_kind == RouteMoveKindV1::WingBootsJump)
        .count();
    RouteCandidatePoolProvenanceV1 {
        legal_candidate_count: trace.context.legal_next_nodes.len(),
        emitted_candidate_count: trace.candidates.len(),
        normal_edge_count,
        wing_boots_jump_count,
        complete_legal_pool: trace.context.legal_next_nodes.len() == trace.candidates.len(),
        ordering: RouteCandidateOrderingV1::SafetyThenScoreThenX,
    }
}

fn route_projection_metadata_v1(
    observed_path_count: usize,
    path_budget: usize,
    path_budget_exhausted: bool,
) -> RouteProjectionMetadataV1 {
    RouteProjectionMetadataV1 {
        source: RouteProjectionSourceV1::VisibleMapDfs,
        path_budget,
        observed_path_count,
        coverage: route_projection_coverage_v1(observed_path_count, path_budget_exhausted),
    }
}

fn route_projection_coverage_v1(
    observed_path_count: usize,
    path_budget_exhausted: bool,
) -> RouteProjectionCoverageV1 {
    if observed_path_count == 0 {
        RouteProjectionCoverageV1::NoVisibleContinuation
    } else if path_budget_exhausted {
        RouteProjectionCoverageV1::PossiblyTruncatedByPathBudget
    } else {
        RouteProjectionCoverageV1::CompleteWithinBudget
    }
}

fn route_action_from_target_v1(target: &MapRouteTargetV1) -> RouteMapActionV1 {
    match target.move_kind {
        RouteMoveKindV1::NormalEdge => RouteMapActionV1::Go { x: target.x },
        RouteMoveKindV1::WingBootsJump => RouteMapActionV1::Fly {
            x: target.x,
            y: target.y,
        },
    }
}

fn route_move_candidate_id_v1(target: &MapRouteTargetV1) -> String {
    format!(
        "route_move:{}:x{}:y{}",
        route_move_kind_candidate_id_v1(target.move_kind),
        target.x,
        target.y
    )
}

fn route_move_kind_candidate_id_v1(kind: RouteMoveKindV1) -> &'static str {
    match kind {
        RouteMoveKindV1::NormalEdge => "normal_edge",
        RouteMoveKindV1::WingBootsJump => "wing_boots_jump",
    }
}
