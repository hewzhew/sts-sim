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
pub struct SearchComponentProfile {
    pub elapsed_ms: u128,
    pub calls: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SearchProfileBreakdown {
    pub root: SearchPhaseProfile,
    pub recursive: SearchPhaseProfile,
    pub chooser_ms: u128,
    pub audit_ms: u128,
    pub planner: SearchComponentProfile,
    pub turn_close_projection: SearchComponentProfile,
    pub exact_turn: SearchComponentProfile,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub timeout_source: Option<String>,
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
    #[serde(skip)]
    advance_step_samples: Vec<u32>,
}

impl SearchProfileBreakdown {
    pub fn record_planner_call(&mut self, elapsed_ms: u128) {
        self.planner.calls = self.planner.calls.saturating_add(1);
        self.planner.elapsed_ms = self.planner.elapsed_ms.saturating_add(elapsed_ms);
    }

    pub fn record_projection_call(&mut self, elapsed_ms: u128) {
        self.turn_close_projection.calls = self.turn_close_projection.calls.saturating_add(1);
        self.turn_close_projection.elapsed_ms = self
            .turn_close_projection
            .elapsed_ms
            .saturating_add(elapsed_ms);
    }

    pub fn record_exact_turn_call(&mut self, elapsed_ms: u128) {
        self.exact_turn.calls = self.exact_turn.calls.saturating_add(1);
        self.exact_turn.elapsed_ms = self.exact_turn.elapsed_ms.saturating_add(elapsed_ms);
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits = self.cache_hits.saturating_add(1);
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses = self.cache_misses.saturating_add(1);
    }

    pub fn record_engine_step_advance(&mut self, elapsed_ms: u128, steps: u32) {
        self.advance_ms = self.advance_ms.saturating_add(elapsed_ms);
        self.advance_calls = self.advance_calls.saturating_add(1);
        self.advance_engine_steps = self.advance_engine_steps.saturating_add(steps as u64);
        self.advance_steps_max = self.advance_steps_max.max(steps);
        self.advance_step_samples.push(steps);
    }

    pub fn finalize_samples(&mut self) {
        if self.advance_step_samples.is_empty() {
            self.advance_steps_p50 = 0;
            self.advance_steps_p95 = 0;
            return;
        }
        let mut samples = self.advance_step_samples.clone();
        samples.sort_unstable();
        let len = samples.len();
        let p50 = len.saturating_sub(1) / 2;
        let p95 = ((len.saturating_sub(1)) * 95) / 100;
        self.advance_steps_p50 = samples[p50];
        self.advance_steps_p95 = samples[p95];
    }

    pub fn note_timeout_source(&mut self, source: &'static str) {
        if self.timeout_source.is_none() {
            self.timeout_source = Some(source.to_string());
        }
    }
}
