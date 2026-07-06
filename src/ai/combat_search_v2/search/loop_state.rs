use std::collections::HashMap;
use std::time::Instant;

use super::super::frontier::{
    remember_best_complete, remember_best_frontier, remember_win_candidate, FrontierQueue,
    QueueEntry, ResourceVector, SearchNode,
};
use super::super::*;
use super::turn_plan_seeding::TurnPlanSeedTracker;
use super::win_acceptance::accepted_complete_win;

pub(super) struct SearchLoopState {
    pub(super) stats: CombatSearchV2Stats,
    pub(super) diagnostics: SearchDiagnosticsCollector,
    pub(super) exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>>,
    pub(super) dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>>,
    pub(super) rollout_cache: RolloutCache,
    pub(super) performance: CombatSearchV2PerformanceReport,
    pub(super) frontier: FrontierQueue,
    pub(super) turn_plan_seed_tracker: TurnPlanSeedTracker,
    pub(super) best_complete: Option<SearchNode>,
    pub(super) best_win: Option<SearchNode>,
    pub(super) win_candidates: Vec<SearchNode>,
    pub(super) best_frontier: Option<SearchNode>,
    pub(super) unresolved_leaf_count: u64,
    pub(super) max_actions_cut_count: u64,
    pub(super) engine_step_limit_count: u64,
    pub(super) potion_budget_cut_count: u64,
    pub(super) exhausted: bool,
    pub(super) accepted_complete_candidate: bool,
}

impl SearchLoopState {
    pub(super) fn new(config: &CombatSearchV2Config) -> Self {
        Self {
            stats: CombatSearchV2Stats::default(),
            diagnostics: SearchDiagnosticsCollector::default(),
            exact_transpositions: HashMap::new(),
            dominance: HashMap::new(),
            rollout_cache: RolloutCache::new(
                config.rollout_policy,
                config.rollout_max_evaluations,
                config.rollout_max_actions,
                config.rollout_beam_width,
            ),
            performance: CombatSearchV2PerformanceReport::default(),
            frontier: FrontierQueue::new(config.frontier_policy),
            turn_plan_seed_tracker: TurnPlanSeedTracker::default(),
            best_complete: None,
            best_win: None,
            win_candidates: Vec::new(),
            best_frontier: None,
            unresolved_leaf_count: 0,
            max_actions_cut_count: 0,
            engine_step_limit_count: 0,
            potion_budget_cut_count: 0,
            exhausted: false,
            accepted_complete_candidate: false,
        }
    }

    pub(super) fn push_frontier(&mut self, node: SearchNode) {
        self.frontier.push_node(node);
    }

    pub(super) fn pop_frontier(&mut self) -> Option<QueueEntry> {
        let started = Instant::now();
        let entry = self.frontier.pop();
        self.performance.frontier_pop_calls = self.performance.frontier_pop_calls.saturating_add(1);
        self.performance.frontier_pop_elapsed_us = self
            .performance
            .frontier_pop_elapsed_us
            .saturating_add(started.elapsed().as_micros());
        entry
    }

    pub(super) fn remember_best_frontier(&mut self, node: &SearchNode) {
        remember_best_frontier(&mut self.best_frontier, node);
    }

    pub(super) fn remember_complete(&mut self, node: SearchNode) {
        remember_best_complete(&mut self.best_complete, node);
    }

    pub(super) fn remember_win(&mut self, node: SearchNode, config: &CombatSearchV2Config) -> bool {
        self.stats.terminal_wins = self.stats.terminal_wins.saturating_add(1);
        if self.stats.nodes_to_first_win.is_none() {
            self.stats.nodes_to_first_win = Some(self.stats.nodes_generated);
        }
        remember_win_candidate(&mut self.win_candidates, &node);
        remember_best_complete(&mut self.best_win, node.clone());
        remember_best_complete(&mut self.best_complete, node);
        self.best_win
            .as_ref()
            .is_some_and(|best| accepted_complete_win(best, config))
            && self.win_candidates.len() >= config.min_win_candidates_before_stop.max(1)
    }

    pub(super) fn remember_loss(&mut self, node: SearchNode) {
        self.stats.terminal_losses = self.stats.terminal_losses.saturating_add(1);
        self.remember_complete(node);
    }

    pub(super) fn finish_diagnostics_and_timing(
        &mut self,
        started: Instant,
        root_for_turn_plan_diagnostics: &SearchNode,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
    ) {
        let shadow_audit_started = Instant::now();
        self.diagnostics
            .run_discard_order_exact_shadow_audit(stepper, config);
        self.performance.shadow_audit_elapsed_us = shadow_audit_started.elapsed().as_micros();

        let root_turn_plan_diagnostics_started = Instant::now();
        self.diagnostics
            .observe_root_turn_plan(root_for_turn_plan_diagnostics, stepper);
        self.performance.root_turn_plan_diagnostics_elapsed_us =
            root_turn_plan_diagnostics_started.elapsed().as_micros();

        let total_elapsed = started.elapsed();
        self.stats.elapsed_ms = total_elapsed.as_millis();
        self.performance.total_elapsed_us = total_elapsed.as_micros();
        self.performance.unattributed_elapsed_us =
            self.performance.total_elapsed_us.saturating_sub(
                self.performance
                    .engine_step_elapsed_us
                    .saturating_add(self.performance.rollout_estimate_elapsed_us)
                    .saturating_add(self.performance.frontier_pop_elapsed_us)
                    .saturating_add(self.performance.pre_expand_elapsed_us)
                    .saturating_add(self.performance.expansion_elapsed_us)
                    .saturating_add(self.performance.child_bookkeeping_elapsed_us)
                    .saturating_add(self.performance.turn_plan_frontier_seed_elapsed_us)
                    .saturating_add(self.performance.shadow_audit_elapsed_us)
                    .saturating_add(self.performance.root_turn_plan_diagnostics_elapsed_us),
            );
    }
}
