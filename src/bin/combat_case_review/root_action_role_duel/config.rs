use sts_simulator::ai::combat_search_v2::CombatSearchV2Config;

use super::super::options::ReviewOptions;
use super::super::search_intervention::ReviewSearchIntervention;
use super::super::search_runner::review_key_setup_profile;

pub(super) fn duel_search_config(
    options: &ReviewOptions,
    label: &'static str,
) -> CombatSearchV2Config {
    let profile = review_key_setup_profile(label, options.slow_nodes, options.slow_ms, options);
    ReviewSearchIntervention::default()
        .with_input_label(label)
        .apply_to_profile(profile)
}
