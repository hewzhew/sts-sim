use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config, CombatSearchV2FrontierPolicy,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy,
};

#[derive(Clone, Copy)]
pub(crate) struct QualityLaneSpec {
    pub(crate) label: &'static str,
    pub(super) intent: &'static str,
    pub(super) frontier_policy: CombatSearchV2FrontierPolicy,
    pub(super) turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    pub(super) rollout_policy: CombatSearchV2RolloutPolicy,
    pub(super) child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    pub(super) potion_policy: CombatSearchV2PotionPolicy,
    pub(super) max_potions_used: Option<u32>,
    pub(super) phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
}

impl QualityLaneSpec {
    pub(crate) fn config(self, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Config {
        CombatSearchV2Config {
            max_nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            stop_on_win_hp_loss_at_most: Some(0),
            min_win_candidates_before_stop: 4,
            potion_policy: self.potion_policy,
            max_potions_used: self.max_potions_used,
            rollout_policy: self.rollout_policy,
            child_rollout_policy: self.child_rollout_policy,
            turn_plan_policy: self.turn_plan_policy,
            frontier_policy: self.frontier_policy,
            phase_guard_policy: self.phase_guard_policy,
            ..CombatSearchV2Config::default()
        }
    }
}

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
