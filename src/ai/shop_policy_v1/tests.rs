use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, ShopPolicyActionV1, ShopPolicyClassV1,
    ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use crate::state::shop::{ShopCard, ShopRelic, ShopState};

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

#[test]
fn shop_policy_converts_high_gold_into_affordable_relic_even_below_old_high_impact_threshold() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 146,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(context.conversion_pressure);
    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Relic {
                relic: RelicId::Anchor,
                ..
            },
            ..
        }
    ));
}
