use crate::ai::route_planner_v1::{
    MapDecisionPacketV1, RouteCandidateTraceV1, RouteDecisionTraceV1,
};

use super::super::noncombat_policy_annotation::validate_noncombat_policy_record;
use super::super::trace_annotation::{
    RoutePlannerCandidateSummaryV1, RoutePlannerFirstEliteEvidenceV1,
    RoutePlannerSelectionEvidenceV1, RunControlTraceAnnotationV1,
};
use super::super::view_model::room_type_label;
use super::format::{render_route_plan_auto_step_summary, safety_label};

pub(super) fn route_plan_trace_annotation(
    trace: &RouteDecisionTraceV1,
    selected_index: usize,
    candidate: &RouteCandidateTraceV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    let noncombat_record = trace.to_noncombat_decision_record_v1();
    validate_noncombat_policy_record("route planner", &noncombat_record)?;

    Ok(RunControlTraceAnnotationV1::RoutePlannerSelection {
        summary: render_route_plan_auto_step_summary(candidate),
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
        top_candidates: route_plan_top_candidate_summaries(trace),
        candidate_pool: Vec::new(),
        label_role: "behavior_policy_not_teacher".to_string(),
        map_decision_packet: Some(MapDecisionPacketV1::from_route_decision_trace_v1(trace)),
        route_evidence: Some(route_plan_selection_evidence(candidate)),
        noncombat_record: Some(noncombat_record),
    })
}

pub(super) fn route_policy_stop_annotation(
    trace: &RouteDecisionTraceV1,
    reason: &str,
) -> Result<RunControlTraceAnnotationV1, String> {
    let mut stopped_trace = trace.clone();
    stopped_trace.selected_index = None;
    stopped_trace.warnings.push(reason.to_string());
    let mut noncombat_record = stopped_trace.to_noncombat_decision_record_v1();
    noncombat_record.selection.reason = reason.to_string();
    noncombat_record.selection.selection_mode = "route_policy_stop".to_string();
    validate_noncombat_policy_record("route planner stop", &noncombat_record)?;

    Ok(RunControlTraceAnnotationV1::RoutePlannerCandidatePool {
        summary: format!("route planner stopped: {}", first_line(reason)),
        selected_index: None,
        candidate_count: stopped_trace.candidates.len(),
        top_candidates: route_plan_top_candidate_summaries(&stopped_trace),
        candidate_pool: Vec::new(),
        label_role: "behavior_policy_not_teacher".to_string(),
        map_decision_packet: Some(MapDecisionPacketV1::from_route_decision_trace_v1(
            &stopped_trace,
        )),
        stop_reason: reason.to_string(),
        noncombat_record: Some(noncombat_record),
    })
}

fn route_plan_top_candidate_summaries(
    trace: &RouteDecisionTraceV1,
) -> Vec<RoutePlannerCandidateSummaryV1> {
    route_plan_candidate_summaries(trace, Some(3))
}

fn route_plan_candidate_summaries(
    trace: &RouteDecisionTraceV1,
    limit: Option<usize>,
) -> Vec<RoutePlannerCandidateSummaryV1> {
    trace
        .candidates
        .iter()
        .take(limit.unwrap_or(usize::MAX))
        .enumerate()
        .map(route_plan_candidate_summary)
        .collect()
}

fn route_plan_candidate_summary(
    (rank, candidate): (usize, &RouteCandidateTraceV1),
) -> RoutePlannerCandidateSummaryV1 {
    let evidence = route_plan_selection_evidence(candidate);
    RoutePlannerCandidateSummaryV1 {
        rank,
        target_x: candidate.target.x,
        target_y: candidate.target.y,
        room_type: room_type_label(candidate.target.room_type).to_string(),
        move_kind: format!("{:?}", candidate.target.move_kind),
        safety: safety_label(candidate.safety).to_string(),
        score: candidate.total_score,
        elite_prep_bp: evidence.elite_prep_bp,
        first_elite: evidence.first_elite,
        reasons: candidate.reasons.clone(),
        cautions: candidate.cautions.clone(),
        command: candidate
            .suggested_command
            .as_deref()
            .unwrap_or("unknown-command")
            .to_string(),
    }
}

fn route_plan_selection_evidence(
    candidate: &RouteCandidateTraceV1,
) -> RoutePlannerSelectionEvidenceV1 {
    let first_elite = &candidate.path_summary.first_elite;
    RoutePlannerSelectionEvidenceV1 {
        elite_prep_bp: score_to_basis_points(candidate.score_terms.elite_prep),
        first_elite: RoutePlannerFirstEliteEvidenceV1 {
            paths_with_first_elite: first_elite.paths_with_first_elite,
            forced: first_elite.forced,
            optional: first_elite.optional,
            min_hallway_fights_before: first_elite.min_hallway_fights_before,
            max_hallway_fights_before: first_elite.max_hallway_fights_before,
            min_unknowns_before: first_elite.min_unknowns_before,
            max_unknowns_before: first_elite.max_unknowns_before,
            min_fires_before: first_elite.min_fires_before,
            max_fires_before: first_elite.max_fires_before,
            min_shops_before: first_elite.min_shops_before,
            max_shops_before: first_elite.max_shops_before,
            can_bail_to_rest_before: first_elite.can_bail_to_rest_before,
            can_bail_to_shop_before: first_elite.can_bail_to_shop_before,
        },
    }
}

fn score_to_basis_points(score: f32) -> i32 {
    (score * 100.0).round() as i32
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
