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

#[cfg(test)]
mod tests {
    use super::build_live_combat_snapshot;
    use serde_json::json;

    #[test]
    fn includes_room_type_and_potions_in_live_snapshot() {
        let snapshot = build_live_combat_snapshot(&json!({
            "room_type": "MonsterRoomElite",
            "potions": [{"id": "Block Potion"}],
            "combat_state": {
                "turn": 2,
                "player": {"current_hp": 50}
            }
        }));

        assert_eq!(snapshot["room_type"], "MonsterRoomElite");
        assert_eq!(snapshot["potions"][0]["id"], "Block Potion");
        assert_eq!(snapshot["turn"], 2);
    }
}
