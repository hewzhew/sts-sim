use crate::ai::route_planner_v1::{
    plan_route_decision_v1, RoutePlannerConfigV1, RouteSafetyFlagV1,
};
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;

use super::fixtures::{candidate_by_room, run_with_start_nodes, selected_candidate};

#[test]
fn route_planner_can_gate_obvious_forced_elite_risk() {
    let mut run = run_with_start_nodes(&[RoomType::MonsterRoomElite], Some(RoomType::MonsterRoom));
    run.current_hp = 1;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    assert!(trace
        .candidates
        .iter()
        .any(|candidate| candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative));
}

#[test]
fn route_planner_prefers_early_monster_over_question_and_low_value_shop() {
    let mut run = run_with_start_nodes(
        &[
            RoomType::MonsterRoom,
            RoomType::EventRoom,
            RoomType::ShopRoom,
        ],
        Some(RoomType::RestRoom),
    );
    run.gold = 80;

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let selected = selected_candidate(&trace);

    assert_eq!(
        selected.target.room_type,
        Some(RoomType::MonsterRoom),
        "Act 1 opening data-collection route should prefer an easy-pool combat over early ?/low-gold shop"
    );
}

#[test]
fn route_planner_shop_need_increases_with_gold() {
    let mut low_gold = run_with_start_nodes(
        &[RoomType::MonsterRoom, RoomType::ShopRoom],
        Some(RoomType::RestRoom),
    );
    low_gold.gold = 80;
    let mut high_gold = low_gold.clone();
    high_gold.gold = 250;

    let low = plan_route_decision_v1(
        &low_gold,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let high = plan_route_decision_v1(
        &high_gold,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let low_shop = candidate_by_room(&low, RoomType::ShopRoom);
    let high_shop = candidate_by_room(&high, RoomType::ShopRoom);

    assert!(
        high_shop.needs.need_shop > low_shop.needs.need_shop,
        "higher gold should raise the route-level shop need"
    );
    assert!(
        high_shop.score_terms.shop > low_shop.score_terms.shop,
        "higher shop need should flow into decomposed score terms"
    );
}

#[test]
fn route_planner_question_room_uses_mixed_unknown_belief() {
    let mut run = run_with_start_nodes(&[RoomType::EventRoom], Some(RoomType::RestRoom));
    run.event_generator.monster_chance = 0.25;
    run.event_generator.shop_chance = 0.10;
    run.event_generator.treasure_chance = 0.05;

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let question = selected_candidate(&trace);

    assert_eq!(question.target.room_type, Some(RoomType::EventRoom));
    assert_eq!(trace.context.counters.unknown_belief.monster_chance, 0.25);
    assert_eq!(trace.context.counters.unknown_belief.shop_chance, 0.10);
    assert_eq!(trace.context.counters.unknown_belief.treasure_chance, 0.05);
    assert!(
        question.features.expected_card_rewards > 0.0
            && question.features.shop_access > 0.0
            && question.features.expected_relics > 0.0
            && question.features.event_access > 0.0,
        "? rooms must be scored as mixed outcomes, not pure events"
    );
}
