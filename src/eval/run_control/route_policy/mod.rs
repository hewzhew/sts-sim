use crate::ai::route_planner_v1::{
    plan_route_decision_v1, render_route_decision_trace_v1, route_targets as ai_route_targets,
    summarize_route_from as ai_summarize_route_from, MapRouteTargetV1, RouteCandidateTraceV1,
    RouteMoveKindV1, RoutePathSummaryV1, RoutePlannerConfigV1, RouteSafetyFlagV1,
};
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::view_model::room_type_label;

pub(in crate::eval::run_control) fn render_route_suggestion(session: &RunControlSession) -> String {
    let trace = plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        RoutePlannerConfigV1::default(),
    );
    render_route_decision_trace_v1(&trace)
}

pub(in crate::eval::run_control) fn apply_route_go(
    session: &mut RunControlSession,
) -> Result<RunControlCommandOutcome, String> {
    if !matches!(session.engine_state, EngineState::MapNavigation) {
        return Err(format!(
            "route-go is only valid on Map. Use `rs` for read-only route evidence from this screen.\n{}",
            render_route_suggestion(session)
        ));
    }

    let trace = plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        RoutePlannerConfigV1::default(),
    );
    let selected_index = trace
        .selected_index
        .ok_or_else(|| "route planner found no legal map target".to_string())?;
    let candidate = trace
        .candidates
        .get(selected_index)
        .cloned()
        .ok_or_else(|| "route planner selected an out-of-range map target".to_string())?;
    if candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative {
        return Err(format!(
            "route planner selected only reject-unless-forced routes; inspect with `rs` and choose manually with `go <x>` or `fly <x> <y>`.\n{}",
            render_route_go_selection(&candidate)
        ));
    }

    let input = route_candidate_input(&candidate)?;
    let selection = render_route_go_selection(&candidate);
    let outcome = session.apply_input(input)?;
    Ok(RunControlCommandOutcome {
        message: format!("{selection}\n{}", outcome.message),
        ..outcome
    })
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

fn route_candidate_input(candidate: &RouteCandidateTraceV1) -> Result<ClientInput, String> {
    let x = usize::try_from(candidate.target.x).map_err(|_| {
        format!(
            "route planner selected invalid map x={}",
            candidate.target.x
        )
    })?;
    let y = usize::try_from(candidate.target.y).map_err(|_| {
        format!(
            "route planner selected invalid map y={}",
            candidate.target.y
        )
    })?;
    match candidate.target.move_kind {
        RouteMoveKindV1::NormalEdge => Ok(ClientInput::SelectMapNode(x)),
        RouteMoveKindV1::WingBootsJump => Ok(ClientInput::FlyToNode(x, y)),
    }
}

fn render_route_go_selection(candidate: &RouteCandidateTraceV1) -> String {
    let mut lines = Vec::new();
    lines.push("Route planner selected:".to_string());
    lines.push(format!(
        "  x={} {} [{} score={:.2}]",
        candidate.target.x,
        room_type_label(candidate.target.room_type),
        safety_label(candidate.safety),
        candidate.total_score
    ));
    if candidate.target.move_kind == RouteMoveKindV1::WingBootsJump {
        lines.push("  uses Wing Boots charge".to_string());
    }
    if let Some(command) = candidate.suggested_command.as_ref() {
        lines.push(format!("  command: {command}"));
    }
    lines.push("  label_role: behavior_policy_not_teacher".to_string());
    if !candidate.reasons.is_empty() {
        lines.push(format!("  reason: {}", candidate.reasons.join("; ")));
    }
    if !candidate.cautions.is_empty() {
        lines.push(format!("  caution: {}", candidate.cautions.join("; ")));
    }
    lines.join("\n")
}

fn safety_label(safety: RouteSafetyFlagV1) -> &'static str {
    match safety {
        RouteSafetyFlagV1::Ok => "ok",
        RouteSafetyFlagV1::RiskyButAllowed => "risky",
        RouteSafetyFlagV1::RejectUnlessNoAlternative => "reject_unless_forced",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::commands::RunControlCommand;
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

    #[test]
    fn route_go_rejects_locked_route_selection() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let err = apply_route_go(&mut session).expect_err("route-go should reject Neow bonus");

        assert!(err.contains("route-go is only valid on Map"));
        assert!(err.contains("route selection is locked"));
        assert_eq!(session.decision_step, 1);
    }

    #[test]
    fn route_go_executes_selected_map_target() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let before_y = session.run_state.map.current_y;

        let outcome = apply_route_go(&mut session).expect("route-go should choose a map node");

        assert!(outcome.message.contains("Route planner selected:"));
        assert!(outcome
            .message
            .contains("label_role: behavior_policy_not_teacher"));
        assert!(outcome.action_result.is_some());
        assert!(session.run_state.map.current_y > before_y);
        assert_eq!(session.decision_step, 1);
    }
}
