use serde_json::Value;

use crate::content::powers::store;
use crate::diff::protocol::{power_id_from_java, power_instance_id_from_java};
use crate::runtime::combat::Power;

use super::super::build::build_powers_from_snapshot_for_owner;
use super::super::internal_state::{
    sync_monster_internal_state_from_snapshot, sync_power_extra_data_from_snapshot,
    sync_power_extra_data_from_snapshot_power,
};

fn snapshot_power_matches(power: &Power, snapshot_power: &Value) -> bool {
    let Some(pid_str) = snapshot_power.get("id").and_then(|v| v.as_str()) else {
        return false;
    };
    let Some(pid) = power_id_from_java(pid_str) else {
        return false;
    };
    if power.power_type != pid {
        return false;
    }
    if crate::content::powers::uses_distinct_instances(pid) {
        return power.instance_id == power_instance_id_from_java(pid_str);
    }
    true
}

pub fn sync_player_powers_from_snapshot(
    cs: &mut crate::runtime::combat::CombatState,
    snapshot: &Value,
) {
    let player_val = &snapshot["player"];
    let previous_player_powers = store::powers_for(cs, 0).map(|powers| powers.to_vec());

    let mut player_powers = build_powers_from_snapshot_for_owner(0, &player_val["powers"]);
    sync_power_extra_data_from_snapshot(previous_player_powers.as_deref(), &mut player_powers);
    if let Some(snapshot_powers) = player_val["powers"].as_array() {
        overlay_snapshot_power_fields(&mut player_powers, snapshot_powers);
    }
    store::set_powers_for(cs, 0, player_powers);
}

pub fn sync_monster_powers_from_snapshot(
    cs: &mut crate::runtime::combat::CombatState,
    monster_index: usize,
    snapshot_monster: &Value,
) {
    let entity_id = cs.entities.monsters[monster_index].id;
    let previous_powers = store::powers_for(cs, entity_id).map(|powers| powers.to_vec());

    let mut powers = build_powers_from_snapshot_for_owner(entity_id, &snapshot_monster["powers"]);
    sync_monster_internal_state_from_snapshot(
        cs.entities.monsters[monster_index].monster_type,
        previous_powers.as_deref(),
        snapshot_monster,
        &mut powers,
    );
    if let Some(snapshot_powers) = snapshot_monster["powers"].as_array() {
        overlay_snapshot_power_fields(&mut powers, snapshot_powers);
    }

    if !powers.is_empty() {
        store::set_powers_for(cs, entity_id, powers);
    } else {
        store::remove_entity_powers(cs, entity_id);
    }
}

fn overlay_snapshot_power_fields(powers: &mut [Power], snapshot_powers: &[Value]) {
    for snapshot_power in snapshot_powers {
        if let Some(power) = powers
            .iter_mut()
            .find(|power| snapshot_power_matches(power, snapshot_power))
        {
            sync_power_extra_data_from_snapshot_power(power, snapshot_power);
        }
    }
}
