use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2Config, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;

use super::super::key_card_counterfactual::{move_key_card, KeyCardCounterfactualPlacement};
use super::super::key_card_lifecycle::KeyCardReason;
use super::super::options::ReviewOptions;
use super::digest::{candidates_before_target, selected_candidate, target_candidate};
use super::types::KeyCardDecisionMicroscopeVariant;

pub(super) fn run_variant(
    options: &ReviewOptions,
    original_case: &CombatCase,
    card: &CombatCard,
    reason: KeyCardReason,
) -> KeyCardDecisionMicroscopeVariant {
    let mut case = original_case.clone();
    let placement = KeyCardCounterfactualPlacement::OpeningHand;
    if move_key_card(&mut case.position.combat, card.uuid, placement).is_none() {
        return skipped_variant(card, reason, placement, "card_not_in_active_combat_zones");
    }
    let Some(card_index) = case
        .position
        .combat
        .zones
        .hand
        .iter()
        .position(|hand_card| hand_card.uuid == card.uuid)
    else {
        return skipped_variant(card, reason, placement, "card_not_in_opening_hand");
    };

    let microscope = explain_combat_search_v2_initial_decision(
        &case.position.engine,
        &case.position.combat,
        microscope_config(options, card),
    );
    let target_candidate = target_candidate(&microscope, card_index);
    let candidates_before_target = candidates_before_target(&microscope, &target_candidate);
    let selected_candidate = selected_candidate(&microscope);

    KeyCardDecisionMicroscopeVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: None,
        target_candidate,
        candidates_before_target,
        selected_candidate,
        microscope: Some(microscope),
    }
}

fn skipped_variant(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
    skipped_reason: &'static str,
) -> KeyCardDecisionMicroscopeVariant {
    KeyCardDecisionMicroscopeVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: Some(skipped_reason),
        target_candidate: None,
        candidates_before_target: Vec::new(),
        selected_candidate: None,
        microscope: None,
    }
}

fn microscope_config(options: &ReviewOptions, card: &CombatCard) -> CombatSearchV2Config {
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    CombatSearchV2Config {
        max_nodes: options.slow_nodes,
        wall_time: Some(Duration::from_millis(options.slow_ms)),
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(options.diagnostic_potion_max),
        rollout_policy,
        child_rollout_policy: options.child_rollout_policy(),
        setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
        input_label: Some(format!(
            "key_card_decision_microscope:{}#{}",
            java_id(card.id),
            card.uuid
        )),
        ..CombatSearchV2Config::default()
    }
}
