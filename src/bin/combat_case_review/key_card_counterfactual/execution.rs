use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;

use super::super::focus::review_focus;
use super::super::key_card_lifecycle::{key_card_lifecycle, KeyCardReason};
use super::super::options::ReviewOptions;
use super::super::search_runner::run_configured_search;
use super::movement::move_key_card;
use super::types::{KeyCardCounterfactualPlacement, KeyCardCounterfactualVariant};

pub(super) fn run_key_card_variant(
    options: &ReviewOptions,
    original_case: &CombatCase,
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
) -> KeyCardCounterfactualVariant {
    let mut case = original_case.clone();
    if move_key_card(&mut case.position.combat, card.uuid, placement).is_none() {
        return skipped_variant(card, reason, placement, "card_not_in_active_combat_zones");
    }
    sync_combat_summary(&mut case);
    let (search, _) = run_configured_search(
        placement.search_label(),
        &case,
        counterfactual_search_config(options),
        options.action_preview_limit,
    );
    let focus = review_focus(std::slice::from_ref(&search));
    let key_card_lifecycle = key_card_lifecycle(&case.position, focus.as_ref());
    KeyCardCounterfactualVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: None,
        search: Some(search),
        focus,
        key_card_lifecycle,
    }
}

fn skipped_variant(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
    skipped_reason: &'static str,
) -> KeyCardCounterfactualVariant {
    KeyCardCounterfactualVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: Some(skipped_reason),
        search: None,
        focus: None,
        key_card_lifecycle: None,
    }
}

fn counterfactual_search_config(options: &ReviewOptions) -> CombatSearchV2Config {
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
        ..CombatSearchV2Config::default()
    }
}

fn sync_combat_summary(case: &mut CombatCase) {
    case.combat.hand = case
        .position
        .combat
        .zones
        .hand
        .iter()
        .map(sts_simulator::eval::combat_case::card_summary)
        .collect();
    case.combat.draw_count = case.position.combat.zones.draw_pile.len();
    case.combat.discard_count = case.position.combat.zones.discard_pile.len();
    case.combat.exhaust_count = case.position.combat.zones.exhaust_pile.len();
}
