use super::frame::LiveFrame;
use super::unix_time_millis;
use super::watch::{build_noncombat_context_summary, build_noncombat_screen_summary};
use serde_json::{json, Map, Value};
use std::io::Write;

#[derive(Clone, Debug)]
pub(super) struct PendingHumanNoncombatAudit {
    pub(super) session_id: String,
    pub(super) domain: &'static str,
    pub(super) last_seen_frame: u64,
    pub(super) last_seen_screen: String,
    pub(super) last_seen_room_phase: String,
    pub(super) last_seen_response_id: Option<i64>,
    pub(super) last_seen_state_frame_id: Option<i64>,
    pub(super) last_bot_recommendation: String,
    pub(super) last_observed_command_id: Option<i64>,
    pub(super) hold_polls: u32,
    pub(super) polluted: bool,
    pub(super) pollution_reasons: Vec<String>,
    pub(super) payload: Map<String, Value>,
}

fn should_ignore_as_hold_command(kind: &str, command: &str) -> bool {
    if matches!(kind, "wait" | "state" | "handoff" | "scenario" | "start") {
        return true;
    }
    let trimmed = command.trim();
    trimmed.eq_ignore_ascii_case("WAIT 30")
        || trimmed.eq_ignore_ascii_case("STATE")
        || trimmed.eq_ignore_ascii_case("HANDOFF")
}

fn has_return_like_command(frame: &LiveFrame) -> bool {
    let available = frame.available_commands();
    available.contains(&"return") || available.contains(&"cancel")
}

fn push_update(payload: &mut Map<String, Value>, update: Value) {
    let entry = payload
        .entry("updates".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = entry.as_array_mut() {
        arr.push(update);
    }
}

fn append_human_command(
    pending: &mut PendingHumanNoncombatAudit,
    frame: &LiveFrame,
    current_screen: &str,
    current_room_phase: &str,
) -> Option<String> {
    let protocol_meta = frame.protocol_meta()?;
    let command_id = protocol_meta
        .get("last_command_id")
        .and_then(Value::as_i64)?;
    if pending.last_observed_command_id == Some(command_id) {
        return None;
    }
    pending.last_observed_command_id = Some(command_id);
    let kind = protocol_meta
        .get("last_command_kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    let command = protocol_meta
        .get("last_command")
        .and_then(Value::as_str)
        .unwrap_or("");
    if should_ignore_as_hold_command(kind, command) {
        return None;
    }

    let normalized = command.trim().to_string();
    push_update(
        &mut pending.payload,
        json!({
            "kind": "observed_command",
            "frame": pending.last_seen_frame,
            "response_id": frame.response_id(),
            "state_frame_id": frame.state_frame_id(),
            "last_command_id": command_id,
            "command_kind": kind,
            "command": normalized,
            "screen": current_screen,
            "room_phase": current_room_phase,
        }),
    );
    Some(normalized)
}

pub(super) fn human_noncombat_domain_for_frame(
    frame: &LiveFrame,
    pending: Option<&PendingHumanNoncombatAudit>,
) -> Option<&'static str> {
    let screen = frame.screen();
    let room_type = frame.room_type();

    if let Some(pending) = pending {
        match pending.domain {
            "shop"
                if room_type == "ShopRoom"
                    && (matches!(screen, "SHOP_ROOM" | "SHOP_SCREEN" | "GRID")
                        || (screen == "MAP" && has_return_like_command(frame))) =>
            {
                return Some("shop");
            }
            "event"
                if room_type == "EventRoom"
                    && (matches!(screen, "EVENT" | "GRID")
                        || (screen == "MAP" && has_return_like_command(frame))) =>
            {
                return Some("event");
            }
            "campfire"
                if room_type == "RestRoom"
                    && (matches!(screen, "REST" | "GRID")
                        || (screen == "MAP" && has_return_like_command(frame))) =>
            {
                return Some("campfire");
            }
            "boss_reward" if screen == "BOSS_REWARD" => {
                return Some("boss_reward");
            }
            "reward_claim" if screen == "COMBAT_REWARD" => {
                return Some("reward_claim");
            }
            "map" if screen == "MAP" => {
                return Some("map");
            }
            "grid" if screen == "GRID" => {
                return Some("grid");
            }
            _ => {}
        }
    }

    match screen {
        "CARD_REWARD" => None,
        "SHOP_ROOM" | "SHOP_SCREEN" => Some("shop"),
        "EVENT" => Some("event"),
        "REST" => Some("campfire"),
        "COMBAT_REWARD" => Some("reward_claim"),
        "BOSS_REWARD" => Some("boss_reward"),
        "MAP" => Some("map"),
        "GRID" => Some(match room_type {
            "ShopRoom" => "shop",
            "EventRoom" => "event",
            "RestRoom" => "campfire",
            _ => "grid",
        }),
        _ => None,
    }
}

pub(super) fn build_pending_human_noncombat_audit(
    frame: &LiveFrame,
    frame_count: u64,
    domain: &'static str,
    bot_recommendation: &str,
) -> PendingHumanNoncombatAudit {
    let response_id = frame.response_id();
    let state_frame_id = frame.state_frame_id();
    let screen = frame.screen();
    let room_phase = frame.room_phase();
    let room_type = frame.room_type();
    let session_id = format!(
        "human_noncombat:{}:{}:{}",
        domain,
        response_id.unwrap_or(-1),
        state_frame_id.unwrap_or(-1)
    );
    let mut payload = Map::new();
    payload.insert("kind".to_string(), json!("human_noncombat_session"));
    payload.insert("session_id".to_string(), json!(session_id));
    payload.insert("logged_at_unix_ms".to_string(), json!(unix_time_millis()));
    payload.insert("domain".to_string(), json!(domain));
    payload.insert(
        "entry".to_string(),
        json!({
            "frame": frame_count,
            "response_id": response_id,
            "state_frame_id": state_frame_id,
            "screen": screen,
            "room_phase": room_phase,
            "room_type": room_type,
            "bot_recommendation": bot_recommendation,
            "screen_summary": build_noncombat_screen_summary(frame.root()),
            "context_summary": build_noncombat_context_summary(frame.root()),
        }),
    );
    payload.insert("updates".to_string(), Value::Array(Vec::new()));

    PendingHumanNoncombatAudit {
        session_id,
        domain,
        last_seen_frame: frame_count,
        last_seen_screen: screen.to_string(),
        last_seen_room_phase: room_phase.to_string(),
        last_seen_response_id: response_id,
        last_seen_state_frame_id: state_frame_id,
        last_bot_recommendation: bot_recommendation.to_string(),
        last_observed_command_id: None,
        hold_polls: 0,
        polluted: false,
        pollution_reasons: Vec::new(),
        payload,
    }
}

pub(super) fn update_human_noncombat_audit(
    pending: &mut PendingHumanNoncombatAudit,
    frame: &LiveFrame,
    frame_count: u64,
    bot_recommendation: &str,
) -> Option<String> {
    pending.hold_polls += 1;
    pending.last_seen_frame = frame_count;
    pending.last_seen_response_id = frame.response_id();
    pending.last_seen_state_frame_id = frame.state_frame_id();

    if pending.last_seen_screen != frame.screen() {
        pending.last_seen_screen = frame.screen().to_string();
        push_update(
            &mut pending.payload,
            json!({
                "kind": "screen_transition",
                "frame": frame_count,
                "response_id": frame.response_id(),
                "state_frame_id": frame.state_frame_id(),
                "screen": frame.screen(),
                "room_phase": frame.room_phase(),
                "room_type": frame.room_type(),
                "screen_summary": build_noncombat_screen_summary(frame.root()),
            }),
        );
    }
    if pending.last_seen_room_phase != frame.room_phase() {
        pending.last_seen_room_phase = frame.room_phase().to_string();
        push_update(
            &mut pending.payload,
            json!({
                "kind": "room_phase_transition",
                "frame": frame_count,
                "response_id": frame.response_id(),
                "state_frame_id": frame.state_frame_id(),
                "screen": frame.screen(),
                "room_phase": frame.room_phase(),
            }),
        );
    }
    if pending.last_bot_recommendation != bot_recommendation {
        pending.last_bot_recommendation = bot_recommendation.to_string();
        push_update(
            &mut pending.payload,
            json!({
                "kind": "bot_recommendation",
                "frame": frame_count,
                "response_id": frame.response_id(),
                "state_frame_id": frame.state_frame_id(),
                "screen": frame.screen(),
                "room_phase": frame.room_phase(),
                "bot_recommendation": bot_recommendation,
                "screen_summary": build_noncombat_screen_summary(frame.root()),
            }),
        );
    }

    append_human_command(pending, frame, frame.screen(), frame.room_phase())
}

pub(super) fn mark_human_noncombat_audit_polluted(
    pending: &mut PendingHumanNoncombatAudit,
    reason: impl Into<String>,
) {
    let reason = reason.into();
    if !pending
        .pollution_reasons
        .iter()
        .any(|existing| existing == &reason)
    {
        pending.pollution_reasons.push(reason);
    }
    pending.polluted = true;
}

pub(super) fn finalize_human_noncombat_audit(
    mut pending: PendingHumanNoncombatAudit,
    frame: Option<&LiveFrame>,
    frame_count: u64,
    audit: &mut std::fs::File,
    log: &mut std::fs::File,
    status: &str,
    reason: &str,
) {
    let final_screen = frame
        .map(|value| value.screen().to_string())
        .unwrap_or_else(|| pending.last_seen_screen.clone());
    let final_room_phase = frame
        .map(|value| value.room_phase().to_string())
        .unwrap_or_else(|| pending.last_seen_room_phase.clone());
    let final_response_id = frame
        .and_then(LiveFrame::response_id)
        .or(pending.last_seen_response_id);
    let final_state_frame_id = frame
        .and_then(LiveFrame::state_frame_id)
        .or(pending.last_seen_state_frame_id);
    let final_screen_summary = frame
        .map(|value| build_noncombat_screen_summary(value.root()))
        .unwrap_or(Value::Null);
    let final_context_summary = frame
        .map(|value| build_noncombat_context_summary(value.root()))
        .unwrap_or(Value::Null);

    let final_command = pending
        .payload
        .get("updates")
        .and_then(Value::as_array)
        .and_then(|updates| {
            updates.iter().rev().find_map(|update| {
                if update.get("kind").and_then(Value::as_str) == Some("observed_command") {
                    update
                        .get("command")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                } else {
                    None
                }
            })
        });
    let bot_human_agree = final_command
        .as_ref()
        .map(|command| command == &pending.last_bot_recommendation);

    pending
        .payload
        .insert("hold_polls".to_string(), json!(pending.hold_polls));
    pending.payload.insert(
        "final".to_string(),
        json!({
            "frame": frame_count,
            "response_id": final_response_id,
            "state_frame_id": final_state_frame_id,
            "screen": final_screen,
            "room_phase": final_room_phase,
            "screen_summary": final_screen_summary,
            "context_summary": final_context_summary,
        }),
    );
    pending
        .payload
        .insert("final_status".to_string(), json!(status));
    pending
        .payload
        .insert("final_reason".to_string(), json!(reason));
    pending.payload.insert(
        "final_human_command".to_string(),
        final_command
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    pending.payload.insert(
        "last_bot_recommendation".to_string(),
        json!(pending.last_bot_recommendation),
    );
    pending.payload.insert(
        "bot_human_agree".to_string(),
        bot_human_agree.map(Value::Bool).unwrap_or(Value::Null),
    );
    pending
        .payload
        .insert("polluted".to_string(), json!(pending.polluted));
    pending.payload.insert(
        "pollution_reasons".to_string(),
        json!(pending.pollution_reasons),
    );

    let line = Value::Object(pending.payload);
    let _ = writeln!(audit, "{}", line);
    let _ = audit.flush();
    let _ = writeln!(
        log,
        "[F{}] HUMAN NONCOMBAT COMPLETE session={} domain={} status={} reason={} final_command={} agree={} polluted={}",
        frame_count,
        pending.session_id,
        pending.domain,
        status,
        reason,
        final_command.unwrap_or_else(|| "<none>".to_string()),
        bot_human_agree
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        pending.polluted
    );
}
