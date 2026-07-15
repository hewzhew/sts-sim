use std::collections::{HashSet, VecDeque};

use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::map::RoomType;

pub(super) fn has_visible_future_shop(session: &RunControlSession) -> bool {
    future_shop_distance(session).is_some()
}

pub(super) fn future_shop_distance(session: &RunControlSession) -> Option<u8> {
    let map = &session.run_state.map;
    if map.graph.is_empty() {
        return None;
    }
    let mut frontier = VecDeque::new();
    if map.current_y == -1 {
        if let Some(row) = map.graph.first() {
            frontier.extend(
                row.iter()
                    .filter(|node| !node.edges.is_empty())
                    .map(|node| (node.x, node.y, 1_u8)),
            );
        }
    } else if let Some(current) = map.get_current_node() {
        frontier.extend(
            current
                .edges
                .iter()
                .map(|edge| (edge.dst_x, edge.dst_y, 1_u8)),
        );
    }

    let mut visited = HashSet::new();
    while let Some((x, y, distance)) = frontier.pop_front() {
        if !visited.insert((x, y)) {
            continue;
        }
        let Some(node) = map
            .graph
            .get(y.max(0) as usize)
            .and_then(|row| row.get(x.max(0) as usize))
        else {
            continue;
        };
        if node.class == Some(RoomType::ShopRoom) {
            return Some(distance);
        }
        let next_distance = distance.saturating_add(1);
        frontier.extend(
            node.edges
                .iter()
                .map(|edge| (edge.dst_x, edge.dst_y, next_distance)),
        );
    }
    None
}
