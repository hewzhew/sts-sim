use super::super::super::frontier::SearchNode;
use super::super::super::CombatSearchV2Config;
use super::SearchLoopState;

impl SearchLoopState {
    pub(in crate::ai::combat_search_v2::search) fn remember_best_frontier(
        &mut self,
        node: &SearchNode,
    ) {
        self.trajectories.remember_best_frontier(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn remember_complete(&mut self, node: SearchNode) {
        self.root_evidence.observe_exact_complete(&node);
        self.trajectories.remember_complete(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn remember_win(
        &mut self,
        node: SearchNode,
        config: &CombatSearchV2Config,
    ) -> bool {
        let nodes_generated_at_discovery = self.stats.nodes_generated;
        self.remember_win_observed_at(node, config, nodes_generated_at_discovery, true)
    }

    pub(in crate::ai::combat_search_v2::search) fn remember_promoted_win_observed_at(
        &mut self,
        node: SearchNode,
        config: &CombatSearchV2Config,
        nodes_generated_at_discovery: u64,
    ) -> bool {
        let first_retained_path_observation = !self.trajectories.contains_exact_win_path(&node);
        self.remember_win_observed_at(
            node,
            config,
            nodes_generated_at_discovery,
            first_retained_path_observation,
        )
    }

    fn remember_win_observed_at(
        &mut self,
        node: SearchNode,
        config: &CombatSearchV2Config,
        nodes_generated_at_discovery: u64,
        count_observation: bool,
    ) -> bool {
        if count_observation {
            self.stats.terminal_wins = self.stats.terminal_wins.saturating_add(1);
            if self.stats.nodes_to_first_win.is_none() {
                self.stats.nodes_to_first_win = Some(nodes_generated_at_discovery);
            }
        }
        self.root_evidence.observe_exact_win(&node);
        self.trajectories
            .remember_win(node, config, self.initial_external_burden_count)
    }

    pub(in crate::ai::combat_search_v2::search) fn remember_loss(&mut self, node: SearchNode) {
        self.stats.terminal_losses = self.stats.terminal_losses.saturating_add(1);
        self.remember_complete(node);
    }
}
