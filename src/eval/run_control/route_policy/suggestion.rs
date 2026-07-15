use crate::ai::route_planner_v1::render_route_decision_trace_v1;

use super::super::session::RunControlSession;
use super::planner::plan_route_for_session;

pub(in crate::eval::run_control) fn render_route_suggestion(session: &RunControlSession) -> String {
    let trace = plan_route_for_session(session);
    render_route_decision_trace_v1(&trace)
}
