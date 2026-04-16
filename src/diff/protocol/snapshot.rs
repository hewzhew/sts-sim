use serde_json::Value;

/// Build the compact combat snapshot shape consumed by sync/replay code.
pub fn build_live_combat_snapshot(game_state: &Value) -> Value {
    let mut snapshot = game_state
        .get("combat_state")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = snapshot.as_object_mut() {
        if let Some(room_type) = game_state.get("room_type").cloned() {
            obj.insert("room_type".to_string(), room_type);
        }
        if let Some(potions) = game_state.get("potions").cloned() {
            obj.insert("potions".to_string(), potions);
        }
    }
    snapshot
}
