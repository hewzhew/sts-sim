use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchProfilePhase {
    Root,
    Recursive,
}

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

#[derive(Default)]
struct PhaseAccumulator {
    branch_before_total: u64,
    branch_after_total: u64,
    branch_samples: u32,
}

pub(crate) struct SearchProfileCollector {
    enabled: bool,
    breakdown: SearchProfileBreakdown,
    root_accumulator: PhaseAccumulator,
    recursive_accumulator: PhaseAccumulator,
    advance_steps: Vec<u32>,
}

impl SearchProfileCollector {
    pub(crate) fn new(level: SearchProfilingLevel) -> Self {
        Self {
            enabled: level != SearchProfilingLevel::Off,
            breakdown: SearchProfileBreakdown::default(),
            root_accumulator: PhaseAccumulator::default(),
            recursive_accumulator: PhaseAccumulator::default(),
            advance_steps: Vec::new(),
        }
    }

    pub(crate) fn record_legal_move_gen(
        &mut self,
        phase: SearchProfilePhase,
        elapsed_ms: u128,
        _legal_moves: usize,
    ) {
        if !self.enabled {
            return;
        }
        let target = self.phase_mut(phase);
        target.legal_move_gen_ms += elapsed_ms;
        target.legal_move_gen_calls += 1;
    }

    pub(crate) fn record_transition_reduce(
        &mut self,
        phase: SearchProfilePhase,
        elapsed_ms: u128,
        input_count: usize,
        output_count: usize,
    ) {
        if !self.enabled {
            return;
        }
        let target = self.phase_mut(phase);
        target.transition_reduce_ms += elapsed_ms;
        target.transition_reduce_inputs += input_count as u32;
        target.transition_reduce_outputs += output_count as u32;
        let accumulator = self.branch_accumulator_mut(phase);
        accumulator.branch_before_total += input_count as u64;
        accumulator.branch_after_total += output_count as u64;
        accumulator.branch_samples += 1;
    }

    pub(crate) fn record_clone_calls(&mut self, phase: SearchProfilePhase, clone_calls: u32) {
        if !self.enabled {
            return;
        }
        self.phase_mut(phase).clone_calls += clone_calls;
    }

    pub(crate) fn record_leaf_eval(&mut self, phase: SearchProfilePhase, elapsed_ms: u128) {
        if !self.enabled {
            return;
        }
        let target = self.phase_mut(phase);
        target.leaf_eval_ms += elapsed_ms;
        target.leaf_eval_calls += 1;
    }

    pub(crate) fn record_advance(&mut self, elapsed_ms: u128, engine_steps: usize) {
        if !self.enabled {
            return;
        }
        self.breakdown.advance_ms += elapsed_ms;
        self.breakdown.advance_calls += 1;
        self.breakdown.advance_engine_steps += engine_steps as u64;
        self.advance_steps.push(engine_steps as u32);
    }

    pub(crate) fn record_sequence_judge_ms(&mut self, elapsed_ms: u128) {
        if !self.enabled {
            return;
        }
        self.breakdown.sequence_judge_ms += elapsed_ms;
    }

    pub(crate) fn record_search_total_ms(&mut self, elapsed_ms: u128) {
        if !self.enabled {
            return;
        }
        self.breakdown.search_total_ms = elapsed_ms;
    }

    pub(crate) fn record_node_expanded(&mut self) {
        if !self.enabled {
            return;
        }
        self.breakdown.nodes.nodes_expanded += 1;
    }

    pub(crate) fn record_terminal_node(&mut self) {
        if !self.enabled {
            return;
        }
        self.breakdown.nodes.terminal_nodes += 1;
    }

    pub(crate) fn finish(mut self) -> SearchProfileBreakdown {
        if !self.enabled {
            return self.breakdown;
        }

        self.breakdown.root.avg_branch_before_reduce = average_branch(
            self.root_accumulator.branch_before_total,
            self.root_accumulator.branch_samples,
        );
        self.breakdown.root.avg_branch_after_reduce = average_branch(
            self.root_accumulator.branch_after_total,
            self.root_accumulator.branch_samples,
        );
        self.breakdown.recursive.avg_branch_before_reduce = average_branch(
            self.recursive_accumulator.branch_before_total,
            self.recursive_accumulator.branch_samples,
        );
        self.breakdown.recursive.avg_branch_after_reduce = average_branch(
            self.recursive_accumulator.branch_after_total,
            self.recursive_accumulator.branch_samples,
        );

        self.advance_steps.sort_unstable();
        self.breakdown.advance_steps_p50 = percentile(&self.advance_steps, 0.50);
        self.breakdown.advance_steps_p95 = percentile(&self.advance_steps, 0.95);
        self.breakdown.advance_steps_max = self.advance_steps.last().copied().unwrap_or_default();

        self.breakdown
    }

    fn phase_mut(&mut self, phase: SearchProfilePhase) -> &mut SearchPhaseProfile {
        match phase {
            SearchProfilePhase::Root => &mut self.breakdown.root,
            SearchProfilePhase::Recursive => &mut self.breakdown.recursive,
        }
    }

    fn branch_accumulator_mut(&mut self, phase: SearchProfilePhase) -> &mut PhaseAccumulator {
        match phase {
            SearchProfilePhase::Root => &mut self.root_accumulator,
            SearchProfilePhase::Recursive => &mut self.recursive_accumulator,
        }
    }
}

fn average_branch(total: u64, samples: u32) -> f32 {
    if samples == 0 {
        0.0
    } else {
        total as f32 / samples as f32
    }
}

fn percentile(values: &[u32], q: f32) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let idx = ((values.len() - 1) as f32 * q).round() as usize;
    values[idx.min(values.len() - 1)]
}
