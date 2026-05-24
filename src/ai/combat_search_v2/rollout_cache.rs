use std::collections::HashMap;
use std::time::Instant;

use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct RolloutCache {
    pub(super) policy: CombatSearchV2RolloutPolicy,
    pub(super) max_evaluations: usize,
    pub(super) max_actions: usize,
    pub(super) evaluations: u64,
    pub(super) cache_hits: u64,
    pub(super) budget_skips: u64,
    pub(super) truncated: u64,
    pub(super) terminal_wins: u64,
    pub(super) terminal_losses: u64,
    pub(super) rollouts_with_pending_choice: u64,
    pub(super) rollouts_stopped_on_high_fanout_pending_choice: u64,
    pub(super) pending_choice_actions_simulated: u64,
    pub(super) max_pending_choice_estimated_action_fanout: usize,
    pub(super) cache: HashMap<CombatExactStateKey, RolloutNodeEstimate>,
}

impl RolloutCache {
    pub(super) fn new(
        policy: CombatSearchV2RolloutPolicy,
        max_evaluations: usize,
        max_actions: usize,
    ) -> Self {
        Self {
            policy,
            max_evaluations,
            max_actions,
            ..Self::default()
        }
    }

    pub(super) fn estimate(
        &mut self,
        node: &SearchNode,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
        deadline: Option<Instant>,
    ) -> RolloutNodeEstimate {
        if self.policy == CombatSearchV2RolloutPolicy::Disabled {
            return RolloutNodeEstimate::unevaluated();
        }

        let key = combat_exact_state_key(&node.engine, &node.combat);
        if let Some(cached) = self.cache.get(&key).copied() {
            self.cache_hits = self.cache_hits.saturating_add(1);
            return cached;
        }
        if self.evaluations as usize >= self.max_evaluations {
            self.budget_skips = self.budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            self.budget_skips = self.budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }

        self.evaluations = self.evaluations.saturating_add(1);
        let estimate = match self.policy {
            CombatSearchV2RolloutPolicy::Disabled => RolloutNodeEstimate::unevaluated(),
            CombatSearchV2RolloutPolicy::ConservativeNoPotion => {
                rollout::conservative_no_potion_rollout(
                    node,
                    stepper,
                    config,
                    self.max_actions,
                    deadline,
                )
            }
        };
        if estimate.truncated {
            self.truncated = self.truncated.saturating_add(1);
        }
        if estimate.pending_choices_seen > 0 {
            self.rollouts_with_pending_choice = self.rollouts_with_pending_choice.saturating_add(1);
        }
        if estimate.stopped_on_high_fanout_pending_choice {
            self.rollouts_stopped_on_high_fanout_pending_choice = self
                .rollouts_stopped_on_high_fanout_pending_choice
                .saturating_add(1);
        }
        self.pending_choice_actions_simulated = self
            .pending_choice_actions_simulated
            .saturating_add(estimate.pending_choice_actions_simulated as u64);
        self.max_pending_choice_estimated_action_fanout = self
            .max_pending_choice_estimated_action_fanout
            .max(estimate.max_pending_choice_estimated_action_fanout);
        match estimate.terminal {
            SearchTerminalLabel::Win => self.terminal_wins = self.terminal_wins.saturating_add(1),
            SearchTerminalLabel::Loss => {
                self.terminal_losses = self.terminal_losses.saturating_add(1)
            }
            SearchTerminalLabel::Unresolved => {}
        }
        self.cache.insert(key, estimate);
        estimate
    }

    pub(super) fn finish(&self, best_frontier: Option<&SearchNode>) -> CombatSearchV2RolloutReport {
        CombatSearchV2RolloutReport {
            policy: self.policy.label(),
            behavioral_effect:
                "estimated_frontier_priority_only_no_terminal_proof_no_baseline_claim",
            max_evaluations: self.max_evaluations,
            max_actions_per_rollout: self.max_actions,
            evaluations: self.evaluations,
            cache_hits: self.cache_hits,
            budget_skips: self.budget_skips,
            truncated_rollouts: self.truncated,
            terminal_wins: self.terminal_wins,
            terminal_losses: self.terminal_losses,
            rollouts_with_pending_choice: self.rollouts_with_pending_choice,
            rollouts_stopped_on_high_fanout_pending_choice: self
                .rollouts_stopped_on_high_fanout_pending_choice,
            pending_choice_actions_simulated: self.pending_choice_actions_simulated,
            max_pending_choice_estimated_action_fanout: self
                .max_pending_choice_estimated_action_fanout,
            best_frontier_estimate: best_frontier
                .and_then(|node| node.rollout_estimate.to_report()),
            notes: vec![
                "rollout estimates are not terminal proof",
                "conservative_no_potion uses only legal simulator actions and disables potion actions",
                "rollout cache is keyed by exact combat runtime state",
                "unresolved rollout priority uses phase-adjusted enemy effort from phase_profile",
                "high-fanout pending choices stop rollout estimates instead of selecting an arbitrary branch",
                "small pending choices may be followed by rollout, but their actions are still exact simulator inputs and never proof claims",
            ],
        }
    }
}
