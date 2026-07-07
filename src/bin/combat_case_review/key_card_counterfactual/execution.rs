use sts_simulator::ai::combat_search_v2::CombatSearchProfile;
use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;

use super::super::focus::review_focus;
use super::super::key_card_lifecycle::{key_card_lifecycle, KeyCardReason};
use super::super::options::ReviewOptions;
use super::super::search_runner::{review_key_setup_profile, run_profile_search};
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
    let (search, _) = run_profile_search(
        &case,
        counterfactual_search_profile(options, placement.search_label()),
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

fn counterfactual_search_profile(
    options: &ReviewOptions,
    label: &'static str,
) -> CombatSearchProfile {
    review_key_setup_profile(label, options.slow_nodes, options.slow_ms, options)
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
