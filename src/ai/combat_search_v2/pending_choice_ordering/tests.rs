use super::*;
use crate::runtime::combat::CombatCard;
use crate::state::selection::{SelectionResolution, SelectionScope};
use crate::test_support::blank_test_combat;

fn hand_select(uuids: impl IntoIterator<Item = u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(SelectionScope::Hand, uuids))
}

fn grid_select(uuids: impl IntoIterator<Item = u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(SelectionScope::Grid, uuids))
}

#[test]
fn move_to_draw_prefers_higher_value_card() {
    let mut combat = blank_test_combat();
    combat.zones.discard_pile = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Carnage, 20),
    ];
    let engine = EngineState::PendingChoice(PendingChoice::GridSelect {
        source_pile: PileType::Discard,
        candidate_uuids: vec![10, 20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: GridSelectReason::MoveToDrawPile,
    });

    let strike = pending_choice_ordering_hint(&engine, &combat, &grid_select([10]))
        .expect("strike candidate should rank");
    let carnage = pending_choice_ordering_hint(&engine, &combat, &grid_select([20]))
        .expect("carnage candidate should rank");

    assert!(carnage.primary > strike.primary);
}

#[test]
fn upgrade_selection_prefers_higher_upgrade_delta() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Bash, 20),
    ];
    let engine = EngineState::PendingChoice(PendingChoice::HandSelect {
        candidate_uuids: vec![10, 20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: HandSelectReason::Upgrade,
    });

    let strike = pending_choice_ordering_hint(&engine, &combat, &hand_select([10]))
        .expect("strike upgrade candidate should rank");
    let bash = pending_choice_ordering_hint(&engine, &combat, &hand_select([20]))
        .expect("bash upgrade candidate should rank");

    assert!(bash.primary > strike.primary);
}

#[test]
fn scry_discard_prefers_status_over_empty_selection() {
    let combat = blank_test_combat();
    let engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![CardId::Slimed, CardId::Bash],
        card_uuids: vec![10, 20],
    });

    let keep_all =
        pending_choice_ordering_hint(&engine, &combat, &ClientInput::SubmitScryDiscard(vec![]))
            .expect("empty scry discard should rank");
    let discard_slimed =
        pending_choice_ordering_hint(&engine, &combat, &ClientInput::SubmitScryDiscard(vec![0]))
            .expect("slimed scry discard should rank");

    assert!(discard_slimed.primary > keep_all.primary);
    assert_eq!(
        discard_slimed.role,
        PendingChoiceOrderingRole::RemovalSelection
    );
}

#[test]
fn cancel_is_explicitly_low_priority_but_still_ranked() {
    let combat = blank_test_combat();
    let engine = EngineState::PendingChoice(PendingChoice::DiscoverySelect(
        crate::state::core::DiscoveryChoiceState {
            cards: vec![CardId::Carnage],
            colorless: false,
            card_type: None,
            amount: 1,
            can_skip: true,
        },
    ));

    let cancel = pending_choice_ordering_hint(&engine, &combat, &ClientInput::Cancel)
        .expect("cancel should rank");
    let pick =
        pending_choice_ordering_hint(&engine, &combat, &ClientInput::SubmitDiscoverChoice(0))
            .expect("pick should rank");

    assert_eq!(cancel.role, PendingChoiceOrderingRole::Cancel);
    assert!(pick.primary > cancel.primary);
}
