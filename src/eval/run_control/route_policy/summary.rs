use crate::state::map::node::{MapRoomNode, RoomType};

use super::{MapRouteTarget, RouteSummary};
use crate::eval::run_control::session::RunControlSession;

const ROUTE_PATH_BUDGET: usize = 2_000;

#[derive(Clone, Copy, Debug, Default)]
struct PathStats {
    early_pressure: usize,
    elites: usize,
    shops: usize,
    fires: usize,
    first_shop_floor: Option<i32>,
    first_fire_floor: Option<i32>,
}

pub(in crate::eval::run_control) fn route_targets(
    session: &RunControlSession,
) -> Vec<MapRouteTarget> {
    let map = &session.run_state.map;
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
    session
        .run_state
        .map
        .graph
        .get(target_y.max(0) as usize)
        .into_iter()
        .flat_map(|row| row.iter())
        .filter(|node| map.can_travel_to(node.x, node.y, false))
        .map(route_target_from_node)
        .collect()
}

pub(in crate::eval::run_control) fn summarize_route_from(
    session: &RunControlSession,
    x: i32,
    y: i32,
) -> RouteSummary {
    if y >= 15 {
        return empty_summary_with_path();
    }

    let mut paths = Vec::new();
    collect_path_stats(
        session,
        x,
        y,
        PathStats::default(),
        &mut paths,
        ROUTE_PATH_BUDGET,
    );
    if paths.is_empty() {
        return empty_summary();
    }
    let min = |f: fn(&PathStats) -> usize| paths.iter().map(f).min().unwrap_or(0);
    let max = |f: fn(&PathStats) -> usize| paths.iter().map(f).max().unwrap_or(0);
    RouteSummary {
        path_count: paths.len(),
        min_early_pressure: min(|stats| stats.early_pressure),
        max_early_pressure: max(|stats| stats.early_pressure),
        min_elites: min(|stats| stats.elites),
        max_elites: max(|stats| stats.elites),
        min_shops: min(|stats| stats.shops),
        max_shops: max(|stats| stats.shops),
        min_fires: min(|stats| stats.fires),
        max_fires: max(|stats| stats.fires),
        first_shop_floor: paths
            .iter()
            .filter_map(|stats| stats.first_shop_floor)
            .min(),
        first_fire_floor: paths
            .iter()
            .filter_map(|stats| stats.first_fire_floor)
            .min(),
    }
}

fn boss_target() -> MapRouteTarget {
    MapRouteTarget {
        x: 0,
        y: 15,
        class: Some(RoomType::MonsterRoomBoss),
        has_emerald_key: false,
    }
}

fn route_target_from_node(node: &MapRoomNode) -> MapRouteTarget {
    MapRouteTarget {
        x: node.x,
        y: node.y,
        class: node.class,
        has_emerald_key: node.has_emerald_key,
    }
}

fn empty_summary_with_path() -> RouteSummary {
    RouteSummary {
        path_count: 1,
        ..empty_summary()
    }
}

fn empty_summary() -> RouteSummary {
    RouteSummary {
        path_count: 0,
        min_early_pressure: 0,
        max_early_pressure: 0,
        min_elites: 0,
        max_elites: 0,
        min_shops: 0,
        max_shops: 0,
        min_fires: 0,
        max_fires: 0,
        first_shop_floor: None,
        first_fire_floor: None,
    }
}

fn collect_path_stats(
    session: &RunControlSession,
    x: i32,
    y: i32,
    current: PathStats,
    paths: &mut Vec<PathStats>,
    budget: usize,
) {
    if paths.len() >= budget {
        return;
    }
    let Some(node) = session
        .run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
    else {
        return;
    };
    let current = update_path_stats(current, node);
    if node.edges.is_empty() || y >= 14 {
        paths.push(current);
        return;
    }
    for edge in &node.edges {
        collect_path_stats(session, edge.dst_x, edge.dst_y, current, paths, budget);
    }
}

fn update_path_stats(mut stats: PathStats, node: &MapRoomNode) -> PathStats {
    match node.class {
        Some(RoomType::MonsterRoom) => {
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
        }
        Some(RoomType::MonsterRoomElite) => {
            stats.elites += 1;
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
        }
        Some(RoomType::ShopRoom) => {
            stats.shops += 1;
            stats.first_shop_floor.get_or_insert(node.y + 1);
        }
        Some(RoomType::RestRoom) => {
            stats.fires += 1;
            stats.first_fire_floor.get_or_insert(node.y + 1);
        }
        _ => {}
    }
    stats
}
