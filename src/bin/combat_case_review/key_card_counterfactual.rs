use sts_simulator::eval::combat_case::CombatCase;

use super::key_card_lifecycle::key_card_targets;
use super::options::ReviewOptions;

#[path = "key_card_counterfactual/execution.rs"]
mod execution;
#[path = "key_card_counterfactual/movement.rs"]
mod movement;
#[path = "key_card_counterfactual/types.rs"]
mod types;

pub(super) use movement::move_key_card;
pub(super) use types::{KeyCardCounterfactualPlacement, KeyCardCounterfactualProbe};

use execution::run_key_card_variant;

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
