use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};

use super::super::options::ReviewOptions;

pub(super) fn duel_search_config(
    options: &ReviewOptions,
    label: &'static str,
) -> CombatSearchV2Config {
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    CombatSearchV2Config {
        max_nodes: options.slow_nodes,
        wall_time: Some(Duration::from_millis(options.slow_ms)),
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(options.diagnostic_potion_max),
        rollout_policy,
        child_rollout_policy: options.child_rollout_policy(),
        setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
        input_label: Some(label.to_string()),
        ..CombatSearchV2Config::default()
    }
}
