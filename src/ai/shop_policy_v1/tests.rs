use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, shop_card_conversion_priority_v1,
    ShopPolicyActionV1, ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
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

#[test]
fn shop_card_priority_penalizes_transition_cards_when_deck_is_bloated() {
    let mut compact = RunState::new(1, 0, false, "Ironclad");
    compact.act_num = 3;
    compact.floor_num = 46;

    let mut bloated = compact.clone();
    add_deck_bloat(&mut bloated, 34);

    assert!(
        shop_card_conversion_priority_v1(CardId::PommelStrike, &compact)
            >= ShopPolicyConfigV1::default().high_impact_card_purchase_priority_threshold
    );
    assert!(
        shop_card_conversion_priority_v1(CardId::PommelStrike, &bloated) <= 0,
        "large decks should not keep treating a normal transition card as shop conversion"
    );
}

#[test]
fn shop_card_priority_preserves_boss_patch_cards_when_deck_is_bloated() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    add_deck_bloat(&mut run_state, 34);

    assert!(
        shop_card_conversion_priority_v1(CardId::Shockwave, &run_state)
            >= ShopPolicyConfigV1::default().high_impact_card_purchase_priority_threshold,
        "deck bloat pressure should not block a clear boss/elite answer"
    );
}

#[test]
fn shop_policy_does_not_convert_gold_into_transition_card_when_deck_is_bloated() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    add_deck_bloat(&mut run_state, 34);
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

    assert!(context.conversion_pressure);
    assert!(
        !matches!(
            decision.action,
            ShopPolicyActionV1::Purchase {
                target: ShopPurchaseTargetV1::Card {
                    card: CardId::PommelStrike,
                    ..
                },
                ..
            }
        ),
        "conversion pressure should not force ordinary card bloat when the deck is already oversized"
    );
}

fn add_deck_bloat(run_state: &mut RunState, count: usize) {
    for _ in 0..count {
        run_state.add_card_to_deck(CardId::Strike);
    }
}
