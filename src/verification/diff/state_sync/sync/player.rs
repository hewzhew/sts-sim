use serde_json::Value;

use super::power::sync_player_powers_from_snapshot;

pub fn sync_player_from_snapshot(cs: &mut crate::runtime::combat::CombatState, snapshot: &Value) {
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
    cs.turn
        .set_energy(player_val["energy"].as_u64().unwrap_or(3) as u8);

    sync_player_powers_from_snapshot(cs, snapshot);
}
