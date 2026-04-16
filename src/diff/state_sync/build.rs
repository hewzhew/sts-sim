mod cards;
mod monster;

use serde_json::Value;
use std::collections::{HashMap, VecDeque};

use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{
    CombatMeta, CombatRng, CombatState, EngineRuntime, Intent, MonsterEntity, PlayerEntity, Power,
    RelicBuses, TurnRuntime,
};
use crate::runtime::rng::RngPool;

use super::internal_state::{
    initialize_power_internal_state_from_snapshot, initialize_relic_runtime_state,
    seed_monster_internal_state_from_snapshot, snapshot_runtime_amount_for_relic,
    snapshot_runtime_counter_for_relic, snapshot_runtime_used_up_for_relic,
    sync_relic_runtime_state_from_snapshot,
};
use super::rng::sync_rng;
use crate::diff::protocol::{
    java_potion_id_to_rust, power_id_from_java, power_instance_id_from_java, relic_id_from_java,
};
pub(crate) use cards::{
    build_draw_pile_from_snapshot, build_hand_from_snapshot, build_limbo_from_snapshot,
    build_pile_from_ids, build_runtime_hints_from_snapshot,
};
pub(crate) use monster::apply_monster_entity_snapshot;

fn stable_u32_from_str(s: &str) -> u32 {
    let mut hash = 0x811C9DC5u32;
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub fn snapshot_uuid(raw: &Value, fallback: u32) -> u32 {
    if let Some(uuid) = raw.as_u64() {
        uuid as u32
    } else if let Some(uuid) = raw.as_str() {
        stable_u32_from_str(uuid)
    } else {
        fallback
    }
}

pub fn build_powers_from_snapshot(powers_arr: &Value) -> Vec<Power> {
    let mut powers = Vec::new();
    if let Some(arr) = powers_arr.as_array() {
        for p in arr {
            if let Some(pid) = power_id_from_java(p["id"].as_str().unwrap_or("")) {
                let amount = p["amount"].as_i64().unwrap_or(0) as i32;
                let mut power = Power {
                    power_type: pid,
                    instance_id: p["id"].as_str().and_then(power_instance_id_from_java),
                    amount,
                    extra_data: 0,
                    just_applied: p
                        .get("just_applied")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                };
                initialize_power_internal_state_from_snapshot(&mut power, p);
                powers.push(power);
            }
        }
    }
    powers
}

pub fn build_powers_from_snapshot_for_owner(
    owner: crate::core::EntityId,
    powers_arr: &Value,
) -> Vec<Power> {
    let mut powers = build_powers_from_snapshot(powers_arr);
    for power in &mut powers {
        if power.power_type == crate::content::powers::PowerId::Ritual {
            power.extra_data =
                crate::content::powers::core::ritual::infer_extra_data(owner, power.just_applied);
        }
    }
    powers
}

pub fn build_combat_state(snapshot: &Value, relics_val: &Value) -> CombatState {
    let player_val = &snapshot["player"];

    let mut player = PlayerEntity {
        id: 0,
        current_hp: player_val["current_hp"]
            .as_i64()
            .unwrap_or(player_val["hp"].as_i64().unwrap_or(80)) as i32,
        max_hp: player_val["max_hp"].as_i64().unwrap_or(80) as i32,
        block: player_val["block"].as_i64().unwrap_or(0) as i32,
        gold_delta_this_combat: 0,
        gold: 99,
        max_orbs: 0,
        orbs: vec![],
        stance: crate::runtime::combat::StanceId::Neutral,
        relics: vec![],
        relic_buses: RelicBuses::default(),
        energy_master: 3,
    };

    let effective_relics = if snapshot
        .get("relics")
        .and_then(|r| r.as_array())
        .map_or(false, |a| !a.is_empty())
    {
        &snapshot["relics"]
    } else {
        relics_val
    };
    if let Some(relics_arr) = effective_relics.as_array() {
        for r in relics_arr {
            let relic_name = if r.is_string() {
                r.as_str().unwrap()
            } else {
                r["id"].as_str().unwrap_or("")
            };
            if let Some(relic_id) = relic_id_from_java(relic_name) {
                let mut rs = RelicState::new(relic_id);
                initialize_relic_runtime_state(&mut rs);
                sync_relic_runtime_state_from_snapshot(
                    &mut rs,
                    snapshot_runtime_counter_for_relic(relic_id, r),
                    snapshot_runtime_used_up_for_relic(relic_id, r),
                    snapshot_runtime_amount_for_relic(relic_id, r),
                );
                player.add_relic(rs);
            }
        }
    }

    let turn = snapshot.get("turn").and_then(|v| v.as_u64()).unwrap_or(1);
    if turn >= 1 {
        for relic in player.relics.iter_mut() {
            if relic.id == RelicId::Lantern {
                relic.used_up = true;
            }
        }
    }

    let monsters_arr = snapshot["monsters"].as_array().unwrap();
    let mut monsters = Vec::new();
    for (i, m) in monsters_arr.iter().enumerate() {
        let entity_id = i + 1;
        let mut entity = MonsterEntity {
            id: entity_id,
            monster_type: 0,
            current_hp: 0,
            max_hp: 0,
            block: 0,
            slot: i as u8,
            is_dying: false,
            half_dead: false,
            is_escaped: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_dmg: 0,
            logical_position: i as i32,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        };
        apply_monster_entity_snapshot(m, i, &mut entity);
        monsters.push(entity);
    }

    let mut power_db: HashMap<usize, Vec<Power>> = HashMap::new();
    power_db.insert(
        0,
        build_powers_from_snapshot_for_owner(0, &player_val["powers"]),
    );
    for (i, m) in monsters_arr.iter().enumerate() {
        let entity_id = i + 1;
        let mut powers = build_powers_from_snapshot_for_owner(entity_id, &m["powers"]);
        seed_monster_internal_state_from_snapshot(monsters[i].monster_type, m, &mut powers);
        if !powers.is_empty() {
            power_db.insert(entity_id, powers);
        }
    }

    let hand = build_hand_from_snapshot(snapshot);
    let draw_pile = build_draw_pile_from_snapshot(snapshot);
    let discard_pile = build_pile_from_ids("discard_pile_ids", snapshot, 3000);
    let exhaust_pile = build_pile_from_ids("exhaust_pile_ids", snapshot, 4000);

    let mut parsed_potions = vec![None, None, None];
    if let Some(arr) = snapshot.get("potions").and_then(|v| v.as_array()) {
        parsed_potions.clear();
        for (i, p) in arr.iter().enumerate() {
            let pid_str = p["id"].as_str().unwrap_or("Potion Slot");
            if pid_str != "Potion Slot" {
                if let Some(rust_id) = java_potion_id_to_rust(pid_str) {
                    parsed_potions.push(Some(crate::content::potions::Potion::new(
                        rust_id, i as u32,
                    )));
                } else {
                    parsed_potions.push(None);
                }
            } else {
                parsed_potions.push(None);
            }
        }
    }

    let mut rng_pool = RngPool::new(12345);
    sync_rng(&mut rng_pool, snapshot);

    let mut cs = CombatState {
        meta: CombatMeta {
            ascension_level: 0,
            player_class: "Ironclad",
            is_boss_fight: snapshot
                .get("room_type")
                .map_or(false, |s| s.as_str() == Some("MonsterRoomBoss")),
            is_elite_fight: snapshot.get("room_type").map_or(false, |s| {
                s.as_str() == Some("MonsterRoomElite") || s.as_str() == Some("EventRoom")
            }),
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: snapshot["turn"].as_u64().unwrap_or(1) as u32,
            ..TurnRuntime::fresh_player_turn(player_val["energy"].as_u64().unwrap_or(3) as u8)
        },
        zones: crate::runtime::combat::CardZones {
            draw_pile,
            hand,
            discard_pile,
            exhaust_pile,
            limbo: build_limbo_from_snapshot(snapshot),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 5000,
        },
        entities: crate::runtime::combat::EntityState {
            player,
            monsters,
            potions: parsed_potions,
            power_db,
        },
        engine: EngineRuntime::new(),
        rng: CombatRng::new(rng_pool),
        runtime: build_runtime_hints_from_snapshot(snapshot),
    };
    crate::content::relics::restore_combat_energy_master(&mut cs);
    cs.recompute_turn_start_draw_modifier();
    cs.update_hand_cards();
    cs
}
