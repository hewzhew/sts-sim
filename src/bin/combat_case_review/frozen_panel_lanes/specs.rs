use sts_simulator::ai::combat_search_v2::{
    CombatSearchActionPriorPluginId, CombatSearchRolloutPluginId, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};

use super::super::options::ReviewOptions;
use super::types::{FrozenPanelLaneConfigSummary, FrozenPanelLaneSpec};

impl FrozenPanelLaneSpec {
    pub(super) fn config_summary(
        self,
        options: &ReviewOptions,
        rollout_plugin: CombatSearchRolloutPluginId,
    ) -> FrozenPanelLaneConfigSummary {
        FrozenPanelLaneConfigSummary {
            max_nodes: options.slow_nodes,
            wall_ms: options.slow_ms,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly.label(),
            potion_policy: "all",
            max_potions_used: options.diagnostic_potion_max,
            rollout_policy: CombatSearchV2RolloutPolicy::from(rollout_plugin).label(),
            child_rollout_policy: options.child_rollout_policy().label(),
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::from(self.action_prior_plugin)
                .label(),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default.label(),
        }
    }

    #[cfg(test)]
    fn config_summary_without_setup_bias(
        self,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        (
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly.label(),
            "all",
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion.label(),
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::LazyOnPop
                .label(),
            CombatSearchV2PhaseGuardPolicy::Default.label(),
        )
    }
}

pub(crate) fn frozen_panel_lane_specs() -> [FrozenPanelLaneSpec; 2] {
    [
        FrozenPanelLaneSpec {
            lane: "baseline",
            action_prior_plugin: CombatSearchActionPriorPluginId::Default,
        },
        FrozenPanelLaneSpec {
            lane: "key_setup_bias",
            action_prior_plugin: CombatSearchActionPriorPluginId::KeyCardOnline,
        },
    ]
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::combat_search_v2::CombatSearchActionPriorPluginId;

    use super::frozen_panel_lane_specs;

    #[test]
    fn frozen_panel_lane_specs_only_differ_by_setup_bias() {
        let specs = frozen_panel_lane_specs();

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].lane, "baseline");
        assert_eq!(
            specs[0].action_prior_plugin,
            CombatSearchActionPriorPluginId::Default
        );
        assert_eq!(specs[1].lane, "key_setup_bias");
        assert_eq!(
            specs[1].action_prior_plugin,
            CombatSearchActionPriorPluginId::KeyCardOnline
        );
        assert_eq!(
            specs[0].config_summary_without_setup_bias(),
            specs[1].config_summary_without_setup_bias()
        );
    }
}
