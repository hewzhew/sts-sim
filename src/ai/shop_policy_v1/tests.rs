use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, shop_card_conversion_priority_v1,
    ShopPolicyActionV1, ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::map::{MapEdge, MapRoomNode, MapState, RoomType};
use crate::state::run::RunState;
use crate::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

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
    let mut bloated = RunState::new(1, 0, false, "Ironclad");
    bloated.act_num = 3;
    bloated.floor_num = 46;
    add_deck_bloat(&mut bloated, 34);

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

#[test]
fn shop_policy_buys_elite_potion_when_first_elite_prep_window_is_open() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 4;
    run_state.gold = 100;
    install_current_room_route(
        &mut run_state,
        RoomType::ShopRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoomElite],
    );
    let mut shop = ShopState::new();
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Potion {
                potion: PotionId::FirePotion,
                ..
            },
            ..
        }
    ));
}

fn add_deck_bloat(run_state: &mut RunState, count: usize) {
    for _ in 0..count {
        run_state.add_card_to_deck(CardId::Strike);
    }
}

fn install_current_room_route(
    run_state: &mut RunState,
    current_room: RoomType,
    future_rooms: &[RoomType],
) {
    let mut graph = Vec::new();
    let mut current = map_node(0, 0, current_room);
    if !future_rooms.is_empty() {
        current.edges.insert(MapEdge::new(0, 0, 0, 1));
    }
    graph.push(vec![current]);
    for (idx, room) in future_rooms.iter().enumerate() {
        let y = idx as i32 + 1;
        let mut node = map_node(0, y, *room);
        if idx + 1 < future_rooms.len() {
            node.edges.insert(MapEdge::new(0, y, 0, y + 1));
        }
        graph.push(vec![node]);
    }
    run_state.map = MapState::new(graph);
    run_state.map.current_x = 0;
    run_state.map.current_y = 0;
}

fn map_node(x: i32, y: i32, room_type: RoomType) -> MapRoomNode {
    let mut node = MapRoomNode::new(x, y);
    node.class = Some(room_type);
    node
}
