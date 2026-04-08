use serde_json::Value;
use std::collections::{HashMap, VecDeque};

use crate::combat::{
    CombatCard, CombatPhase, CombatState, EphemeralCounters, Intent, MonsterEntity, PlayerEntity,
    Power, RelicBuses,
};
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::rng::{RngPool, StsRng};

use super::mapper::{
    card_id_from_java, intent_from_java, java_potion_id_to_rust, monster_id_from_java,
    power_id_from_java, relic_id_from_java,
};

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

fn snapshot_monster_is_half_dead(monster: &Value) -> bool {
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

// ============================================================================
// State Construction from Java Snapshot
// ============================================================================

pub fn build_pile_from_ids(ids_key: &str, snapshot: &Value, base_uuid: u32) -> Vec<CombatCard> {
    let obj_key = ids_key.replace("_ids", "");

    // 1. Try Live Comm style (array of Card Objects)
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

    // 2. Try Replay File style (Array of string IDs)
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

    // 3. Fallback: use size fields with Defend dummies
    let size_key = ids_key.replace("_ids", "_size");
    let size = snapshot
        .get(&size_key)
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    (0..size)
        .map(|i| CombatCard::new(CardId::Defend, base_uuid + i as u32))
        .collect()
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
            // Sync actual cost from Java (handles upgrades, Snecko Eye, Madness, etc.)
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
                powers.push(Power {
                    power_type: pid,
                    amount,
                    extra_data: match pid {
                        crate::content::powers::PowerId::Malleable
                        | crate::content::powers::PowerId::Flight => amount,
                        _ => 0,
                    },
                    just_applied: false,
                });
            }
        }
    }
    powers
}

pub fn build_combat_state(snapshot: &Value, relics_val: &Value) -> CombatState {
    let player_val = &snapshot["player"];

    // Build relics
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
        stance: crate::combat::StanceId::Neutral,
        relics: vec![],
        relic_buses: RelicBuses::default(),
        energy_master: 3,
    };

    // Add relics — prefer per-combat relics from snapshot, fall back to init relics
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
                }
                player.add_relic(rs);
            }
        }
    }
    // Seed internal-only relic state when the snapshot format does not expose it.
    // Centennial Puzzle must remain available until the local simulation actually
    // triggers it; otherwise Sharp Hide / first HP loss parity breaks immediately.
    let turn = snapshot.get("turn").and_then(|v| v.as_u64()).unwrap_or(1);
    if turn >= 1 {
        for relic in player.relics.iter_mut() {
            match relic.id {
                RelicId::Lantern => relic.used_up = true,
                _ => {}
            }
        }
    }

    // Build monsters
    let monsters_arr = snapshot["monsters"].as_array().unwrap();
    let mut monsters = Vec::new();
    for (i, m) in monsters_arr.iter().enumerate() {
        let entity_id = i + 1; // 1-indexed, player is 0
        let intent_dmg = m["move_base_damage"].as_i64().unwrap_or(-1) as i32;
        let intent_hits = m["move_hits"].as_i64().unwrap_or(1) as i32;
        let intent_str = m["intent"].as_str().unwrap_or("UNKNOWN");
        let half_dead = snapshot_monster_is_half_dead(m);
        let is_gone = m.get("is_gone").and_then(|v| v.as_bool()).unwrap_or(false);
        monsters.push(MonsterEntity {
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
        });
    }

    // Build power_db
    let mut power_db: HashMap<usize, Vec<Power>> = HashMap::new();
    power_db.insert(0, build_powers_from_snapshot(&player_val["powers"]));
    for (i, m) in monsters_arr.iter().enumerate() {
        let entity_id = i + 1;
        let mut powers = build_powers_from_snapshot(&m["powers"]);
        if monsters[i].monster_type == crate::content::monsters::EnemyId::TheGuardian as usize {
            let java_mode_shift = powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::ModeShift)
                .map(|p| p.amount);
            if let Some(amount) = java_mode_shift {
                powers.push(crate::combat::Power {
                    power_type: crate::content::powers::PowerId::GuardianThreshold,
                    amount,
                    extra_data: 0,
                    just_applied: false,
                });
            }
        }
        if !powers.is_empty() {
            power_db.insert(entity_id, powers);
        }
    }

    // Build hand
    let hand = build_hand_from_snapshot(snapshot);
    let draw_pile = build_pile_from_ids("draw_pile_ids", snapshot, 2000);
    let discard_pile = build_pile_from_ids("discard_pile_ids", snapshot, 3000);
    let exhaust_pile = build_pile_from_ids("exhaust_pile_ids", snapshot, 4000);

    // Build potions
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
        let parse_rng = |name: &str| -> Option<crate::rng::StsRng> {
            rng_state.get(name).map(|v| crate::rng::StsRng {
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
        ascension_level: 0,
        turn_count: snapshot["turn"].as_u64().unwrap_or(1) as u32,
        current_phase: CombatPhase::PlayerTurn,
        energy: player_val["energy"].as_u64().unwrap_or(3) as u8,
        draw_pile,
        hand,
        discard_pile,
        exhaust_pile,
        limbo: vec![],
        player,
        monsters,
        potions: parsed_potions,
        power_db,
        action_queue: VecDeque::new(),
        counters: EphemeralCounters::default(),
        card_uuid_counter: 5000,
        rng: rng_pool,
        is_boss_fight: snapshot
            .get("room_type")
            .map_or(false, |s| s.as_str() == Some("MonsterRoomBoss")),
        is_elite_fight: snapshot.get("room_type").map_or(false, |s| {
            s.as_str() == Some("MonsterRoomElite") || s.as_str() == Some("EventRoom")
        }),
        meta_changes: Vec::new(),
    };
    cs.update_hand_cards();
    cs
}

pub fn carry_internal_runtime_state(previous: &CombatState, next: &mut CombatState) {
    for monster in &next.monsters {
        let Some(prev_powers) = previous.power_db.get(&monster.id) else {
            continue;
        };
        let next_powers = next.power_db.entry(monster.id).or_default();

        if monster.monster_type == crate::content::monsters::EnemyId::TheGuardian as usize
            && !next_powers
                .iter()
                .any(|p| p.power_type == crate::content::powers::PowerId::GuardianThreshold)
        {
            if let Some(prev_threshold) = prev_powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::GuardianThreshold)
                .cloned()
            {
                next_powers.push(prev_threshold);
            }
        }

        if let Some(next_malleable) = next_powers
            .iter_mut()
            .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
        {
            if let Some(prev_malleable) = prev_powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
            {
                next_malleable.extra_data = prev_malleable.extra_data;
            }
        }

        if let Some(next_flight) = next_powers
            .iter_mut()
            .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
        {
            if let Some(prev_flight) = prev_powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
            {
                next_flight.extra_data = prev_flight.extra_data;
            }
        }
    }
}

// ============================================================================
// State Sync: Overwrite Rust state with Java snapshot before each action
// ============================================================================

pub fn sync_state(cs: &mut CombatState, snapshot: &Value) {
    let player_val = &snapshot["player"];

    // Sync player
    cs.player.current_hp = player_val["current_hp"].as_i64().unwrap_or(
        player_val["hp"]
            .as_i64()
            .unwrap_or(cs.player.current_hp as i64),
    ) as i32;
    cs.player.max_hp = player_val["max_hp"]
        .as_i64()
        .unwrap_or(cs.player.max_hp as i64) as i32;
    cs.player.block = player_val["block"].as_i64().unwrap_or(0) as i32;
    cs.energy = player_val["energy"].as_u64().unwrap_or(3) as u8;

    // Sync player powers
    let existing_player_powers = cs.power_db.get(&0).cloned();
    let mut player_powers = build_powers_from_snapshot(&player_val["powers"]);
    if let Some(existing_base) = existing_player_powers.as_ref().and_then(|ps| {
        ps.iter()
            .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
            .map(|p| p.extra_data)
    }) {
        if let Some(malleable) = player_powers
            .iter_mut()
            .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
        {
            malleable.extra_data = existing_base;
        }
    }
    cs.power_db.insert(0, player_powers);

    // Sync hand
    cs.hand = build_hand_from_snapshot(snapshot);

    // Sync monsters — handle count changes (spawns/splits)
    let monsters_arr = snapshot["monsters"].as_array().unwrap();

    while cs.monsters.len() < monsters_arr.len() {
        let new_id = cs.monsters.len() + 1;
        cs.monsters.push(MonsterEntity {
            id: new_id,
            monster_type: 0,
            current_hp: 0,
            max_hp: 0,
            block: 0,
            slot: cs.monsters.len() as u8,
            is_dying: false,
            half_dead: false,
            is_escaped: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_dmg: 0,
            logical_position: 0,
        });
    }
    while cs.monsters.len() > monsters_arr.len() {
        cs.monsters.pop();
    }

    for (i, m_val) in monsters_arr.iter().enumerate() {
        let existing_internal_powers = cs.power_db.get(&cs.monsters[i].id).cloned();
        let half_dead = snapshot_monster_is_half_dead(m_val);
        let is_gone = m_val
            .get("is_gone")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        cs.monsters[i].current_hp = m_val["current_hp"]
            .as_i64()
            .unwrap_or(m_val["hp"].as_i64().unwrap_or(0))
            as i32;
        cs.monsters[i].max_hp = m_val["max_hp"].as_i64().unwrap_or(0) as i32;
        cs.monsters[i].block = m_val["block"].as_i64().unwrap_or(0) as i32;
        cs.monsters[i].is_dying = is_gone && !half_dead;
        cs.monsters[i].half_dead = half_dead;
        cs.monsters[i].monster_type = monster_id_from_java(m_val["id"].as_str().unwrap_or(""))
            .map(|e| e as usize)
            .unwrap_or(0);

        if let Some(move_id) = m_val.get("move_id").and_then(|v| v.as_u64()) {
            cs.monsters[i].next_move_byte = move_id as u8;
        }
        let intent_dmg = m_val["move_base_damage"].as_i64().unwrap_or(-1) as i32;
        let intent_hits = m_val["move_hits"].as_i64().unwrap_or(1) as i32;
        let intent_str = m_val["intent"].as_str().unwrap_or("UNKNOWN");
        cs.monsters[i].current_intent = intent_from_java(intent_str, intent_dmg, intent_hits);
        cs.monsters[i].intent_dmg = m_val["move_adjusted_damage"].as_i64().unwrap_or(0) as i32;

        let entity_id = cs.monsters[i].id;
        let mut powers = build_powers_from_snapshot(&m_val["powers"]);

        if cs.monsters[i].monster_type == crate::content::monsters::EnemyId::TheGuardian as usize {
            let existing_threshold = existing_internal_powers.as_ref().and_then(|ps| {
                ps.iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::GuardianThreshold)
                    .cloned()
            });
            let java_mode_shift = powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::ModeShift)
                .map(|p| p.amount);

            match java_mode_shift {
                Some(amount) => {
                    if let Some(existing_threshold) = existing_threshold {
                        powers.push(existing_threshold);
                    } else {
                        powers.push(crate::combat::Power {
                            power_type: crate::content::powers::PowerId::GuardianThreshold,
                            amount,
                            extra_data: 0,
                            just_applied: false,
                        });
                    }
                }
                None => {
                    if let Some(existing_threshold) = existing_threshold {
                        powers.push(existing_threshold);
                    }
                }
            }
        }

        let existing_malleable_base = existing_internal_powers.as_ref().and_then(|ps| {
            ps.iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
                .map(|p| p.extra_data)
        });
        if let Some(malleable) = powers
            .iter_mut()
            .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
        {
            malleable.extra_data = existing_malleable_base.unwrap_or(malleable.amount);
        }

        let existing_flight_base = existing_internal_powers.as_ref().and_then(|ps| {
            ps.iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
                .map(|p| p.extra_data)
        });
        if let Some(flight) = powers
            .iter_mut()
            .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
        {
            flight.extra_data = existing_flight_base.unwrap_or(flight.amount);
        }

        if !powers.is_empty() {
            cs.power_db.insert(entity_id, powers);
        } else {
            cs.power_db.remove(&entity_id);
        }
    }

    cs.update_hand_cards();

    // Sync piles
    cs.draw_pile = build_pile_from_ids("draw_pile_ids", snapshot, 2000);
    cs.discard_pile = build_pile_from_ids("discard_pile_ids", snapshot, 3000);
    cs.exhaust_pile = build_pile_from_ids("exhaust_pile_ids", snapshot, 4000);

    sync_rng(&mut cs.rng, snapshot);

    if let Some(potions_arr) = snapshot.get("potions").and_then(|v| v.as_array()) {
        for (i, p_val) in potions_arr.iter().enumerate() {
            if i < cs.potions.len() {
                cs.potions[i] = p_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(java_potion_id_to_rust)
                    .map(|id| crate::content::potions::Potion::new(id, 0));
            }
        }
    }

    if let Some(relics_arr) = snapshot.get("relics").and_then(|v| v.as_array()) {
        for r_val in relics_arr {
            if r_val.is_null() {
                continue;
            }
            if let Some(relic_name) = r_val.get("id").and_then(|v| v.as_str()) {
                if let Some(relic_id) = relic_id_from_java(relic_name) {
                    if let Some(rs) = cs.player.relics.iter_mut().find(|r| r.id == relic_id) {
                        if let Some(counter) = r_val.get("counter").and_then(|c| c.as_i64()) {
                            rs.counter = counter as i32;
                        }
                    }
                }
            }
        }
    }

    cs.action_queue.clear();
    cs.current_phase = CombatPhase::PlayerTurn;
}

pub fn sync_rng(rng: &mut RngPool, snapshot: &Value) {
    let rng_state = match snapshot.get("rng_state") {
        Some(v) if v.is_object() && !v.as_object().unwrap().is_empty() => v,
        _ => return,
    };

    if let Some(ai) = rng_state.get("ai_rng") {
        sync_rng_channel(&mut rng.ai_rng, ai);
    }
    if let Some(shuffle) = rng_state.get("shuffle_rng") {
        sync_rng_channel(&mut rng.shuffle_rng, shuffle);
    }
    if let Some(card) = rng_state.get("card_rng") {
        sync_rng_channel(&mut rng.card_random_rng, card);
    }
    if let Some(misc) = rng_state.get("misc_rng") {
        sync_rng_channel(&mut rng.misc_rng, misc);
    }
    if let Some(monster_hp) = rng_state.get("monster_hp_rng") {
        sync_rng_channel(&mut rng.monster_hp_rng, monster_hp);
    }
    if let Some(potion) = rng_state.get("potion_rng") {
        sync_rng_channel(&mut rng.potion_rng, potion);
    }
}

fn sync_rng_channel(rng: &mut StsRng, json: &Value) {
    if let Some(s0) = json.get("seed0").and_then(|v| v.as_i64()) {
        rng.seed0 = s0 as u64;
    }
    if let Some(s1) = json.get("seed1").and_then(|v| v.as_i64()) {
        rng.seed1 = s1 as u64;
    }
    if let Some(c) = json.get("counter").and_then(|v| v.as_u64()) {
        rng.counter = c as u32;
    }
}
