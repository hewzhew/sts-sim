use crate::state::map::node::{MapRoomNode, RoomType};
use crate::state::RunState;

use super::types::{
    MapRouteTargetV1, NodeFeaturesV1, RouteMoveKindV1, RoutePathSummaryV1, RoutePlannerConfigV1,
    UnknownRoomBeliefV1,
};

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
        .find(|relic| relic.id == crate::content::relics::RelicId::WingBoots)
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

pub(super) fn node_features(
    target: &MapRouteTargetV1,
    belief: &UnknownRoomBeliefV1,
    hp: i32,
    max_hp: i32,
    has_empty_potion_slot: bool,
    config: &RoutePlannerConfigV1,
) -> NodeFeaturesV1 {
    let hp_ratio = if max_hp > 0 {
        hp as f32 / max_hp as f32
    } else {
        0.0
    };
    let mut features = base_node_features(target.room_type);
    if target.has_emerald_key && features.is_elite {
        features.is_burning_elite = true;
    }
    if target.room_type == Some(RoomType::EventRoom) {
        apply_unknown_room_belief(&mut features, belief, config);
    }
    if !has_empty_potion_slot {
        features.expected_potion_gain = 0.0;
    }
    if hp_ratio < config.low_hp_ratio {
        features.death_risk += features.expected_hp_loss_p90 / max_hp.max(1) as f32;
    }
    features.death_risk = features.death_risk.clamp(0.0, 1.0);
    features
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

fn base_node_features(room_type: Option<RoomType>) -> NodeFeaturesV1 {
    match room_type {
        Some(RoomType::MonsterRoom) => NodeFeaturesV1 {
            node_type: room_type,
            expected_card_rewards: 1.0,
            expected_gold_gain: 15.0,
            expected_potion_gain: 0.35,
            expected_hp_loss_mean: 7.0,
            expected_hp_loss_p90: 14.0,
            death_risk: 0.05,
            ..empty_features()
        },
        Some(RoomType::MonsterRoomElite) => NodeFeaturesV1 {
            node_type: room_type,
            expected_card_rewards: 1.0,
            expected_relics: 1.0,
            expected_gold_gain: 30.0,
            expected_potion_gain: 0.25,
            expected_hp_loss_mean: 24.0,
            expected_hp_loss_p90: 40.0,
            death_risk: 0.25,
            is_elite: true,
            ..empty_features()
        },
        Some(RoomType::RestRoom) => NodeFeaturesV1 {
            node_type: room_type,
            upgrade_access: 1.0,
            heal_access: 1.0,
            is_rest: true,
            ..empty_features()
        },
        Some(RoomType::ShopRoom) => NodeFeaturesV1 {
            node_type: room_type,
            shop_access: 1.0,
            remove_access: 1.0,
            is_shop: true,
            ..empty_features()
        },
        Some(RoomType::TreasureRoom) => NodeFeaturesV1 {
            node_type: room_type,
            expected_relics: 1.0,
            ..empty_features()
        },
        Some(RoomType::EventRoom) => NodeFeaturesV1 {
            node_type: room_type,
            is_question_mark: true,
            ..empty_features()
        },
        Some(RoomType::MonsterRoomBoss) => NodeFeaturesV1 {
            node_type: room_type,
            expected_relics: 1.0,
            expected_hp_loss_mean: 35.0,
            expected_hp_loss_p90: 60.0,
            death_risk: 0.45,
            ..empty_features()
        },
        _ => NodeFeaturesV1 {
            node_type: room_type,
            ..empty_features()
        },
    }
}

fn apply_unknown_room_belief(
    features: &mut NodeFeaturesV1,
    belief: &UnknownRoomBeliefV1,
    config: &RoutePlannerConfigV1,
) {
    features.expected_card_rewards = belief.monster_chance;
    features.expected_relics = belief.treasure_chance + belief.elite_chance;
    features.expected_gold_gain = belief.monster_chance * 15.0;
    features.expected_potion_gain = belief.monster_chance * 0.35;
    features.shop_access = belief.shop_chance;
    features.remove_access = belief.shop_chance;
    features.event_access = belief.event_chance;
    features.expected_hp_loss_mean = belief.monster_chance * config.base_monster_hp_loss
        + belief.elite_chance * config.base_elite_hp_loss;
    features.expected_hp_loss_p90 = features.expected_hp_loss_mean * 1.8;
    features.death_risk = belief.monster_chance * 0.05 + belief.elite_chance * 0.25;
}

fn empty_features() -> NodeFeaturesV1 {
    NodeFeaturesV1 {
        node_type: None,
        expected_card_rewards: 0.0,
        expected_relics: 0.0,
        expected_gold_gain: 0.0,
        expected_potion_gain: 0.0,
        shop_access: 0.0,
        remove_access: 0.0,
        upgrade_access: 0.0,
        heal_access: 0.0,
        event_access: 0.0,
        expected_hp_loss_mean: 0.0,
        expected_hp_loss_p90: 0.0,
        death_risk: 0.0,
        is_elite: false,
        is_burning_elite: false,
        is_rest: false,
        is_shop: false,
        is_question_mark: false,
    }
}
