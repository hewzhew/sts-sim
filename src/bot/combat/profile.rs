use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchProfilingLevel {
    Off,
    Summary,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SearchPhaseProfile {
    pub legal_move_gen_ms: u128,
    pub legal_move_gen_calls: u32,
    pub transition_reduce_ms: u128,
    pub transition_reduce_inputs: u32,
    pub transition_reduce_outputs: u32,
    pub clone_calls: u32,
    pub leaf_eval_ms: u128,
    pub leaf_eval_calls: u32,
    pub avg_branch_before_reduce: f32,
    pub avg_branch_after_reduce: f32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SearchNodeCounters {
    pub nodes_expanded: u32,
    pub terminal_nodes: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SearchProfileBreakdown {
    pub root: SearchPhaseProfile,
    pub recursive: SearchPhaseProfile,
    pub advance_ms: u128,
    pub advance_calls: u32,
    pub advance_engine_steps: u64,
    pub advance_steps_p50: u32,
    pub advance_steps_p95: u32,
    pub advance_steps_max: u32,
    pub sequence_judge_ms: u128,
    pub root_diag_render_ms: u128,
    pub search_total_ms: u128,
    pub nodes: SearchNodeCounters,
}
