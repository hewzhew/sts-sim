use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::testing::fixtures::scenario::{
    ScenarioAssertion, ScenarioFixture, ScenarioKind, ScenarioOracleKind, ScenarioProvenance,
    ScenarioStep,
};

pub fn build_fixture_from_record_window(
    records: &BTreeMap<i64, Value>,
    start_response_id: i64,
    end_response_id: i64,
    name: String,
    assertions: Vec<ScenarioAssertion>,
    tags: Vec<String>,
    provenance: Option<ScenarioProvenance>,
) -> Result<ScenarioFixture, String> {
    let mut missing = Vec::new();
    for rid in start_response_id..=end_response_id {
        if !records.contains_key(&rid) {
            missing.push(rid);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "missing response_ids in live window: {:?}",
            missing
        ));
    }

    let initial = records
        .get(&start_response_id)
        .ok_or_else(|| format!("missing start response_id {start_response_id}"))?;
    let mut steps = Vec::new();
    let mut previous_command = initial
        .get("protocol_meta")
        .and_then(|m| m.get("last_command"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    for rid in (start_response_id + 1)..=end_response_id {
        let root = records
            .get(&rid)
            .ok_or_else(|| format!("missing response_id {rid}"))?;
        let meta = root
            .get("protocol_meta")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("response_id={rid} missing protocol_meta"))?;

        let human_choice = meta.get("recent_human_card_reward_choice");
        if let Some(choice) = human_choice.and_then(|v| v.as_object()) {
            match choice.get("choice_kind").and_then(|v| v.as_str()) {
                Some("card") => {
                    if let Some(choice_index) = choice.get("choice_index").and_then(|v| v.as_u64())
                    {
                        steps.push(ScenarioStep {
                            command: format!("HUMAN_CARD_REWARD {}", choice_index),
                            label: Some(format!("response_id={rid}")),
                            response_id: Some(rid as u64),
                            frame_id: root
                                .get("protocol_meta")
                                .and_then(|m| m.get("state_frame_id"))
                                .and_then(|v| v.as_i64())
                                .and_then(|v| u64::try_from(v).ok()),
                            command_kind: Some("human_card_reward".to_string()),
                            structured: None,
                        });
                    }
                }
                Some("skip") | Some("bowl") => {
                    steps.push(ScenarioStep {
                        command: "HUMAN_CARD_REWARD SKIP".to_string(),
                        label: Some(format!("response_id={rid}")),
                        response_id: Some(rid as u64),
                        frame_id: root
                            .get("protocol_meta")
                            .and_then(|m| m.get("state_frame_id"))
                            .and_then(|v| v.as_i64())
                            .and_then(|v| u64::try_from(v).ok()),
                        command_kind: Some("human_card_reward".to_string()),
                        structured: None,
                    });
                }
                _ => {}
            }
        }

        let command = meta
            .get("last_command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("response_id={rid} has no protocol_meta.last_command"))?;

        if human_choice.is_some()
            && previous_command
                .as_ref()
                .is_some_and(|prev| prev.as_str() == command)
        {
            continue;
        }

        steps.push(ScenarioStep {
            command: command.to_string(),
            label: Some(format!("response_id={rid}")),
            response_id: Some(rid as u64),
            frame_id: meta
                .get("state_frame_id")
                .and_then(|v| v.as_i64())
                .and_then(|v| u64::try_from(v).ok()),
            command_kind: meta
                .get("last_command_kind")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
            structured: None,
        });
        previous_command = Some(command.to_string());
    }

    Ok(ScenarioFixture {
        name,
        kind: ScenarioKind::Combat,
        oracle_kind: ScenarioOracleKind::Live,
        initial_game_state: initial
            .get("game_state")
            .cloned()
            .ok_or_else(|| "start record missing game_state".to_string())?,
        initial_protocol_meta: Some(json!({
            "last_command": initial.get("protocol_meta").and_then(|m| m.get("last_command")).cloned(),
            "last_command_kind": initial.get("protocol_meta").and_then(|m| m.get("last_command_kind")).cloned(),
            "response_id": start_response_id,
            "state_frame_id": initial.get("protocol_meta").and_then(|m| m.get("state_frame_id")).cloned(),
        })),
        steps,
        assertions,
        provenance,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_live_fixture_from_contiguous_window() {
        let mut records = BTreeMap::new();
        records.insert(
            10,
            json!({
                "protocol_meta": {"response_id": 10, "last_command": "STATE", "last_command_kind": "state"},
                "game_state": {
                    "room_type": "MonsterRoom",
                    "combat_state": {
                        "turn": 1,
                        "player": {"current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": []},
                        "monsters": [{"id": "JawWorm", "current_hp": 40, "max_hp": 40, "block": 0, "powers": [], "intent": "UNKNOWN", "move_base_damage": -1, "move_adjusted_damage": -1, "move_hits": 1, "move_id": 0, "is_gone": false}],
                        "hand": [{"id": "Strike_R", "uuid": "h-0", "upgrades": 0, "cost": 1, "misc": 0}],
                        "draw_pile": [],
                        "discard_pile": [],
                        "exhaust_pile": [],
                        "potions": []
                    },
                    "relics": [],
                    "potions": []
                }
            }),
        );
        records.insert(
            11,
            json!({
                "protocol_meta": {"response_id": 11, "last_command": "PLAY 1 0", "last_command_kind": "play"},
                "game_state": {"combat_state": {}}
            }),
        );
        let fixture = build_fixture_from_record_window(
            &records,
            10,
            11,
            "window".to_string(),
            Vec::new(),
            vec!["watch_capture".to_string()],
            Some(ScenarioProvenance {
                source: Some("live_comm_watch".to_string()),
                source_path: None,
                response_id_range: Some((10, 11)),
                failure_frame: None,
                notes: vec!["test".to_string()],
                ..Default::default()
            }),
        )
        .expect("fixture should build");

        assert_eq!(fixture.oracle_kind, ScenarioOracleKind::Live);
        assert_eq!(fixture.steps.len(), 1);
        assert_eq!(fixture.steps[0].command, "PLAY 1 0");
        assert_eq!(fixture.tags, vec!["watch_capture"]);
    }

    #[test]
    fn human_reward_choice_reuses_special_step_and_skips_duplicate_command() {
        let mut records = BTreeMap::new();
        records.insert(
            20,
            json!({
                "protocol_meta": {"response_id": 20, "last_command": "CHOOSE 0", "last_command_kind": "choose"},
                "game_state": {"combat_state": {}}
            }),
        );
        records.insert(
            21,
            json!({
                "protocol_meta": {
                    "response_id": 21,
                    "last_command": "CHOOSE 0",
                    "last_command_kind": "choose",
                    "recent_human_card_reward_choice": {"choice_kind": "card", "choice_index": 0}
                },
                "game_state": {"combat_state": {}}
            }),
        );

        let fixture = build_fixture_from_record_window(
            &records,
            20,
            21,
            "reward".to_string(),
            Vec::new(),
            Vec::new(),
            None,
        )
        .expect("fixture should build");

        assert_eq!(fixture.steps.len(), 1);
        assert_eq!(fixture.steps[0].command, "HUMAN_CARD_REWARD 0");
    }
}
