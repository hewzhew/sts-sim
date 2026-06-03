use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteMoveKindV1, RouteSafetyFlagV1};
use crate::state::core::ClientInput;

use super::super::session::{RunControlCommandOutcome, RunControlSession};
use super::format::{render_route_go_auto_step_summary, render_route_go_selection};
use super::planner::plan_route_for_session;
use super::suggestion::render_route_suggestion;
use super::trace::route_go_trace_annotation;

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
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "route-go is only valid on Map. Use `rs` for read-only route evidence from this screen.\n{}",
            render_route_suggestion(session)
        ));
    }

    let trace = plan_route_for_session(session);
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
