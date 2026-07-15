use sts_simulator::ai::combat_search_v2::{
    CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchAttemptPolicy,
    CombatSearchBudgetSpec, CombatSearchChildRolloutPluginId, CombatSearchEngineProfile,
    CombatSearchFrontierPluginId, CombatSearchPhaseGuardPluginId, CombatSearchPluginStack,
    CombatSearchPotionPlugin, CombatSearchProfile, CombatSearchRolloutPluginId,
    CombatSearchTurnPlanPluginId, CombatSearchV2Config, CombatSearchV2PotionPolicy,
};

#[derive(Clone, Copy)]
pub(crate) struct QualityLaneSpec {
    pub(crate) label: &'static str,
    pub(super) intent: &'static str,
    plugins: QualityLanePlugins,
    objective: QualitySearchObjective,
}

#[derive(Clone, Copy)]
struct QualityLanePlugins {
    pub(super) frontier_plugin: CombatSearchFrontierPluginId,
    pub(super) turn_plan_plugin: CombatSearchTurnPlanPluginId,
    pub(super) rollout_plugin: CombatSearchRolloutPluginId,
    pub(super) child_rollout_plugin: CombatSearchChildRolloutPluginId,
    pub(super) potion: CombatSearchPotionPlugin,
    pub(super) phase_guard_plugin: CombatSearchPhaseGuardPluginId,
}

#[derive(Clone, Copy)]
pub(crate) struct QualitySearchObjective {
    stop_on_win_hp_loss_at_most: Option<u32>,
    min_win_candidates_before_stop: usize,
}

impl QualityLaneSpec {
    pub(crate) fn profile(self, max_nodes: usize, wall_ms: u64) -> CombatSearchProfile {
        self.plugins.profile(self.label, max_nodes, wall_ms)
    }

    pub(crate) fn config(self, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Config {
        self.objective.apply(self.profile(max_nodes, wall_ms))
    }
}

impl QualityLanePlugins {
    fn profile(self, label: &'static str, max_nodes: usize, wall_ms: u64) -> CombatSearchProfile {
        CombatSearchProfile {
            label,
            engine: CombatSearchEngineProfile {
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
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                artifacts: CombatSearchArtifactPluginId::None,
            },
        }
    }
}

impl QualitySearchObjective {
    pub(crate) fn strict_low_loss() -> Self {
        Self {
            stop_on_win_hp_loss_at_most: Some(0),
            min_win_candidates_before_stop: 4,
        }
    }

    pub(crate) fn apply(self, profile: CombatSearchProfile) -> CombatSearchV2Config {
        let mut config = profile.to_config();
        config.stop_on_win_hp_loss_at_most = self.stop_on_win_hp_loss_at_most;
        config.min_win_candidates_before_stop = self.min_win_candidates_before_stop;
        config
    }
}

pub(crate) fn quality_lane_specs() -> [QualityLaneSpec; 4] {
    [
        QualityLaneSpec {
            label: "quality_balanced_rr",
            intent: "baseline round-robin frontier with adaptive rollout",
            plugins: QualityLanePlugins {
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
            objective: QualitySearchObjective::strict_low_loss(),
        },
        QualityLaneSpec {
            label: "quality_champ_split_guard",
            intent: "penalize crossing Champ half-hp threshold before a clear burst window",
            plugins: QualityLanePlugins {
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
            objective: QualitySearchObjective::strict_low_loss(),
        },
        QualityLaneSpec {
            label: "quality_immediate_rescue_no_potion",
            intent: "force immediate child rollout so low-hp tactical lines are not under-sampled",
            plugins: QualityLanePlugins {
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
            objective: QualitySearchObjective::strict_low_loss(),
        },
        QualityLaneSpec {
            label: "quality_immediate_potion_rescue",
            intent:
                "try semantic potion rescue with immediate rollout before declaring a combat gap",
            plugins: QualityLanePlugins {
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
            objective: QualitySearchObjective::strict_low_loss(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::CombatSearchV2RolloutPolicy;

    #[test]
    fn quality_objective_does_not_change_lane_plugins() {
        let spec = quality_lane_specs()[1];
        let profile_config = spec.profile(123, 456).to_config();
        let quality_config = spec.config(123, 456);

        assert_eq!(quality_config.max_nodes, 123);
        assert_eq!(quality_config.wall_time, profile_config.wall_time);
        assert_eq!(
            quality_config.rollout_policy,
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        );
        assert_eq!(quality_config.potion_policy, profile_config.potion_policy);
        assert_eq!(
            quality_config.phase_guard_policy,
            profile_config.phase_guard_policy
        );
        assert_eq!(quality_config.stop_on_win_hp_loss_at_most, Some(0));
        assert_eq!(quality_config.min_win_candidates_before_stop, 4);
    }
}
