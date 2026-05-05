use serde_json::Value;

use crate::content::monsters::EnemyId;
use crate::protocol::java::{intent_from_java, monster_id_from_java};
use crate::runtime::combat::MonsterEntity;

fn runtime_state<'a>(monster: &'a Value, monster_type: EnemyId) -> &'a Value {
    monster.get("runtime_state").unwrap_or_else(|| {
        panic!("strict state_sync: monster.runtime_state missing for {monster_type:?}")
    })
}

fn runtime_state_bool(monster: &Value, monster_type: EnemyId, key: &str) -> bool {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

fn runtime_state_u8(monster: &Value, monster_type: EnemyId, key: &str) -> u8 {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as u8)
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

fn runtime_state_i32(monster: &Value, monster_type: EnemyId, key: &str) -> i32 {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_i64())
        .map(|value| value as i32)
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

pub(crate) fn seed_hexaghost_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Hexaghost;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.hexaghost.activated = runtime_state_bool(monster, monster_type, "activated");
    entity.hexaghost.orb_active_count = runtime_state_u8(monster, monster_type, "orb_active_count");
    entity.hexaghost.burn_upgraded = runtime_state_bool(monster, monster_type, "burn_upgraded");
    entity.hexaghost.divider_damage =
        match runtime_state_i32(monster, monster_type, "divider_damage") {
            value if value >= 0 => Some(value),
            _ => None,
        };
}

pub(crate) fn seed_darkling_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Darkling;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.darkling.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.darkling.nip_dmg = runtime_state_i32(monster, monster_type, "nip_dmg");
}

pub(crate) fn seed_byrd_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Byrd;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.byrd.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.byrd.is_flying = runtime_state_bool(monster, monster_type, "is_flying");
    entity.byrd.protocol_seeded = true;
}

pub(crate) fn seed_louse_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if !matches!(
        entity.monster_type,
        x if x == EnemyId::LouseNormal as usize || x == EnemyId::LouseDefensive as usize
    ) {
        return;
    }

    let monster_type = EnemyId::from_id(entity.monster_type)
        .expect("strict state_sync: louse monster type must be initialized before runtime seeding");
    entity.louse.bite_damage = Some(runtime_state_i32(monster, monster_type, "bite_damage"));
}

pub(crate) fn seed_chosen_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Chosen;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.chosen.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.chosen.used_hex = runtime_state_bool(monster, monster_type, "used_hex");
    entity.chosen.protocol_seeded = true;
}

pub(crate) fn seed_snecko_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Snecko;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.snecko.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.snecko.protocol_seeded = true;
}

pub(crate) fn seed_shelled_parasite_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::ShelledParasite;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.shelled_parasite.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.shelled_parasite.protocol_seeded = true;
}

pub(crate) fn seed_bronze_automaton_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::BronzeAutomaton;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.bronze_automaton.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.bronze_automaton.num_turns = runtime_state_u8(monster, monster_type, "num_turns");
    entity.bronze_automaton.protocol_seeded = true;
}

pub(crate) fn seed_bronze_orb_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::BronzeOrb;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.bronze_orb.used_stasis = runtime_state_bool(monster, monster_type, "used_stasis");
    entity.bronze_orb.protocol_seeded = true;
}

pub(crate) fn seed_book_of_stabbing_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::BookOfStabbing;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.book_of_stabbing.stab_count = runtime_state_u8(monster, monster_type, "stab_count");
    entity.book_of_stabbing.protocol_seeded = true;
}

pub(crate) fn seed_collector_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::TheCollector;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.collector.initial_spawn = runtime_state_bool(monster, monster_type, "initial_spawn");
    entity.collector.ult_used = runtime_state_bool(monster, monster_type, "ult_used");
    entity.collector.turns_taken = runtime_state_u8(monster, monster_type, "turns_taken");
    entity.collector.protocol_seeded = true;
}

pub(crate) fn seed_champ_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Champ;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.champ.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.champ.num_turns = runtime_state_u8(monster, monster_type, "num_turns");
    entity.champ.forge_times = runtime_state_u8(monster, monster_type, "forge_times");
    entity.champ.threshold_reached = runtime_state_bool(monster, monster_type, "threshold_reached");
    entity.champ.protocol_seeded = true;
}

pub(crate) fn seed_thief_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = match entity.monster_type {
        x if x == EnemyId::Looter as usize => EnemyId::Looter,
        x if x == EnemyId::Mugger as usize => EnemyId::Mugger,
        _ => return,
    };

    entity.thief.slash_count = runtime_state_u8(monster, monster_type, "slash_count");
    entity.thief.stolen_gold = runtime_state_i32(monster, monster_type, "stolen_gold");
    entity.thief.protocol_seeded = true;
}

pub(crate) fn seed_lagavulin_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Lagavulin;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.lagavulin.idle_count = runtime_state_u8(monster, monster_type, "idle_count");
    entity.lagavulin.debuff_turn_count = runtime_state(monster, monster_type)
        .get("debuff_turn_count")
        .and_then(|value| value.as_u64())
        .map(|value| value as u8)
        .unwrap_or(0);
    entity.lagavulin.is_out = runtime_state(monster, monster_type)
        .get("is_out")
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| entity.planned_move_id() != 5);
    entity.lagavulin.is_out_triggered =
        runtime_state_bool(monster, monster_type, "is_out_triggered");
}

pub(crate) fn seed_guardian_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::TheGuardian;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.guardian.damage_threshold =
        runtime_state_i32(monster, monster_type, "guardian_threshold");
    entity.guardian.damage_taken = runtime_state_i32(monster, monster_type, "damage_taken");
    entity.guardian.is_open = runtime_state_bool(monster, monster_type, "is_open");
    entity.guardian.close_up_triggered =
        runtime_state_bool(monster, monster_type, "close_up_triggered");
}

pub(crate) fn seed_move_history_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let current_move_id = monster.get("move_id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let second_last_move_id = monster
        .get("second_last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let last_move_id = monster
        .get("last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    entity.move_history_mut().clear();
    if second_last_move_id != 0 {
        entity.move_history_mut().push_back(second_last_move_id);
    }
    if last_move_id != 0 {
        entity.move_history_mut().push_back(last_move_id);
    }
    if current_move_id != 0 {
        entity.move_history_mut().push_back(current_move_id);
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

pub(crate) fn monster_protocol_state_from_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    let instance_id = monster.get("monster_instance_id").and_then(|v| v.as_u64());
    let spawn_order = monster
        .get("spawn_order")
        .and_then(|v| v.as_u64())
        .or(instance_id);
    let draw_x = snapshot_i32(monster, "draw_x");
    let group_index = monster
        .get("monster_index")
        .and_then(|v| v.as_u64())
        .map(|value| value as usize)
        .or(Some(index));

    if let Some(draw_x) = draw_x {
        entity.logical_position = draw_x;
    } else if entity.logical_position == 0 {
        entity.logical_position = index as i32;
    }

    let intent_base_damage = monster
        .get("move_base_damage")
        .and_then(|v| v.as_i64())
        .or_else(|| monster.get("move_adjusted_damage").and_then(|v| v.as_i64()))
        .unwrap_or(-1) as i32;
    let intent_hits = monster["move_hits"].as_i64().unwrap_or(1) as i32;
    let intent_str = monster["intent"].as_str().unwrap_or("UNKNOWN");

    crate::runtime::combat::MonsterProtocolState {
        observation: crate::runtime::combat::MonsterProtocolObservationState {
            visible_intent: intent_from_java(intent_str, intent_base_damage, intent_hits),
            preview_damage_per_hit: monster["move_adjusted_damage"].as_i64().unwrap_or(0) as i32,
        },
        identity: crate::runtime::combat::MonsterProtocolIdentity {
            instance_id,
            spawn_order,
            draw_x,
            group_index,
        },
    }
}

pub(crate) fn apply_monster_truth_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) {
    let half_dead = snapshot_monster_is_half_dead(monster);
    let is_gone = monster
        .get("is_gone")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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
    entity.set_planned_move_id(monster["move_id"].as_u64().unwrap_or(0) as u8);
    seed_move_history_from_snapshot(monster, entity);
    seed_hexaghost_runtime_from_snapshot(monster, entity);
    seed_byrd_runtime_from_snapshot(monster, entity);
    seed_louse_runtime_from_snapshot(monster, entity);
    seed_chosen_runtime_from_snapshot(monster, entity);
    seed_snecko_runtime_from_snapshot(monster, entity);
    seed_shelled_parasite_runtime_from_snapshot(monster, entity);
    seed_bronze_automaton_runtime_from_snapshot(monster, entity);
    seed_bronze_orb_runtime_from_snapshot(monster, entity);
    seed_book_of_stabbing_runtime_from_snapshot(monster, entity);
    seed_collector_runtime_from_snapshot(monster, entity);
    seed_champ_runtime_from_snapshot(monster, entity);
    seed_thief_runtime_from_snapshot(monster, entity);
    seed_darkling_runtime_from_snapshot(monster, entity);
    seed_lagavulin_runtime_from_snapshot(monster, entity);
    seed_guardian_runtime_from_snapshot(monster, entity);
}

pub(crate) fn apply_monster_observation_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    monster_protocol_state_from_snapshot(monster, index, entity)
}

pub(crate) fn apply_monster_split_snapshot(
    truth_monster: &Value,
    observation_monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    apply_monster_truth_snapshot(truth_monster, index, entity);
    apply_monster_observation_snapshot(observation_monster, index, entity)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        apply_monster_observation_snapshot, apply_monster_split_snapshot,
        apply_monster_truth_snapshot,
    };
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{
        ByrdRuntimeState, ChosenRuntimeState, DarklingRuntimeState, GuardianRuntimeState,
        HexaghostRuntimeState, LagavulinRuntimeState, MonsterEntity, MonsterMoveState,
        ShelledParasiteRuntimeState, SneckoRuntimeState,
    };

    fn blank_monster_entity() -> MonsterEntity {
        MonsterEntity {
            id: 1,
            monster_type: 0,
            current_hp: 0,
            max_hp: 0,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            move_state: MonsterMoveState::default(),
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: Default::default(),
            jaw_worm: Default::default(),
            thief: Default::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: Default::default(),
            bronze_orb: Default::default(),
            book_of_stabbing: Default::default(),
            collector: Default::default(),
            champ: Default::default(),
            awakened_one: Default::default(),
            corrupt_heart: Default::default(),
            darkling: DarklingRuntimeState::default(),
            lagavulin: LagavulinRuntimeState::default(),
            guardian: GuardianRuntimeState::default(),
        }
    }

    fn chosen_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Chosen",
            "current_hp": 95,
            "max_hp": 95,
            "block": 11,
            "move_id": 3,
            "move_base_damage": 5,
            "move_hits": 2,
            "second_last_move_id": 1,
            "last_move_id": 2,
            "powers": [],
            "runtime_state": {
                "first_turn": false,
                "used_hex": true
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn snecko_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Snecko",
            "current_hp": 118,
            "max_hp": 118,
            "block": 0,
            "move_id": 1,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_turn": true
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn shelled_parasite_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Shelled Parasite",
            "current_hp": 70,
            "max_hp": 70,
            "block": 14,
            "intent": "ATTACK_BUFF",
            "move_id": 3,
            "move_base_damage": 12,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn mugger_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Mugger",
            "current_hp": 50,
            "max_hp": 50,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 1,
            "move_base_damage": 11,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "slash_count": 1,
                "stolen_gold": 20
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn guardian_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "TheGuardian",
            "current_hp": 240,
            "max_hp": 240,
            "block": 0,
            "intent": "DEFEND",
            "move_id": 6,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [
                {
                    "id": "Mode Shift",
                    "name": "Mode Shift",
                    "amount": 30
                }
            ],
            "runtime_state": {
                "guardian_threshold": 30,
                "damage_taken": 0,
                "is_open": true,
                "close_up_triggered": false
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn collector_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "TheCollector",
            "current_hp": 282,
            "max_hp": 282,
            "block": 0,
            "move_id": 2,
            "move_base_damage": 18,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "initial_spawn": false,
                "ult_used": true,
                "turns_taken": 4
            },
            "is_gone": false,
            "half_dead": false
        })
    }

    fn book_of_stabbing_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BookOfStabbing",
            "current_hp": 170,
            "max_hp": 170,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 7,
            "move_hits": 3,
            "last_move_id": 2,
            "second_last_move_id": 1,
            "powers": [],
            "runtime_state": {
                "stab_count": 3
            }
        })
    }

    fn bronze_automaton_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BronzeAutomaton",
            "current_hp": 300,
            "max_hp": 300,
            "block": 0,
            "move_id": 5,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_turn": false,
                "num_turns": 3
            }
        })
    }

    fn bronze_orb_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BronzeOrb",
            "current_hp": 55,
            "max_hp": 55,
            "block": 0,
            "move_id": 3,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "used_stasis": true
            }
        })
    }

    fn champ_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Champ",
            "current_hp": 420,
            "max_hp": 420,
            "block": 0,
            "move_id": 4,
            "move_base_damage": 14,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_turn": false,
                "num_turns": 3,
                "forge_times": 1,
                "threshold_reached": true
            }
        })
    }

    fn chosen_observation_snapshot() -> serde_json::Value {
        json!({
            "id": "Chosen",
            "current_hp": 95,
            "max_hp": 95,
            "block": 11,
            "intent": "ATTACK_DEBUFF",
            "move_adjusted_damage": 8,
            "move_hits": 2,
            "monster_instance_id": 42,
            "spawn_order": 77,
            "monster_index": 4,
            "draw_x": 900,
            "powers": [],
            "is_gone": false,
            "half_dead": false
        })
    }

    #[test]
    fn truth_import_does_not_populate_observation_fields() {
        let snapshot = chosen_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 1, &mut entity);

        assert_eq!(entity.current_hp, 95);
        assert_eq!(entity.max_hp, 95);
        assert_eq!(entity.block, 11);
        assert_eq!(entity.slot, 1);
        assert_eq!(entity.planned_move_id(), 3);
        assert_eq!(
            entity.move_history().iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(!entity.chosen.first_turn);
        assert!(entity.chosen.used_hex);
        assert!(entity.chosen.protocol_seeded);
        assert_eq!(entity.logical_position, 0);
    }

    #[test]
    fn observation_import_populates_visible_intent_fields() {
        let snapshot = chosen_observation_snapshot();
        let mut entity = blank_monster_entity();

        let protocol = apply_monster_observation_snapshot(&snapshot, 1, &mut entity);

        assert_eq!(
            protocol.observation.visible_intent,
            crate::runtime::combat::Intent::AttackDebuff { damage: 8, hits: 2 }
        );
        assert_eq!(protocol.observation.preview_damage_per_hit, 8);
        assert_eq!(protocol.identity.instance_id, Some(42));
        assert_eq!(protocol.identity.spawn_order, Some(77));
        assert_eq!(protocol.identity.draw_x, Some(900));
        assert_eq!(protocol.identity.group_index, Some(4));
        assert_eq!(entity.logical_position, 900);
    }

    #[test]
    fn split_import_combines_truth_and_observation() {
        let truth = chosen_truth_snapshot();
        let observation = chosen_observation_snapshot();
        let mut entity = blank_monster_entity();

        let protocol = apply_monster_split_snapshot(&truth, &observation, 1, &mut entity);

        assert_eq!(entity.current_hp, 95);
        assert_eq!(
            protocol.observation.visible_intent,
            crate::runtime::combat::Intent::AttackDebuff { damage: 8, hits: 2 }
        );
        assert_eq!(protocol.observation.preview_damage_per_hit, 8);
        assert_eq!(
            entity.move_history().iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(protocol.identity.instance_id, Some(42));
        assert!(entity.chosen.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_snecko_runtime_first_turn() {
        let snapshot = snecko_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert!(entity.snecko.first_turn);
        assert!(entity.snecko.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_shelled_parasite_runtime_first_move() {
        let snapshot = shelled_parasite_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert!(!entity.shelled_parasite.first_move);
        assert!(entity.shelled_parasite.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_mugger_thief_runtime() {
        let snapshot = mugger_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert_eq!(entity.thief.slash_count, 1);
        assert_eq!(entity.thief.stolen_gold, 20);
        assert!(entity.thief.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_guardian_runtime_state() {
        let snapshot = guardian_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 6);
        assert_eq!(entity.guardian.damage_threshold, 30);
        assert_eq!(entity.guardian.damage_taken, 0);
        assert!(entity.guardian.is_open);
        assert!(!entity.guardian.close_up_triggered);
    }

    #[test]
    fn truth_import_seeds_collector_runtime_state() {
        let snapshot = collector_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert!(!entity.collector.initial_spawn);
        assert!(entity.collector.ult_used);
        assert_eq!(entity.collector.turns_taken, 4);
        assert!(entity.collector.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_book_of_stabbing_runtime_state() {
        let snapshot = book_of_stabbing_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BookOfStabbing as usize);
        assert_eq!(entity.book_of_stabbing.stab_count, 3);
        assert!(entity.book_of_stabbing.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_bronze_automaton_runtime_state() {
        let snapshot = bronze_automaton_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BronzeAutomaton as usize);
        assert!(!entity.bronze_automaton.first_turn);
        assert_eq!(entity.bronze_automaton.num_turns, 3);
        assert!(entity.bronze_automaton.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_bronze_orb_runtime_state() {
        let snapshot = bronze_orb_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BronzeOrb as usize);
        assert!(entity.bronze_orb.used_stasis);
        assert!(entity.bronze_orb.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_champ_runtime_state() {
        let snapshot = champ_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Champ as usize);
        assert!(!entity.champ.first_turn);
        assert_eq!(entity.champ.num_turns, 3);
        assert_eq!(entity.champ.forge_times, 1);
        assert!(entity.champ.threshold_reached);
        assert!(entity.champ.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_louse_runtime_damage_when_exported() {
        let snapshot = json!({
            "id": "Louse",
            "current_hp": 12,
            "max_hp": 12,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 5,
            "move_adjusted_damage": 9,
            "move_hits": 1,
            "second_last_move_id": 4,
            "last_move_id": 3,
            "powers": [],
            "runtime_state": {
                "bite_damage": 7
            },
            "is_gone": false,
            "half_dead": false
        });
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert_eq!(entity.louse_bite_damage(), Some(7));
    }

    #[test]
    fn truth_import_requires_runtime_bite_damage_for_louse() {
        let snapshot = json!({
            "id": "Louse",
            "current_hp": 12,
            "max_hp": 12,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 7,
            "move_adjusted_damage": 8,
            "move_hits": 1,
            "second_last_move_id": 4,
            "last_move_id": 3,
            "powers": [],
            "is_gone": false,
            "half_dead": false
        });
        let mut entity = blank_monster_entity();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_monster_truth_snapshot(&snapshot, 0, &mut entity);
        }));

        assert!(result.is_err());
    }
}
