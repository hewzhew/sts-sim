use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::focus::review_focus;
use super::super::key_card_lifecycle::key_card_lifecycle;
use super::super::options::ReviewOptions;
use super::super::search_runner::run_configured_search;
use super::types::{FrozenPanelLaneResult, FrozenPanelLaneSpec};

pub(super) fn run_frozen_panel_lane(
    options: &ReviewOptions,
    case: &CombatCase,
    spec: FrozenPanelLaneSpec,
    rollout_policy: CombatSearchV2RolloutPolicy,
) -> FrozenPanelLaneResult {
    let summary = spec.config_summary(options, rollout_policy);
    let (review, _) = run_configured_search(
        spec.lane,
        case,
        CombatSearchV2Config {
            max_nodes: options.slow_nodes,
            wall_time: Some(Duration::from_millis(options.slow_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::All,
            max_potions_used: Some(options.diagnostic_potion_max),
            rollout_policy,
            child_rollout_policy: options.child_rollout_policy(),
            setup_bias_policy: spec.setup_bias_policy,
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
            ..CombatSearchV2Config::default()
        },
        options.action_preview_limit,
    );
    let focus = review_focus(std::slice::from_ref(&review));
    let key_card_lifecycle = key_card_lifecycle(&case.position, focus.as_ref());
    FrozenPanelLaneResult {
        lane: spec.lane,
        search_config_summary: summary,
        review,
        focus,
        key_card_lifecycle,
    }
}
