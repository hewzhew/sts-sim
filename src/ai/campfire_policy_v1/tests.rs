use crate::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePolicyActionV1,
    CampfirePolicyClassV1, CampfirePolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::state::core::CampfireChoice;
use crate::state::map::{MapEdge, MapRoomNode, MapState, RoomType};
use crate::state::run::RunState;

#[test]
fn campfire_context_exposes_rest_and_smith_candidates() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 20;
    run_state.max_hp = 80;
    install_visible_rest_route(&mut run_state);
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == CampfirePolicyClassV1::RestRecovery));
    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == CampfirePolicyClassV1::UpgradeAgency));
}

#[test]
fn campfire_policy_smiths_clear_upgrade_when_first_elite_prep_window_is_open() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 70;
    run_state.max_hp = 80;
    install_current_room_route(
        &mut run_state,
        RoomType::RestRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoomElite],
    );
    let bash_index = run_state
        .master_deck
        .iter()
        .position(|card| card.id == CardId::Bash)
        .expect("Ironclad starter deck should include Bash");
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Smith { deck_index, .. } if deck_index == bash_index
    ));
}

fn install_visible_rest_route(run_state: &mut RunState) {
    let mut rest = MapRoomNode::new(0, 0);
    rest.class = Some(RoomType::RestRoom);
    rest.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut next = MapRoomNode::new(0, 1);
    next.class = Some(RoomType::MonsterRoom);
    run_state.map = MapState::new(vec![vec![rest], vec![next]]);
    run_state.map.current_x = 0;
    run_state.map.current_y = 0;
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
