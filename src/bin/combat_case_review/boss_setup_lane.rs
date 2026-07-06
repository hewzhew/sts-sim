use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::focus::{review_focus, CombatReviewFocus};
use super::key_card_lifecycle::{key_card_lifecycle, KeyCardLifecycleReport};
use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct BossSetupLaneReview {
    schema: &'static str,
    contract: &'static str,
    lane: &'static str,
    skipped_reason: Option<&'static str>,
    search: Option<SearchReview>,
    focus: Option<CombatReviewFocus>,
    key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

pub(super) fn run_boss_setup_lane(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<BossSetupLaneReview> {
    if !options.boss_setup_lane {
        return None;
    }
    if !case.position.combat.meta.is_boss_fight {
        return Some(skipped("not_boss_fight"));
    }

    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let (search, _) = run_configured_search(
        "boss_setup_key_card_online",
        case,
        CombatSearchV2Config {
            max_nodes: options.slow_nodes,
            wall_time: Some(Duration::from_millis(options.slow_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::All,
            max_potions_used: Some(options.diagnostic_potion_max),
            rollout_policy,
            child_rollout_policy: options.child_rollout_policy(),
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
            ..CombatSearchV2Config::default()
        },
        options.action_preview_limit,
    );
    let focus = review_focus(std::slice::from_ref(&search));
    let key_card_lifecycle = key_card_lifecycle(&case.position, focus.as_ref());

    Some(BossSetupLaneReview {
        schema: "boss_setup_lane_v0",
        contract: "review_only_key_setup_bias_search_not_runner_execution",
        lane: "boss_setup_key_card_online",
        skipped_reason: None,
        search: Some(search),
        focus,
        key_card_lifecycle,
    })
}

fn skipped(reason: &'static str) -> BossSetupLaneReview {
    BossSetupLaneReview {
        schema: "boss_setup_lane_v0",
        contract: "review_only_key_setup_bias_search_not_runner_execution",
        lane: "boss_setup_key_card_online",
        skipped_reason: Some(reason),
        search: None,
        focus: None,
        key_card_lifecycle: None,
    }
}
