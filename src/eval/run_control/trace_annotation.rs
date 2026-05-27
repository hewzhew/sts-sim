use serde::{Deserialize, Serialize};

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
    },
    AutoCombatCapture {
        case_id: String,
        capture_path: String,
        benchmark_manifest_path: String,
        label_role: String,
    },
}
