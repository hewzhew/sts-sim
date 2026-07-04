use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config,
    CombatSearchV2PotionPolicy, CombatSearchV2Report, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::search_review::search_review;
use super::search_types::SearchReview;
use super::Args;

pub(crate) fn review_child_rollout_policy(args: &Args) -> CombatSearchV2ChildRolloutPolicy {
    if args.immediate_child_rollout && !args.lazy_child_rollout {
        CombatSearchV2ChildRolloutPolicy::Immediate
    } else {
        CombatSearchV2ChildRolloutPolicy::LazyOnPop
    }
}

pub(crate) fn run_search(
    label: &'static str,
    case: &CombatCase,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    action_preview_limit: usize,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    disable_rollout: bool,
) -> (SearchReview, CombatSearchV2Report) {
    let rollout_policy = if disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    run_configured_search(
        label,
        case,
        CombatSearchV2Config {
            max_nodes: nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            turn_plan_policy,
            potion_policy,
            max_potions_used,
            rollout_policy,
            child_rollout_policy,
            ..CombatSearchV2Config::default()
        },
        action_preview_limit,
    )
}

pub(crate) fn run_configured_search(
    label: &'static str,
    case: &CombatCase,
    config: CombatSearchV2Config,
    action_preview_limit: usize,
) -> (SearchReview, CombatSearchV2Report) {
    let nodes = config.max_nodes;
    let wall_ms = config
        .wall_time
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default();
    let turn_plan_policy = config.turn_plan_policy;
    let potion_policy = config.potion_policy;
    let max_potions_used = config.max_potions_used;
    let phase_guard_policy = config.phase_guard_policy.label();
    let rollout_policy = config.rollout_policy.label();
    let report = run_combat_search_v2(&case.position.engine, &case.position.combat, config);
    let review = search_review(
        label,
        nodes,
        wall_ms,
        turn_plan_policy,
        potion_policy,
        max_potions_used,
        phase_guard_policy,
        &report,
        action_preview_limit,
        rollout_policy,
    );
    (review, report)
}
