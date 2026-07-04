use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2Report, CombatSearchV2TurnPlanPolicy,
};

use super::search_types::{
    SearchDiagnosticProgressFacts, SearchPerformanceReview, SearchReview, SearchReviewFacts,
    SearchRolloutPerformanceReview,
};

pub(super) fn search_review(
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    phase_guard_policy: &'static str,
    report: &CombatSearchV2Report,
    action_preview_limit: usize,
    rollout_policy: &'static str,
) -> SearchReview {
    let best = report.best_win_trajectory.as_ref();
    SearchReview {
        label,
        nodes,
        wall_ms,
        rollout_policy,
        turn_plan_policy: turn_plan_policy.label(),
        phase_guard_policy,
        child_rollout_policy: report.search_policy.child_rollout_policy,
        potion_policy: potion_policy_label(potion_policy),
        max_potions_used,
        complete_win: best.is_some(),
        hp_loss: best.map(|trajectory| trajectory.hp_loss),
        final_hp: best.map(|trajectory| trajectory.final_hp),
        turns: best.map(|trajectory| trajectory.turns),
        potions_used: best.map(|trajectory| trajectory.potions_used),
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        nodes_to_first_win: report.stats.nodes_to_first_win,
        terminal_wins: report.stats.terminal_wins,
        elapsed_ms: report.stats.elapsed_ms,
        deadline_hit: report.stats.deadline_hit,
        node_budget_hit: report.stats.node_budget_hit,
        performance: SearchPerformanceReview {
            total_us: report.performance.total_elapsed_us,
            rollout_us: report.performance.rollout_estimate_elapsed_us,
            rollout_calls: report.performance.rollout_estimate_calls,
            root_rollout_calls: report.performance.root_rollout_estimate_calls,
            child_rollout_calls: report.performance.child_rollout_estimate_calls,
            deferred_child_rollout_calls: report.performance.deferred_child_rollout_estimate_calls,
            turn_plan_seed_rollout_calls: report.performance.turn_plan_seed_rollout_estimate_calls,
            rollout_evaluations: report.rollout.evaluations,
            rollout_budget_skips: report.rollout.budget_skips,
            rollout_max_evaluation_budget_skips: report.rollout.max_evaluation_budget_skips,
            rollout_deadline_budget_skips: report.rollout.deadline_budget_skips,
            deferred_child_rollout_admitted_signal: report
                .performance
                .deferred_child_rollout_admitted_signal,
            deferred_child_rollout_admitted_periodic: report
                .performance
                .deferred_child_rollout_admitted_periodic,
            deferred_child_rollout_skipped_low_signal: report
                .performance
                .deferred_child_rollout_skipped_low_signal,
            deferred_child_rollout_skipped_budget_share: report
                .performance
                .deferred_child_rollout_skipped_budget_share,
            turn_plan_seed_us: report.performance.turn_plan_frontier_seed_elapsed_us,
            engine_step_us: report.performance.engine_step_elapsed_us,
            frontier_pop_us: report.performance.frontier_pop_elapsed_us,
            expansion_us: report.performance.expansion_elapsed_us,
            child_bookkeeping_us: report.performance.child_bookkeeping_elapsed_us,
            rollout_profile: SearchRolloutPerformanceReview {
                cache_queries: report.rollout.cache_queries,
                cache_hits: report.rollout.cache_hits,
                cache_misses: report.rollout.cache_misses,
                cache_lookup_us: report.rollout.performance.cache_lookup_us,
                policy_dispatch_us: report.rollout.performance.policy_dispatch_us,
                no_potion_iterations: report.rollout.performance.no_potion_iterations,
                no_potion_phase_profile_us: report.rollout.performance.no_potion_phase_profile_us,
                no_potion_legal_actions_us: report.rollout.performance.no_potion_legal_actions_us,
                no_potion_choose_action_us: report.rollout.performance.no_potion_choose_action_us,
                no_potion_choose_ordering_us: report
                    .rollout
                    .performance
                    .no_potion_choose_ordering_us,
                no_potion_probe_us: report.rollout.performance.no_potion_probe_us,
                no_potion_probe_score_calls: report.rollout.performance.no_potion_probe_score_calls,
                no_potion_probe_actions_evaluated: report
                    .rollout
                    .performance
                    .no_potion_probe_actions_evaluated,
                no_potion_probe_step_reuses: report.rollout.performance.no_potion_probe_step_reuses,
                no_potion_probe_engine_step_us: report
                    .rollout
                    .performance
                    .no_potion_probe_engine_step_us,
                no_potion_probe_phase_profile_us: report
                    .rollout
                    .performance
                    .no_potion_probe_phase_profile_us,
                no_potion_probe_action_facts_us: report
                    .rollout
                    .performance
                    .no_potion_probe_action_facts_us,
                no_potion_engine_step_us: report.rollout.performance.no_potion_engine_step_us,
                no_potion_child_build_us: report.rollout.performance.no_potion_child_build_us,
            },
        },
        facts: SearchReviewFacts {
            diagnostic_progress: diagnostic_progress_facts(report, action_preview_limit),
        },
    }
}

fn diagnostic_progress_facts(
    report: &CombatSearchV2Report,
    action_preview_limit: usize,
) -> Option<SearchDiagnosticProgressFacts> {
    if let Some(trajectory) = report.best_complete_trajectory.as_ref() {
        return Some(SearchDiagnosticProgressFacts {
            source: "best_complete",
            terminal: trajectory.terminal,
            estimated: trajectory.estimated,
            final_hp: trajectory.final_hp,
            hp_loss: trajectory.hp_loss,
            turns: trajectory.turns,
            potions_used: trajectory.potions_used,
            cards_played: trajectory.cards_played,
            living_enemy_count: trajectory.final_state.living_enemy_count,
            total_enemy_hp: trajectory.final_state.total_enemy_hp,
            visible_incoming_damage: Some(trajectory.final_state.visible_incoming_damage),
            action_count: Some(trajectory.actions.len()),
            exact_prefix_action_count: Some(trajectory.actions.len()),
            action_key_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.action_key.clone())
                .collect(),
            input_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.input.clone())
                .collect(),
        });
    }
    report
        .rollout
        .best_frontier_estimate
        .as_ref()
        .map(|rollout| {
            let frontier = report.best_frontier_trajectory.as_ref();
            let exact_prefix_actions = frontier
                .map(|trajectory| trajectory.actions.as_slice())
                .unwrap_or(&[]);
            let exact_prefix_action_count = Some(exact_prefix_actions.len());
            SearchDiagnosticProgressFacts {
                source: "rollout_frontier",
                terminal: rollout.terminal,
                estimated: rollout.estimated,
                final_hp: rollout.final_hp,
                hp_loss: rollout.hp_loss,
                turns: rollout.turns,
                potions_used: rollout.potions_used,
                cards_played: rollout.cards_played,
                living_enemy_count: rollout.living_enemy_count,
                total_enemy_hp: rollout.total_enemy_hp,
                visible_incoming_damage: frontier
                    .map(|trajectory| trajectory.final_state.visible_incoming_damage),
                action_count: Some(
                    rollout
                        .actions_simulated
                        .saturating_add(exact_prefix_actions.len()),
                ),
                exact_prefix_action_count,
                action_key_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.action_key.clone())
                    .collect(),
                input_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.input.clone())
                    .collect(),
            }
        })
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}
