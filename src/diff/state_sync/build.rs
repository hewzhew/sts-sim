use serde_json::Value;
use std::collections::{HashMap, VecDeque};

use crate::runtime::combat::{
    CombatCard, CombatMeta, CombatPhase, CombatRng, CombatRuntimeHints, CombatState, EngineRuntime,
    EphemeralCounters, MonsterEntity, PlayerEntity, Power, QueuedCardHint, RelicBuses, TurnRuntime,
};
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::rng::RngPool;

use crate::diff::protocol::{
    card_id_from_java, intent_from_java, java_potion_id_to_rust, monster_id_from_java,
    power_id_from_java, power_instance_id_from_java, relic_id_from_java,
};
use super::internal_state::{
    initialize_power_internal_state_from_snapshot, initialize_relic_runtime_state,
    seed_monster_internal_state_from_snapshot, snapshot_runtime_amount_for_relic,
    snapshot_runtime_counter_for_relic, snapshot_runtime_used_up_for_relic,
};

pub(crate) fn seed_hexaghost_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if entity.monster_type != crate::content::monsters::EnemyId::Hexaghost as usize {
        return;
    }

    if let Some(value) = monster.get("hexaghost_activated").and_then(|v| v.as_bool()) {
        entity.hexaghost.activated = value;
    }
    if let Some(value) = monster
        .get("hexaghost_orb_active_count")
        .and_then(|v| v.as_u64())
    {
        entity.hexaghost.orb_active_count = value as u8;
    }
    if let Some(value) = monster
        .get("hexaghost_burn_upgraded")
        .and_then(|v| v.as_bool())
    {
        entity.hexaghost.burn_upgraded = value;
    }
}

pub(crate) fn seed_darkling_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if entity.monster_type != crate::content::monsters::EnemyId::Darkling as usize {
        return;
    }

    if let Some(value) = monster.get("darkling_first_move").and_then(|v| v.as_bool()) {
        entity.darkling.first_move = value;
    }
    if let Some(value) = monster.get("darkling_nip_dmg").and_then(|v| v.as_i64()) {
        entity.darkling.nip_dmg = value as i32;
    }
}

pub(crate) fn seed_chosen_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if entity.monster_type != crate::content::monsters::EnemyId::Chosen as usize {
        return;
    }

    let mut seeded = false;
    if let Some(value) = monster.get("chosen_first_turn").and_then(|v| v.as_bool()) {
        seeded = true;
        entity.chosen.first_turn = value;
    }
    if let Some(value) = monster.get("chosen_used_hex").and_then(|v| v.as_bool()) {
        seeded = true;
        entity.chosen.used_hex = value;
    }
    if seeded {
        entity.chosen.protocol_seeded = true;
    }
}

pub(crate) fn seed_lagavulin_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if entity.monster_type != crate::content::monsters::EnemyId::Lagavulin as usize {
        return;
    }

    if let Some(value) = monster.get("lagavulin_idle_count").and_then(|v| v.as_u64()) {
        entity.lagavulin.idle_count = value as u8;
    }
    if let Some(value) = monster
        .get("lagavulin_is_out_triggered")
        .and_then(|v| v.as_bool())
    {
        entity.lagavulin.is_out_triggered = value;
    }
}

pub(crate) fn seed_move_history_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let intent_hidden = monster
        .get("intent")
        .and_then(|v| v.as_str())
        .is_some_and(|intent| intent == "NONE");
    let current_move_id = monster.get("move_id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let second_last_move_id = monster
        .get("second_last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let last_move_id = monster
        .get("last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    entity.move_history.clear();
    if intent_hidden && current_move_id == 0 {
        return;
    }
    if second_last_move_id != 0 {
        entity.move_history.push_back(second_last_move_id);
    }
    if last_move_id != 0 {
        entity.move_history.push_back(last_move_id);
    }
    if current_move_id != 0 {
        entity.move_history.push_back(current_move_id);
    }
}

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

pub(crate) fn snapshot_monster_is_half_dead(monster: &Value) -> bool {
    if monster
        .get("half_dead")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return true;
    }

    if !monster
        .get("is_gone")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return false;
    }

    let monster_id = monster["id"].as_str().unwrap_or("");
    let move_id = monster.get("move_id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    (monster_id == "AwakenedOne" && move_id == 3)
        || (monster_id == "Darkling" && matches!(move_id, 4 | 5))
}

fn snapshot_i32(monster: &Value, key: &str) -> Option<i32> {
    monster
        .get(key)
        .and_then(|v| v.as_i64().map(|value| value as i32))
        .or_else(|| {
            monster
                .get(key)
                .and_then(|v| v.as_f64().map(|value| value.round() as i32))
        })
}

pub(crate) fn seed_monster_protocol_identity_from_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) {
    entity.protocol_identity.instance_id =
        monster.get("monster_instance_id").and_then(|v| v.as_u64());
    entity.protocol_identity.spawn_order = monster
        .get("spawn_order")
        .and_then(|v| v.as_u64())
        .or(entity.protocol_identity.instance_id);
    entity.protocol_identity.draw_x = snapshot_i32(monster, "draw_x");
    entity.protocol_identity.group_index = monster
        .get("monster_index")
        .and_then(|v| v.as_u64())
        .map(|value| value as usize)
        .or(Some(index));

    if let Some(draw_x) = entity.protocol_identity.draw_x {
        entity.logical_position = draw_x;
    } else if entity.logical_position == 0 {
        entity.logical_position = index as i32;
    }
}

pub fn build_pile_from_ids(ids_key: &str, snapshot: &Value, base_uuid: u32) -> Vec<CombatCard> {
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

pub fn build_limbo_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
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

pub(crate) fn build_runtime_hints_from_snapshot(snapshot: &Value) -> CombatRuntimeHints {
    let using_card = snapshot
        .get("using_card")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let colorless_combat_pool = snapshot
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

    let card_queue = snapshot
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

    CombatRuntimeHints {
        using_card,
        card_queue,
        colorless_combat_pool,
        ..CombatRuntimeHints::default()
    }
}

pub fn build_draw_pile_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
    let mut pile = build_pile_from_ids("draw_pile_ids", snapshot, 2000);
    // Java combat snapshots serialize draw_pile from bottom to top, while the
    // engine draws from index 0 as the top card. Reverse on ingest so live
    // prediction draws the same next card Java will draw.
    pile.reverse();
    pile
}

pub fn build_hand_from_snapshot(snapshot: &Value) -> Vec<CombatCard> {
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
                if let Some(counter) = r.get("counter").and_then(|c| c.as_i64()) {
                    rs.counter = counter as i32;
                    initialize_relic_runtime_state(&mut rs);
                }
                if let Some(runtime_counter) = snapshot_runtime_counter_for_relic(relic_id, r) {
                    rs.counter = runtime_counter;
                }
                let snapshot_used_up = r.get("used_up").and_then(|v| v.as_bool());
                let runtime_used_up = snapshot_runtime_used_up_for_relic(relic_id, r);
                let runtime_amount = snapshot_runtime_amount_for_relic(relic_id, r);
                if let Some(runtime_used_up) = runtime_used_up {
                    rs.used_up = runtime_used_up;
                } else if let Some(used_up) = snapshot_used_up {
                    rs.used_up = used_up;
                }
                if let Some(runtime_amount) = runtime_amount {
                    rs.amount = runtime_amount;
                }
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
        let intent_dmg = m["move_base_damage"].as_i64().unwrap_or(-1) as i32;
        let intent_hits = m["move_hits"].as_i64().unwrap_or(1) as i32;
        let intent_str = m["intent"].as_str().unwrap_or("UNKNOWN");
        let half_dead = snapshot_monster_is_half_dead(m);
        let is_gone = m.get("is_gone").and_then(|v| v.as_bool()).unwrap_or(false);
        let mut entity = MonsterEntity {
            id: entity_id,
            monster_type: monster_id_from_java(m["id"].as_str().unwrap_or(""))
                .map(|e| e as usize)
                .unwrap_or(0),
            current_hp: m["current_hp"]
                .as_i64()
                .unwrap_or(m["hp"].as_i64().unwrap_or(0)) as i32,
            max_hp: m["max_hp"].as_i64().unwrap_or(0) as i32,
            block: m["block"].as_i64().unwrap_or(0) as i32,
            slot: i as u8,
            is_dying: is_gone && !half_dead,
            half_dead,
            is_escaped: false,
            next_move_byte: m["move_id"].as_u64().unwrap_or(0) as u8,
            current_intent: intent_from_java(intent_str, intent_dmg, intent_hits),
            move_history: VecDeque::new(),
            intent_dmg: m["move_adjusted_damage"].as_i64().unwrap_or(0) as i32,
            logical_position: i as i32,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        };
        seed_monster_protocol_identity_from_snapshot(m, i, &mut entity);
        seed_move_history_from_snapshot(m, &mut entity);
        seed_hexaghost_runtime_from_snapshot(m, &mut entity);
        seed_chosen_runtime_from_snapshot(m, &mut entity);
        seed_darkling_runtime_from_snapshot(m, &mut entity);
        seed_lagavulin_runtime_from_snapshot(m, &mut entity);
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
    if let Some(rng_state) = snapshot.get("rng_state") {
        let parse_rng = |name: &str| -> Option<crate::runtime::rng::StsRng> {
            rng_state.get(name).map(|v| crate::runtime::rng::StsRng {
                seed0: v.get("seed0").and_then(|x| x.as_i64()).unwrap_or(0) as u64,
                seed1: v.get("seed1").and_then(|x| x.as_i64()).unwrap_or(0) as u64,
                counter: v.get("counter").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            })
        };
        if let Some(r) = parse_rng("ai_rng") {
            rng_pool.ai_rng = r;
        }
        if let Some(r) = parse_rng("shuffle_rng") {
            rng_pool.shuffle_rng = r;
        }
        if let Some(r) = parse_rng("card_rng") {
            rng_pool.card_random_rng = r;
        }
        if let Some(r) = parse_rng("misc_rng") {
            rng_pool.misc_rng = r;
        }
        if let Some(r) = parse_rng("monster_hp_rng") {
            rng_pool.monster_hp_rng = r;
        }
        if let Some(r) = parse_rng("potion_rng") {
            rng_pool.potion_rng = r;
        }
    }

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
            current_phase: CombatPhase::PlayerTurn,
            energy: player_val["energy"].as_u64().unwrap_or(3) as u8,
            turn_start_draw_modifier: 0,
            counters: EphemeralCounters::default(),
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
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(rng_pool),
        runtime: build_runtime_hints_from_snapshot(snapshot),
    };
    crate::content::relics::restore_combat_energy_master(&mut cs);
    cs.recompute_turn_start_draw_modifier();
    cs.update_hand_cards();
    cs
}
