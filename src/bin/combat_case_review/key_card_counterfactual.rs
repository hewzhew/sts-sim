use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::{CombatCard, CombatState};

use super::focus::{review_focus, CombatReviewFocus};
use super::key_card_lifecycle::{
    key_card_lifecycle, key_card_targets, KeyCardLifecycleReport, KeyCardReason,
};
use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct KeyCardCounterfactualProbe {
    schema: &'static str,
    contract: &'static str,
    skipped_reason: Option<&'static str>,
    variants: Vec<KeyCardCounterfactualVariant>,
}

#[derive(Serialize)]
struct KeyCardCounterfactualVariant {
    card: String,
    uuid: u32,
    reason: &'static str,
    placement: &'static str,
    skipped_reason: Option<&'static str>,
    search: Option<SearchReview>,
    focus: Option<CombatReviewFocus>,
    key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

#[derive(Clone, Copy)]
pub(super) enum KeyCardCounterfactualPlacement {
    OpeningHand,
    DrawTop,
}

impl KeyCardCounterfactualPlacement {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::OpeningHand => "opening_hand",
            Self::DrawTop => "draw_top",
        }
    }

    fn search_label(self) -> &'static str {
        match self {
            Self::OpeningHand => "key_card_counterfactual_opening_hand",
            Self::DrawTop => "key_card_counterfactual_draw_top",
        }
    }
}

pub(super) fn run_key_card_counterfactual_probe(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<KeyCardCounterfactualProbe> {
    if !options.key_card_counterfactual {
        return None;
    }
    let targets = key_card_targets(&case.position.combat);
    if targets.is_empty() {
        return Some(KeyCardCounterfactualProbe {
            schema: "key_card_counterfactual_probe_v0",
            contract:
                "diagnostic_only_mutate_key_card_position_then_search_no_runner_policy_change",
            skipped_reason: Some("no_key_cards"),
            variants: Vec::new(),
        });
    }

    let mut variants = Vec::new();
    for target in targets {
        for placement in [
            KeyCardCounterfactualPlacement::OpeningHand,
            KeyCardCounterfactualPlacement::DrawTop,
        ] {
            variants.push(run_key_card_variant(
                options,
                case,
                &target.card,
                target.reason,
                placement,
            ));
        }
    }

    Some(KeyCardCounterfactualProbe {
        schema: "key_card_counterfactual_probe_v0",
        contract: "diagnostic_only_mutate_key_card_position_then_search_no_runner_policy_change",
        skipped_reason: None,
        variants,
    })
}

fn run_key_card_variant(
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
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let (search, _) = run_configured_search(
        placement.search_label(),
        &case,
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
        },
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

pub(super) fn move_key_card(
    combat: &mut CombatState,
    uuid: u32,
    placement: KeyCardCounterfactualPlacement,
) -> Option<()> {
    if matches!(placement, KeyCardCounterfactualPlacement::OpeningHand)
        && combat.zones.hand.iter().any(|card| card.uuid == uuid)
    {
        return Some(());
    }
    if matches!(placement, KeyCardCounterfactualPlacement::DrawTop)
        && combat
            .zones
            .draw_pile
            .first()
            .is_some_and(|card| card.uuid == uuid)
    {
        return Some(());
    }

    let card = take_card_by_uuid(combat, uuid)?;
    match placement {
        KeyCardCounterfactualPlacement::OpeningHand => combat.zones.hand.push(card),
        KeyCardCounterfactualPlacement::DrawTop => combat.zones.add_to_draw_pile_top(card),
    }
    Some(())
}

fn take_card_by_uuid(combat: &mut CombatState, uuid: u32) -> Option<CombatCard> {
    CombatState::remove_card_by_uuid(&mut combat.zones.hand, uuid)
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.draw_pile, uuid))
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.discard_pile, uuid))
        .or_else(|| CombatState::remove_card_by_uuid(&mut combat.zones.exhaust_pile, uuid))
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::test_support::blank_test_combat;

    #[test]
    fn moves_requested_card_uuid_to_opening_hand() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 1)];
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::DemonForm, 42),
            CombatCard::new(CardId::Defend, 2),
        ];

        move_key_card(&mut combat, 42, KeyCardCounterfactualPlacement::OpeningHand)
            .expect("card should move");

        assert_eq!(combat.zones.hand.last().map(|card| card.uuid), Some(42));
        assert!(!combat.zones.draw_pile.iter().any(|card| card.uuid == 42));
    }

    #[test]
    fn moves_requested_card_uuid_to_draw_top() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::DemonForm, 42)];
        combat.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 2)];

        move_key_card(&mut combat, 42, KeyCardCounterfactualPlacement::DrawTop)
            .expect("card should move");

        assert_eq!(
            combat.zones.draw_pile.first().map(|card| card.uuid),
            Some(42)
        );
        assert!(!combat.zones.hand.iter().any(|card| card.uuid == 42));
    }
}
