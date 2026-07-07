use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::map::RoomType;

pub(super) fn has_visible_future_shop(session: &RunControlSession) -> bool {
    let map = &session.run_state.map;
    if map.graph.is_empty() {
        return false;
    }
    let mut frontier = Vec::new();
    if map.current_y == -1 {
        if let Some(row) = map.graph.first() {
            frontier.extend(row.iter().map(|node| (node.x, node.y)));
        }
    } else if let Some(current) = map.get_current_node() {
        frontier.extend(current.edges.iter().map(|edge| (edge.dst_x, edge.dst_y)));
    }

    while let Some((x, y)) = frontier.pop() {
        let Some(node) = map
            .graph
            .get(y.max(0) as usize)
            .and_then(|row| row.get(x.max(0) as usize))
        else {
            continue;
        };
        if node.class == Some(RoomType::ShopRoom) {
            return true;
        }
        frontier.extend(node.edges.iter().map(|edge| (edge.dst_x, edge.dst_y)));
    }
    false
}
