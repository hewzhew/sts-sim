use serde_json::Value;
use std::collections::VecDeque;

use crate::runtime::combat::{Intent, MonsterEntity};

use super::super::build::apply_monster_entity_snapshot;
use super::power::sync_monster_powers_from_snapshot;

pub fn sync_monsters_from_snapshot(cs: &mut crate::runtime::combat::CombatState, snapshot: &Value) {
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
            intent_preview_damage: 0,
            logical_position: 0,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        });
    }
    while cs.entities.monsters.len() > monsters_arr.len() {
        cs.entities.monsters.pop();
    }

    for (i, snapshot_monster) in monsters_arr.iter().enumerate() {
        apply_monster_entity_snapshot(snapshot_monster, i, &mut cs.entities.monsters[i]);
        sync_monster_powers_from_snapshot(cs, i, snapshot_monster);
    }
}
