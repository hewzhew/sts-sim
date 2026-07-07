use sts_simulator::ai::combat_search_v2::{
    CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchBudgetSpec,
    CombatSearchChildRolloutPluginId, CombatSearchFrontierPluginId, CombatSearchPhaseGuardPluginId,
    CombatSearchPluginStack, CombatSearchPotionPlugin, CombatSearchProfile,
    CombatSearchRolloutPluginId, CombatSearchTurnPlanPluginId, CombatSearchV2Config,
    CombatSearchV2PotionPolicy,
};

#[derive(Clone, Copy)]
pub(crate) struct QualityLaneSpec {
    pub(crate) label: &'static str,
    pub(super) intent: &'static str,
    pub(super) frontier_plugin: CombatSearchFrontierPluginId,
    pub(super) turn_plan_plugin: CombatSearchTurnPlanPluginId,
    pub(super) rollout_plugin: CombatSearchRolloutPluginId,
    pub(super) child_rollout_plugin: CombatSearchChildRolloutPluginId,
    pub(super) potion: CombatSearchPotionPlugin,
    pub(super) phase_guard_plugin: CombatSearchPhaseGuardPluginId,
}

impl QualityLaneSpec {
    pub(crate) fn profile(self, max_nodes: usize, wall_ms: u64) -> CombatSearchProfile {
        CombatSearchProfile {
            label: self.label,
            budget: CombatSearchBudgetSpec { max_nodes, wall_ms },
            plugins: CombatSearchPluginStack {
                frontier: self.frontier_plugin,
                turn_plan: self.turn_plan_plugin,
                rollout: self.rollout_plugin,
                child_rollout: self.child_rollout_plugin,
                potion: self.potion,
                phase_guard: self.phase_guard_plugin,
                ..CombatSearchPluginStack::default()
            },
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::None,
        }
    }

    pub(crate) fn config(self, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Config {
        let mut config = self.profile(max_nodes, wall_ms).to_config();
        config.stop_on_win_hp_loss_at_most = Some(0);
        config.min_win_candidates_before_stop = 4;
        config
    }
}

pub(crate) fn quality_lane_specs() -> [QualityLaneSpec; 4] {
    [
        QualityLaneSpec {
            label: "quality_balanced_rr",
            intent: "baseline round-robin frontier with adaptive rollout",
            frontier_plugin: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            turn_plan_plugin: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            rollout_plugin: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_plugin: CombatSearchChildRolloutPluginId::LazyOnPop,
            potion: CombatSearchPotionPlugin {
                policy: CombatSearchV2PotionPolicy::Never,
                max_potions_used: Some(0),
            },
            phase_guard_plugin: CombatSearchPhaseGuardPluginId::Default,
        },
        QualityLaneSpec {
            label: "quality_champ_split_guard",
            intent: "penalize crossing Champ half-hp threshold before a clear burst window",
            frontier_plugin: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            turn_plan_plugin: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            rollout_plugin: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_plugin: CombatSearchChildRolloutPluginId::Immediate,
            potion: CombatSearchPotionPlugin {
                policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
                max_potions_used: Some(2),
            },
            phase_guard_plugin: CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        },
        QualityLaneSpec {
            label: "quality_immediate_rescue_no_potion",
            intent: "force immediate child rollout so low-hp tactical lines are not under-sampled",
            frontier_plugin: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            turn_plan_plugin: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            rollout_plugin: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_plugin: CombatSearchChildRolloutPluginId::Immediate,
            potion: CombatSearchPotionPlugin {
                policy: CombatSearchV2PotionPolicy::Never,
                max_potions_used: Some(0),
            },
            phase_guard_plugin: CombatSearchPhaseGuardPluginId::Default,
        },
        QualityLaneSpec {
            label: "quality_immediate_potion_rescue",
            intent:
                "try semantic potion rescue with immediate rollout before declaring a combat gap",
            frontier_plugin: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            turn_plan_plugin: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            rollout_plugin: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_plugin: CombatSearchChildRolloutPluginId::Immediate,
            potion: CombatSearchPotionPlugin {
                policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
                max_potions_used: Some(2),
            },
            phase_guard_plugin: CombatSearchPhaseGuardPluginId::Default,
        },
    ]
}
