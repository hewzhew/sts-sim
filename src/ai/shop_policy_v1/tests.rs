use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, ShopPolicyActionV1, ShopPolicyClassV1,
    ShopPolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::state::run::RunState;
use crate::state::shop::{ShopCard, ShopState};

#[test]
fn shop_policy_purges_visible_curse() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purge {
            card: CardId::Doubt,
            ..
        }
    ));
    assert!(decision
        .context
        .candidates
        .iter()
        .any(|candidate| candidate.class == ShopPolicyClassV1::CursePurge));
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(
        &decision.to_noncombat_decision_record_v1(),
    )
    .expect("shop policy record should validate");
}

#[test]
fn shop_policy_does_not_purge_starter_when_purchase_competes() {
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
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(matches!(decision.action, ShopPolicyActionV1::Stop { .. }));
}
