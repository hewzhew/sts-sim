use serde_json::Value;

pub(super) struct LiveFrame {
    root: Value,
}

impl LiveFrame {
    pub(super) fn parse(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line).map(|root| Self { root })
    }

    pub(super) fn root(&self) -> &Value {
        &self.root
    }

    pub(super) fn game_state(&self) -> &Value {
        self.root.get("game_state").unwrap_or(&Value::Null)
    }

    pub(super) fn protocol_meta(&self) -> Option<&Value> {
        self.root.get("protocol_meta")
    }

    pub(super) fn response_id(&self) -> Option<i64> {
        self.protocol_meta()?
            .get("response_id")
            .and_then(|v| v.as_i64())
    }

    pub(super) fn state_frame_id(&self) -> Option<i64> {
        self.protocol_meta()?
            .get("state_frame_id")
            .and_then(|v| v.as_i64())
    }

    pub(super) fn last_command_kind(&self) -> Option<&str> {
        self.protocol_meta()?
            .get("last_command_kind")
            .and_then(|v| v.as_str())
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.root.get("error").and_then(|v| v.as_str())
    }

    pub(super) fn available_commands(&self) -> Vec<&str> {
        self.root
            .get("available_commands")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default()
    }

    pub(super) fn in_game(&self) -> bool {
        self.root
            .get("in_game")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub(super) fn ready_for_command(&self) -> bool {
        self.root
            .get("ready_for_command")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub(super) fn screen(&self) -> &str {
        self.game_state()
            .get("screen_type")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
    }

    pub(super) fn screen_name(&self) -> &str {
        self.game_state()
            .get("screen_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    pub(super) fn room_phase(&self) -> &str {
        self.game_state()
            .get("room_phase")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    pub(super) fn is_combat(&self) -> bool {
        self.game_state()
            .get("combat_state")
            .is_some_and(|v| !v.is_null())
    }

    pub(super) fn combat_state(&self) -> Option<&Value> {
        self.game_state()
            .get("combat_state")
            .filter(|v| !v.is_null())
    }

    pub(super) fn relics(&self) -> &Value {
        self.game_state().get("relics").unwrap_or(&Value::Null)
    }

    pub(super) fn screen_state(&self) -> Option<&Value> {
        self.game_state().get("screen_state")
    }

    pub(super) fn room_type(&self) -> &str {
        self.game_state()
            .get("room_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    pub(super) fn combat_session(&self) -> Option<&Value> {
        self.protocol_meta()
            .and_then(|m| m.get("combat_session"))
            .filter(|v| !v.is_null())
    }

    pub(super) fn combat_session_state(&self) -> Option<&str> {
        self.combat_session()?.get("state").and_then(|v| v.as_str())
    }

    pub(super) fn combat_session_owner(&self) -> Option<&str> {
        self.combat_session()?.get("owner").and_then(|v| v.as_str())
    }

    pub(super) fn combat_session_id(&self) -> Option<&str> {
        self.combat_session()?
            .get("session_id")
            .and_then(|v| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::LiveFrame;

    #[test]
    fn live_frame_accessors_extract_common_protocol_fields() {
        let frame = LiveFrame::parse(
            r#"{
                "in_game": true,
                "ready_for_command": false,
                "protocol_meta": {
                    "response_id": 12,
                    "state_frame_id": 34,
                    "last_command_kind": "play",
                    "combat_session": {
                        "session_id": "combat-1",
                        "owner": "human",
                        "state": "active"
                    }
                },
                "available_commands": ["play", "end"],
                "game_state": {
                    "screen_type": "NONE",
                    "screen_name": "",
                    "room_phase": "COMBAT",
                    "room_type": "MonsterRoomBoss",
                    "combat_state": {"hand": []},
                    "relics": []
                }
            }"#,
        )
        .unwrap();

        assert!(frame.in_game());
        assert!(!frame.ready_for_command());
        assert_eq!(frame.response_id(), Some(12));
        assert_eq!(frame.state_frame_id(), Some(34));
        assert_eq!(frame.last_command_kind(), Some("play"));
        assert_eq!(frame.screen(), "NONE");
        assert_eq!(frame.room_phase(), "COMBAT");
        assert_eq!(frame.room_type(), "MonsterRoomBoss");
        assert!(frame.is_combat());
        assert_eq!(frame.combat_session_id(), Some("combat-1"));
        assert_eq!(frame.combat_session_owner(), Some("human"));
        assert_eq!(frame.combat_session_state(), Some("active"));
        assert_eq!(frame.available_commands(), vec!["play", "end"]);
    }
}
