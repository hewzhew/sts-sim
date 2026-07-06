use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};

use super::types::QualityLaneSpec;

pub(crate) fn quality_lane_specs() -> [QualityLaneSpec; 4] {
    [
        QualityLaneSpec {
            label: "quality_balanced_rr",
            intent: "baseline round-robin frontier with adaptive rollout",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
        QualityLaneSpec {
            label: "quality_champ_split_guard",
            intent: "penalize crossing Champ half-hp threshold before a clear burst window",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
            max_potions_used: Some(2),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::ChampSplitGuard,
        },
        QualityLaneSpec {
            label: "quality_immediate_rescue_no_potion",
            intent: "force immediate child rollout so low-hp tactical lines are not under-sampled",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
        QualityLaneSpec {
            label: "quality_immediate_potion_rescue",
            intent:
                "try semantic potion rescue with immediate rollout before declaring a combat gap",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            potion_policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
            max_potions_used: Some(2),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
        },
    ]
}
