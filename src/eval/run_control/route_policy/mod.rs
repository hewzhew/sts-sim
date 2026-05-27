use crate::ai::route_planner_v1::{
    plan_route_decision_v1, render_route_decision_trace_v1, route_targets as ai_route_targets,
    summarize_route_from as ai_summarize_route_from, MapRouteTargetV1, RouteCandidateTraceV1,
    RouteMoveKindV1, RoutePathSummaryV1, RoutePlannerConfigV1, RouteSafetyFlagV1,
};
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::{RoutePlannerCandidateSummaryV1, RunControlTraceAnnotationV1};
use super::view_model::room_type_label;

pub(in crate::eval::run_control) struct RouteGoApplied {
    pub outcome: RunControlCommandOutcome,
    pub auto_step_summary: String,
}

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
    Ok(apply_route_go_with_summary(session)?.outcome)
}

pub(in crate::eval::run_control) fn apply_route_go_with_summary(
    session: &mut RunControlSession,
) -> Result<RouteGoApplied, String> {
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
    let auto_step_summary = render_route_go_auto_step_summary(&candidate);
    let trace_annotation =
        route_go_trace_annotation(&trace, selected_index, &candidate, &auto_step_summary);
    let outcome = session.apply_input(input)?;
    Ok(RouteGoApplied {
        auto_step_summary,
        outcome: RunControlCommandOutcome {
            message: format!("{selection}\n{}", outcome.message),
            ..outcome
        }
        .with_trace_annotations(vec![trace_annotation]),
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

fn render_route_go_auto_step_summary(candidate: &RouteCandidateTraceV1) -> String {
    let command = candidate
        .suggested_command
        .as_deref()
        .unwrap_or("unknown-command");
    format!(
        "route planner: x={} {} [{} score={:.2}] command={} label_role=behavior_policy_not_teacher",
        candidate.target.x,
        room_type_label(candidate.target.room_type),
        safety_label(candidate.safety),
        candidate.total_score,
        command,
    )
}

fn route_go_trace_annotation(
    trace: &crate::ai::route_planner_v1::RouteDecisionTraceV1,
    selected_index: usize,
    candidate: &RouteCandidateTraceV1,
    summary: &str,
) -> RunControlTraceAnnotationV1 {
    RunControlTraceAnnotationV1::RoutePlannerSelection {
        summary: summary.to_string(),
        selected_index: Some(selected_index),
        candidate_count: trace.candidates.len(),
        target_x: candidate.target.x,
        target_y: candidate.target.y,
        room_type: room_type_label(candidate.target.room_type).to_string(),
        move_kind: format!("{:?}", candidate.target.move_kind),
        safety: safety_label(candidate.safety).to_string(),
        score: candidate.total_score,
        command: candidate
            .suggested_command
            .as_deref()
            .unwrap_or("unknown-command")
            .to_string(),
        top_candidates: route_go_top_candidate_summaries(trace),
        label_role: "behavior_policy_not_teacher".to_string(),
    }
}

fn route_go_top_candidate_summaries(
    trace: &crate::ai::route_planner_v1::RouteDecisionTraceV1,
) -> Vec<RoutePlannerCandidateSummaryV1> {
    trace
        .candidates
        .iter()
        .take(3)
        .enumerate()
        .map(|(rank, candidate)| RoutePlannerCandidateSummaryV1 {
            rank,
            target_x: candidate.target.x,
            target_y: candidate.target.y,
            room_type: room_type_label(candidate.target.room_type).to_string(),
            move_kind: format!("{:?}", candidate.target.move_kind),
            safety: safety_label(candidate.safety).to_string(),
            score: candidate.total_score,
            command: candidate
                .suggested_command
                .as_deref()
                .unwrap_or("unknown-command")
                .to_string(),
        })
        .collect()
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
