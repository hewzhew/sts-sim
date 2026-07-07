use sts_simulator::ai::combat_search_v2::{
    CombatSearchRolloutPluginId, CombatSearchV2PotionPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::focus::review_focus;
use super::super::key_card_lifecycle::key_card_lifecycle;
use super::super::options::ReviewOptions;
use super::super::search_runner::{review_search_profile, run_profile_search};
use super::types::{FrozenPanelLaneResult, FrozenPanelLaneSpec};

pub(super) fn run_frozen_panel_lane(
    options: &ReviewOptions,
    case: &CombatCase,
    spec: FrozenPanelLaneSpec,
    rollout_plugin: CombatSearchRolloutPluginId,
) -> FrozenPanelLaneResult {
    let summary = spec.config_summary(options, rollout_plugin);
    let profile = review_search_profile(spec.lane, options.slow_nodes, options.slow_ms, options)
        .with_action_prior_plugin(spec.action_prior_plugin)
        .with_rollout_plugin(rollout_plugin)
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(options.diagnostic_potion_max);
    let (review, _) = run_profile_search(case, profile, options.action_preview_limit);
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
