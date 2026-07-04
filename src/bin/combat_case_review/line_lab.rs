use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    run_combat_line_lab_from_parent_v0, run_combat_line_lab_v0, CombatLineLabReport,
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2TrajectoryReport, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::search_runner::review_child_rollout_policy;
use super::Args;

pub(super) fn run_line_lab(
    args: &Args,
    case: &CombatCase,
    parent: Option<&CombatSearchV2TrajectoryReport>,
) -> Option<CombatLineLabReport> {
    if !args.line_lab {
        return None;
    }
    let config = line_lab_search_config(args);
    Some(match parent {
        Some(parent) => run_combat_line_lab_from_parent_v0(
            &case.position,
            parent,
            config,
            args.line_lab_ms,
            args.line_lab_cuts,
        ),
        None => {
            run_combat_line_lab_v0(&case.position, config, args.line_lab_ms, args.line_lab_cuts)
        }
    })
}

fn line_lab_search_config(args: &Args) -> CombatSearchV2Config {
    CombatSearchV2Config {
        max_nodes: args.slow_nodes,
        wall_time: Some(Duration::from_millis(args.line_lab_ms)),
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        child_rollout_policy: review_child_rollout_policy(args),
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(args.diagnostic_potion_max),
        rollout_policy: if args.disable_rollout {
            CombatSearchV2RolloutPolicy::Disabled
        } else {
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        },
        ..Default::default()
    }
}
