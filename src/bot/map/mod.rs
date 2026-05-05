use crate::bot::shared::{analyze_run_needs, RunNeedSnapshot};
use crate::map::node::RoomType;
use crate::state::run::RunState;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MapOptionScore {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<RoomType>,
    pub immediate_benefit: i32,
    pub path_benefit: i32,
    pub risk_penalty: i32,
    pub situational_bonus: i32,
    pub total_score: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MapDecisionDiagnostics {
    pub chosen_x: Option<i32>,
    pub chosen_y: Option<i32>,
    pub top_options: Vec<MapOptionScore>,
}

#[derive(Clone, Copy)]
struct MapContext<'a> {
    run_state: &'a RunState,
    need: RunNeedSnapshot,
}

#[derive(Clone, Copy)]
struct MapEvaluation {
    immediate_benefit: i32,
    risk_penalty: i32,
    situational_bonus: i32,
    rationale_key: &'static str,
}

pub fn decide(run_state: &RunState) -> Option<(i32, MapDecisionDiagnostics)> {
    let context = MapContext {
        run_state,
        need: analyze_run_needs(run_state),
    };
    let mut options = next_nodes(run_state)
        .into_iter()
        .filter_map(|(x, y)| build_option(&context, x, y))
        .collect::<Vec<_>>();

    options.sort_by(|lhs, rhs| {
        rhs.total_score
            .cmp(&lhs.total_score)
            .then_with(|| lhs.y.cmp(&rhs.y))
            .then_with(|| lhs.x.cmp(&rhs.x))
    });

    let chosen = options.first().cloned();
    Some((
        chosen.as_ref()?.x,
        MapDecisionDiagnostics {
            chosen_x: chosen.as_ref().map(|option| option.x),
            chosen_y: chosen.as_ref().map(|option| option.y),
            top_options: options.into_iter().take(8).collect(),
        },
    ))
}

fn build_option(context: &MapContext<'_>, x: i32, y: i32) -> Option<MapOptionScore> {
    let room_type = room_type_at(context.run_state, x, y)?;
    let evaluation = evaluate_room(context, room_type, x, y);
    let path_benefit = forecast_path_value(context, x, y, 2);
    let total_score = evaluation.immediate_benefit + evaluation.situational_bonus + path_benefit
        - evaluation.risk_penalty;
    Some(MapOptionScore {
        x,
        y,
        room_type: Some(room_type),
        immediate_benefit: evaluation.immediate_benefit,
        path_benefit,
        risk_penalty: evaluation.risk_penalty,
        situational_bonus: evaluation.situational_bonus,
        total_score,
        rationale_key: evaluation.rationale_key,
    })
}

fn next_nodes(run_state: &RunState) -> Vec<(i32, i32)> {
    if run_state.map.current_y < 0 {
        return run_state
            .map
            .graph
            .first()
            .into_iter()
            .flat_map(|row| row.iter())
            .filter(|node| node.class.is_some() || !node.edges.is_empty())
            .map(|node| (node.x, node.y))
            .collect();
    }

    run_state
        .map
        .get_current_node()
        .map(|node| {
            node.edges
                .iter()
                .map(|edge| (edge.dst_x, edge.dst_y))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn forecast_path_value(context: &MapContext<'_>, x: i32, y: i32, depth: usize) -> i32 {
    if depth == 0 {
        return 0;
    }
    let Some(node) = context
        .run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
    else {
        return 0;
    };

    let mut best = 0;
    for edge in &node.edges {
        let Some(room_type) = room_type_at(context.run_state, edge.dst_x, edge.dst_y) else {
            continue;
        };
        let child = evaluate_room(context, room_type, edge.dst_x, edge.dst_y);
        let child_local = child.immediate_benefit + child.situational_bonus - child.risk_penalty;
        let candidate =
            child_local + forecast_path_value(context, edge.dst_x, edge.dst_y, depth - 1) / 2;
        best = best.max(candidate);
    }
    best
}

fn evaluate_room(context: &MapContext<'_>, room_type: RoomType, x: i32, y: i32) -> MapEvaluation {
    let need = context.need;
    let hp_ratio = need.hp_ratio;
    let has_emerald_key = node_has_emerald_key(context.run_state, x, y);

    match room_type {
        RoomType::RestRoom => {
            let immediate_benefit = 24
                + if hp_ratio < 0.45 {
                    46
                } else if hp_ratio < 0.60 {
                    22
                } else {
                    8
                };
            let risk_penalty = if hp_ratio >= 0.75 {
                18 + need.upgrade_pressure / 10
            } else if hp_ratio >= 0.60 {
                8
            } else {
                0
            };
            let situational_bonus = if need.rest_distance.is_none_or(|distance| distance > 2) {
                8
            } else {
                0
            };
            MapEvaluation {
                immediate_benefit,
                risk_penalty,
                situational_bonus,
                rationale_key: "route_to_rest",
            }
        }
        RoomType::ShopRoom => {
            let immediate_benefit = 18
                + if need.purge_pressure >= 100 { 20 } else { 0 }
                + if context.run_state.gold >= need.gold_reserve + 90 {
                    18
                } else if context.run_state.gold >= need.gold_reserve + 40 {
                    8
                } else {
                    0
                };
            let risk_penalty = if context.run_state.gold < need.gold_reserve {
                18
            } else if context.run_state.gold < need.gold_reserve + 40 {
                8
            } else {
                0
            };
            let situational_bonus = if need.shop_distance.is_none_or(|distance| distance > 2) {
                6
            } else {
                0
            };
            MapEvaluation {
                immediate_benefit,
                risk_penalty,
                situational_bonus,
                rationale_key: "route_to_shop",
            }
        }
        RoomType::MonsterRoomElite => {
            let immediate_benefit =
                28 + if hp_ratio >= 0.70 {
                    20
                } else if hp_ratio >= 0.60 {
                    10
                } else {
                    0
                } + if has_emerald_key && !context.run_state.keys[1] {
                    16
                } else {
                    0
                };
            let risk_penalty = if hp_ratio < 0.45 {
                44 + need.survival_pressure / 8
            } else if hp_ratio < 0.60 {
                22
            } else {
                6
            };
            let situational_bonus = if need.elite_distance.is_none_or(|distance| distance > 2) {
                8
            } else {
                0
            };
            MapEvaluation {
                immediate_benefit,
                risk_penalty,
                situational_bonus,
                rationale_key: if has_emerald_key && !context.run_state.keys[1] {
                    "route_to_emerald_elite"
                } else {
                    "route_to_elite"
                },
            }
        }
        RoomType::MonsterRoom => MapEvaluation {
            immediate_benefit: 26,
            risk_penalty: if need.survival_pressure >= 120 { 10 } else { 2 },
            situational_bonus: if need.damage_gap > 0 || need.block_gap > 0 {
                6
            } else {
                0
            },
            rationale_key: "route_to_hallway",
        },
        RoomType::EventRoom => MapEvaluation {
            immediate_benefit: if hp_ratio < 0.40 { 12 } else { 24 },
            risk_penalty: if hp_ratio < 0.40 {
                14
            } else if need.survival_pressure >= 120 {
                6
            } else {
                0
            },
            situational_bonus: if need.missing_keys > 0 { 4 } else { 0 },
            rationale_key: "route_to_event",
        },
        RoomType::TreasureRoom => MapEvaluation {
            immediate_benefit: 34,
            risk_penalty: 0,
            situational_bonus: if need.missing_keys > 0 { 4 } else { 0 },
            rationale_key: "route_to_treasure",
        },
        RoomType::MonsterRoomBoss | RoomType::TrueVictoryRoom => MapEvaluation {
            immediate_benefit: 0,
            risk_penalty: 0,
            situational_bonus: 0,
            rationale_key: "route_terminal",
        },
    }
}

fn node_has_emerald_key(run_state: &RunState, x: i32, y: i32) -> bool {
    run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
        .is_some_and(|node| node.has_emerald_key)
}

fn room_type_at(run_state: &RunState, x: i32, y: i32) -> Option<RoomType> {
    if x < 0 || y < 0 {
        return None;
    }
    run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
        .and_then(|node| node.class)
}
