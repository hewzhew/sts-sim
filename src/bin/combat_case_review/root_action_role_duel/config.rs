use sts_simulator::ai::combat_search_v2::{
    CombatSearchActionPriorPluginId, CombatSearchV2Config, CombatSearchV2PotionPolicy,
};

use super::super::options::ReviewOptions;
use super::super::search_intervention::ReviewSearchIntervention;
use super::super::search_runner::review_search_profile;

pub(super) fn duel_search_config(
    options: &ReviewOptions,
    label: &'static str,
) -> CombatSearchV2Config {
    let profile = review_search_profile(label, options.slow_nodes, options.slow_ms, options)
        .with_action_prior_plugin(CombatSearchActionPriorPluginId::KeyCardOnline)
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(options.diagnostic_potion_max);
    ReviewSearchIntervention::default()
        .with_input_label(label)
        .apply_to_profile(profile)
}
