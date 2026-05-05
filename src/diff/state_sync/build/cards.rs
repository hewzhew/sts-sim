use serde_json::Value;

use crate::content::cards::CardId;
use crate::protocol::java::card_id_from_java;
use crate::runtime::combat::{CombatCard, CombatRuntimeHints, QueuedCardHint};

use super::snapshot_uuid;

pub(crate) fn build_pile_from_ids(
    ids_key: &str,
    snapshot: &Value,
    base_uuid: u32,
) -> Vec<CombatCard> {
    let obj_key = ids_key.replace("_ids", "");

    if let Some(arr) = snapshot.get(&obj_key).and_then(|v| v.as_array()) {
        let mut pile = Vec::new();
        for (i, card_val) in arr.iter().enumerate() {
            let id_str = card_val["id"].as_str().unwrap_or("Defend_R");
            if let Some(card_id) = card_id_from_java(id_str) {
                let mut card = CombatCard::new(
                    card_id,
                    snapshot_uuid(&card_val["uuid"], base_uuid + i as u32),
                );
                card.upgrades = card_val["upgrades"].as_u64().unwrap_or(0) as u8;
                card.misc_value = card_val["misc"].as_i64().unwrap_or(0) as i32;
                if let Some(base_damage) = card_val.get("base_damage").and_then(|v| v.as_i64()) {
                    card.base_damage_override = Some(base_damage as i32);
                }
                if let Some(cost) = card_val["cost"].as_i64() {
                    let def = crate::content::cards::get_card_definition(card_id);
                    if cost != def.cost as i64 {
                        card.cost_for_turn = Some(cost as u8);
                    }
                }
                pile.push(card);
            }
        }
        return pile;
    }

    if let Some(arr) = snapshot.get(ids_key).and_then(|v| v.as_array()) {
        return arr
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let id_str = v.as_str().unwrap_or("Defend_R");
                let card_id = card_id_from_java(id_str).unwrap_or(CardId::Defend);
                CombatCard::new(card_id, base_uuid + i as u32)
            })
            .collect();
    }

    let size_key = ids_key.replace("_ids", "_size");
    let size = snapshot
        .get(&size_key)
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    (0..size)
        .map(|i| CombatCard::new(CardId::Defend, base_uuid + i as u32))
        .collect()
}

fn build_card_from_snapshot_value(card_val: &Value, fallback_uuid: u32) -> Option<CombatCard> {
    let id_str = card_val
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("Defend_R");
    let card_id = card_id_from_java(id_str)?;
    let mut card = CombatCard::new(
        card_id,
        card_val
            .get("uuid")
            .map(|uuid| snapshot_uuid(uuid, fallback_uuid))
            .unwrap_or(fallback_uuid),
    );
    card.upgrades = card_val
        .get("upgrades")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    card.misc_value = card_val.get("misc").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if let Some(base_damage) = card_val.get("base_damage").and_then(|v| v.as_i64()) {
        card.base_damage_override = Some(base_damage as i32);
    }
    if let Some(cost) = card_val.get("cost").and_then(|v| v.as_i64()) {
        let def = crate::content::cards::get_card_definition(card_id);
        if cost != def.cost as i64 {
            card.cost_for_turn = Some(cost as u8);
        }
    }
    Some(card)
}

pub(crate) fn build_limbo_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
    let mut limbo = Vec::new();
    let mut next_fallback_uuid = 4500u32;

    let mut collect_from_powers = |powers: &Value| {
        if let Some(arr) = powers.as_array() {
            for power in arr {
                if power.get("id").and_then(|v| v.as_str()) != Some("Stasis") {
                    continue;
                }
                let Some(card_val) = power.get("card") else {
                    continue;
                };
                if let Some(card) = build_card_from_snapshot_value(card_val, next_fallback_uuid) {
                    if !limbo
                        .iter()
                        .any(|existing: &CombatCard| existing.uuid == card.uuid)
                    {
                        limbo.push(card);
                        next_fallback_uuid = next_fallback_uuid.saturating_add(1);
                    }
                }
            }
        }
    };

    collect_from_powers(&snapshot["player"]["powers"]);
    if let Some(monsters) = snapshot.get("monsters").and_then(|v| v.as_array()) {
        for monster in monsters {
            collect_from_powers(&monster["powers"]);
        }
    }

    limbo
}

pub(crate) fn build_runtime_hints_from_snapshots(
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) -> CombatRuntimeHints {
    let using_card = observation_snapshot
        .get("using_card")
        .and_then(|v| v.as_bool())
        .or_else(|| truth_snapshot.get("using_card").and_then(|v| v.as_bool()))
        .unwrap_or(false);

    let colorless_combat_pool = truth_snapshot
        .get("colorless_combat_pool")
        .and_then(|v| v.as_array())
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card.get("id").and_then(|v| v.as_str()))
                .filter_map(card_id_from_java)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let card_queue = truth_snapshot
        .get("card_queue")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let card = item.get("card")?;
                    let card_id = card.get("id").and_then(|v| v.as_str())?;
                    let card_id = card_id_from_java(card_id)?;
                    Some(QueuedCardHint {
                        card_uuid: snapshot_uuid(&card["uuid"], 0),
                        card_id,
                        target_monster_index: item
                            .get("monster_index")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize),
                        energy_on_use: item
                            .get("energy_on_use")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32,
                        ignore_energy_total: item
                            .get("ignore_energy_total")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        autoplay: item
                            .get("autoplay")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        random_target: item
                            .get("random_target")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        is_end_turn_autoplay: item
                            .get("is_end_turn_autoplay")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        purge_on_use: item
                            .get("purge_on_use")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let combat_smoked = truth_snapshot
        .get("room_smoked")
        .and_then(|v| v.as_bool())
        .or_else(|| {
            observation_snapshot
                .get("room_smoked")
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    CombatRuntimeHints {
        using_card,
        card_queue,
        colorless_combat_pool,
        combat_smoked,
        ..CombatRuntimeHints::default()
    }
}

pub(crate) fn build_draw_pile_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
    let mut pile = build_pile_from_ids("draw_pile_ids", snapshot, 2000);
    pile.reverse();
    pile
}

pub(crate) fn build_hand_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
    let hand_arr = snapshot["hand"].as_array().unwrap();
    let mut hand = Vec::new();
    for (i, card_val) in hand_arr.iter().enumerate() {
        let card_id_str = card_val["id"].as_str().unwrap_or("Strike_R");
        if let Some(card_id) = card_id_from_java(card_id_str) {
            let mut card =
                CombatCard::new(card_id, snapshot_uuid(&card_val["uuid"], i as u32 + 1000));
            card.upgrades = card_val["upgrades"].as_u64().unwrap_or(0) as u8;
            card.misc_value = card_val["misc"].as_i64().unwrap_or(0) as i32;
            if let Some(base_damage) = card_val.get("base_damage").and_then(|v| v.as_i64()) {
                card.base_damage_override = Some(base_damage as i32);
            }
            if let Some(cost) = card_val["cost"].as_i64() {
                let def = crate::content::cards::get_card_definition(card_id);
                if cost != def.cost as i64 {
                    card.cost_for_turn = Some(cost as u8);
                }
            }
            hand.push(card);
        }
    }
    hand
}
