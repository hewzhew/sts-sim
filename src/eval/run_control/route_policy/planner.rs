use crate::ai::route_planner_v1::{plan_route_decision_v1, RouteDecisionTraceV1};

use super::super::session::RunControlSession;

pub(super) fn plan_route_for_session(session: &RunControlSession) -> RouteDecisionTraceV1 {
    plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        Default::default(),
    )
}
