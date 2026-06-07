use crate::ai::route_planner_v1::{route_targets, summarize_route_from, RoutePlannerConfigV1};
use crate::state::map::node::RoomType;
use crate::state::run::RunState;

use super::types::NeowMapFeaturesV1;

pub fn neow_map_features_from_run_state_v1(run_state: &RunState) -> NeowMapFeaturesV1 {
    let config = RoutePlannerConfigV1::default();
    let targets = route_targets(run_state);
    let summaries = targets
        .iter()
        .map(|target| summarize_route_from(run_state, target.x, target.y, &config))
        .collect::<Vec<_>>();
    let early_shop_available = summaries
        .iter()
        .filter_map(|summary| summary.first_shop_floor)
        .any(|floor| floor <= 4);
    let early_elite_available = has_elite_by_floor(run_state, 6);
    let shop_before_first_elite = has_shop_before_first_elite(run_state);
    let lament_elite_snipe_candidate = has_lament_elite_snipe_candidate(run_state);
    let path_count = summaries
        .iter()
        .map(|summary| summary.path_count)
        .sum::<usize>();
    let path_flexibility =
        ((targets.len() as f32) / 4.0 + (path_count as f32).log10() / 3.0).clamp(0.0, 1.0);

    NeowMapFeaturesV1 {
        early_shop_available,
        shop_before_first_elite,
        early_elite_available,
        lament_elite_snipe_candidate,
        path_flexibility,
    }
}

fn has_elite_by_floor(run_state: &RunState, max_visible_floor: i32) -> bool {
    run_state.map.graph.iter().any(|row| {
        row.iter().any(|node| {
            node.y + 1 <= max_visible_floor && node.class == Some(RoomType::MonsterRoomElite)
        })
    })
}

fn has_shop_before_first_elite(run_state: &RunState) -> bool {
    route_targets(run_state).iter().any(|target| {
        let mut stack = vec![(target.x, target.y, None::<i32>, None::<i32>)];
        while let Some((x, y, first_shop, first_elite)) = stack.pop() {
            let Some(node) = run_state
                .map
                .graph
                .get(y.max(0) as usize)
                .and_then(|row| row.get(x.max(0) as usize))
            else {
                continue;
            };
            let first_shop = match (first_shop, node.class) {
                (None, Some(RoomType::ShopRoom)) => Some(node.y + 1),
                _ => first_shop,
            };
            let first_elite = match (first_elite, node.class) {
                (None, Some(RoomType::MonsterRoomElite)) => Some(node.y + 1),
                _ => first_elite,
            };
            if let Some(shop_floor) = first_shop {
                if shop_floor <= 4 && first_elite.is_none_or(|elite_floor| shop_floor < elite_floor)
                {
                    return true;
                }
            }
            for edge in &node.edges {
                stack.push((edge.dst_x, edge.dst_y, first_shop, first_elite));
            }
        }
        false
    })
}

fn has_lament_elite_snipe_candidate(run_state: &RunState) -> bool {
    route_targets(run_state)
        .iter()
        .any(|target| elite_within_three_combats(run_state, target.x, target.y, 0))
}

fn elite_within_three_combats(run_state: &RunState, x: i32, y: i32, combats_so_far: usize) -> bool {
    let Some(node) = run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
    else {
        return false;
    };
    let combats = combats_so_far
        + usize::from(matches!(
            node.class,
            Some(RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss)
        ));
    if node.class == Some(RoomType::MonsterRoomElite) {
        return combats <= 3;
    }
    if combats >= 3 {
        return false;
    }
    node.edges
        .iter()
        .any(|edge| elite_within_three_combats(run_state, edge.dst_x, edge.dst_y, combats))
}
