use super::super::value::combat_eval_from_rollout_estimate;
use super::super::*;

pub(super) fn adaptive_no_potion_rollout_policy(node: &SearchNode) -> CombatSearchV2RolloutPolicy {
    let profile = combat_search_phase_profile(&node.engine, &node.combat);
    if profile.enemy_mechanics.guardian_open_count > 0
        || profile.enemy_mechanics.guardian_defensive_count > 0
        || profile.enemy_mechanics.bronze_automaton_count > 0
        || profile.enemy_mechanics.bronze_orb_count > 0
    {
        CombatSearchV2RolloutPolicy::PhaseAwareNoPotion
    } else {
        CombatSearchV2RolloutPolicy::ConservativeNoPotion
    }
}

pub(super) fn better_rollout_estimate(
    left: RolloutNodeEstimate,
    right: RolloutNodeEstimate,
) -> RolloutNodeEstimate {
    let left_eval = combat_eval_from_rollout_estimate(&left);
    let right_eval = combat_eval_from_rollout_estimate(&right);
    if right_eval > left_eval {
        right
    } else {
        left
    }
}
