use serde::{Deserialize, Serialize};

use crate::ai::noncombat_decision_v1::NonCombatDecisionRecordV1;
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
