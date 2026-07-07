use super::super::*;

pub(super) fn search_policy_report(config: &CombatSearchV2Config) -> CombatSearchV2PolicyReport {
    let plugins = CombatSearchPluginStack::from_config(config);
    CombatSearchV2PolicyReport {
        kind: "best_first_atomic_action_graph_search_v2",
        terminal_policy: "whole_combat_terminal_only",
        expansion_order:
            "conservative_duplicate_action_equivalence_then_semantic_turn_action_ordering_then_frontier_value_v1",
        action_prior_policy: CombatSearchV2SetupBiasPolicy::from(plugins.action_prior).label(),
        phase_guard_policy: CombatSearchV2PhaseGuardPolicy::from(plugins.phase_guard).label(),
        frontier_value: COMBAT_SEARCH_FRONTIER_VALUE_POLICY,
        frontier_policy: CombatSearchV2FrontierPolicy::from(plugins.frontier).label(),
        turn_branching: "turn_transition_classification_with_late_frontier_tie_break",
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::from(plugins.turn_plan).label(),
        potion_policy: plugins.potion.policy.label(),
        transposition_table: "exact_runtime_state_key_with_resource_coverage",
        dominance_pruning:
            "global_dominance_bucket_resource_vector_plus_same_parent_same_turn_sibling_coverage",
        rollout_value:
            "combat_eval_v2_risk_bucketed_unresolved_estimate_used_for_frontier_priority_only_not_terminal_claims",
        rollout_policy: CombatSearchV2RolloutPolicy::from(plugins.rollout).label(),
        child_rollout_policy: CombatSearchV2ChildRolloutPolicy::from(plugins.child_rollout).label(),
        llm_authority: "none",
    }
}

pub(super) fn budget_report(config: &CombatSearchV2Config) -> CombatSearchV2BudgetReport {
    CombatSearchV2BudgetReport {
        max_nodes: config.max_nodes,
        max_actions_per_line: config.max_actions_per_line,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
        stop_on_win_hp_loss_at_most: config.stop_on_win_hp_loss_at_most,
        min_win_candidates_before_stop: config.min_win_candidates_before_stop,
        max_potions_used: config.max_potions_used,
        rollout_max_evaluations: config.rollout_max_evaluations,
        rollout_max_actions: config.rollout_max_actions,
        rollout_beam_width: config.rollout_beam_width,
    }
}
