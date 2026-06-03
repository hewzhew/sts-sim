use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteDecisionTraceV1};

use super::super::trace_annotation::{RoutePlannerCandidateSummaryV1, RunControlTraceAnnotationV1};
use super::super::view_model::room_type_label;
use super::format::{render_route_go_auto_step_summary, safety_label};

pub(super) fn route_go_trace_annotation(
    trace: &RouteDecisionTraceV1,
    selected_index: usize,
    candidate: &RouteCandidateTraceV1,
) -> RunControlTraceAnnotationV1 {
    RunControlTraceAnnotationV1::RoutePlannerSelection {
        summary: render_route_go_auto_step_summary(candidate),
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
        noncombat_record: Some(trace.to_noncombat_decision_record_v1()),
    }
}

fn route_go_top_candidate_summaries(
    trace: &RouteDecisionTraceV1,
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
