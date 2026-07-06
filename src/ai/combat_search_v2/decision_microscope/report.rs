use super::super::*;
use super::{
    CombatSearchV2DecisionMicroscopeConfigReport, CombatSearchV2DecisionSelectedAction,
    CombatSearchV2DecisionTrajectorySummary,
};

pub(super) fn selected_first_action(
    engine: &EngineState,
    combat: &CombatState,
    config: &CombatSearchV2Config,
    search_report: &CombatSearchV2Report,
) -> Option<CombatSearchV2DecisionSelectedAction> {
    let (selection_source, action) = if let Some(action) = search_report
        .best_win_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.actions.first())
    {
        ("best_win_trajectory_first_action", action)
    } else {
        (
            "best_complete_trajectory_first_action",
            search_report
                .best_complete_trajectory
                .as_ref()?
                .actions
                .first()?,
        )
    };
    Some(CombatSearchV2DecisionSelectedAction {
        action_id: action.action_id,
        action_key: action.action_key.clone(),
        action_debug: action.action_debug.clone(),
        action_role: combat_search_action_ordering_role_label_for_state_with_policy(
            engine,
            combat,
            &action.input,
            config.phase_guard_policy,
            config.setup_bias_policy,
        ),
        selection_source,
    })
}

pub(super) fn trajectory_summary(
    trajectory: &CombatSearchV2TrajectoryReport,
) -> CombatSearchV2DecisionTrajectorySummary {
    CombatSearchV2DecisionTrajectorySummary {
        terminal: trajectory.terminal,
        estimated: trajectory.estimated,
        final_hp: trajectory.final_hp,
        hp_loss: trajectory.hp_loss,
        turns: trajectory.turns,
        potions_used: trajectory.potions_used,
        potions_discarded: trajectory.potions_discarded,
        cards_played: trajectory.cards_played,
        action_count: trajectory.actions.len(),
    }
}

pub(super) fn config_report(
    config: &CombatSearchV2Config,
) -> CombatSearchV2DecisionMicroscopeConfigReport {
    CombatSearchV2DecisionMicroscopeConfigReport {
        max_nodes: config.max_nodes,
        max_actions_per_line: config.max_actions_per_line,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
        potion_policy: config.potion_policy.label(),
        max_potions_used: config.max_potions_used,
        rollout_policy: config.rollout_policy.label(),
        rollout_max_evaluations: config.rollout_max_evaluations,
        rollout_max_actions: config.rollout_max_actions,
        rollout_beam_width: config.rollout_beam_width,
        frontier_policy: config.frontier_policy.label(),
        phase_guard_policy: config.phase_guard_policy.label(),
        setup_bias_policy: config.setup_bias_policy.label(),
    }
}
