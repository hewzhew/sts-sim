pub(super) fn evidence_warnings(invalid_card_identity_observed: bool) -> Vec<&'static str> {
    let mut warnings = vec![
        "unresolved_cannot_be_claimed_better_than_a_complete_baseline",
        "no_stepwise_human_action_agreement_objective",
        "no_llm_control_path",
        "combat_only_runner_does_not_validate_out_of_combat_strategy_quality",
        "default_potion_policy_disables_potions_until_a_real_potion_option_planner_exists",
    ];
    if invalid_card_identity_observed {
        warnings.push(
            "duplicate_active_card_uuid_with_conflicting_card_ids_observed_input_or_rollout_state_invalid_until_investigated",
        );
    }
    warnings
}
