use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteMoveKindV1, RouteSafetyFlagV1};

use super::super::view_model::room_type_label;

pub(super) fn render_route_plan_selection(candidate: &RouteCandidateTraceV1) -> String {
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

pub(super) fn render_route_plan_auto_step_summary(candidate: &RouteCandidateTraceV1) -> String {
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

pub(super) fn safety_label(safety: RouteSafetyFlagV1) -> &'static str {
    match safety {
        RouteSafetyFlagV1::Ok => "ok",
        RouteSafetyFlagV1::RiskyButAllowed => "risky",
        RouteSafetyFlagV1::RejectUnlessNoAlternative => "reject_unless_forced",
    }
}
