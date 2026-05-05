use crate::bot::deck_profile::{deck_profile, DeckProfile};
use crate::content::cards;
use crate::map::node::{MapEdge, RoomType};
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunNeedSnapshot {
    pub hp_ratio: f32,
    pub survival_pressure: i32,
    pub damage_gap: i32,
    pub block_gap: i32,
    pub control_gap: i32,
    pub purge_pressure: i32,
    pub upgrade_pressure: i32,
    pub rest_distance: Option<i32>,
    pub shop_distance: Option<i32>,
    pub elite_distance: Option<i32>,
    pub deck_size: usize,
    pub missing_keys: u8,
    pub gold_reserve: i32,
}

pub(crate) fn analyze_run_needs(run_state: &RunState) -> RunNeedSnapshot {
    let profile = deck_profile(run_state);
    let hp_ratio = run_state.current_hp as f32 / run_state.max_hp.max(1) as f32;
    let damage_gap = compute_damage_gap(&profile);
    let block_gap = compute_block_gap(&profile);
    let control_gap = compute_control_gap(&profile);
    let purge_pressure = compute_purge_pressure(run_state);
    let upgrade_pressure = compute_upgrade_pressure(run_state, &profile);
    let rest_distance = nearest_room_distance(run_state, RoomType::RestRoom);
    let shop_distance = nearest_room_distance(run_state, RoomType::ShopRoom);
    let elite_distance = nearest_room_distance(run_state, RoomType::MonsterRoomElite);
    let missing_keys = run_state.keys.iter().filter(|key| !**key).count() as u8;

    let mut survival_pressure = ((0.70 - hp_ratio).max(0.0) * 240.0).round() as i32;
    if damage_gap + block_gap + control_gap >= 40 {
        survival_pressure += 20;
    }
    if elite_distance.is_some_and(|distance| distance <= 2) {
        survival_pressure += 30;
    }
    if rest_distance.is_none() {
        survival_pressure += 20;
    }
    if run_state.act_num >= 3 && missing_keys > 0 {
        survival_pressure += 10;
    }

    let gold_reserve = if hp_ratio < 0.40 {
        90
    } else if purge_pressure >= 110 {
        70
    } else if upgrade_pressure >= 80 {
        45
    } else {
        20
    };

    RunNeedSnapshot {
        hp_ratio,
        survival_pressure,
        damage_gap,
        block_gap,
        control_gap,
        purge_pressure,
        upgrade_pressure,
        rest_distance,
        shop_distance,
        elite_distance,
        deck_size: run_state.master_deck.len(),
        missing_keys,
        gold_reserve,
    }
}

pub(crate) fn nearest_room_distance(run_state: &RunState, target: RoomType) -> Option<i32> {
    let map = &run_state.map;
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    if map.current_y < 0 {
        if let Some(row) = map.graph.first() {
            for node in row {
                if node.class.is_some() || !node.edges.is_empty() {
                    queue.push_back((node.x, node.y, 1));
                }
            }
        }
    } else if let Some(node) = map.get_current_node() {
        for edge in &node.edges {
            queue.push_back((edge.dst_x, edge.dst_y, 1));
        }
    }

    while let Some((x, y, distance)) = queue.pop_front() {
        if !seen.insert((x, y)) {
            continue;
        }
        let Some(room_type) = room_type_at(run_state, x, y) else {
            continue;
        };
        if room_type == target {
            return Some(distance);
        }
        for edge in child_edges(run_state, x, y) {
            queue.push_back((edge.dst_x, edge.dst_y, distance + 1));
        }
    }

    None
}

fn room_type_at(run_state: &RunState, x: i32, y: i32) -> Option<RoomType> {
    if y < 0 || x < 0 {
        return None;
    }
    let row = run_state.map.graph.get(y as usize)?;
    let node = row.get(x as usize)?;
    node.class
}

fn child_edges(run_state: &RunState, x: i32, y: i32) -> Vec<MapEdge> {
    if y < 0 || x < 0 {
        return Vec::new();
    }
    run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
        .map(|node| node.edges.iter().cloned().collect())
        .unwrap_or_default()
}

fn compute_damage_gap(profile: &DeckProfile) -> i32 {
    let damage_signal =
        profile.attack_count * 2 + profile.strength_payoffs * 5 + profile.draw_sources * 2;
    (28 - damage_signal).max(0)
}

fn compute_block_gap(profile: &DeckProfile) -> i32 {
    let block_signal =
        profile.block_core * 4 + profile.block_payoffs * 4 + profile.exhaust_engines * 2;
    (22 - block_signal).max(0)
}

fn compute_control_gap(profile: &DeckProfile) -> i32 {
    let control_signal =
        profile.draw_sources * 2 + profile.power_scalers * 2 + profile.status_payoffs * 3;
    (16 - control_signal).max(0)
}

fn compute_purge_pressure(run_state: &RunState) -> i32 {
    run_state
        .master_deck
        .iter()
        .map(|card| {
            crate::bot::deck_scoring::curse_remove_severity(card.id) * 16
                + i32::from(cards::is_starter_basic(card.id)) * 14
        })
        .sum()
}

fn compute_upgrade_pressure(run_state: &RunState, profile: &DeckProfile) -> i32 {
    let upgradable = run_state
        .master_deck
        .iter()
        .filter(|card| {
            let def = cards::get_card_definition(card.id);
            card.id == crate::content::cards::CardId::SearingBlow
                || (card.upgrades == 0
                    && !matches!(
                        def.card_type,
                        crate::content::cards::CardType::Curse
                            | crate::content::cards::CardType::Status
                    ))
        })
        .count() as i32;
    upgradable * 6 + profile.power_scalers * 4 + profile.draw_sources * 2
}
