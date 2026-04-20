use crate::runtime::combat::{MonsterEntity, MonsterMoveState};
use serde_json::Value;

use super::super::build::apply_monster_split_snapshot;
use super::power::sync_monster_powers_from_snapshot;

pub fn sync_monsters_from_snapshots(
    cs: &mut crate::runtime::combat::CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) {
    let truth_monsters = truth_snapshot["monsters"].as_array().unwrap();
    let observation_monsters = observation_snapshot
        .get("monsters")
        .and_then(|v| v.as_array());

    while cs.entities.monsters.len() < truth_monsters.len() {
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
            move_state: MonsterMoveState::default(),
            logical_position: 0,
            hexaghost: Default::default(),
            louse: Default::default(),
            jaw_worm: Default::default(),
            thief: Default::default(),
            byrd: Default::default(),
            chosen: Default::default(),
            snecko: Default::default(),
            shelled_parasite: Default::default(),
            bronze_automaton: Default::default(),
            bronze_orb: Default::default(),
            book_of_stabbing: Default::default(),
            collector: Default::default(),
            champ: Default::default(),
            awakened_one: Default::default(),
            corrupt_heart: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
            guardian: Default::default(),
        });
    }
    while cs.entities.monsters.len() > truth_monsters.len() {
        cs.entities.monsters.pop();
    }

    for (i, truth_monster) in truth_monsters.iter().enumerate() {
        let observation_monster = observation_monsters
            .and_then(|monsters| monsters.get(i))
            .unwrap_or(truth_monster);
        let protocol = apply_monster_split_snapshot(
            truth_monster,
            observation_monster,
            i,
            &mut cs.entities.monsters[i],
        );
        cs.runtime
            .monster_protocol
            .insert(cs.entities.monsters[i].id, protocol);
        sync_monster_powers_from_snapshot(cs, i, truth_monster);
    }
}
