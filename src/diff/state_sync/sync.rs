use serde_json::Value;
use std::collections::VecDeque;

use crate::combat::{CombatPhase, Intent, MonsterEntity};

use super::super::mapper::{
    intent_from_java, java_potion_id_to_rust, monster_id_from_java, relic_id_from_java,
};
use super::build::{
    build_draw_pile_from_snapshot, build_hand_from_snapshot, build_pile_from_ids,
    build_powers_from_snapshot, seed_darkling_runtime_from_snapshot,
    seed_hexaghost_runtime_from_snapshot,
    seed_move_history_from_snapshot, snapshot_monster_is_half_dead,
};
use super::internal_state::{
    sync_monster_internal_state_from_snapshot, sync_power_extra_data_from_snapshot,
    sync_power_extra_data_from_snapshot_power, sync_relic_runtime_state_from_snapshot,
};
use super::rng::sync_rng;
use crate::content::powers::store;

pub fn sync_state(cs: &mut crate::combat::CombatState, snapshot: &Value) {
    let player_val = &snapshot["player"];

    cs.entities.player.current_hp = player_val["current_hp"].as_i64().unwrap_or(
        player_val["hp"]
            .as_i64()
            .unwrap_or(cs.entities.player.current_hp as i64),
    ) as i32;
    cs.entities.player.max_hp = player_val["max_hp"]
        .as_i64()
        .unwrap_or(cs.entities.player.max_hp as i64) as i32;
    cs.entities.player.block = player_val["block"].as_i64().unwrap_or(0) as i32;
    cs.turn.energy = player_val["energy"].as_u64().unwrap_or(3) as u8;

    let existing_player_powers = cs.entities.power_db.get(&0).cloned();
    let mut player_powers = build_powers_from_snapshot(&player_val["powers"]);
    sync_power_extra_data_from_snapshot(existing_player_powers.as_deref(), &mut player_powers);
    if let Some(snapshot_powers) = player_val["powers"].as_array() {
        for snapshot_power in snapshot_powers {
            if let Some(pid_str) = snapshot_power.get("id").and_then(|v| v.as_str()) {
                if let Some(pid) = crate::diff::mapper::power_id_from_java(pid_str) {
                    if let Some(power) = player_powers
                        .iter_mut()
                        .find(|power| power.power_type == pid)
                    {
                        sync_power_extra_data_from_snapshot_power(power, snapshot_power);
                    }
                }
            }
        }
    }
    store::set_powers_for(cs, 0, player_powers);

    cs.zones.hand = build_hand_from_snapshot(snapshot);

    let monsters_arr = snapshot["monsters"].as_array().unwrap();

    while cs.entities.monsters.len() < monsters_arr.len() {
        let new_id = cs.entities.monsters.len() + 1;
        cs.entities.monsters.push(MonsterEntity {
            id: new_id,
            monster_type: 0,
            current_hp: 0,
            max_hp: 0,
            block: 0,
            slot: cs.entities.monsters.len() as u8,
            is_dying: false,
            half_dead: false,
            is_escaped: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_dmg: 0,
            logical_position: 0,
            hexaghost: Default::default(),
            darkling: Default::default(),
        });
    }
    while cs.entities.monsters.len() > monsters_arr.len() {
        cs.entities.monsters.pop();
    }

    for (i, m_val) in monsters_arr.iter().enumerate() {
        let existing_internal_powers = cs
            .entities
            .power_db
            .get(&cs.entities.monsters[i].id)
            .cloned();
        let half_dead = snapshot_monster_is_half_dead(m_val);
        let is_gone = m_val
            .get("is_gone")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        cs.entities.monsters[i].current_hp = m_val["current_hp"]
            .as_i64()
            .unwrap_or(m_val["hp"].as_i64().unwrap_or(0))
            as i32;
        cs.entities.monsters[i].max_hp = m_val["max_hp"].as_i64().unwrap_or(0) as i32;
        cs.entities.monsters[i].block = m_val["block"].as_i64().unwrap_or(0) as i32;
        cs.entities.monsters[i].is_dying = is_gone && !half_dead;
        cs.entities.monsters[i].half_dead = half_dead;
        cs.entities.monsters[i].monster_type =
            monster_id_from_java(m_val["id"].as_str().unwrap_or(""))
                .map(|e| e as usize)
                .unwrap_or(0);

        if let Some(move_id) = m_val.get("move_id").and_then(|v| v.as_u64()) {
            cs.entities.monsters[i].next_move_byte = move_id as u8;
        }
        let intent_dmg = m_val["move_base_damage"].as_i64().unwrap_or(-1) as i32;
        let intent_hits = m_val["move_hits"].as_i64().unwrap_or(1) as i32;
        let intent_str = m_val["intent"].as_str().unwrap_or("UNKNOWN");
        cs.entities.monsters[i].current_intent =
            intent_from_java(intent_str, intent_dmg, intent_hits);
        cs.entities.monsters[i].intent_dmg =
            m_val["move_adjusted_damage"].as_i64().unwrap_or(0) as i32;
        seed_move_history_from_snapshot(m_val, &mut cs.entities.monsters[i]);
        seed_hexaghost_runtime_from_snapshot(m_val, &mut cs.entities.monsters[i]);
        seed_darkling_runtime_from_snapshot(m_val, &mut cs.entities.monsters[i]);

        let entity_id = cs.entities.monsters[i].id;
        let mut powers = build_powers_from_snapshot(&m_val["powers"]);
        sync_monster_internal_state_from_snapshot(
            cs.entities.monsters[i].monster_type,
            existing_internal_powers.as_deref(),
            m_val,
            &mut powers,
        );
        if let Some(snapshot_powers) = m_val["powers"].as_array() {
            for snapshot_power in snapshot_powers {
                if let Some(pid_str) = snapshot_power.get("id").and_then(|v| v.as_str()) {
                    if let Some(pid) = crate::diff::mapper::power_id_from_java(pid_str) {
                        if let Some(power) = powers.iter_mut().find(|power| power.power_type == pid)
                        {
                            sync_power_extra_data_from_snapshot_power(power, snapshot_power);
                        }
                    }
                }
            }
        }

        if !powers.is_empty() {
            store::set_powers_for(cs, entity_id, powers);
        } else {
            store::remove_entity_powers(cs, entity_id);
        }
    }

    cs.update_hand_cards();

    cs.zones.draw_pile = build_draw_pile_from_snapshot(snapshot);
    cs.zones.discard_pile = build_pile_from_ids("discard_pile_ids", snapshot, 3000);
    cs.zones.exhaust_pile = build_pile_from_ids("exhaust_pile_ids", snapshot, 4000);

    sync_rng(&mut cs.rng, snapshot);

    if let Some(potions_arr) = snapshot.get("potions").and_then(|v| v.as_array()) {
        for (i, p_val) in potions_arr.iter().enumerate() {
            if i < cs.entities.potions.len() {
                cs.entities.potions[i] = p_val
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
                    if let Some(rs) = cs
                        .entities
                        .player
                        .relics
                        .iter_mut()
                        .find(|r| r.id == relic_id)
                    {
                        if let Some(counter) = r_val.get("counter").and_then(|c| c.as_i64()) {
                            let previous_relic = rs.clone();
                            sync_relic_runtime_state_from_snapshot(
                                Some(&previous_relic),
                                rs,
                                counter as i32,
                            );
                        }
                    }
                }
            }
        }
    }
    crate::content::relics::restore_combat_energy_master(cs);

    cs.engine.action_queue.clear();
    cs.turn.current_phase = CombatPhase::PlayerTurn;
}
