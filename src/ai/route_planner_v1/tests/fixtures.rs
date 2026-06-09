use crate::ai::route_planner_v1::{RouteCandidateTraceV1, RouteDecisionTraceV1};
use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
use crate::state::map::state::MapState;
use crate::state::RunState;

pub(super) fn selected_candidate(trace: &RouteDecisionTraceV1) -> &RouteCandidateTraceV1 {
    trace
        .selected_index
        .and_then(|idx| trace.candidates.get(idx))
        .expect("route trace should have a selected candidate")
}

pub(super) fn candidate_by_room(
    trace: &RouteDecisionTraceV1,
    room_type: RoomType,
) -> &RouteCandidateTraceV1 {
    trace
        .candidates
        .iter()
        .find(|candidate| candidate.target.room_type == Some(room_type))
        .expect("route trace should include requested room type")
}

pub(super) fn run_with_start_nodes(
    room_types: &[RoomType],
    next_room: Option<RoomType>,
) -> RunState {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map = MapState::new(start_node_graph(room_types, next_room));
    run
}

pub(super) fn run_with_current_node_and_next_row(
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

pub(super) fn run_with_start_paths(paths: &[&[RoomType]]) -> RunState {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let max_len = paths.iter().map(|path| path.len()).max().unwrap_or(1);
    let mut graph = Vec::new();
    for y in 0..max_len {
        let mut row = Vec::new();
        for (x, path) in paths.iter().enumerate() {
            let room = path
                .get(y)
                .copied()
                .or_else(|| path.last().copied())
                .unwrap_or(RoomType::MonsterRoom);
            let mut node = map_node(x as i32, y as i32, Some(room));
            if y + 1 < max_len {
                node.edges
                    .insert(MapEdge::new(x as i32, y as i32, x as i32, y as i32 + 1));
            }
            row.push(node);
        }
        graph.push(row);
    }
    run.map = MapState::new(graph);
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
