use crate::ai::route_planner_v1::{
    plan_route_decision_v1, RoutePlannerConfigV1, RouteSafetyFlagV1,
};
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::CombatCard;
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::RunState;

use super::fixtures::{
    candidate_by_room, run_with_start_nodes, run_with_start_paths, selected_candidate,
};

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

#[test]
fn route_path_summary_tracks_first_elite_preparation_window() {
    let run = run_with_start_paths(&[&[
        RoomType::MonsterRoom,
        RoomType::EventRoom,
        RoomType::RestRoom,
        RoomType::MonsterRoomElite,
    ]]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let candidate = selected_candidate(&trace);
    let first_elite = &candidate.path_summary.first_elite;

    assert!(first_elite.forced);
    assert!(!first_elite.optional);
    assert_eq!(first_elite.min_hallway_fights_before, 1);
    assert_eq!(first_elite.max_hallway_fights_before, 1);
    assert_eq!(first_elite.min_unknowns_before, 1);
    assert_eq!(first_elite.max_unknowns_before, 1);
    assert!(first_elite.can_bail_to_rest_before);
    assert!(!first_elite.can_bail_to_shop_before);
}

#[test]
fn route_score_exposes_first_elite_preparation_as_its_own_term() {
    let run = run_with_start_paths(&[
        &[RoomType::MonsterRoom, RoomType::MonsterRoomElite],
        &[
            RoomType::MonsterRoom,
            RoomType::MonsterRoom,
            RoomType::RestRoom,
            RoomType::MonsterRoomElite,
        ],
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let immediate_elite_path = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include immediate elite path");
    let prepared_elite_path = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 1)
        .expect("trace should include prepared elite path");

    assert!(
        prepared_elite_path.score_terms.elite_prep > immediate_elite_path.score_terms.elite_prep,
        "route scoring should expose first-elite preparation separately from the total score"
    );
}

#[test]
fn cursed_key_makes_treasure_route_curse_debt_visible() {
    let no_key = run_with_start_nodes(&[RoomType::TreasureRoom], Some(RoomType::RestRoom));
    let mut cursed_key = no_key.clone();
    cursed_key.relics.push(RelicState::new(RelicId::CursedKey));

    let no_key_trace = plan_route_decision_v1(
        &no_key,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let cursed_key_trace = plan_route_decision_v1(
        &cursed_key,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let no_key_treasure = candidate_by_room(&no_key_trace, RoomType::TreasureRoom);
    let cursed_key_treasure = candidate_by_room(&cursed_key_trace, RoomType::TreasureRoom);

    assert_eq!(no_key_treasure.score_terms.curse_debt, 0.0);
    assert_eq!(cursed_key_treasure.features.expected_curse_debt, 1.0);
    assert!(
        cursed_key_treasure.score_terms.curse_debt < 0.0,
        "Cursed Key treasure should expose a negative curse debt term"
    );
    assert!(
        cursed_key_treasure
            .cautions
            .iter()
            .any(|caution| caution.contains("Cursed Key chest curse debt")),
        "route trace should explain the Cursed Key treasure debt"
    );
}

#[test]
fn rest_after_forced_first_elite_does_not_count_as_elite_bailout() {
    let run = run_with_start_paths(&[&[
        RoomType::MonsterRoom,
        RoomType::MonsterRoomElite,
        RoomType::RestRoom,
    ]]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let candidate = selected_candidate(&trace);

    assert_eq!(
        candidate.path_summary.first_elite.max_hallway_fights_before,
        1
    );
    assert!(!candidate.path_summary.first_elite.can_bail_to_rest_before);
    assert!(!candidate.path_summary.first_elite.can_bail_to_shop_before);
    assert_eq!(
        candidate.safety,
        RouteSafetyFlagV1::RejectUnlessNoAlternative
    );
}

#[test]
fn act1_elite_need_penalizes_unprepared_starter_deck_even_at_high_hp() {
    let mut starter =
        run_with_start_nodes(&[RoomType::MonsterRoomElite], Some(RoomType::MonsterRoom));
    starter.floor_num = 5;
    starter.current_hp = starter.max_hp;

    let starter_trace = plan_route_decision_v1(
        &starter,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let starter_needs = &selected_candidate(&starter_trace).needs;

    assert!(
        starter_needs.can_take_elite < 0.65,
        "high HP alone should not make an unprepared Act1 starter deck look elite-ready"
    );
}

#[test]
fn act1_elite_need_rewards_frontload_and_sentries_coverage() {
    let mut weak = run_with_start_nodes(&[RoomType::MonsterRoomElite], Some(RoomType::MonsterRoom));
    weak.floor_num = 5;
    weak.current_hp = weak.max_hp;

    let mut prepared = weak.clone();
    replace_master_deck(
        &mut prepared,
        &[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Cleave,
            CardId::PommelStrike,
            CardId::Clothesline,
        ],
    );

    let weak_trace = plan_route_decision_v1(
        &weak,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let prepared_trace = plan_route_decision_v1(
        &prepared,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let weak_elite = selected_candidate(&weak_trace).needs.can_take_elite;
    let prepared_elite = selected_candidate(&prepared_trace).needs.can_take_elite;

    assert!(
        prepared_elite > weak_elite + 0.25,
        "Act1 elite readiness should distinguish real combat preparation from raw HP"
    );
    assert!(
        prepared_elite > 0.75,
        "frontload plus AoE/weak coverage should make the route layer more willing to fight elites"
    );
}

fn replace_master_deck(run: &mut RunState, cards: &[CardId]) {
    run.master_deck = cards
        .iter()
        .enumerate()
        .map(|(idx, &card_id)| CombatCard::new(card_id, idx as u32))
        .collect();
}
