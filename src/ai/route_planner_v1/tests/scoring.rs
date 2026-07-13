use crate::ai::route_planner_v1::{
    plan_route_decision_v1, MapDecisionPacketV1, RoutePlannerConfigV1, RouteProjectionCoverageV1,
    RouteSafetyFlagV1,
};
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::CombatCard;
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::map::node::{MapEdge, MapRoomNode};
use crate::state::map::state::MapState;
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
    let mut very_high_gold_near_boss = low_gold.clone();
    very_high_gold_near_boss.act_num = 2;
    very_high_gold_near_boss.floor_num = 29;
    very_high_gold_near_boss.gold = 800;

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
    let very_high_near_boss = plan_route_decision_v1(
        &very_high_gold_near_boss,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let low_shop = candidate_by_room(&low, RoomType::ShopRoom);
    let high_shop = candidate_by_room(&high, RoomType::ShopRoom);
    let very_high_near_boss_shop = candidate_by_room(&very_high_near_boss, RoomType::ShopRoom);

    assert!(
        high_shop.needs.need_shop > low_shop.needs.need_shop,
        "higher gold should raise the route-level shop need"
    );
    assert!(
        high_shop.score_terms.shop > low_shop.score_terms.shop,
        "higher shop need should flow into decomposed score terms"
    );
    assert!(
        very_high_near_boss_shop.needs.need_shop > high_shop.needs.need_shop,
        "large unconverted gold near the act boss should create stronger route-level shop pressure"
    );
    assert!(
        very_high_near_boss_shop.score_terms.shop > high_shop.score_terms.shop,
        "stronger conversion pressure should flow into route score terms"
    );
}

#[test]
fn high_unconverted_gold_prefers_guaranteed_shop_route_over_optional_shop_access() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.act_num = 3;
    run.floor_num = 32;
    run.current_hp = run.max_hp;
    run.gold = 900;
    run.event_state = None;
    run.map = MapState::new(vec![
        vec![
            linked_node(
                0,
                0,
                RoomType::MonsterRoom,
                &[
                    (0, 1),
                    (1, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (6, 1),
                    (7, 1),
                ],
            ),
            linked_node(1, 0, RoomType::MonsterRoom, &[(8, 1)]),
        ],
        vec![
            linked_node(0, 1, RoomType::MonsterRoom, &[(0, 2)]),
            linked_node(1, 1, RoomType::EventRoom, &[(1, 2)]),
            linked_node(2, 1, RoomType::MonsterRoom, &[(2, 2)]),
            linked_node(3, 1, RoomType::EventRoom, &[(3, 2)]),
            linked_node(4, 1, RoomType::RestRoom, &[(4, 2)]),
            linked_node(5, 1, RoomType::MonsterRoom, &[(5, 2)]),
            linked_node(6, 1, RoomType::ShopRoom, &[(6, 2)]),
            linked_node(7, 1, RoomType::ShopRoom, &[(7, 2)]),
            linked_node(8, 1, RoomType::ShopRoom, &[(8, 2)]),
        ],
        (0..=8)
            .map(|x| linked_node(x, 2, RoomType::RestRoom, &[]))
            .collect(),
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let optional_shop = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include optional-shop route");
    let guaranteed_shop = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 1)
        .expect("trace should include guaranteed-shop route");

    assert_eq!(optional_shop.path_summary.min_shops, 0);
    assert_eq!(optional_shop.path_summary.max_shops, 1);
    assert!(optional_shop.path_summary.path_count > guaranteed_shop.path_summary.path_count);
    assert_eq!(guaranteed_shop.path_summary.min_shops, 1);
    assert_eq!(guaranteed_shop.path_summary.max_shops, 1);
    assert!(
        guaranteed_shop.score_terms.shop > optional_shop.score_terms.shop,
        "guaranteed shop access should be valued above optional shop access when gold is high"
    );
    assert!(
        optional_shop
            .reasons
            .iter()
            .any(|reason| reason.contains("optional shop access")),
        "route trace should distinguish optional shop access"
    );
    assert!(
        guaranteed_shop
            .reasons
            .iter()
            .any(|reason| reason.contains("guaranteed shop access")),
        "route trace should distinguish guaranteed shop access"
    );
    assert_eq!(
        selected_candidate(&trace).target.x,
        1,
        "high unconverted gold should prefer the route that guarantees a shop"
    );
}

#[test]
fn campfire_score_does_not_claim_full_heal_and_upgrade_from_same_access() {
    let mut run = run_with_start_paths(&[&[RoomType::RestRoom, RoomType::MonsterRoom]]);
    run.current_hp = 48;
    run.max_hp = 80;

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let fire = selected_candidate(&trace);
    let independently_additive_value = fire.needs.need_upgrade * fire.value_factors.upgrade_access
        + fire.needs.need_heal * fire.value_factors.heal_access;
    let realized_value = fire.score_terms.upgrade + fire.score_terms.heal;

    assert!(fire.value_factors.upgrade_access > 0.0);
    assert!(fire.value_factors.heal_access > 0.0);
    assert!(
        realized_value < independently_additive_value,
        "one campfire action cannot realize both the full heal and full upgrade value"
    );
}

#[test]
fn late_act_high_gold_prefers_controllable_shop_path_over_no_shop_fire_path() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.act_num = 3;
    run.floor_num = 41;
    run.current_hp = 86;
    run.max_hp = 101;
    run.gold = 405;
    run.event_state = None;
    replace_master_deck(
        &mut run,
        &[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Armaments,
            CardId::PommelStrike,
            CardId::ShrugItOff,
            CardId::Feed,
            CardId::Corruption,
            CardId::Cleave,
            CardId::Cleave,
            CardId::BattleTrance,
        ],
    );
    for card_id in [CardId::Bash, CardId::Armaments, CardId::Corruption] {
        run.master_deck
            .iter_mut()
            .find(|card| card.id == card_id)
            .expect("fixture should contain upgraded card")
            .upgrades = 1;
    }
    run.map = MapState::new(vec![
        vec![
            linked_node(0, 0, RoomType::RestRoom, &[(0, 1)]),
            linked_node(1, 0, RoomType::EventRoom, &[(1, 1)]),
        ],
        vec![
            linked_node(0, 1, RoomType::MonsterRoomElite, &[(0, 2)]),
            linked_node(1, 1, RoomType::ShopRoom, &[(1, 2)]),
        ],
        vec![
            linked_node(0, 2, RoomType::RestRoom, &[]),
            linked_node(1, 2, RoomType::MonsterRoomElite, &[(1, 3)]),
        ],
        vec![linked_node(1, 3, RoomType::RestRoom, &[])],
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let shop_path = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 1)
        .expect("trace should include the controllable shop path");

    assert_eq!(shop_path.path_summary.min_shops, 1);
    assert_eq!(shop_path.path_summary.max_shops, 1);
    assert_eq!(
        selected_candidate(&trace).target.x,
        1,
        "healthy late-act high-gold routing should preserve a controllable shop before the boss"
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
fn route_path_summary_tracks_recovery_pressure_window() {
    let run = run_with_start_paths(&[
        &[RoomType::MonsterRoom, RoomType::RestRoom],
        &[RoomType::RestRoom, RoomType::MonsterRoom],
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let damage_before_recovery = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include monster-first route");
    let recovery_before_damage = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 1)
        .expect("trace should include rest-first route");

    assert_eq!(
        damage_before_recovery
            .path_summary
            .min_damage_rooms_before_recovery,
        1
    );
    assert_eq!(
        damage_before_recovery
            .path_summary
            .paths_with_recovery_before_damage,
        0
    );
    assert_eq!(
        recovery_before_damage
            .path_summary
            .min_damage_rooms_before_recovery,
        0
    );
    assert_eq!(
        recovery_before_damage
            .path_summary
            .paths_with_recovery_before_damage,
        1
    );
}

#[test]
fn route_projection_complete_when_exact_path_count_equals_budget() {
    let run = run_with_start_paths(&[&[RoomType::MonsterRoom, RoomType::RestRoom]]);
    let config = RoutePlannerConfigV1 {
        path_budget: 1,
        ..RoutePlannerConfigV1::default()
    };

    let trace = plan_route_decision_v1(&run, &EngineState::MapNavigation, config);
    let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
    let candidate = selected_candidate(&trace);
    let packet_candidate = &packet.candidates[0];

    assert_eq!(candidate.path_summary.path_count, 1);
    assert!(!candidate.path_summary.path_budget_exhausted);
    assert_eq!(packet_candidate.projection.metadata.observed_path_count, 1);
    assert_eq!(
        packet_candidate.projection.metadata.coverage,
        RouteProjectionCoverageV1::CompleteWithinBudget
    );
}

#[test]
fn route_projection_records_budget_exhaustion_when_visible_dfs_is_cut() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map = MapState::new(vec![
        vec![linked_node(0, 0, RoomType::MonsterRoom, &[(0, 1), (1, 1)])],
        vec![
            linked_node(0, 1, RoomType::RestRoom, &[]),
            linked_node(1, 1, RoomType::ShopRoom, &[]),
        ],
    ]);
    let config = RoutePlannerConfigV1 {
        path_budget: 1,
        ..RoutePlannerConfigV1::default()
    };

    let trace = plan_route_decision_v1(&run, &EngineState::MapNavigation, config);
    let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
    let candidate = selected_candidate(&trace);
    let packet_candidate = &packet.candidates[0];

    assert_eq!(candidate.path_summary.path_count, 1);
    assert!(candidate.path_summary.path_budget_exhausted);
    assert_eq!(
        packet_candidate.projection.metadata.coverage,
        RouteProjectionCoverageV1::PossiblyTruncatedByPathBudget
    );
}

#[test]
fn very_low_hp_rejects_forced_damage_before_recovery() {
    let mut run = run_with_start_paths(&[
        &[RoomType::MonsterRoom, RoomType::RestRoom],
        &[RoomType::RestRoom, RoomType::MonsterRoom],
    ]);
    run.current_hp = 8;
    run.max_hp = 80;

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let damage_before_recovery = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include monster-first route");
    let recovery_before_damage = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 1)
        .expect("trace should include rest-first route");

    assert_eq!(
        damage_before_recovery.safety,
        RouteSafetyFlagV1::RejectUnlessNoAlternative
    );
    assert_ne!(
        recovery_before_damage.safety,
        RouteSafetyFlagV1::RejectUnlessNoAlternative
    );
    assert_eq!(trace.selected_index, Some(0));
    assert_eq!(trace.candidates[0].target.x, 1);
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

#[test]
fn cumulative_hallway_and_elite_pressure_loses_to_campfire_route() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.act_num = 2;
    run.floor_num = 20;
    run.current_hp = 44;
    run.max_hp = 74;
    run.map = MapState::new(vec![
        vec![
            linked_node(0, 0, RoomType::MonsterRoom, &[(0, 1)]),
            linked_node(1, 0, RoomType::RestRoom, &[(1, 1)]),
        ],
        vec![
            linked_node(0, 1, RoomType::MonsterRoomElite, &[]),
            linked_node(1, 1, RoomType::MonsterRoom, &[]),
        ],
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let pressured = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include the hallway-to-elite route");

    assert_eq!(pressured.viability.surviving_path_count, 0);
    assert_eq!(
        pressured
            .viability
            .representative
            .as_ref()
            .expect("candidate should preserve representative risk")
            .cumulative_hp_loss_p90,
        54.0
    );
    assert_eq!(
        pressured.safety,
        RouteSafetyFlagV1::RejectUnlessNoAlternative
    );
    assert_eq!(selected_candidate(&trace).target.x, 1);
}

#[test]
fn full_health_route_still_prices_projected_hp_loss() {
    let mut run = run_with_start_paths(&[&[RoomType::MonsterRoom, RoomType::RestRoom]]);
    run.current_hp = 85;
    run.max_hp = 85;
    replace_master_deck(
        &mut run,
        &[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ],
    );

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let candidate = selected_candidate(&trace);

    assert!(candidate.value_factors.hp_loss_p90 > 0.0);
    assert!(
        candidate.needs.avoid_damage > 0.0,
        "full health should reduce HP conservation pressure, not erase it"
    );
    assert!(candidate.score_terms.hp_loss < 0.0);
}

#[test]
fn near_exhausted_projected_hp_path_is_not_labeled_ok() {
    let mut run = run_with_start_paths(&[
        &[
            RoomType::MonsterRoom,
            RoomType::MonsterRoom,
            RoomType::MonsterRoom,
            RoomType::MonsterRoomElite,
        ],
        &[
            RoomType::EventRoom,
            RoomType::RestRoom,
            RoomType::RestRoom,
            RoomType::RestRoom,
        ],
    ]);
    run.act_num = 2;
    run.floor_num = 17;
    run.current_hp = 85;
    run.max_hp = 85;
    replace_master_deck(
        &mut run,
        &[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Cleave,
            CardId::PommelStrike,
        ],
    );

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let dangerous = trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.x == 0)
        .expect("trace should include the hallway-to-elite path");
    let viability = dangerous
        .viability
        .representative
        .as_ref()
        .expect("candidate should retain its p90 projection");

    assert_eq!(viability.cumulative_hp_loss_p90, 82.0);
    assert_eq!(viability.projected_hp_after_segment, 3.0);
    assert_ne!(
        dangerous.safety,
        RouteSafetyFlagV1::Ok,
        "a p90 suffix that consumes nearly all HP must remain visibly risky"
    );
}

#[test]
fn route_value_uses_one_real_suffix_instead_of_cross_path_maxima() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.current_hp = run.max_hp;
    run.map = MapState::new(vec![
        vec![linked_node(0, 0, RoomType::MonsterRoom, &[(0, 1), (1, 1)])],
        vec![
            linked_node(0, 1, RoomType::MonsterRoomElite, &[]),
            linked_node(1, 1, RoomType::RestRoom, &[]),
        ],
    ]);

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let candidate = selected_candidate(&trace);
    let representative = candidate
        .viability
        .representative_path_summary
        .as_ref()
        .expect("candidate should identify its representative suffix");

    assert_eq!(candidate.path_summary.max_elites, 1);
    assert_eq!(candidate.path_summary.max_fires, 1);
    assert_ne!(representative.max_elites > 0, representative.max_fires > 0);
    assert!(
        candidate.value_factors.relic_access == 0.0 || candidate.value_factors.heal_access == 0.0,
        "elite and campfire access from different suffixes must not be combined"
    );
}

#[test]
fn synthetic_boss_target_is_included_in_the_danger_segment() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.current_hp = 80;
    run.max_hp = 80;
    run.map = MapState::new(
        (0..15)
            .map(|y| vec![linked_node(0, y, RoomType::RestRoom, &[])])
            .collect(),
    );
    run.map.current_x = 0;
    run.map.current_y = 14;

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let boss = selected_candidate(&trace);
    let viability = boss
        .viability
        .representative
        .as_ref()
        .expect("synthetic boss target should have a risk projection");

    assert_eq!(boss.target.room_type, Some(RoomType::MonsterRoomBoss));
    assert_eq!(viability.cumulative_hp_loss_p90, 60.0);
    assert_eq!(boss.value_factors.hp_loss_p90, 60.0);
}

fn replace_master_deck(run: &mut RunState, cards: &[CardId]) {
    run.master_deck = cards
        .iter()
        .enumerate()
        .map(|(idx, &card_id)| CombatCard::new(card_id, idx as u32))
        .collect();
}

fn linked_node(x: i32, y: i32, room_type: RoomType, edges: &[(i32, i32)]) -> MapRoomNode {
    let mut node = MapRoomNode::new(x, y);
    node.class = Some(room_type);
    for &(dst_x, dst_y) in edges {
        node.edges.insert(MapEdge::new(x, y, dst_x, dst_y));
    }
    node
}
