use serde::{Deserialize, Serialize};

use super::context::RouteDecisionContextV1;
use super::features::{
    MapRouteTargetV1, NodeFeaturesV1, RouteMoveKindV1, RoutePathSummaryV1, RouteSafetyFlagV1,
};
use super::score::{NeedVectorV1, RouteScoreTermsV1};
use super::trace::{RouteDecisionTraceV1, RouteObjectiveV1, RouteSelectionModeV1};

pub const MAP_DECISION_PACKET_SCHEMA_NAME: &str = "MapDecisionPacketV1";
pub const MAP_DECISION_PACKET_SCHEMA_VERSION: u32 = 1;

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
    pub candidates: Vec<RouteMoveCandidateV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMoveEvaluationV1 {
    pub safety: RouteSafetyFlagV1,
    pub score_terms: RouteScoreTermsV1,
    pub total_score: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_cautions: Vec<String>,
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
            candidates: trace
                .candidates
                .iter()
                .enumerate()
                .map(|(rank, candidate)| {
                    let action = route_action_from_target_v1(&candidate.target);
                    RouteMoveCandidateV1 {
                        candidate_id: route_move_candidate_id_v1(rank, &candidate.target),
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
                        },
                        needs: candidate.needs.clone(),
                        evaluation: RouteMoveEvaluationV1 {
                            safety: candidate.safety,
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

fn route_action_from_target_v1(target: &MapRouteTargetV1) -> RouteMapActionV1 {
    match target.move_kind {
        RouteMoveKindV1::NormalEdge => RouteMapActionV1::Go { x: target.x },
        RouteMoveKindV1::WingBootsJump => RouteMapActionV1::Fly {
            x: target.x,
            y: target.y,
        },
    }
}

fn route_move_candidate_id_v1(rank: usize, target: &MapRouteTargetV1) -> String {
    format!(
        "route_move:{rank}:{:?}:x{}:y{}",
        target.move_kind, target.x, target.y
    )
}
