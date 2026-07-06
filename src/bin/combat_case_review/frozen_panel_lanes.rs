use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::focus::{review_focus, CombatReviewFocus};
use super::key_card_lifecycle::{key_card_lifecycle, KeyCardLifecycleReport};
use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct FrozenPanelLaneReview {
    schema: &'static str,
    contract: &'static str,
    lanes: Vec<FrozenPanelLaneResult>,
}

#[derive(Serialize)]
struct FrozenPanelLaneResult {
    lane: &'static str,
    search_config_summary: FrozenPanelLaneConfigSummary,
    review: SearchReview,
    focus: Option<CombatReviewFocus>,
    key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct FrozenPanelLaneConfigSummary {
    max_nodes: usize,
    wall_ms: u64,
    turn_plan_policy: &'static str,
    potion_policy: &'static str,
    max_potions_used: u32,
    rollout_policy: &'static str,
    child_rollout_policy: &'static str,
    setup_bias_policy: &'static str,
    phase_guard_policy: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenPanelLaneSpec {
    pub(crate) lane: &'static str,
    pub(crate) setup_bias_policy: CombatSearchV2SetupBiasPolicy,
}

impl FrozenPanelLaneSpec {
    fn config_summary(
        self,
        options: &ReviewOptions,
        rollout_policy: CombatSearchV2RolloutPolicy,
    ) -> FrozenPanelLaneConfigSummary {
        FrozenPanelLaneConfigSummary {
            max_nodes: options.slow_nodes,
            wall_ms: options.slow_ms,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly.label(),
            potion_policy: "all",
            max_potions_used: options.diagnostic_potion_max,
            rollout_policy: rollout_policy.label(),
            child_rollout_policy: options.child_rollout_policy().label(),
            setup_bias_policy: self.setup_bias_policy.label(),
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
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::Default,
        },
        FrozenPanelLaneSpec {
            lane: "key_setup_bias",
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
        },
    ]
}

pub(super) fn run_frozen_panel_lanes(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<FrozenPanelLaneReview> {
    if !options.frozen_panel_lanes {
        return None;
    }

    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let lanes = frozen_panel_lane_specs()
        .into_iter()
        .map(|spec| {
            let summary = spec.config_summary(options, rollout_policy);
            let (review, _) = run_configured_search(
                spec.lane,
                case,
                CombatSearchV2Config {
                    max_nodes: options.slow_nodes,
                    wall_time: Some(Duration::from_millis(options.slow_ms)),
                    turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                    potion_policy: CombatSearchV2PotionPolicy::All,
                    max_potions_used: Some(options.diagnostic_potion_max),
                    rollout_policy,
                    child_rollout_policy: options.child_rollout_policy(),
                    setup_bias_policy: spec.setup_bias_policy,
                    phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
                    ..CombatSearchV2Config::default()
                },
                options.action_preview_limit,
            );
            let focus = review_focus(std::slice::from_ref(&review));
            let key_card_lifecycle = key_card_lifecycle(&case.position, focus.as_ref());
            FrozenPanelLaneResult {
                lane: spec.lane,
                search_config_summary: summary,
                review,
                focus,
                key_card_lifecycle,
            }
        })
        .collect();

    Some(FrozenPanelLaneReview {
        schema: "frozen_panel_lanes_v0a",
        contract: "review_only_three_case_panel_lanes_no_runner_policy_change",
        lanes,
    })
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::combat_search_v2::CombatSearchV2SetupBiasPolicy;

    use super::frozen_panel_lane_specs;

    #[test]
    fn frozen_panel_lane_specs_only_differ_by_setup_bias() {
        let specs = frozen_panel_lane_specs();

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].lane, "baseline");
        assert_eq!(
            specs[0].setup_bias_policy,
            CombatSearchV2SetupBiasPolicy::Default
        );
        assert_eq!(specs[1].lane, "key_setup_bias");
        assert_eq!(
            specs[1].setup_bias_policy,
            CombatSearchV2SetupBiasPolicy::KeyCardOnline
        );
        assert_eq!(
            specs[0].config_summary_without_setup_bias(),
            specs[1].config_summary_without_setup_bias()
        );
    }
}
