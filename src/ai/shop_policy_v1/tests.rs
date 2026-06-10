use crate::ai::shop_policy_v1::{build_shop_decision_context_v1, ShopPolicyClassV1};
use crate::content::cards::CardId;
use crate::state::run::RunState;
use crate::state::shop::{ShopCard, ShopState};

#[test]
fn shop_context_exposes_visible_curse_purge_candidate() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == ShopPolicyClassV1::CursePurge));
}

#[test]
fn shop_context_exposes_purchase_candidates_without_selecting_policy() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck(CardId::Inflame);
    run_state.add_card_to_deck(CardId::HeavyBlade);
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::PommelStrike,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);

    assert!(context
        .candidates
        .iter()
        .any(|candidate| { candidate.label.contains("Pommel") }));
}
