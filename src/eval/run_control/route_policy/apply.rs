use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteMoveKindV1, RouteSafetyFlagV1};
use crate::state::core::ClientInput;

use super::super::session::{
    RunControlDecisionParentSnapshotV1, RunControlSession, RunControlSessionCheckpointV1,
    RunProgressOutcome,
};
use super::format::{render_route_plan_auto_step_summary, render_route_plan_selection};
use super::planner::plan_route_for_session;
use super::suggestion::render_route_suggestion;
use super::trace::{route_plan_trace_annotation, route_policy_stop_annotation};

pub(in crate::eval::run_control) struct RoutePlanApplied {
    pub outcome: RunProgressOutcome,
    pub auto_step_summary: String,
}

pub(in crate::eval::run_control) fn apply_route_plan(
    session: &mut RunControlSession,
) -> Result<RunProgressOutcome, String> {
    Ok(apply_route_plan_with_summary(session)?.outcome)
}

pub(in crate::eval::run_control) fn apply_route_plan_with_summary(
    session: &mut RunControlSession,
) -> Result<RoutePlanApplied, String> {
    apply_route_plan_with_safety_mode(session, RoutePlanSafetyMode::RejectForcedRisk)
}

pub(in crate::eval::run_control) fn apply_route_plan_with_summary_allowing_forced_risk(
    session: &mut RunControlSession,
) -> Result<RoutePlanApplied, String> {
    apply_route_plan_with_safety_mode(session, RoutePlanSafetyMode::AllowForcedRisk)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoutePlanSafetyMode {
    RejectForcedRisk,
    AllowForcedRisk,
}

fn apply_route_plan_with_safety_mode(
    session: &mut RunControlSession,
    safety_mode: RoutePlanSafetyMode,
) -> Result<RoutePlanApplied, String> {
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "route planning is only valid on a map boundary.\n{}",
            render_route_suggestion(session)
        ));
    }

    let trace = plan_route_for_session(session);
    let Some(selected_index) = trace.selected_index else {
        return Err("route planner found no legal map target".to_string());
    };
    let candidate = trace
        .candidates
        .get(selected_index)
        .cloned()
        .ok_or_else(|| "route planner selected an out-of-range map target".to_string())?;
    if safety_mode == RoutePlanSafetyMode::RejectForcedRisk
        && candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative
    {
        return Err(format!(
            "route planner selected only reject-unless-forced routes; an explicit owner choice is required.\n{}",
            render_route_plan_selection(&candidate)
        ));
    }

    let input = route_candidate_input(&candidate)?;
    let selection = render_route_plan_selection(&candidate);
    let auto_step_summary = render_route_plan_auto_step_summary(&candidate);
    let trace_annotation = route_plan_trace_annotation(&trace, selected_index, &candidate)?;
    let parent_snapshot = RunControlDecisionParentSnapshotV1 {
        source: "route_planner".to_string(),
        command: candidate
            .suggested_command
            .clone()
            .unwrap_or_else(|| route_candidate_input_label(&candidate)),
        snapshot: RunControlSessionCheckpointV1::from_session(session),
    };
    let outcome = session.apply_input(input)?;
    Ok(RoutePlanApplied {
        auto_step_summary,
        outcome: RunProgressOutcome {
            message: format!("{selection}\n{}", outcome.message),
            ..outcome
        }
        .with_trace_annotations(vec![trace_annotation])
        .with_decision_parent_snapshots(vec![parent_snapshot]),
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

fn route_candidate_input_label(candidate: &RouteCandidateTraceV1) -> String {
    match candidate.target.move_kind {
        RouteMoveKindV1::NormalEdge => format!("go {}", candidate.target.x),
        RouteMoveKindV1::WingBootsJump => {
            format!("fly {} {}", candidate.target.x, candidate.target.y)
        }
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
