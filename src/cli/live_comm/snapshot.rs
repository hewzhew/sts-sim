use super::frame::LiveFrame;
use super::io::LiveCommIo;
use crate::protocol::java::{build_live_observation_snapshot, build_live_truth_snapshot};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;

#[derive(Serialize)]
pub(crate) struct FailureSnapshotRecord {
    pub snapshot_id: String,
    pub frame: u64,
    pub response_id: Option<i64>,
    pub state_frame_id: Option<i64>,
    pub screen: String,
    pub room_phase: String,
    pub room_type: String,
    pub trigger_kind: String,
    pub reasons: Vec<String>,
    pub normalized_state: Value,
    pub decision_context: Value,
    pub protocol_context: Value,
}

pub(crate) fn write_failure_snapshot(
    live_io: &mut LiveCommIo,
    frame_count: u64,
    frame: &LiveFrame,
    trigger_kind: &str,
    reasons: Vec<String>,
    decision_context: Value,
) -> Option<String> {
    let snapshot_id = format!(
        "f{}_r{}_s{}_{}",
        frame_count,
        frame.response_id().unwrap_or(-1),
        frame.state_frame_id().unwrap_or(-1),
        trigger_kind
    );
    let record = FailureSnapshotRecord {
        snapshot_id: snapshot_id.clone(),
        frame: frame_count,
        response_id: frame.response_id(),
        state_frame_id: frame.state_frame_id(),
        screen: frame.screen().to_string(),
        room_phase: frame.room_phase().to_string(),
        room_type: frame.room_type().to_string(),
        trigger_kind: trigger_kind.to_string(),
        reasons: reasons.clone(),
        normalized_state: build_normalized_state(frame),
        decision_context,
        protocol_context: build_protocol_context(frame),
    };
    let encoded = serde_json::to_string(&record).ok()?;
    let _ = writeln!(live_io.failure_snapshots, "{}", encoded);
    let _ = live_io.failure_snapshots.flush();
    let _ = writeln!(
        live_io.focus_log,
        "[SNAPSHOT] frame={} kind={} reasons={} snapshot_id={}",
        frame_count,
        trigger_kind,
        reasons.join(","),
        snapshot_id
    );
    Some(snapshot_id)
}

fn build_normalized_state(frame: &LiveFrame) -> Value {
    let gs = frame.game_state();
    let (truth_snapshot, observation_snapshot) = if frame.is_combat() {
        (
            Some(build_live_truth_snapshot(gs)),
            Some(build_live_observation_snapshot(gs)),
        )
    } else {
        (None, None)
    };
    let player = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("player"));
    let hand = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("hand"))
        .and_then(Value::as_array)
        .map(|cards| compact_card_entries(cards))
        .unwrap_or_default();
    let draw = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("draw_pile"))
        .and_then(Value::as_array)
        .map(|cards| compact_card_entries(cards))
        .unwrap_or_default();
    let discard = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("discard_pile"))
        .and_then(Value::as_array)
        .map(|cards| compact_card_entries(cards))
        .unwrap_or_default();
    let exhaust = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("exhaust_pile"))
        .and_then(Value::as_array)
        .map(|cards| compact_card_entries(cards))
        .unwrap_or_default();
    let monsters = truth_snapshot
        .as_ref()
        .and_then(|state| state.get("monsters"))
        .and_then(Value::as_array)
        .map(|monsters| {
            let observation_monsters = observation_snapshot
                .as_ref()
                .and_then(|state| state.get("monsters"))
                .and_then(Value::as_array);
            monsters
                .iter()
                .enumerate()
                .map(|(index, monster)| {
                    let observation_monster =
                        observation_monsters.and_then(|entries| entries.get(index));
                    json!({
                        "id": monster.get("id").and_then(Value::as_str),
                        "name": monster.get("name").and_then(Value::as_str),
                        "current_hp": monster.get("current_hp").or_else(|| monster.get("hp")).and_then(Value::as_i64),
                        "max_hp": monster.get("max_hp").and_then(Value::as_i64),
                        "block": monster.get("block").and_then(Value::as_i64),
                        "intent": observation_monster
                            .and_then(|value| value.get("intent"))
                            .and_then(Value::as_str),
                        "powers": compact_power_entries(monster.get("powers").and_then(Value::as_array)),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "screen_name": frame.screen_name(),
        "floor": gs.get("floor").and_then(Value::as_i64),
        "act": gs.get("act").and_then(Value::as_i64),
        "gold": gs.get("gold").and_then(Value::as_i64),
        "player": {
            "current_hp": player.and_then(|value| value.get("current_hp").or_else(|| value.get("hp"))).and_then(Value::as_i64),
            "max_hp": player.and_then(|value| value.get("max_hp")).and_then(Value::as_i64),
            "block": player.and_then(|value| value.get("block")).and_then(Value::as_i64),
            "energy": truth_snapshot
                .as_ref()
                .and_then(|state| state.get("energy"))
                .or_else(|| player.and_then(|value| value.get("energy")))
                .or_else(|| gs.get("energy"))
                .and_then(Value::as_i64),
            "powers": compact_power_entries(player.and_then(|value| value.get("powers")).and_then(Value::as_array)),
        },
        "zones": {
            "hand": hand,
            "draw": draw,
            "discard": discard,
            "exhaust": exhaust,
            "hand_count": truth_snapshot.as_ref().and_then(|state| state.get("hand")).and_then(Value::as_array).map(|cards| cards.len()),
            "draw_count": truth_snapshot.as_ref().and_then(|state| state.get("draw_pile")).and_then(Value::as_array).map(|cards| cards.len()),
            "discard_count": truth_snapshot.as_ref().and_then(|state| state.get("discard_pile")).and_then(Value::as_array).map(|cards| cards.len()),
            "exhaust_count": truth_snapshot.as_ref().and_then(|state| state.get("exhaust_pile")).and_then(Value::as_array).map(|cards| cards.len()),
        },
        "monsters": monsters,
        "relics": compact_id_entries(gs.get("relics").and_then(Value::as_array)),
        "potions": compact_id_entries(gs.get("potions").and_then(Value::as_array)),
        "screen_state": compact_screen_state(gs.get("screen_state")),
    })
}

fn build_protocol_context(frame: &LiveFrame) -> Value {
    let protocol_meta = frame.protocol_meta().cloned().unwrap_or(Value::Null);
    json!({
        "available_commands": frame.available_commands(),
        "combat_session": frame.combat_session().cloned().unwrap_or(Value::Null),
        "continuation_state": frame.continuation_state().cloned().unwrap_or(Value::Null),
        "reward_session": protocol_meta.get("reward_session").cloned().unwrap_or(Value::Null),
        "last_command_kind": frame.last_command_kind(),
        "protocol_meta": protocol_meta,
    })
}

fn compact_card_entries(cards: &[Value]) -> Vec<Value> {
    cards
        .iter()
        .map(|card| {
            json!({
                "id": card.get("id").and_then(Value::as_str),
                "name": card.get("name").and_then(Value::as_str),
                "cost": card.get("cost").and_then(Value::as_i64),
                "upgrades": card.get("upgrades").and_then(Value::as_i64),
            })
        })
        .collect()
}

fn compact_power_entries(powers: Option<&Vec<Value>>) -> Vec<Value> {
    powers
        .map(|powers| {
            powers
                .iter()
                .map(|power| {
                    json!({
                        "id": power.get("id").and_then(Value::as_str),
                        "amount": power.get("amount").and_then(Value::as_i64),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn compact_id_entries(entries: Option<&Vec<Value>>) -> Vec<Value> {
    entries
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    json!({
                        "id": entry.get("id").and_then(Value::as_str),
                        "name": entry.get("name").and_then(Value::as_str),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn compact_screen_state(screen_state: Option<&Value>) -> Value {
    let Some(screen_state) = screen_state else {
        return Value::Null;
    };
    let event_id = screen_state
        .get("event_name")
        .and_then(Value::as_str)
        .or_else(|| screen_state.get("event_id").and_then(Value::as_str))
        .and_then(crate::engine::event_handler::event_id_from_name);
    let current_screen = screen_state
        .get("current_screen_index")
        .or_else(|| screen_state.get("current_screen"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let semantics_required = event_id
        .map(|event_id| {
            crate::engine::event_handler::live_event_requires_semantics_state(
                event_id,
                current_screen,
            )
        })
        .unwrap_or(false);
    json!({
        "event_id": screen_state.get("event_id").and_then(Value::as_str),
        "event_name": screen_state.get("event_name").and_then(Value::as_str),
        "current_screen": screen_state.get("current_screen").and_then(Value::as_i64),
        "current_screen_index": screen_state.get("current_screen_index").and_then(Value::as_i64),
        "current_screen_key": screen_state.get("current_screen_key").and_then(Value::as_str),
        "screen_source": screen_state.get("screen_source").and_then(Value::as_str),
        "event_semantics_state": screen_state.get("event_semantics_state").cloned().unwrap_or(Value::Null),
        "event_semantics_required": semantics_required,
        "event_semantics_keys": event_id
            .and_then(|event_id| crate::engine::event_handler::live_event_semantics_state_keys(event_id, current_screen))
            .map(|keys| keys.to_vec())
            .unwrap_or_default(),
        "reward_count": screen_state.get("rewards").and_then(Value::as_array).map(|rewards| rewards.len()),
        "card_count": screen_state.get("cards").and_then(Value::as_array).map(|cards| cards.len()),
        "choice_count": screen_state.get("options").and_then(Value::as_array).map(|options| options.len()),
    })
}
