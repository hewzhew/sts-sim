use std::collections::HashMap;
use std::time::Instant;

use super::super::frontier::{FrontierQueue, QueueEntry, ResourceVector, SearchNode};
use super::super::*;
use super::best_trajectories::SearchTrajectoryBook;
use super::turn_plan_seeding::TurnPlanSeedTracker;

pub(super) struct SearchLoopState {
    pub(super) stats: CombatSearchV2Stats,
    pub(super) diagnostics: SearchDiagnosticsCollector,
    pub(super) exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>>,
    pub(super) dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>>,
    pub(super) rollout_cache: RolloutCache,
    pub(super) performance: CombatSearchV2PerformanceReport,
    pub(super) frontier: FrontierQueue,
    pub(super) turn_plan_seed_tracker: TurnPlanSeedTracker,
    pub(super) trajectories: SearchTrajectoryBook,
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
            trajectories: SearchTrajectoryBook::default(),
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
        self.trajectories.remember_best_frontier(node);
    }

    pub(super) fn remember_complete(&mut self, node: SearchNode) {
        self.trajectories.remember_complete(node);
    }

    pub(super) fn remember_win(&mut self, node: SearchNode, config: &CombatSearchV2Config) -> bool {
        self.stats.terminal_wins = self.stats.terminal_wins.saturating_add(1);
        if self.stats.nodes_to_first_win.is_none() {
            self.stats.nodes_to_first_win = Some(self.stats.nodes_generated);
        }
        self.trajectories.remember_win(node, config)
    }

    pub(super) fn remember_loss(&mut self, node: SearchNode) {
        self.stats.terminal_losses = self.stats.terminal_losses.saturating_add(1);
        self.remember_complete(node);
    }
}
