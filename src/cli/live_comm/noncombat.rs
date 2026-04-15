use super::reward_audit::{
    build_human_card_reward_pending, manual_card_reward_followup_command, reward_session_is_live,
    reward_session_protocol_supported, PendingHumanCardRewardAudit,
};
use crate::bot::Agent;
use crate::cli::live_comm_noncombat::{choose_best_index, decide_noncombat_with_agent};
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

pub(super) fn route_noncombat_command(
    agent: &mut Agent,
    parsed: &Value,
    screen: &str,
    avail: &[&str],
) -> String {
    let has = |c: &str| avail.contains(&c);
    let gs = &parsed["game_state"];
    let choice_list: Vec<&str> = gs
        .get("choice_list")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
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
        agent_cmd
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
