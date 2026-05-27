use crate::ai::route_planner_v1::{
    plan_route_decision_v1, route_targets, RoutePlannerConfigV1, RouteSafetyFlagV1,
    ROUTE_DECISION_TRACE_SCHEMA_NAME,
};
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
use crate::state::map::state::MapState;
use crate::state::RunState;

#[test]
fn route_planner_trace_is_behavior_policy_not_teacher_label() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    assert_eq!(trace.schema_name, ROUTE_DECISION_TRACE_SCHEMA_NAME);
    assert_eq!(trace.label_role, "behavior_policy_not_teacher");
    assert!(!trace.candidates.is_empty());
    assert!(trace.selected_index.is_some());
    assert!(trace.candidates.iter().all(|candidate| {
        candidate
            .suggested_command
            .as_deref()
            .is_some_and(|command| command.starts_with("go "))
    }));
}

#[test]
fn route_planner_does_not_emit_executable_command_when_map_locked() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::EventRoom,
        RoutePlannerConfigV1::default(),
    );

    assert!(trace
        .warnings
        .iter()
        .any(|warning| warning.contains("locked")));
    assert!(trace
        .candidates
        .iter()
        .all(|candidate| candidate.suggested_command.is_none()));
}

#[test]
fn route_planner_trace_serializes_structured_evidence() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    let value = serde_json::to_value(&trace).expect("trace should serialize");

    assert_eq!(value["schema_name"], ROUTE_DECISION_TRACE_SCHEMA_NAME);
    assert_eq!(value["label_role"], "behavior_policy_not_teacher");
    assert!(value["candidates"][0]["score_terms"].is_object());
    assert!(value["candidates"][0]["needs"].is_object());
}

#[test]
fn route_planner_expands_wing_boots_next_floor_jumps() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map.current_x = 0;
    run.map.current_y = 0;
    run.relics.push(RelicState::new(RelicId::WingBoots));

    let targets = route_targets(&run);

    assert!(
        targets.iter().any(|target| target.move_kind
            == crate::ai::route_planner_v1::RouteMoveKindV1::WingBootsJump),
        "Wing Boots should add non-edge choices on the next floor when charges are available"
    );
}

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

#[test]
fn route_planner_preserves_wing_boots_charge_when_normal_route_is_comparable() {
    let mut run = run_with_current_node_and_next_row(
        RoomType::MonsterRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoom],
        Some(RoomType::RestRoom),
    );
    run.relics.push(RelicState::new(RelicId::WingBoots));

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let selected = selected_candidate(&trace);

    assert_eq!(
        selected.target.move_kind,
        crate::ai::route_planner_v1::RouteMoveKindV1::NormalEdge
    );
    assert!(trace.candidates.iter().any(|candidate| {
        candidate.target.move_kind == crate::ai::route_planner_v1::RouteMoveKindV1::WingBootsJump
    }));
}

fn selected_candidate(
    trace: &crate::ai::route_planner_v1::RouteDecisionTraceV1,
) -> &crate::ai::route_planner_v1::RouteCandidateTraceV1 {
    trace
        .selected_index
        .and_then(|idx| trace.candidates.get(idx))
        .expect("route trace should have a selected candidate")
}

fn candidate_by_room(
    trace: &crate::ai::route_planner_v1::RouteDecisionTraceV1,
    room_type: RoomType,
) -> &crate::ai::route_planner_v1::RouteCandidateTraceV1 {
    trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.room_type == Some(room_type))
        .expect("route trace should include requested room type")
}

fn run_with_start_nodes(room_types: &[RoomType], next_room: Option<RoomType>) -> RunState {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map = MapState::new(start_node_graph(room_types, next_room));
    run
}

fn run_with_current_node_and_next_row(
    current_room: RoomType,
    next_rooms: &[RoomType],
    final_room: Option<RoomType>,
) -> RunState {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let mut current = map_node(0, 0, Some(current_room));
    current.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut next_row = next_rooms
        .iter()
        .enumerate()
        .map(|(x, room)| {
            let mut node = map_node(x as i32, 1, Some(*room));
            node.edges.insert(MapEdge::new(x as i32, 1, 0, 2));
            node
        })
        .collect::<Vec<_>>();
    if next_row.is_empty() {
        next_row.push(map_node(0, 1, final_room));
    }
    run.map = MapState::new(vec![
        vec![current],
        next_row,
        vec![map_node(0, 2, final_room)],
    ]);
    run.map.current_x = 0;
    run.map.current_y = 0;
    run
}

fn start_node_graph(room_types: &[RoomType], next_room: Option<RoomType>) -> Vec<Vec<MapRoomNode>> {
    let mut row = room_types
        .iter()
        .enumerate()
        .map(|(x, room)| {
            let mut node = map_node(x as i32, 0, Some(*room));
            node.edges.insert(MapEdge::new(x as i32, 0, 0, 1));
            node
        })
        .collect::<Vec<_>>();
    if row.is_empty() {
        row.push(map_node(0, 0, Some(RoomType::MonsterRoom)));
    }
    vec![row, vec![map_node(0, 1, next_room)]]
}

fn map_node(x: i32, y: i32, room_type: Option<RoomType>) -> MapRoomNode {
    let mut node = MapRoomNode::new(x, y);
    node.class = room_type;
    node
}
