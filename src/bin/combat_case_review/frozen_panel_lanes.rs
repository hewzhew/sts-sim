use sts_simulator::ai::combat_search_v2::CombatSearchV2RolloutPolicy;
use sts_simulator::eval::combat_case::CombatCase;

use super::options::ReviewOptions;

#[path = "frozen_panel_lanes/execution.rs"]
mod execution;
#[path = "frozen_panel_lanes/specs.rs"]
mod specs;
#[path = "frozen_panel_lanes/types.rs"]
mod types;

pub(super) use specs::frozen_panel_lane_specs;
pub(super) use types::FrozenPanelLaneReview;

use execution::run_frozen_panel_lane;

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
        .map(|spec| run_frozen_panel_lane(options, case, spec, rollout_policy))
        .collect();

    Some(FrozenPanelLaneReview {
        schema: "frozen_panel_lanes_v0a",
        contract: "review_only_three_case_panel_lanes_no_runner_policy_change",
        lanes,
    })
}
