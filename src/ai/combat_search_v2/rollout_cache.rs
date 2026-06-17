use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use super::value::combat_eval_from_rollout_estimate;
use super::*;
use crate::ai::combat_search_v2::rollout_profile::RolloutPerformanceCounters;
use crate::ai::combat_search_v2::rollout_scheduler::turn_beam_extension_budget;

#[derive(Clone, Debug, Default)]
pub(super) struct RolloutCache {
    pub(super) policy: CombatSearchV2RolloutPolicy,
    pub(super) max_evaluations: usize,
    pub(super) max_actions: usize,
    pub(super) beam_width: usize,
    pub(super) turn_beam_extension_budget: usize,
    pub(super) turn_beam_calls: u64,
    pub(super) turn_beam_conservative_anchor_present: u64,
    pub(super) turn_beam_conservative_anchor_selected: u64,
    pub(super) turn_beam_conservative_anchor_terminal_wins: u64,
    pub(super) turn_beam_extensions: u64,
    pub(super) turn_beam_extension_budget_skips: u64,
    pub(super) turn_beam_turn_plan_calls: u64,
    pub(super) turn_beam_turn_plan_inner_nodes_expanded: u64,
    pub(super) turn_beam_turn_plan_inner_nodes_generated: u64,
    pub(super) turn_beam_turn_plans_kept: u64,
    pub(super) turn_beam_turn_plans_kept_by_bucket: BTreeMap<&'static str, u64>,
    pub(super) turn_beam_terminal_candidates_kept: u64,
    pub(super) turn_beam_best_pv_len: usize,
    pub(super) turn_beam_best_pv_terminal: Option<SearchTerminalLabel>,
    pub(super) evaluations: u64,
    pub(super) cache_queries: u64,
    pub(super) cache_hits: u64,
    pub(super) cache_misses: u64,
    pub(super) cache_inserts: u64,
    pub(super) budget_skips: u64,
    pub(super) max_evaluation_budget_skips: u64,
    pub(super) deadline_budget_skips: u64,
    pub(super) truncated: u64,
    pub(super) terminal_wins: u64,
    pub(super) terminal_losses: u64,
    pub(super) rollouts_with_pending_choice: u64,
    pub(super) rollouts_stopped_on_high_fanout_pending_choice: u64,
    pub(super) pending_choice_actions_simulated: u64,
    pub(super) max_pending_choice_estimated_action_fanout: usize,
    pub(super) performance: RolloutPerformanceCounters,
    pub(super) cache: HashMap<CombatExactStateKey, RolloutNodeEstimate>,
}

impl RolloutCache {
    pub(super) fn new(
        policy: CombatSearchV2RolloutPolicy,
        max_evaluations: usize,
        max_actions: usize,
        beam_width: usize,
    ) -> Self {
        Self {
            policy,
            max_evaluations,
            max_actions,
            beam_width,
            turn_beam_extension_budget: turn_beam_extension_budget(max_evaluations, beam_width),
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

        let cache_lookup_started = Instant::now();
        let key = combat_exact_state_key(&node.engine, &node.combat);
        self.cache_queries = self.cache_queries.saturating_add(1);
        if let Some(cached) = self.cache.get(&key).copied() {
            self.performance.cache_lookup_elapsed_us = self
                .performance
                .cache_lookup_elapsed_us
                .saturating_add(cache_lookup_started.elapsed().as_micros());
            self.cache_hits = self.cache_hits.saturating_add(1);
            return cached;
        }
        self.performance.cache_lookup_elapsed_us = self
            .performance
            .cache_lookup_elapsed_us
            .saturating_add(cache_lookup_started.elapsed().as_micros());
        self.cache_misses = self.cache_misses.saturating_add(1);
        if self.evaluations as usize >= self.max_evaluations {
            self.budget_skips = self.budget_skips.saturating_add(1);
            self.max_evaluation_budget_skips = self.max_evaluation_budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            self.budget_skips = self.budget_skips.saturating_add(1);
            self.deadline_budget_skips = self.deadline_budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }

        self.evaluations = self.evaluations.saturating_add(1);
        let policy_dispatch_started = Instant::now();
        let estimate = match self.policy {
            CombatSearchV2RolloutPolicy::Disabled => RolloutNodeEstimate::unevaluated(),
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion => {
                match adaptive_no_potion_rollout_policy(node) {
                    CombatSearchV2RolloutPolicy::PhaseAwareNoPotion => {
                        rollout::phase_aware_no_potion_rollout(
                            node,
                            stepper,
                            config,
                            self.max_actions,
                            deadline,
                            &mut self.performance,
                        )
                    }
                    _ => rollout::conservative_no_potion_rollout(
                        node,
                        stepper,
                        config,
                        self.max_actions,
                        deadline,
                        &mut self.performance,
                    ),
                }
            }
            CombatSearchV2RolloutPolicy::ConservativeNoPotion => {
                rollout::conservative_no_potion_rollout(
                    node,
                    stepper,
                    config,
                    self.max_actions,
                    deadline,
                    &mut self.performance,
                )
            }
            CombatSearchV2RolloutPolicy::PhaseAwareNoPotion => {
                rollout::phase_aware_no_potion_rollout(
                    node,
                    stepper,
                    config,
                    self.max_actions,
                    deadline,
                    &mut self.performance,
                )
            }
            CombatSearchV2RolloutPolicy::TurnBeamNoPotion => {
                self.turn_beam_calls = self.turn_beam_calls.saturating_add(1);
                let anchor = rollout::turn_beam_conservative_anchor_rollout(
                    node,
                    stepper,
                    config,
                    self.max_actions,
                    deadline,
                    &mut self.performance,
                );
                self.turn_beam_conservative_anchor_present =
                    self.turn_beam_conservative_anchor_present.saturating_add(1);
                if anchor.terminal == SearchTerminalLabel::Win {
                    self.turn_beam_conservative_anchor_terminal_wins = self
                        .turn_beam_conservative_anchor_terminal_wins
                        .saturating_add(1);
                    self.turn_beam_conservative_anchor_selected = self
                        .turn_beam_conservative_anchor_selected
                        .saturating_add(1);
                    self.observe_turn_beam_best_pv(anchor);
                    anchor
                } else if self.turn_beam_extensions as usize >= self.turn_beam_extension_budget {
                    self.turn_beam_extension_budget_skips =
                        self.turn_beam_extension_budget_skips.saturating_add(1);
                    self.turn_beam_conservative_anchor_selected = self
                        .turn_beam_conservative_anchor_selected
                        .saturating_add(1);
                    self.observe_turn_beam_best_pv(anchor);
                    anchor
                } else {
                    self.turn_beam_extensions = self.turn_beam_extensions.saturating_add(1);
                    let (beam, attribution) = rollout::turn_beam_extension_rollout_with_attribution(
                        node,
                        stepper,
                        config,
                        self.max_actions,
                        deadline,
                        &mut self.performance,
                    );
                    self.observe_turn_beam_extension_attribution(attribution);
                    let selected = better_rollout_estimate(beam, anchor);
                    if selected == anchor {
                        self.turn_beam_conservative_anchor_selected = self
                            .turn_beam_conservative_anchor_selected
                            .saturating_add(1);
                    }
                    self.observe_turn_beam_best_pv(selected);
                    selected
                }
            }
        };
        self.performance.policy_dispatch_elapsed_us = self
            .performance
            .policy_dispatch_elapsed_us
            .saturating_add(policy_dispatch_started.elapsed().as_micros());
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
        self.cache_inserts = self.cache_inserts.saturating_add(1);
        estimate
    }

    pub(super) fn finish(&self, best_frontier: Option<&SearchNode>) -> CombatSearchV2RolloutReport {
        CombatSearchV2RolloutReport {
            policy: self.policy.label(),
            behavioral_effect:
                "estimated_frontier_priority_only_no_terminal_outcome_no_baseline_claim",
            max_evaluations: self.max_evaluations,
            max_actions_per_rollout: self.max_actions,
            beam_width: self.beam_width,
            turn_beam_extension_budget: self.turn_beam_extension_budget,
            turn_beam_extensions: self.turn_beam_extensions,
            turn_beam_extension_budget_skips: self.turn_beam_extension_budget_skips,
            evaluations: self.evaluations,
            cache_queries: self.cache_queries,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            cache_inserts: self.cache_inserts,
            budget_skips: self.budget_skips,
            max_evaluation_budget_skips: self.max_evaluation_budget_skips,
            deadline_budget_skips: self.deadline_budget_skips,
            truncated_rollouts: self.truncated,
            terminal_wins: self.terminal_wins,
            terminal_losses: self.terminal_losses,
            rollouts_with_pending_choice: self.rollouts_with_pending_choice,
            rollouts_stopped_on_high_fanout_pending_choice: self
                .rollouts_stopped_on_high_fanout_pending_choice,
            pending_choice_actions_simulated: self.pending_choice_actions_simulated,
            max_pending_choice_estimated_action_fanout: self
                .max_pending_choice_estimated_action_fanout,
            performance: self.performance.to_report(),
            turn_beam_attribution: CombatSearchV2TurnBeamAttributionReport {
                enabled: self.policy == CombatSearchV2RolloutPolicy::TurnBeamNoPotion,
                calls: self.turn_beam_calls,
                conservative_anchor_present: self.turn_beam_conservative_anchor_present,
                conservative_anchor_selected: self.turn_beam_conservative_anchor_selected,
                conservative_anchor_terminal_wins: self
                    .turn_beam_conservative_anchor_terminal_wins,
                extension_calls: self.turn_beam_extensions,
                turn_plan_calls: self.turn_beam_turn_plan_calls,
                turn_plan_inner_nodes_expanded: self.turn_beam_turn_plan_inner_nodes_expanded,
                turn_plan_inner_nodes_generated: self.turn_beam_turn_plan_inner_nodes_generated,
                turn_plans_kept: self.turn_beam_turn_plans_kept,
                turn_plans_kept_by_bucket: self
                    .turn_beam_turn_plans_kept_by_bucket
                    .iter()
                    .map(|(bucket, count)| CombatSearchV2TurnBeamBucketCountReport {
                        bucket: *bucket,
                        count: *count,
                    })
                    .collect(),
                terminal_candidates_kept: self.turn_beam_terminal_candidates_kept,
                best_pv_len: self.turn_beam_best_pv_len,
                best_pv_terminal: self.turn_beam_best_pv_terminal,
            },
            best_frontier_estimate: best_frontier
                .and_then(|node| node.rollout_estimate.to_report()),
            notes: vec![
                "rollout estimates are not terminal outcome records",
                "conservative_no_potion uses only legal simulator actions and disables potion actions",
                "rollout cache is keyed by exact combat runtime state",
                "unresolved rollout priority uses phase-adjusted enemy effort from phase_profile",
                "high-fanout pending choices stop rollout estimates instead of selecting an arbitrary branch",
                "small pending choices may be followed by rollout, but their actions are still exact simulator inputs and never terminal outcome records",
                "enemy_mechanics_adaptive_no_potion uses phase-aware rollout for typed Guardian/Bronze Automaton mechanics and otherwise falls back to conservative_no_potion",
                "turn_beam_no_potion uses turn-plan end states as an estimate-only beam and still reports no terminal outcome record",
            ],
        }
    }

    fn observe_turn_beam_extension_attribution(
        &mut self,
        attribution: rollout::TurnBeamExtensionAttribution,
    ) {
        self.turn_beam_turn_plan_calls = self
            .turn_beam_turn_plan_calls
            .saturating_add(attribution.turn_plan_calls);
        self.turn_beam_turn_plan_inner_nodes_expanded = self
            .turn_beam_turn_plan_inner_nodes_expanded
            .saturating_add(attribution.turn_plan_inner_nodes_expanded);
        self.turn_beam_turn_plan_inner_nodes_generated = self
            .turn_beam_turn_plan_inner_nodes_generated
            .saturating_add(attribution.turn_plan_inner_nodes_generated);
        self.turn_beam_turn_plans_kept = self
            .turn_beam_turn_plans_kept
            .saturating_add(attribution.turn_plans_kept);
        self.turn_beam_terminal_candidates_kept = self
            .turn_beam_terminal_candidates_kept
            .saturating_add(attribution.terminal_candidates_kept);
        for (bucket, count) in attribution.turn_plans_kept_by_bucket {
            *self
                .turn_beam_turn_plans_kept_by_bucket
                .entry(bucket)
                .or_default() += count;
        }
        if self.turn_beam_best_pv_terminal.is_none()
            || attribution.best_pv_len > self.turn_beam_best_pv_len
        {
            self.turn_beam_best_pv_len = attribution.best_pv_len;
            self.turn_beam_best_pv_terminal = attribution.best_pv_terminal;
        }
    }

    fn observe_turn_beam_best_pv(&mut self, estimate: RolloutNodeEstimate) {
        if self.turn_beam_best_pv_terminal.is_none()
            || estimate.actions_simulated > self.turn_beam_best_pv_len
        {
            self.turn_beam_best_pv_len = estimate.actions_simulated;
            self.turn_beam_best_pv_terminal = Some(estimate.terminal);
        }
    }
}

pub(super) fn adaptive_no_potion_rollout_policy(node: &SearchNode) -> CombatSearchV2RolloutPolicy {
    let profile = combat_search_phase_profile(&node.engine, &node.combat);
    if profile.enemy_mechanics.guardian_open_count > 0
        || profile.enemy_mechanics.guardian_defensive_count > 0
        || profile.enemy_mechanics.bronze_automaton_count > 0
        || profile.enemy_mechanics.bronze_orb_count > 0
    {
        CombatSearchV2RolloutPolicy::PhaseAwareNoPotion
    } else {
        CombatSearchV2RolloutPolicy::ConservativeNoPotion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn adaptive_no_potion_rollout_uses_phase_aware_for_guardian_and_keeps_nob_conservative() {
        let mut guardian_combat = blank_test_combat();
        guardian_combat.entities.monsters = vec![test_monster(EnemyId::TheGuardian)];

        assert_eq!(
            adaptive_no_potion_rollout_policy(&test_search_node(guardian_combat)),
            CombatSearchV2RolloutPolicy::PhaseAwareNoPotion
        );

        let mut nob_combat = blank_test_combat();
        nob_combat.entities.monsters = vec![test_monster(EnemyId::GremlinNob)];

        assert_eq!(
            adaptive_no_potion_rollout_policy(&test_search_node(nob_combat)),
            CombatSearchV2RolloutPolicy::ConservativeNoPotion
        );
    }

    #[test]
    fn adaptive_no_potion_rollout_uses_phase_aware_for_bronze_automaton_mechanics() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::BronzeAutomaton)];

        assert_eq!(
            adaptive_no_potion_rollout_policy(&test_search_node(combat)),
            CombatSearchV2RolloutPolicy::PhaseAwareNoPotion
        );
    }

    #[test]
    fn rollout_report_exposes_cache_audit_and_internal_performance_buckets() {
        let mut cache = RolloutCache::new(
            CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            384,
            80,
            3,
        );
        cache.cache_queries = 4;
        cache.cache_misses = 3;
        cache.cache_inserts = 2;
        cache.max_evaluation_budget_skips = 1;
        cache.deadline_budget_skips = 2;
        cache.performance.cache_lookup_elapsed_us = 11;
        cache.performance.policy_dispatch_elapsed_us = 17;
        cache.performance.no_potion_iterations = 23;
        cache.performance.no_potion_phase_profile_elapsed_us = 29;
        cache.performance.no_potion_legal_actions_elapsed_us = 31;
        cache.performance.no_potion_choose_action_elapsed_us = 37;
        cache.performance.no_potion_choose_ordering_elapsed_us = 39;
        cache.performance.no_potion_probe_elapsed_us = 40;
        cache.performance.no_potion_probe_score_calls = 5;
        cache.performance.no_potion_probe_actions_evaluated = 4;
        cache.performance.no_potion_probe_step_reuses = 3;
        cache.performance.no_potion_probe_engine_step_elapsed_us = 6;
        cache.performance.no_potion_probe_phase_profile_elapsed_us = 7;
        cache.performance.no_potion_probe_action_facts_elapsed_us = 8;
        cache.performance.no_potion_engine_step_elapsed_us = 41;
        cache.performance.no_potion_child_build_elapsed_us = 43;

        let report = cache.finish(None);

        assert_eq!(report.cache_queries, 4);
        assert_eq!(report.cache_misses, 3);
        assert_eq!(report.cache_inserts, 2);
        assert_eq!(report.max_evaluation_budget_skips, 1);
        assert_eq!(report.deadline_budget_skips, 2);
        assert_eq!(report.performance.cache_lookup_us, 11);
        assert_eq!(report.performance.policy_dispatch_us, 17);
        assert_eq!(report.performance.no_potion_iterations, 23);
        assert_eq!(report.performance.no_potion_phase_profile_us, 29);
        assert_eq!(report.performance.no_potion_legal_actions_us, 31);
        assert_eq!(report.performance.no_potion_choose_action_us, 37);
        assert_eq!(report.performance.no_potion_choose_ordering_us, 39);
        assert_eq!(report.performance.no_potion_probe_us, 40);
        assert_eq!(report.performance.no_potion_probe_score_calls, 5);
        assert_eq!(report.performance.no_potion_probe_actions_evaluated, 4);
        assert_eq!(report.performance.no_potion_probe_step_reuses, 3);
        assert_eq!(report.performance.no_potion_probe_engine_step_us, 6);
        assert_eq!(report.performance.no_potion_probe_phase_profile_us, 7);
        assert_eq!(report.performance.no_potion_probe_action_facts_us, 8);
        assert_eq!(report.performance.no_potion_engine_step_us, 41);
        assert_eq!(report.performance.no_potion_child_build_us, 43);
    }

    fn test_search_node(combat: CombatState) -> SearchNode {
        SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat,
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        }
    }
}

fn better_rollout_estimate(
    left: RolloutNodeEstimate,
    right: RolloutNodeEstimate,
) -> RolloutNodeEstimate {
    let left_eval = combat_eval_from_rollout_estimate(left);
    let right_eval = combat_eval_from_rollout_estimate(right);
    if right_eval > left_eval {
        right
    } else {
        left
    }
}
