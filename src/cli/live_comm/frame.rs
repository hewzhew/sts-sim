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

    pub(super) fn has_combat_action_space_capability(&self) -> bool {
        self.protocol_meta()
            .and_then(|meta| meta.get("capabilities"))
            .and_then(|caps| caps.get("combat_action_space"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
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
            .get("combat_truth")
            .or_else(|| self.game_state().get("combat_observation"))
            .is_some_and(|v| !v.is_null())
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

    pub(super) fn continuation_state(&self) -> Option<&Value> {
        self.protocol_meta()
            .and_then(|meta| meta.get("continuation_state"))
            .filter(|value| !value.is_null())
    }

    pub(super) fn brief_summary(&self) -> String {
        let gs = self.game_state();
        let base = format!(
            "screen={} room_phase={} room_type={} floor={} act={}",
            self.screen(),
            self.room_phase(),
            self.room_type(),
            gs.get("floor").and_then(|v| v.as_i64()).unwrap_or(0),
            gs.get("act").and_then(|v| v.as_i64()).unwrap_or(0),
        );

        if !self.is_combat() {
            return base;
        }

        let combat_truth = gs.get("combat_truth");
        let combat_observation = gs.get("combat_observation");
        let player = combat_truth.and_then(|state| state.get("player"));
        let hand_count = combat_truth
            .and_then(|state| state.get("hand"))
            .and_then(Value::as_array)
            .map_or(0, |cards| cards.len());
        let draw_count = combat_truth
            .and_then(|state| state.get("draw_pile"))
            .and_then(Value::as_array)
            .map(|cards| cards.len())
            .or_else(|| {
                combat_observation
                    .and_then(|state| state.get("draw_pile_count"))
                    .and_then(|value| value.as_u64().map(|value| value as usize))
            })
            .unwrap_or(0);
        let discard_count = combat_truth
            .and_then(|state| state.get("discard_pile"))
            .and_then(Value::as_array)
            .map_or(0, |cards| cards.len());
        let monsters = compact_monster_summary(
            combat_truth
                .and_then(|state| state.get("monsters"))
                .and_then(Value::as_array),
            combat_observation
                .and_then(|state| state.get("monsters"))
                .and_then(Value::as_array),
        );

        format!(
            "{base} combat turn={} hp={}/{} blk={} energy={} hand={} draw={} discard={} monsters={}",
            combat_truth
                .and_then(|state| state.get("turn"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            player
                .and_then(|value| value.get("current_hp"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            player
                .and_then(|value| value.get("max_hp"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            player
                .and_then(|value| value.get("block"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            combat_truth
                .and_then(|state| state.get("energy"))
                .or_else(|| player.and_then(|value| value.get("energy")))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            hand_count,
            draw_count,
            discard_count,
            monsters
        )
    }
}

fn compact_monster_summary(
    truth_monsters: Option<&Vec<Value>>,
    observation_monsters: Option<&Vec<Value>>,
) -> String {
    let Some(truth_monsters) = truth_monsters else {
        return "<none>".to_string();
    };
    if truth_monsters.is_empty() {
        return "<none>".to_string();
    }

    let mut parts = truth_monsters
        .iter()
        .take(3)
        .enumerate()
        .map(|(index, monster)| {
            let observation = observation_monsters.and_then(|entries| entries.get(index));
            let name = monster
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| monster.get("id").and_then(Value::as_str))
                .or_else(|| {
                    observation
                        .and_then(|value| value.get("name"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("?");
            let move_suffix = monster
                .get("move_id")
                .and_then(Value::as_i64)
                .map(|move_id| format!(" move={move_id}"))
                .unwrap_or_default();
            format!(
                "{} hp={}/{} blk={} intent={}{}",
                name,
                monster
                    .get("current_hp")
                    .or_else(|| monster.get("hp"))
                    .and_then(Value::as_i64)
                    .unwrap_or(-1),
                monster.get("max_hp").and_then(Value::as_i64).unwrap_or(-1),
                monster.get("block").and_then(Value::as_i64).unwrap_or(0),
                observation
                    .and_then(|value| value.get("intent"))
                    .and_then(Value::as_str)
                    .unwrap_or("?"),
                move_suffix,
            )
        })
        .collect::<Vec<_>>();

    if truth_monsters.len() > 3 {
        parts.push(format!("...+{}", truth_monsters.len() - 3));
    }

    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::LiveFrame;

    #[test]
    fn brief_summary_includes_combat_context() {
        let frame = LiveFrame::parse(
            r#"{
                "protocol_meta":{"response_id":7,"state_frame_id":8},
                "game_state":{
                    "screen_type":"NONE",
                    "room_phase":"COMBAT",
                    "room_type":"MonsterRoom",
                    "floor":18,
                    "act":2,
                    "combat_truth":{
                        "turn":1,
                        "energy":3,
                        "player":{"current_hp":80,"max_hp":80,"block":0},
                        "hand":[{},{}],
                        "draw_pile":[{},{}],
                        "discard_pile":[{}],
                        "monsters":[
                            {"name":"Spheric Guardian","current_hp":20,"max_hp":20,"block":40,"move_id":2}
                        ]
                    },
                    "combat_observation":{
                        "monsters":[
                            {"intent":"DEFEND"}
                        ]
                    }
                }
            }"#,
        )
        .unwrap();

        let summary = frame.brief_summary();
        assert!(summary.contains("combat turn=1"));
        assert!(summary.contains("hand=2"));
        assert!(summary.contains("Spheric Guardian"));
        assert!(summary.contains("intent=DEFEND"));
        assert!(summary.contains("move=2"));
    }
}
