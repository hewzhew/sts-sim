use std::collections::{HashSet, VecDeque};

use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::map::RoomType;

pub(super) fn has_visible_future_shop(session: &RunControlSession) -> bool {
    future_shop_distance(session).is_some()
}

pub(super) fn future_shop_distance(session: &RunControlSession) -> Option<u8> {
    future_room_distance(session, RoomType::ShopRoom)
}

pub(super) fn future_elite_distance(session: &RunControlSession) -> Option<u8> {
    future_room_distance(session, RoomType::MonsterRoomElite)
}

/// Returns a conservative distance only when every currently legal map
/// continuation reaches an elite before it can reach the boss/end of map.
///
/// Merely seeing an elite on some branch is not evidence that the current shop
/// is preparing for that elite.  Oracle exploration may choose a different
/// continuation after leaving the shop, so encounter-specific purchase credit
/// requires an unavoidable first elite.
pub(super) fn forced_future_elite_distance(session: &RunControlSession) -> Option<u8> {
    let map = &session.run_state.map;
    if map.graph.is_empty() {
        return None;
    }
    let starts = if map.current_y == -1 {
        map.graph
            .first()
            .into_iter()
            .flatten()
            .filter(|node| !node.edges.is_empty())
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>()
    } else {
        map.get_current_node()
            .into_iter()
            .flat_map(|node| node.edges.iter())
            .map(|edge| (edge.dst_x, edge.dst_y))
            .collect::<Vec<_>>()
    };
    if starts.is_empty() {
        return None;
    }

    starts
        .into_iter()
        .map(|(x, y)| forced_elite_distance_from(map, x, y))
        .collect::<Option<Vec<_>>>()
        .and_then(|distances| distances.into_iter().max())
}

fn forced_elite_distance_from(
    map: &sts_simulator::state::map::MapState,
    x: i32,
    y: i32,
) -> Option<u8> {
    let node = map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))?;
    if node.class == Some(RoomType::MonsterRoomElite) {
        return Some(1);
    }
    if node.class == Some(RoomType::MonsterRoomBoss) || node.edges.is_empty() {
        return None;
    }
    node.edges
        .iter()
        .map(|edge| forced_elite_distance_from(map, edge.dst_x, edge.dst_y))
        .collect::<Option<Vec<_>>>()
        .and_then(|distances| distances.into_iter().max())
        .map(|distance| distance.saturating_add(1))
}

fn future_room_distance(session: &RunControlSession, target: RoomType) -> Option<u8> {
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
        if node.class == Some(target) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    #[test]
    fn reachable_elite_is_not_forced_when_the_shop_has_a_non_elite_continuation() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 20260713006,
            ascension_level: 0,
            ..RunControlConfig::default()
        });
        session.run_state.map.current_x = 6;
        session.run_state.map.current_y = 1;

        assert_eq!(
            session
                .run_state
                .map
                .get_current_node()
                .and_then(|node| node.class),
            Some(RoomType::ShopRoom)
        );
        assert!(future_elite_distance(&session).is_some());
        assert_eq!(forced_future_elite_distance(&session), None);
    }
}
