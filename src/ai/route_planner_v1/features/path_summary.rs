use crate::state::map::node::{MapRoomNode, RoomType};
use crate::state::RunState;

use super::super::types::{RoutePathSummaryV1, RoutePlannerConfigV1};

#[derive(Clone, Copy, Debug, Default)]
struct PathStats {
    early_pressure: usize,
    elites: usize,
    shops: usize,
    fires: usize,
    unknowns: usize,
    treasures: usize,
    first_shop_floor: Option<i32>,
    first_fire_floor: Option<i32>,
}

pub fn summarize_route_from(
    run_state: &RunState,
    x: i32,
    y: i32,
    config: &RoutePlannerConfigV1,
) -> RoutePathSummaryV1 {
    if y >= 15 {
        return empty_summary_with_path();
    }

    let mut paths = Vec::new();
    collect_path_stats(
        run_state,
        x,
        y,
        PathStats::default(),
        &mut paths,
        config.path_budget,
    );
    if paths.is_empty() {
        return empty_summary();
    }
    let min = |f: fn(&PathStats) -> usize| paths.iter().map(f).min().unwrap_or(0);
    let max = |f: fn(&PathStats) -> usize| paths.iter().map(f).max().unwrap_or(0);
    RoutePathSummaryV1 {
        path_count: paths.len(),
        min_early_pressure: min(|stats| stats.early_pressure),
        max_early_pressure: max(|stats| stats.early_pressure),
        min_elites: min(|stats| stats.elites),
        max_elites: max(|stats| stats.elites),
        min_shops: min(|stats| stats.shops),
        max_shops: max(|stats| stats.shops),
        min_fires: min(|stats| stats.fires),
        max_fires: max(|stats| stats.fires),
        min_unknowns: min(|stats| stats.unknowns),
        max_unknowns: max(|stats| stats.unknowns),
        min_treasures: min(|stats| stats.treasures),
        max_treasures: max(|stats| stats.treasures),
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

fn empty_summary_with_path() -> RoutePathSummaryV1 {
    RoutePathSummaryV1 {
        path_count: 1,
        ..empty_summary()
    }
}

fn empty_summary() -> RoutePathSummaryV1 {
    RoutePathSummaryV1 {
        path_count: 0,
        min_early_pressure: 0,
        max_early_pressure: 0,
        min_elites: 0,
        max_elites: 0,
        min_shops: 0,
        max_shops: 0,
        min_fires: 0,
        max_fires: 0,
        min_unknowns: 0,
        max_unknowns: 0,
        min_treasures: 0,
        max_treasures: 0,
        first_shop_floor: None,
        first_fire_floor: None,
    }
}

fn collect_path_stats(
    run_state: &RunState,
    x: i32,
    y: i32,
    current: PathStats,
    paths: &mut Vec<PathStats>,
    budget: usize,
) {
    if paths.len() >= budget {
        return;
    }
    let Some(node) = run_state
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
        collect_path_stats(run_state, edge.dst_x, edge.dst_y, current, paths, budget);
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
        Some(RoomType::EventRoom) => stats.unknowns += 1,
        Some(RoomType::TreasureRoom) => stats.treasures += 1,
        _ => {}
    }
    stats
}
