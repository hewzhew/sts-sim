use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteMoveKindV1, RouteSafetyFlagV1};
use crate::state::core::ClientInput;

use super::super::session::{RunControlCommandOutcome, RunControlSession};
use super::super::view_model::{build_run_control_view_model, CandidateAction};
use super::format::{render_route_go_auto_step_summary, render_route_go_selection};
use super::planner::plan_route_for_session;
use super::suggestion::render_route_suggestion;
use super::trace::{route_go_trace_annotation, route_policy_stop_annotation};

pub(in crate::eval::run_control) struct RouteGoApplied {
    pub outcome: RunControlCommandOutcome,
    pub auto_step_summary: String,
}

pub(in crate::eval::run_control) fn apply_route_go(
    session: &mut RunControlSession,
) -> Result<RunControlCommandOutcome, String> {
    Ok(apply_route_go_with_summary(session)?.outcome)
}

pub(in crate::eval::run_control) fn apply_route_go_with_summary(
    session: &mut RunControlSession,
) -> Result<RouteGoApplied, String> {
    apply_route_go_with_summary_options(session, false)
}

pub(in crate::eval::run_control) fn apply_route_go_with_summary_allowing_reject_unless_forced(
    session: &mut RunControlSession,
) -> Result<RouteGoApplied, String> {
    apply_route_go_with_summary_options(session, true)
}

fn apply_route_go_with_summary_options(
    session: &mut RunControlSession,
    allow_reject_unless_forced: bool,
) -> Result<RouteGoApplied, String> {
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "route-go is only valid on Map. Use `rs` for read-only route evidence from this screen.\n{}",
            render_route_suggestion(session)
        ));
    }

    let trace = plan_route_for_session(session);
    let Some(selected_index) = trace.selected_index else {
        if allow_reject_unless_forced {
            return apply_visible_map_surface_fallback(session);
        }
        return Err("route planner found no legal map target".to_string());
    };
    let candidate = trace
        .candidates
        .get(selected_index)
        .cloned()
        .ok_or_else(|| "route planner selected an out-of-range map target".to_string())?;
    if candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative
        && !allow_reject_unless_forced
    {
        return Err(format!(
            "route planner selected only reject-unless-forced routes; inspect with `rs` and choose manually with `go <x>` or `fly <x> <y>`.\n{}",
            render_route_go_selection(&candidate)
        ));
    }

    let input = route_candidate_input(&candidate)?;
    let selection = render_route_go_selection(&candidate);
    let auto_step_summary = render_route_go_auto_step_summary(&candidate);
    let trace_annotation = route_go_trace_annotation(&trace, selected_index, &candidate)?;
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

fn apply_visible_map_surface_fallback(
    session: &mut RunControlSession,
) -> Result<RouteGoApplied, String> {
    let view = build_run_control_view_model(session);
    let Some(candidate) = view.candidates.iter().find(|candidate| {
        matches!(
            candidate.action,
            CandidateAction::Input(ClientInput::SelectMapNode(_))
                | CandidateAction::Input(ClientInput::FlyToNode(_, _))
        )
    }) else {
        return Err("route planner found no legal map target".to_string());
    };
    let Some(input) = candidate.action.executable_input() else {
        return Err("route planner fallback selected a non-executable map candidate".to_string());
    };
    let summary = match &input {
        ClientInput::SelectMapNode(x) => {
            format!(
                "route planner fallback: visible map path x={} {} label_role=behavior_policy_not_teacher",
                x, candidate.label
            )
        }
        ClientInput::FlyToNode(x, y) => {
            format!(
                "route planner fallback: visible Wing Boots path x={} y={} {} label_role=behavior_policy_not_teacher",
                x, y, candidate.label
            )
        }
        _ => "route planner fallback: visible map action label_role=behavior_policy_not_teacher"
            .to_string(),
    };
    let selection = format!(
        "Route planner fallback selected:\n  {} | command={}\n  label_role: behavior_policy_not_teacher",
        candidate.label,
        candidate.action.command_hint()
    );
    let outcome = session.apply_input(input)?;
    Ok(RouteGoApplied {
        auto_step_summary: summary,
        outcome: RunControlCommandOutcome {
            message: format!("{selection}\n{}", outcome.message),
            ..outcome
        },
    })
}

pub(in crate::eval::run_control) fn route_policy_stop_for_session(
    session: &RunControlSession,
    reason: &str,
) -> Result<
    Option<(
        crate::eval::run_control::RunControlTraceAnnotationV1,
        String,
    )>,
    String,
> {
    if !session.engine_state.is_map_surface() {
        return Ok(None);
    }
    let trace = plan_route_for_session(session);
    if trace.candidates.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        route_policy_stop_annotation(&trace, reason)?,
        format!("route planner policy stopped: {}", first_line(reason)),
    )))
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

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
