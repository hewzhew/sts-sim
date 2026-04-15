use serde_json::Value;

use crate::diff::protocol::{intent_from_java, monster_id_from_java};
use crate::runtime::combat::MonsterEntity;

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

pub(crate) fn apply_monster_entity_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) {
    let half_dead = snapshot_monster_is_half_dead(monster);
    let is_gone = monster
        .get("is_gone")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let intent_dmg = monster["move_base_damage"].as_i64().unwrap_or(-1) as i32;
    let intent_hits = monster["move_hits"].as_i64().unwrap_or(1) as i32;
    let intent_str = monster["intent"].as_str().unwrap_or("UNKNOWN");

    entity.current_hp = monster["current_hp"]
        .as_i64()
        .unwrap_or(monster["hp"].as_i64().unwrap_or(0)) as i32;
    entity.max_hp = monster["max_hp"].as_i64().unwrap_or(0) as i32;
    entity.block = monster["block"].as_i64().unwrap_or(0) as i32;
    entity.slot = index as u8;
    entity.is_dying = is_gone && !half_dead;
    entity.half_dead = half_dead;
    entity.monster_type = monster_id_from_java(monster["id"].as_str().unwrap_or(""))
        .map(|e| e as usize)
        .unwrap_or(0);
    entity.next_move_byte = monster["move_id"].as_u64().unwrap_or(0) as u8;
    entity.current_intent = intent_from_java(intent_str, intent_dmg, intent_hits);
    entity.intent_dmg = monster["move_adjusted_damage"].as_i64().unwrap_or(0) as i32;

    seed_monster_protocol_identity_from_snapshot(monster, index, entity);
    seed_move_history_from_snapshot(monster, entity);
    seed_hexaghost_runtime_from_snapshot(monster, entity);
    seed_chosen_runtime_from_snapshot(monster, entity);
    seed_darkling_runtime_from_snapshot(monster, entity);
    seed_lagavulin_runtime_from_snapshot(monster, entity);
}
