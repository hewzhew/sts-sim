use sts_simulator::ai::combat_search_v2::{
    CombatSearchProfile, CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions,
};

#[derive(Clone, Copy)]
pub(super) struct CombatSearchRecipe {
    max_nodes: usize,
    wall_ms: u64,
    auto_ops: usize,
    wall_limited: bool,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    frontier_policy: Option<CombatSearchV2FrontierPolicy>,
    potion_policy: Option<CombatSearchV2PotionPolicy>,
    max_potions_used: Option<u32>,
    phase_guard_policy: Option<CombatSearchV2PhaseGuardPolicy>,
    setup_bias_policy: Option<CombatSearchV2SetupBiasPolicy>,
}

impl CombatSearchRecipe {
    pub(super) fn from_profile(
        profile: CombatSearchProfile,
        auto_ops: usize,
        wall_limited: bool,
    ) -> Self {
        let config = profile.to_config();
        Self {
            max_nodes: config.max_nodes,
            wall_ms: config
                .wall_time
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or_default(),
            auto_ops,
            wall_limited,
            turn_plan_policy: config.turn_plan_policy,
            child_rollout_policy: config.child_rollout_policy,
            rollout_policy: Some(config.rollout_policy),
            frontier_policy: Some(config.frontier_policy),
            potion_policy: Some(config.potion_policy),
            max_potions_used: config.max_potions_used,
            phase_guard_policy: Some(config.phase_guard_policy),
            setup_bias_policy: Some(config.setup_bias_policy),
        }
    }

    pub(super) fn into_auto_step_options(self) -> RunControlAutoStepOptions {
        RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                max_nodes: Some(self.max_nodes),
                wall_ms: Some(self.wall_ms),
                max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
                turn_plan_policy: Some(self.turn_plan_policy),
                child_rollout_policy: Some(self.child_rollout_policy),
                rollout_policy: self.rollout_policy,
                frontier_policy: self.frontier_policy,
                potion_policy: self.potion_policy,
                max_potions_used: self.max_potions_used,
                phase_guard_policy: self.phase_guard_policy,
                setup_bias_policy: self.setup_bias_policy,
                ..Default::default()
            },
            max_operations: Some(auto_run_chunk_ops(self.auto_ops, self.wall_limited)),
            route: RunControlRouteAutomationMode::Planner,
        }
    }
}

fn auto_run_chunk_ops(auto_ops: usize, wall_limited: bool) -> usize {
    if wall_limited {
        1
    } else {
        auto_ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, CombatSearchActionPriorPluginId,
        CombatSearchArtifactPluginId, CombatSearchBudgetSpec, CombatSearchChildRolloutPluginId,
        CombatSearchFrontierPluginId, CombatSearchPhaseGuardPluginId, CombatSearchPluginStack,
        CombatSearchPotionPlugin, CombatSearchProfile, CombatSearchRolloutPluginId,
        CombatSearchTurnPlanPluginId,
    };

    fn profile_for_test(max_nodes: usize, wall_ms: u64) -> CombatSearchProfile {
        CombatSearchProfile {
            label: "test_profile",
            budget: CombatSearchBudgetSpec { max_nodes, wall_ms },
            plugins: CombatSearchPluginStack::default(),
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
        }
    }

    #[test]
    fn recipe_materializes_core_search_options() {
        let profile = CombatSearchProfile {
            plugins: CombatSearchPluginStack {
                rollout: CombatSearchRolloutPluginId::Disabled,
                frontier: CombatSearchFrontierPluginId::SingleQueue,
                potion: CombatSearchPotionPlugin {
                    policy: CombatSearchV2PotionPolicy::Never,
                    max_potions_used: Some(0),
                },
                phase_guard: CombatSearchPhaseGuardPluginId::ChampSplitGuard,
                ..CombatSearchPluginStack::default()
            },
            ..profile_for_test(123, 456)
        };
        let options = CombatSearchRecipe::from_profile(profile, 7, false).into_auto_step_options();

        assert_eq!(options.search.max_nodes, Some(123));
        assert_eq!(options.search.wall_ms, Some(456));
        assert_eq!(options.max_operations, Some(7));
        assert_eq!(options.route, RunControlRouteAutomationMode::Planner);
        assert_eq!(
            options.search.turn_plan_policy,
            Some(CombatSearchV2TurnPlanPolicy::DiagnosticOnly)
        );
        assert_eq!(
            options.search.child_rollout_policy,
            Some(CombatSearchV2ChildRolloutPolicy::LazyOnPop)
        );
        assert_eq!(
            options.search.rollout_policy,
            Some(CombatSearchV2RolloutPolicy::Disabled)
        );
        assert_eq!(
            options.search.frontier_policy,
            Some(CombatSearchV2FrontierPolicy::SingleQueue)
        );
        assert_eq!(
            options.search.potion_policy,
            Some(CombatSearchV2PotionPolicy::Never)
        );
        assert_eq!(options.search.max_potions_used, Some(0));
        assert_eq!(
            options.search.phase_guard_policy,
            Some(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard)
        );
    }

    #[test]
    fn wall_limited_recipe_uses_single_operation_chunk() {
        let profile = CombatSearchProfile {
            plugins: CombatSearchPluginStack {
                child_rollout: CombatSearchChildRolloutPluginId::Immediate,
                ..CombatSearchPluginStack::default()
            },
            ..profile_for_test(10, 20)
        };
        let options = CombatSearchRecipe::from_profile(profile, 99, true).into_auto_step_options();

        assert_eq!(options.max_operations, Some(1));
    }

    #[test]
    fn recipe_materializes_explicit_combat_search_profile() {
        let profile = CombatSearchProfile {
            label: "test_profile",
            budget: CombatSearchBudgetSpec {
                max_nodes: 321,
                wall_ms: 654,
            },
            plugins: CombatSearchPluginStack {
                action_prior: CombatSearchActionPriorPluginId::KeyCardOnline,
                turn_plan: CombatSearchTurnPlanPluginId::DiagnosticOnly,
                child_rollout: CombatSearchChildRolloutPluginId::Immediate,
                rollout: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
                frontier: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
                potion: CombatSearchPotionPlugin {
                    policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
                    max_potions_used: Some(2),
                },
                phase_guard: CombatSearchPhaseGuardPluginId::ChampSplitGuard,
                ..CombatSearchPluginStack::default()
            },
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
        };

        let options = CombatSearchRecipe::from_profile(profile, 5, false).into_auto_step_options();

        assert_eq!(options.search.max_nodes, Some(321));
        assert_eq!(options.search.wall_ms, Some(654));
        assert_eq!(options.max_operations, Some(5));
        assert_eq!(
            options.search.child_rollout_policy,
            Some(CombatSearchV2ChildRolloutPolicy::Immediate)
        );
        assert_eq!(
            options.search.frontier_policy,
            Some(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets)
        );
        assert_eq!(
            options.search.potion_policy,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted)
        );
        assert_eq!(options.search.max_potions_used, Some(2));
        assert_eq!(
            options.search.phase_guard_policy,
            Some(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard)
        );
        assert_eq!(
            options.search.setup_bias_policy,
            Some(CombatSearchV2SetupBiasPolicy::KeyCardOnline)
        );
    }
}
