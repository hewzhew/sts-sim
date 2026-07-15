use sts_simulator::ai::combat_search_v2::CombatSearchProfile;
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
};

#[derive(Clone, Copy)]
pub(super) struct CombatSearchRecipe {
    profile: CombatSearchProfile,
    auto_ops: usize,
    wall_limited: bool,
}

impl CombatSearchRecipe {
    pub(super) fn from_profile(
        profile: CombatSearchProfile,
        auto_ops: usize,
        wall_limited: bool,
    ) -> Self {
        Self {
            profile,
            auto_ops,
            wall_limited,
        }
    }

    pub(super) fn into_auto_step_options(self) -> RunControlAutoStepOptions {
        RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                profile: Some(self.profile),
                disable_no_win_rescue: true,
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
        CombatSearchArtifactPluginId, CombatSearchAttemptPolicy, CombatSearchBudgetSpec,
        CombatSearchChildRolloutPluginId, CombatSearchEngineProfile, CombatSearchFrontierPluginId,
        CombatSearchPhaseGuardPluginId, CombatSearchPluginStack, CombatSearchPotionPlugin,
        CombatSearchProfile, CombatSearchRolloutPluginId, CombatSearchTurnPlanPluginId,
        CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy,
        CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
        CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
    };

    fn profile_for_test(max_nodes: usize, wall_ms: u64) -> CombatSearchProfile {
        CombatSearchProfile {
            label: "test_profile",
            engine: CombatSearchEngineProfile {
                budget: CombatSearchBudgetSpec { max_nodes, wall_ms },
                plugins: CombatSearchPluginStack::default(),
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
            },
        }
    }

    #[test]
    fn recipe_leaves_hp_loss_policy_to_owner_audit() {
        let options = CombatSearchRecipe::from_profile(profile_for_test(10, 20), 3, false)
            .into_auto_step_options();

        assert_eq!(options.search.max_hp_loss, None);
    }

    #[test]
    fn recipe_carries_core_search_profile() {
        let mut profile = profile_for_test(123, 456);
        profile.engine.plugins = CombatSearchPluginStack {
            rollout: CombatSearchRolloutPluginId::Disabled,
            frontier: CombatSearchFrontierPluginId::SingleQueue,
            potion: CombatSearchPotionPlugin {
                policy: CombatSearchV2PotionPolicy::Never,
                max_potions_used: Some(0),
            },
            phase_guard: CombatSearchPhaseGuardPluginId::ChampSplitGuard,
            ..CombatSearchPluginStack::default()
        };
        let options = CombatSearchRecipe::from_profile(profile, 7, false).into_auto_step_options();
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, 123);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis()),
            Some(456)
        );
        assert_eq!(options.max_operations, Some(7));
        assert_eq!(options.route, RunControlRouteAutomationMode::Planner);
        assert_eq!(
            config.turn_plan_policy,
            CombatSearchV2TurnPlanPolicy::Disabled
        );
        assert_eq!(
            config.child_rollout_policy,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop
        );
        assert_eq!(config.rollout_policy, CombatSearchV2RolloutPolicy::Disabled);
        assert_eq!(
            config.frontier_policy,
            CombatSearchV2FrontierPolicy::SingleQueue
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
        assert_eq!(
            config.phase_guard_policy,
            CombatSearchV2PhaseGuardPolicy::ChampSplitGuard
        );
    }

    #[test]
    fn wall_limited_recipe_uses_single_operation_chunk() {
        let mut profile = profile_for_test(10, 20);
        profile.engine.plugins = CombatSearchPluginStack {
            child_rollout: CombatSearchChildRolloutPluginId::Immediate,
            ..CombatSearchPluginStack::default()
        };
        let options = CombatSearchRecipe::from_profile(profile, 99, true).into_auto_step_options();

        assert_eq!(options.max_operations, Some(1));
    }

    #[test]
    fn portfolio_recipe_disables_internal_no_win_rescue() {
        let profile = profile_for_test(10, 20);
        let options = CombatSearchRecipe::from_profile(profile, 3, false).into_auto_step_options();

        assert!(options.search.disable_no_win_rescue);
    }

    #[test]
    fn recipe_preserves_explicit_combat_search_profile() {
        let profile = CombatSearchProfile {
            label: "test_profile",
            engine: CombatSearchEngineProfile {
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
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
                artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
            },
        };

        let options = CombatSearchRecipe::from_profile(profile, 5, false).into_auto_step_options();
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, 321);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis()),
            Some(654)
        );
        assert_eq!(options.max_operations, Some(5));
        assert_eq!(
            config.child_rollout_policy,
            CombatSearchV2ChildRolloutPolicy::Immediate
        );
        assert_eq!(
            config.frontier_policy,
            CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets
        );
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));
        assert_eq!(
            config.phase_guard_policy,
            CombatSearchV2PhaseGuardPolicy::ChampSplitGuard
        );
        assert_eq!(
            config.setup_bias_policy,
            CombatSearchV2SetupBiasPolicy::KeyCardOnline
        );
    }
}
