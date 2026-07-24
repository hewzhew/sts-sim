use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::map::RoomType;

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
        assert_eq!(forced_future_elite_distance(&session), None);
    }
}
