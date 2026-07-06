use super::super::frontier::{
    remember_best_complete, remember_best_frontier, remember_win_candidate, SearchNode,
};
use super::super::*;
use super::win_acceptance::accepted_complete_win;

#[derive(Default)]
pub(super) struct SearchTrajectoryBook {
    pub(super) best_complete: Option<SearchNode>,
    pub(super) best_win: Option<SearchNode>,
    pub(super) win_candidates: Vec<SearchNode>,
    pub(super) best_frontier: Option<SearchNode>,
}

impl SearchTrajectoryBook {
    pub(super) fn remember_best_frontier(&mut self, node: &SearchNode) {
        remember_best_frontier(&mut self.best_frontier, node);
    }

    pub(super) fn remember_complete(&mut self, node: SearchNode) {
        remember_best_complete(&mut self.best_complete, node);
    }

    pub(super) fn remember_win(&mut self, node: SearchNode, config: &CombatSearchV2Config) -> bool {
        remember_win_candidate(&mut self.win_candidates, &node);
        remember_best_complete(&mut self.best_win, node.clone());
        remember_best_complete(&mut self.best_complete, node);
        self.best_win
            .as_ref()
            .is_some_and(|best| accepted_complete_win(best, config))
            && self.win_candidates.len() >= config.min_win_candidates_before_stop.max(1)
    }
}
