use super::*;
use crate::content::cards::CardId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatCard;
use crate::test_support::blank_test_combat;

#[test]
fn duplicate_active_uuid_is_reported() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 10),
    ];

    let summary = summarize_card_identity(&combat);
    let mut collector = CardIdentityDiagnosticsCollector::default();
    collector.observe(&summary);
    let report = collector.finish();

    assert_eq!(report.states_with_duplicate_active_uuid, 1);
    assert_eq!(report.states_with_uuid_card_id_conflict, 1);
    assert_eq!(report.largest_conflict_groups[0].uuid, 10);
    assert_eq!(report.largest_conflict_groups[0].occurrence_count, 2);
}

#[test]
fn action_payload_placeholder_is_not_active_duplicate() {
    let mut combat = blank_test_combat();
    combat
        .engine
        .action_queue
        .push_back(Action::MakeCopyInHand {
            original: Box::new(CombatCard::new(CardId::Strike, 0)),
            amount: 1,
        });

    let summary = summarize_card_identity(&combat);
    let mut collector = CardIdentityDiagnosticsCollector::default();
    collector.observe(&summary);
    let report = collector.finish();

    assert_eq!(report.states_with_duplicate_active_uuid, 0);
    assert_eq!(report.action_payload_cards_observed, 1);
    assert_eq!(report.action_payload_placeholder_cards, 1);
}
