use serde_json::Value;

use crate::protocol::java::{java_potion_id_to_rust, relic_id_from_java, snapshot_uuid};

use super::super::internal_state::{
    snapshot_runtime_amount_for_relic, snapshot_runtime_counter_for_relic,
    snapshot_runtime_used_up_for_relic, sync_relic_runtime_state_from_snapshot,
};

pub fn sync_player_potions_from_snapshot(
    cs: &mut crate::runtime::combat::CombatState,
    snapshot: &Value,
) {
    if let Some(potions_arr) = snapshot.get("potions").and_then(|v| v.as_array()) {
        for (i, p_val) in potions_arr.iter().enumerate() {
            if i < cs.entities.potions.len() {
                cs.entities.potions[i] = p_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(java_potion_id_to_rust)
                    .map(|id| {
                        crate::content::potions::Potion::with_affordance_truth(
                            id,
                            p_val
                                .get("uuid")
                                .map(|value| snapshot_uuid(value, i as u32))
                                .unwrap_or(i as u32),
                            p_val
                                .get("can_use")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true),
                            p_val
                                .get("can_discard")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true),
                            p_val
                                .get("requires_target")
                                .and_then(|v| v.as_bool())
                                .unwrap_or_else(|| {
                                    crate::content::potions::get_potion_definition(id)
                                        .target_required
                                }),
                        )
                    });
            }
        }
    }
}

pub fn sync_player_relics_from_snapshot(
    cs: &mut crate::runtime::combat::CombatState,
    snapshot: &Value,
) {
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
                        sync_relic_runtime_state_from_snapshot(
                            rs,
                            snapshot_runtime_counter_for_relic(relic_id, r_val),
                            snapshot_runtime_used_up_for_relic(relic_id, r_val),
                            snapshot_runtime_amount_for_relic(relic_id, r_val),
                        );
                    }
                }
            }
        }
    }
}
