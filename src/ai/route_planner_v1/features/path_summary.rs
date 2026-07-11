use crate::ai::route_window_facts::{
    build_route_path_family_from_target, RouteWindowFactsConfig, RouteWindowNode, RouteWindowPath,
    RouteWindowPathFamily,
};
use crate::state::map::node::RoomType;
use crate::state::RunState;

use super::super::types::{RouteFirstEliteSegmentV1, RoutePathSummaryV1, RoutePlannerConfigV1};

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
    first_elite_seen: bool,
    hallway_fights_before_first_elite: usize,
    unknowns_before_first_elite: usize,
    fires_before_first_elite: usize,
    shops_before_first_elite: usize,
    first_recovery_seen: bool,
    damage_rooms_before_first_recovery: usize,
    unknowns_before_first_recovery: usize,
    recovery_before_damage: bool,
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

    let horizon_nodes = 15_usize.saturating_sub(y.max(0) as usize);
    let family = build_route_path_family_from_target(
        run_state,
        x,
        y,
        RouteWindowFactsConfig {
            horizon_nodes,
            path_budget: config.path_budget,
        },
    );
    summarize_route_path_family(&family)
}

pub(in crate::ai::route_planner_v1) fn summarize_route_path_family(
    family: &RouteWindowPathFamily,
) -> RoutePathSummaryV1 {
    let paths = family
        .paths
        .iter()
        .map(|path| {
            path.nodes
                .iter()
                .fold(PathStats::default(), update_path_stats)
        })
        .collect::<Vec<_>>();
    summarize_path_stats(&paths, family.coverage.path_budget_exhausted)
}

pub(in crate::ai::route_planner_v1) fn summarize_route_path(
    path: &RouteWindowPath,
) -> RoutePathSummaryV1 {
    let stats = path
        .nodes
        .iter()
        .fold(PathStats::default(), update_path_stats);
    summarize_path_stats(&[stats], false)
}

fn summarize_path_stats(paths: &[PathStats], path_budget_exhausted: bool) -> RoutePathSummaryV1 {
    if paths.is_empty() {
        return RoutePathSummaryV1 {
            path_budget_exhausted,
            ..empty_summary()
        };
    }
    let min = |f: fn(&PathStats) -> usize| paths.iter().map(f).min().unwrap_or(0);
    let max = |f: fn(&PathStats) -> usize| paths.iter().map(f).max().unwrap_or(0);
    RoutePathSummaryV1 {
        path_count: paths.len(),
        path_budget_exhausted,
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
        min_damage_rooms_before_recovery: min(|stats| stats.damage_rooms_before_first_recovery),
        max_damage_rooms_before_recovery: max(|stats| stats.damage_rooms_before_first_recovery),
        min_unknowns_before_recovery: min(|stats| stats.unknowns_before_first_recovery),
        max_unknowns_before_recovery: max(|stats| stats.unknowns_before_first_recovery),
        paths_with_recovery_before_damage: paths
            .iter()
            .filter(|stats| stats.recovery_before_damage)
            .count(),
        first_elite: first_elite_segment(&paths),
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
        path_budget_exhausted: false,
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
        min_damage_rooms_before_recovery: 0,
        max_damage_rooms_before_recovery: 0,
        min_unknowns_before_recovery: 0,
        max_unknowns_before_recovery: 0,
        paths_with_recovery_before_damage: 0,
        first_elite: RouteFirstEliteSegmentV1::default(),
    }
}

fn first_elite_segment(paths: &[PathStats]) -> RouteFirstEliteSegmentV1 {
    let elite_paths = paths
        .iter()
        .filter(|stats| stats.first_elite_seen)
        .collect::<Vec<_>>();
    if elite_paths.is_empty() {
        return RouteFirstEliteSegmentV1::default();
    }
    let min =
        |f: fn(&PathStats) -> usize| elite_paths.iter().map(|stats| f(stats)).min().unwrap_or(0);
    let max =
        |f: fn(&PathStats) -> usize| elite_paths.iter().map(|stats| f(stats)).max().unwrap_or(0);
    RouteFirstEliteSegmentV1 {
        paths_with_first_elite: elite_paths.len(),
        forced: elite_paths.len() == paths.len(),
        optional: elite_paths.len() < paths.len(),
        min_hallway_fights_before: min(|stats| stats.hallway_fights_before_first_elite),
        max_hallway_fights_before: max(|stats| stats.hallway_fights_before_first_elite),
        min_unknowns_before: min(|stats| stats.unknowns_before_first_elite),
        max_unknowns_before: max(|stats| stats.unknowns_before_first_elite),
        min_fires_before: min(|stats| stats.fires_before_first_elite),
        max_fires_before: max(|stats| stats.fires_before_first_elite),
        min_shops_before: min(|stats| stats.shops_before_first_elite),
        max_shops_before: max(|stats| stats.shops_before_first_elite),
        can_bail_to_rest_before: elite_paths
            .iter()
            .any(|stats| stats.fires_before_first_elite > 0),
        can_bail_to_shop_before: elite_paths
            .iter()
            .any(|stats| stats.shops_before_first_elite > 0),
    }
}

fn update_path_stats(mut stats: PathStats, node: &RouteWindowNode) -> PathStats {
    match node.room_type {
        Some(RoomType::MonsterRoom) => {
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
            if !stats.first_elite_seen {
                stats.hallway_fights_before_first_elite += 1;
            }
            if !stats.first_recovery_seen {
                stats.damage_rooms_before_first_recovery += 1;
            }
        }
        Some(RoomType::MonsterRoomElite) => {
            stats.elites += 1;
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
            if !stats.first_elite_seen {
                stats.first_elite_seen = true;
            }
            if !stats.first_recovery_seen {
                stats.damage_rooms_before_first_recovery += 1;
            }
        }
        Some(RoomType::ShopRoom) => {
            stats.shops += 1;
            stats.first_shop_floor.get_or_insert(node.y + 1);
            if !stats.first_elite_seen {
                stats.shops_before_first_elite += 1;
            }
            if !stats.first_recovery_seen {
                stats.recovery_before_damage = stats.damage_rooms_before_first_recovery == 0
                    && stats.unknowns_before_first_recovery == 0;
                stats.first_recovery_seen = true;
            }
        }
        Some(RoomType::RestRoom) => {
            stats.fires += 1;
            stats.first_fire_floor.get_or_insert(node.y + 1);
            if !stats.first_elite_seen {
                stats.fires_before_first_elite += 1;
            }
            if !stats.first_recovery_seen {
                stats.recovery_before_damage = stats.damage_rooms_before_first_recovery == 0
                    && stats.unknowns_before_first_recovery == 0;
                stats.first_recovery_seen = true;
            }
        }
        Some(RoomType::EventRoom) => {
            stats.unknowns += 1;
            if !stats.first_elite_seen {
                stats.unknowns_before_first_elite += 1;
            }
            if !stats.first_recovery_seen {
                stats.unknowns_before_first_recovery += 1;
            }
        }
        Some(RoomType::TreasureRoom) => stats.treasures += 1,
        _ => {}
    }
    stats
}
