use sts_simulator::ai::combat_search_v2::{
    run_combat_line_lab_from_parent_v0, run_combat_line_lab_v0, CombatLineLabReport,
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2TrajectoryReport,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::options::ReviewOptions;
use super::search_runner::review_search_profile;

pub(super) fn run_line_lab(
    options: &ReviewOptions,
    case: &CombatCase,
    parent: Option<&CombatSearchV2TrajectoryReport>,
) -> Option<CombatLineLabReport> {
    if !options.line_lab {
        return None;
    }
    let config = line_lab_search_config(options);
    Some(match parent {
        Some(parent) => run_combat_line_lab_from_parent_v0(
            &case.position,
            parent,
            config,
            options.line_lab_ms,
            options.line_lab_cuts,
        ),
        None => run_combat_line_lab_v0(
            &case.position,
            config,
            options.line_lab_ms,
            options.line_lab_cuts,
        ),
    })
}

fn line_lab_search_config(options: &ReviewOptions) -> CombatSearchV2Config {
    review_search_profile("line_lab", options.slow_nodes, options.line_lab_ms, options)
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(options.diagnostic_potion_max)
        .to_config()
}
