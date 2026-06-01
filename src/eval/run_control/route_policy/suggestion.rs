use crate::ai::route_planner_v1::{
    render_route_decision_trace_v1, route_targets as ai_route_targets,
    summarize_route_from as ai_summarize_route_from, MapRouteTargetV1, RoutePathSummaryV1,
};

use super::super::session::RunControlSession;
use super::planner::plan_route_for_session;

pub(in crate::eval::run_control) fn render_route_suggestion(session: &RunControlSession) -> String {
    let trace = plan_route_for_session(session);
    render_route_decision_trace_v1(&trace)
}

pub(in crate::eval::run_control) fn route_targets(
    session: &RunControlSession,
) -> Vec<MapRouteTargetV1> {
    ai_route_targets(&session.run_state)
}

pub(in crate::eval::run_control) fn summarize_route_from(
    session: &RunControlSession,
    x: i32,
    y: i32,
) -> RoutePathSummaryV1 {
    ai_summarize_route_from(&session.run_state, x, y, &Default::default())
}
