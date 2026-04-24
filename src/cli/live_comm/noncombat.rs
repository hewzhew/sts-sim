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
use serde_json::Value;
use std::io::Write;

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
    use super::route_noncombat_command;
    use crate::bot::Agent;
    use serde_json::json;

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
}
