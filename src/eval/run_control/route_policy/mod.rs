use crate::ai::route_planner_v1::{
    plan_route_decision_v1, render_route_decision_trace_v1, route_targets as ai_route_targets,
    summarize_route_from as ai_summarize_route_from, MapRouteTargetV1, RoutePathSummaryV1,
    RoutePlannerConfigV1,
};

use super::session::RunControlSession;

pub(in crate::eval::run_control) fn render_route_suggestion(session: &RunControlSession) -> String {
    let trace = plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        RoutePlannerConfigV1::default(),
    );
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
    ai_summarize_route_from(&session.run_state, x, y, &RoutePlannerConfigV1::default())
}

pub(in crate::eval::run_control) fn format_range(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

pub(in crate::eval::run_control) fn recovery_label(summary: &RoutePathSummaryV1) -> &'static str {
    if summary.min_fires > 0 {
        "rest site exists on every visible path"
    } else if summary.max_fires > 0 {
        "rest site exists on some visible paths"
    } else {
        "not visible on this route"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::session::RunControlConfig;
    use crate::state::core::EngineState;

    #[test]
    fn route_suggestion_is_read_only_before_map_navigation() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let rendered = render_route_suggestion(&session);

        assert!(rendered.contains("read-only"));
        assert!(rendered.contains("route selection is locked"));
        assert!(!rendered.contains("Suggested command: go"));
    }

    #[test]
    fn route_suggestion_recommends_without_mutating_map_position() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let before = (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
        );

        let rendered = render_route_suggestion(&session);

        assert!(rendered.contains("Route suggestion"));
        assert!(rendered.contains("Suggested command: go"));
        assert_eq!(
            before,
            (
                session.run_state.map.current_x,
                session.run_state.map.current_y
            )
        );
    }

    #[test]
    fn route_suggest_command_is_read_only() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let before = (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.decision_step,
        );

        let outcome = session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::RouteSuggest)
            .expect("route-suggest should render");

        assert!(outcome.message.contains("Route suggestion"));
        assert!(outcome.action_result.is_none());
        assert_eq!(
            before,
            (
                session.run_state.map.current_x,
                session.run_state.map.current_y,
                session.decision_step
            )
        );
    }
}
