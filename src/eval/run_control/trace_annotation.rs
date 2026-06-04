use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1;
use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::state::core::ClientInput;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAutomationActionV1 {
    pub step_index: usize,
    pub action_key: String,
    pub input: ClientInput,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePlannerCandidateSummaryV1 {
    pub rank: usize,
    pub target_x: i32,
    pub target_y: i32,
    pub room_type: String,
    pub move_kind: String,
    pub safety: String,
    pub score: f32,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunControlTraceAnnotationV1 {
    RoutePlannerSelection {
        summary: String,
        selected_index: Option<usize>,
        candidate_count: usize,
        target_x: i32,
        target_y: i32,
        room_type: String,
        move_kind: String,
        safety: String,
        score: f32,
        command: String,
        top_candidates: Vec<RoutePlannerCandidateSummaryV1>,
        label_role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noncombat_record: Option<NonCombatDecisionRecordV1>,
    },
    NonCombatPolicyDecision {
        record: NonCombatDecisionRecordV1,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        card_reward_packet: Option<PublicRewardDecisionPacketV1>,
    },
    NonCombatHumanBoundary {
        record: NonCombatDecisionRecordV1,
    },
    AutoCombatCapture {
        case_id: String,
        capture_path: String,
        benchmark_manifest_path: String,
        label_role: String,
    },
    CombatAutomationTrajectory {
        source: String,
        action_count: usize,
        actions: Vec<CombatAutomationActionV1>,
        label_role: String,
    },
}

pub(in crate::eval::run_control) fn validate_run_control_trace_annotations_v1(
    annotations: &[RunControlTraceAnnotationV1],
) -> Result<(), String> {
    for (idx, annotation) in annotations.iter().enumerate() {
        validate_run_control_trace_annotation_v1(idx, annotation)?;
    }
    Ok(())
}

fn validate_run_control_trace_annotation_v1(
    idx: usize,
    annotation: &RunControlTraceAnnotationV1,
) -> Result<(), String> {
    match annotation {
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: Some(record),
            ..
        } => validate_noncombat_record_annotation(idx, "route_planner_selection", record),
        RunControlTraceAnnotationV1::NonCombatPolicyDecision { record, .. } => {
            validate_noncombat_record_annotation(idx, "noncombat_policy_decision", record)
        }
        RunControlTraceAnnotationV1::NonCombatHumanBoundary { record } => {
            validate_noncombat_record_annotation(idx, "noncombat_human_boundary", record)
        }
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: None,
            ..
        }
        | RunControlTraceAnnotationV1::AutoCombatCapture { .. }
        | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. } => Ok(()),
    }
}

fn validate_noncombat_record_annotation(
    idx: usize,
    kind: &str,
    record: &NonCombatDecisionRecordV1,
) -> Result<(), String> {
    validate_noncombat_decision_record_v1(record).map_err(|errors| {
        format!(
            "annotation[{idx}] {kind} contains invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })
}
