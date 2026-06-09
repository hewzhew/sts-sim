mod context;
mod features;
mod needs;
mod policy;
mod render;
mod scorer;
mod types;

pub use context::build_route_decision_context_v1;
pub use features::{route_targets, summarize_route_from};
pub use policy::plan_route_decision_v1;
pub use render::render_route_decision_trace_v1;
pub use types::{
    DeckRouteSummaryV1, MapRouteTargetV1, NeedVectorV1, NodeFeaturesV1, PotionRouteSummaryV1,
    RouteCandidateTraceV1, RouteCountersV1, RouteDecisionContextV1, RouteDecisionTraceV1,
    RouteFirstEliteSegmentV1, RouteMoveKindV1, RouteObjectiveV1, RoutePathSummaryV1,
    RoutePlannerConfigV1, RouteRelicSummaryV1, RouteSafetyFlagV1, RouteScoreTermsV1,
    RouteSelectionModeV1, UnknownRoomBeliefV1, ROUTE_DECISION_TRACE_SCHEMA_NAME,
    ROUTE_DECISION_TRACE_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests;
