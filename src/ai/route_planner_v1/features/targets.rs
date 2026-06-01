use crate::content::relics::RelicId;
use crate::state::map::node::{MapRoomNode, RoomType};
use crate::state::RunState;

use super::super::types::{MapRouteTargetV1, RouteMoveKindV1};

pub fn route_targets(run_state: &RunState) -> Vec<MapRouteTargetV1> {
    let map = &run_state.map;
    if map.current_y == 14 && map.can_travel_to(0, 15, false) {
        return vec![boss_target()];
    }
    let target_y = if map.current_y == -1 {
        0
    } else {
        map.current_y + 1
    };
    if target_y == 15 && map.can_travel_to(0, 15, false) {
        return vec![boss_target()];
    }
    let wing_boots_charges = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::WingBoots)
        .map(|relic| relic.counter.max(0))
        .unwrap_or(0);
    run_state
        .map
        .graph
        .get(target_y.max(0) as usize)
        .into_iter()
        .flat_map(|row| row.iter())
        .filter_map(|node| {
            let normal = map.can_travel_to(node.x, node.y, false);
            let wing = wing_boots_charges > 0 && map.can_travel_to(node.x, node.y, true);
            if normal {
                Some(route_target_from_node(node, RouteMoveKindV1::NormalEdge))
            } else if wing {
                Some(route_target_from_node(node, RouteMoveKindV1::WingBootsJump))
            } else {
                None
            }
        })
        .collect()
}

fn boss_target() -> MapRouteTargetV1 {
    MapRouteTargetV1 {
        x: 0,
        y: 15,
        room_type: Some(RoomType::MonsterRoomBoss),
        has_emerald_key: false,
        move_kind: RouteMoveKindV1::NormalEdge,
    }
}

fn route_target_from_node(node: &MapRoomNode, move_kind: RouteMoveKindV1) -> MapRouteTargetV1 {
    MapRouteTargetV1 {
        x: node.x,
        y: node.y,
        room_type: node.class,
        has_emerald_key: node.has_emerald_key,
        move_kind,
    }
}
