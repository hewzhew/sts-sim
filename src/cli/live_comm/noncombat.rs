use super::reward_audit::{
    build_human_card_reward_pending, manual_card_reward_followup_command, reward_session_is_live,
    reward_session_protocol_supported, PendingHumanCardRewardAudit,
};
use crate::bot::Agent;
use crate::cli::live_comm_noncombat::{choose_best_index, decide_noncombat_with_agent};
use crate::protocol::java::{
    build_screen_affordance_snapshot, NoncombatAffordanceSnapshot, ProtocolNoncombatActionKind,
};
use crate::runtime::combat::CombatState;
use serde_json::{json, Value};
use std::io::Write;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScreenActionSpaceValidation {
    pub reasons: Vec<String>,
    expected_field: &'static str,
    expected_capability: &'static str,
    screen: String,
    room_phase: String,
    exported_screen_type: Option<String>,
    action_count: Option<usize>,
}

impl ScreenActionSpaceValidation {
    pub(super) fn decision_context(&self, avail: &[&str]) -> Value {
        json!({
            "validation": "screen_action_space",
            "expected_field": self.expected_field,
            "expected_capability": self.expected_capability,
            "screen": self.screen,
            "room_phase": self.room_phase,
            "exported_screen_type": self.exported_screen_type,
            "action_count": self.action_count,
            "available_commands": avail,
        })
    }

    pub(super) fn expected_field(&self) -> &'static str {
        self.expected_field
    }
}

pub(super) fn maybe_arm_human_card_reward_audit(
    enabled: bool,
    pending_audit: &mut Option<PendingHumanCardRewardAudit>,
    parsed: &Value,
    last_combat_truth: Option<&CombatState>,
    log: &mut std::fs::File,
    frame_count: u64,
) -> bool {
    let screen = parsed["game_state"]["screen_type"].as_str().unwrap_or("");
    let protocol_reward_session = reward_session_protocol_supported(parsed);
    let should_arm = if protocol_reward_session {
        reward_session_is_live(parsed)
    } else {
        screen == "CARD_REWARD" || reward_session_is_live(parsed)
    };
    if !enabled || !should_arm {
        return false;
    }

    match build_human_card_reward_pending(parsed, last_combat_truth) {
        Some(pending) => {
            let should_log_pending = pending_audit
                .as_ref()
                .map(|current| {
                    current.state_frame_id != pending.state_frame_id
                        || current.offered_signature != pending.offered_signature
                })
                .unwrap_or(true);
            if should_log_pending {
                writeln!(
                    log,
                    "[F{}] CARD_REWARD human audit armed via {} → waiting for manual choice",
                    frame_count,
                    if screen == "CARD_REWARD" {
                        "screen_state"
                    } else {
                        "reward_session"
                    }
                )
                .unwrap();
                writeln!(
                    log,
                    "  [CARD_AUDIT] offered={} bot_recommendation={}",
                    pending.offered_signature.join(", "),
                    pending
                        .bot_recommended_choice
                        .map(|idx| idx.to_string())
                        .unwrap_or_else(|| "skip".to_string())
                )
                .unwrap();
                *pending_audit = Some(pending);
            } else if let Some(current) = pending_audit.as_mut() {
                current.offscreen_hold_polls = 0;
                current.last_hold_context = None;
            }
        }
        None => {
            writeln!(
                log,
                "[F{}] CARD_REWARD human audit requested but reward parsing failed",
                frame_count
            )
            .unwrap();
        }
    }

    true
}

fn protocol_noncombat_affordance(parsed: &Value) -> Option<NoncombatAffordanceSnapshot> {
    parsed.get("protocol_meta").and_then(|protocol_meta| {
        build_screen_affordance_snapshot(protocol_meta)
            .ok()
            .flatten()
    })
}

pub(super) fn validate_screen_action_space(
    root: &Value,
    screen: &str,
    room_phase: &str,
    avail: &[&str],
) -> Option<ScreenActionSpaceValidation> {
    if !screen_requires_typed_action_space(screen) {
        return None;
    }

    let (expected_field, expected_capability) = expected_action_space_source(room_phase);
    if !protocol_capability_enabled(root, expected_capability) {
        return None;
    }

    let mut report = ScreenActionSpaceValidation {
        reasons: Vec::new(),
        expected_field,
        expected_capability,
        screen: screen.to_string(),
        room_phase: room_phase.to_string(),
        exported_screen_type: None,
        action_count: None,
    };
    let Some(action_space) = root
        .get("protocol_meta")
        .and_then(|meta| meta.get(expected_field))
        .filter(|value| !value.is_null())
    else {
        report
            .reasons
            .push(format!("missing_screen_action_space:{expected_field}"));
        return Some(report);
    };
    let Some(action_space_object) = action_space.as_object() else {
        report
            .reasons
            .push(format!("invalid_screen_action_space:{expected_field}"));
        return Some(report);
    };

    report.exported_screen_type = action_space_object
        .get("screen_type")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    match report.exported_screen_type.as_deref() {
        Some(exported) if exported == screen => {}
        Some(_) => report.reasons.push(format!(
            "screen_action_space_screen_type_mismatch:{expected_field}"
        )),
        None => report.reasons.push(format!(
            "missing_screen_action_space_screen_type:{expected_field}"
        )),
    }

    let Some(actions) = action_space_object.get("actions").and_then(Value::as_array) else {
        report.reasons.push(format!(
            "missing_screen_action_space_actions:{expected_field}"
        ));
        return (!report.reasons.is_empty()).then_some(report);
    };
    report.action_count = Some(actions.len());
    if actions.is_empty() && frame_has_legacy_screen_command_signal(root, avail) {
        report
            .reasons
            .push(format!("empty_screen_action_space:{expected_field}"));
    }

    for (index, action) in actions.iter().enumerate() {
        let action_id = action
            .get("action_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("{expected_field}[{index}]"));
        let kind = action.get("kind").and_then(Value::as_str);
        if kind.is_none() {
            report.reasons.push(format!(
                "invalid_screen_action_space_kind:{expected_field}:{action_id}"
            ));
        }
        if action.get("command").and_then(Value::as_str).is_none() {
            report.reasons.push(format!(
                "invalid_screen_action_space_command:{expected_field}:{action_id}"
            ));
        }
        if kind.is_some_and(|value| matches!(value, "choose" | "submit_choice"))
            && action.get("choice_index").and_then(Value::as_u64).is_none()
        {
            report.reasons.push(format!(
                "invalid_screen_action_space_choice_index:{expected_field}:{action_id}"
            ));
        }
    }

    (!report.reasons.is_empty()).then_some(report)
}

fn expected_action_space_source(room_phase: &str) -> (&'static str, &'static str) {
    if room_phase == "COMBAT" {
        ("combat_action_space", "combat_action_space")
    } else {
        ("noncombat_action_space", "noncombat_action_space")
    }
}

fn protocol_capability_enabled(root: &Value, capability: &str) -> bool {
    root.get("protocol_meta")
        .and_then(|meta| meta.get("capabilities"))
        .and_then(|caps| caps.get(capability))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn screen_requires_typed_action_space(screen: &str) -> bool {
    matches!(
        screen,
        "EVENT"
            | "CHEST"
            | "SHOP_ROOM"
            | "REST"
            | "CARD_REWARD"
            | "COMBAT_REWARD"
            | "MAP"
            | "BOSS_REWARD"
            | "SHOP_SCREEN"
            | "GRID"
            | "HAND_SELECT"
            | "COMPLETE"
    )
}

fn frame_has_legacy_screen_command_signal(root: &Value, avail: &[&str]) -> bool {
    let command_signal = avail.iter().any(|command| {
        matches!(
            command.to_ascii_lowercase().as_str(),
            "choose" | "proceed" | "skip" | "confirm" | "leave" | "cancel" | "return" | "potion"
        )
    });
    let choice_signal = root
        .get("game_state")
        .and_then(|gs| gs.get("choice_list"))
        .and_then(Value::as_array)
        .is_some_and(|choices| !choices.is_empty());
    command_signal || choice_signal
}

fn protocol_choice_labels(snapshot: Option<&NoncombatAffordanceSnapshot>) -> Vec<String> {
    snapshot
        .map(NoncombatAffordanceSnapshot::choice_labels)
        .unwrap_or_default()
}

fn protocol_command_for_requested(
    snapshot: &NoncombatAffordanceSnapshot,
    requested: &str,
) -> Option<String> {
    if let Some(exact) = snapshot.first_command_matching(requested) {
        return Some(exact.to_string());
    }

    let parts = requested.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [verb, idx] if verb.eq_ignore_ascii_case("choose") => idx
            .parse::<usize>()
            .ok()
            .and_then(|choice_index| snapshot.command_for_choice_index(choice_index))
            .map(ToOwned::to_owned),
        [verb] if verb.eq_ignore_ascii_case("proceed") || verb.eq_ignore_ascii_case("confirm") => {
            snapshot
                .first_command_for_kind(ProtocolNoncombatActionKind::Proceed)
                .map(ToOwned::to_owned)
        }
        [verb]
            if verb.eq_ignore_ascii_case("skip")
                || verb.eq_ignore_ascii_case("cancel")
                || verb.eq_ignore_ascii_case("return")
                || verb.eq_ignore_ascii_case("leave") =>
        {
            snapshot
                .first_command_for_kind(ProtocolNoncombatActionKind::Cancel)
                .map(ToOwned::to_owned)
        }
        [verb, action, slot]
            if verb.eq_ignore_ascii_case("potion") && action.eq_ignore_ascii_case("discard") =>
        {
            slot.parse::<usize>()
                .ok()
                .and_then(|slot| snapshot.command_for_potion_discard_slot(slot))
                .map(ToOwned::to_owned)
        }
        _ => None,
    }
}

fn protocol_fallback_noncombat_command(
    snapshot: &NoncombatAffordanceSnapshot,
    screen: &str,
    choice_list: &[&str],
    potions_full: bool,
) -> Option<String> {
    if screen != "SHOP_ROOM" {
        if let Some(command) = snapshot.first_command_matching("LEAVE") {
            return Some(command.to_string());
        }
    }

    if screen == "SHOP_ROOM" {
        if !choice_list.is_empty() {
            if let Some(command) = snapshot.command_for_choice_index(0) {
                return Some(command.to_string());
            }
        }
        if let Some(command) = snapshot.first_command_for_kind(ProtocolNoncombatActionKind::Proceed)
        {
            return Some(command.to_string());
        }
    }

    if !choice_list.is_empty() {
        if choice_list[0] == "potion" && potions_full {
            if let Some(command) = snapshot.command_for_potion_discard_slot(0) {
                return Some(command.to_string());
            }
        } else if choice_list.len() == 1 && choice_list[0] == "potion" {
            if let Some(command) = snapshot.first_command_matching("SKIP") {
                return Some(command.to_string());
            }
        } else {
            let choice_index = choose_best_index(choice_list);
            if let Some(command) = snapshot.command_for_choice_index(choice_index) {
                return Some(command.to_string());
            }
        }
    }

    snapshot
        .first_command_for_kind(ProtocolNoncombatActionKind::Proceed)
        .or_else(|| snapshot.first_command_for_kind(ProtocolNoncombatActionKind::Cancel))
        .map(ToOwned::to_owned)
}

pub(super) fn route_noncombat_command(
    agent: &mut Agent,
    parsed: &Value,
    screen: &str,
    avail: &[&str],
) -> String {
    let has = |c: &str| avail.contains(&c);
    let gs = &parsed["game_state"];
    let protocol_affordance = protocol_noncombat_affordance(parsed);
    let protocol_choice_labels = protocol_choice_labels(protocol_affordance.as_ref());
    let legacy_choice_list: Vec<&str> = gs
        .get("choice_list")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let choice_list: Vec<&str> = if protocol_choice_labels.is_empty() {
        legacy_choice_list
    } else {
        protocol_choice_labels.iter().map(String::as_str).collect()
    };
    let potions_full = if let Some(arr) = gs.get("potions").and_then(|v| v.as_array()) {
        arr.iter().all(|p| {
            p.get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("Potion Slot")
                != "Potion Slot"
        })
    } else {
        false
    };

    if let Some(manual_reward_cmd) = manual_card_reward_followup_command(parsed, screen) {
        manual_reward_cmd
    } else if let Some(agent_cmd) = decide_noncombat_with_agent(agent, parsed, screen, &choice_list)
    {
        protocol_affordance
            .as_ref()
            .and_then(|snapshot| protocol_command_for_requested(snapshot, &agent_cmd))
            .unwrap_or(agent_cmd)
    } else if let Some(protocol_cmd) = protocol_affordance.as_ref().and_then(|snapshot| {
        protocol_fallback_noncombat_command(snapshot, screen, &choice_list, potions_full)
    }) {
        protocol_cmd
    } else if has("leave") && screen != "SHOP_ROOM" {
        "LEAVE".to_string()
    } else if screen == "SHOP_ROOM" && has("choose") && !choice_list.is_empty() {
        "CHOOSE 0".to_string()
    } else if screen == "SHOP_ROOM" && has("proceed") {
        "PROCEED".to_string()
    } else if has("choose") && !choice_list.is_empty() {
        if choice_list[0] == "potion" && potions_full {
            "POTION DISCARD 0".to_string()
        } else if choice_list.len() == 1 && choice_list[0] == "potion" && has("skip") {
            "SKIP".to_string()
        } else {
            format!("CHOOSE {}", choose_best_index(&choice_list))
        }
    } else if has("proceed") {
        "PROCEED".to_string()
    } else if has("confirm") {
        "CONFIRM".to_string()
    } else if has("skip") {
        "SKIP".to_string()
    } else if has("leave") {
        "LEAVE".to_string()
    } else if has("cancel") || has("return") {
        "RETURN".to_string()
    } else if has("wait") {
        "WAIT 30".to_string()
    } else {
        "STATE".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{route_noncombat_command, validate_screen_action_space};
    use crate::bot::Agent;
    use serde_json::json;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;

    fn load_screen_action_fixture(name: &str) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("protocol_screen_action_space")
            .join(format!("{name}.json"));
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()));
        serde_json::from_str(&raw)
            .unwrap_or_else(|err| panic!("failed to parse fixture {}: {err}", path.display()))
    }

    #[test]
    fn noncombat_route_uses_protocol_action_space_without_choice_list() {
        let parsed = json!({
            "protocol_meta": {
                "noncombat_action_space": {
                    "screen_type": "CHEST",
                    "actions": [
                        {
                            "action_id": "choice:chest:0",
                            "kind": "choose",
                            "command": "CHOOSE 0",
                            "choice_index": 0,
                            "choice_label": "open"
                        }
                    ]
                }
            },
            "game_state": {
                "screen_type": "CHEST"
            }
        });
        let mut agent = Agent::new();

        let command = route_noncombat_command(&mut agent, &parsed, "CHEST", &[]);

        assert_eq!(command, "CHOOSE 0");
    }

    #[test]
    fn noncombat_route_normalizes_agent_command_through_protocol_action_space() {
        let parsed = json!({
            "protocol_meta": {
                "noncombat_action_space": {
                    "screen_type": "REST",
                    "actions": [
                        {
                            "action_id": "proceed:rest",
                            "kind": "proceed",
                            "command": "PROCEED"
                        }
                    ]
                }
            },
            "game_state": {
                "screen_type": "REST",
                "seed": 1,
                "ascension_level": 0,
                "class": "IRONCLAD"
            }
        });
        let mut agent = Agent::new();

        let command = route_noncombat_command(&mut agent, &parsed, "REST", &[]);

        assert_eq!(command, "PROCEED");
    }

    #[test]
    fn noncombat_route_uses_combat_action_space_for_pending_combat_screen() {
        let parsed = json!({
            "protocol_meta": {
                "combat_action_space": {
                    "screen_type": "GRID",
                    "actions": [
                        {
                            "action_id": "choice:grid:0",
                            "kind": "submit_choice",
                            "command": "CHOOSE 0",
                            "choice_index": 0,
                            "choice_label": "strike"
                        },
                        {
                            "action_id": "proceed:grid",
                            "kind": "proceed",
                            "command": "CONFIRM"
                        }
                    ]
                }
            },
            "game_state": {
                "screen_type": "GRID"
            }
        });
        let mut agent = Agent::new();

        let command = route_noncombat_command(&mut agent, &parsed, "GRID", &[]);

        assert_eq!(command, "CHOOSE 0");
    }

    #[test]
    fn noncombat_route_uses_grid_contract_fixture_without_legacy_choice_list() {
        let parsed = load_screen_action_fixture("combat_grid_select");
        let mut agent = Agent::new();

        let command = route_noncombat_command(&mut agent, &parsed, "GRID", &[]);

        assert_eq!(command, "CHOOSE 0");
    }

    #[test]
    fn noncombat_route_uses_discovery_contract_fixture_without_legacy_choice_list() {
        let parsed = load_screen_action_fixture("combat_discovery_card_reward");
        let mut agent = Agent::new();

        let command = route_noncombat_command(&mut agent, &parsed, "CARD_REWARD", &[]);

        assert_eq!(command, "CHOOSE 0");
    }

    #[test]
    fn screen_action_space_validation_accepts_contract_fixtures() {
        for (fixture, screen, room_phase) in [
            ("combat_grid_select", "GRID", "COMBAT"),
            ("combat_discovery_card_reward", "CARD_REWARD", "COMBAT"),
            ("noncombat_card_reward", "CARD_REWARD", "COMPLETE"),
        ] {
            let parsed = load_screen_action_fixture(fixture);

            assert_eq!(
                validate_screen_action_space(&parsed, screen, room_phase, &[]),
                None,
                "{fixture}"
            );
        }
    }

    #[test]
    fn screen_action_space_validation_flags_missing_combat_field() {
        let parsed = json!({
            "available_commands": ["choose"],
            "protocol_meta": {
                "capabilities": {
                    "combat_action_space": true
                }
            },
            "game_state": {
                "screen_type": "GRID",
                "room_phase": "COMBAT"
            }
        });

        let report = validate_screen_action_space(&parsed, "GRID", "COMBAT", &["choose"]).unwrap();

        assert_eq!(report.expected_field(), "combat_action_space");
        assert!(report
            .reasons
            .contains(&"missing_screen_action_space:combat_action_space".to_string()));
    }

    #[test]
    fn screen_action_space_validation_flags_screen_type_mismatch() {
        let parsed = json!({
            "protocol_meta": {
                "capabilities": {
                    "combat_action_space": true
                },
                "combat_action_space": {
                    "screen_type": "CARD_REWARD",
                    "actions": [
                        {
                            "action_id": "choice:grid:0",
                            "kind": "submit_choice",
                            "command": "CHOOSE 0",
                            "choice_index": 0
                        }
                    ]
                }
            },
            "game_state": {
                "screen_type": "GRID",
                "room_phase": "COMBAT"
            }
        });

        let report = validate_screen_action_space(&parsed, "GRID", "COMBAT", &[]).unwrap();

        assert!(report
            .reasons
            .contains(&"screen_action_space_screen_type_mismatch:combat_action_space".to_string()));
    }

    #[test]
    fn screen_action_space_validation_flags_empty_actions_with_legacy_command_signal() {
        let parsed = json!({
            "available_commands": ["choose"],
            "protocol_meta": {
                "capabilities": {
                    "noncombat_action_space": true
                },
                "noncombat_action_space": {
                    "screen_type": "EVENT",
                    "actions": []
                }
            },
            "game_state": {
                "screen_type": "EVENT",
                "room_phase": "EVENT",
                "choice_list": ["option"]
            }
        });

        let report = validate_screen_action_space(&parsed, "EVENT", "EVENT", &["choose"]).unwrap();

        assert!(report
            .reasons
            .contains(&"empty_screen_action_space:noncombat_action_space".to_string()));
    }
}
