use super::combat::build_live_combat_snapshot;
use super::unix_time_millis;
use crate::action::CardDestination;
use crate::cli::live_comm_noncombat::build_live_run_state;
use crate::combat::CombatState;
use crate::diff::mapper::card_id_from_java;
use crate::state::core::{ClientInput, EngineState, PendingChoice};
use serde_json::{json, Map, Value};
use std::io::Write;

pub(super) struct PendingHumanCardRewardAudit {
    pub(super) session_id: Option<String>,
    pub(super) state_frame_id: Option<i64>,
    pub(super) offered_signature: Vec<String>,
    pub(super) payload: Map<String, Value>,
    pub(super) bot_recommended_choice: Option<usize>,
    pub(super) replay_truth: Option<CombatState>,
    pub(super) replay_engine_state: Option<EngineState>,
    pub(super) offscreen_hold_polls: u32,
    pub(super) last_hold_context: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HumanCardRewardAuditDisposition {
    Hold { reason: &'static str },
    Abandon { reason: &'static str },
}

pub(super) fn build_human_card_reward_pending(
    root: &Value,
    last_combat_truth: Option<&CombatState>,
) -> Option<PendingHumanCardRewardAudit> {
    let gs = root.get("game_state")?;
    let rs = build_live_run_state(gs)?;
    let cards = gs
        .get("screen_state")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())?;

    let mut offered_ids = Vec::new();
    let mut offered_signature = Vec::new();
    let mut offered_cards_json = Vec::new();
    for card in cards {
        let java_id = card.get("id").and_then(|v| v.as_str())?;
        let card_id = card_id_from_java(java_id)?;
        let upgrades = card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let name = card
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::content::cards::get_card_definition(card_id).name);
        offered_ids.push(card_id);
        offered_signature.push(format!("{java_id}+{upgrades}"));
        offered_cards_json.push(json!({
            "java_id": java_id,
            "rust_card_id": format!("{:?}", card_id),
            "name": name,
            "upgrades": upgrades
        }));
    }
    if offered_ids.is_empty() {
        return None;
    }

    let evaluation =
        crate::bot::reward_heuristics::evaluate_reward_screen_for_run_detailed(&offered_ids, &rs);
    let meta = root.get("protocol_meta");
    let reward_session = extract_reward_session(root);
    let mut payload = Map::new();
    payload.insert("logged_at_unix_ms".to_string(), json!(unix_time_millis()));
    payload.insert(
        "response_id".to_string(),
        json!(meta
            .and_then(|m| m.get("response_id"))
            .and_then(|v| v.as_i64())),
    );
    payload.insert(
        "state_frame_id".to_string(),
        json!(meta
            .and_then(|m| m.get("state_frame_id"))
            .and_then(|v| v.as_i64())),
    );
    payload.insert(
        "floor".to_string(),
        json!(gs.get("floor").and_then(|v| v.as_i64()).unwrap_or(0)),
    );
    payload.insert(
        "act".to_string(),
        json!(gs.get("act").and_then(|v| v.as_i64()).unwrap_or(0)),
    );
    payload.insert(
        "class".to_string(),
        json!(gs
            .get("class")
            .and_then(|v| v.as_str())
            .unwrap_or("IRONCLAD")),
    );
    payload.insert(
        "current_hp".to_string(),
        json!(gs.get("current_hp").and_then(|v| v.as_i64()).unwrap_or(0)),
    );
    payload.insert(
        "max_hp".to_string(),
        json!(gs.get("max_hp").and_then(|v| v.as_i64()).unwrap_or(0)),
    );
    payload.insert(
        "gold".to_string(),
        json!(gs.get("gold").and_then(|v| v.as_i64()).unwrap_or(0)),
    );
    payload.insert("deck_size".to_string(), json!(rs.master_deck.len()));
    payload.insert(
        "offered_cards".to_string(),
        Value::Array(offered_cards_json),
    );
    payload.insert(
        "bot_evaluation".to_string(),
        reward_screen_evaluation_to_json(&evaluation),
    );
    payload.insert(
        "bot_recommended_choice".to_string(),
        recommended_choice_to_json(evaluation.recommended_choice),
    );
    payload.insert(
        "reward_session".to_string(),
        reward_session.cloned().unwrap_or(Value::Null),
    );

    let (replay_truth, replay_engine_state) =
        build_human_card_reward_replay_context(root, offered_ids.clone(), last_combat_truth);

    Some(PendingHumanCardRewardAudit {
        session_id: reward_session_id(root).map(str::to_string),
        state_frame_id: meta
            .and_then(|m| m.get("state_frame_id"))
            .and_then(|v| v.as_i64()),
        offered_signature,
        payload,
        bot_recommended_choice: evaluation.recommended_choice,
        replay_truth,
        replay_engine_state,
        offscreen_hold_polls: 0,
        last_hold_context: None,
    })
}

pub(super) fn finalize_human_card_reward_audit(
    mut pending: PendingHumanCardRewardAudit,
    root: &Value,
    reward_audit: &mut std::fs::File,
    log: &mut std::fs::File,
    last_combat_truth: &mut Option<CombatState>,
    last_input: &mut Option<ClientInput>,
    expected_combat_state: &mut Option<CombatState>,
) {
    let human_choice = extract_human_card_reward_choice(root);
    if let Some(choice) = human_choice.as_ref() {
        if apply_human_card_reward_to_prediction(
            &mut pending,
            choice,
            last_combat_truth,
            last_input,
            expected_combat_state,
        ) {
            let _ = writeln!(
                log,
                "  [CARD_AUDIT] prediction chain updated from human choice"
            );
        }
    }
    let agrees = compute_human_reward_choice_agreement(
        pending.bot_recommended_choice,
        human_choice.as_ref(),
    );
    pending.payload.insert(
        "human_choice".to_string(),
        human_choice.clone().unwrap_or(Value::Null),
    );
    pending
        .payload
        .insert("bot_human_agree".to_string(), agrees.unwrap_or(Value::Null));
    pending.payload.insert(
        "finalized_at_response_id".to_string(),
        json!(root
            .get("protocol_meta")
            .and_then(|m| m.get("response_id"))
            .and_then(|v| v.as_i64())),
    );

    let line = Value::Object(pending.payload);
    let _ = writeln!(reward_audit, "{}", line);
    let _ = reward_audit.flush();
    let _ = writeln!(
        log,
        "  [CARD_AUDIT COMPLETE] human_choice={} agree={}",
        line.get("human_choice")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        line.get("bot_human_agree")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
}

pub(super) fn finalize_human_card_reward_audit_without_choice(
    mut pending: PendingHumanCardRewardAudit,
    root: &Value,
    reward_audit: &mut std::fs::File,
    log: &mut std::fs::File,
    reason: &str,
) {
    pending
        .payload
        .insert("human_choice".to_string(), Value::Null);
    pending
        .payload
        .insert("bot_human_agree".to_string(), Value::Null);
    pending.payload.insert(
        "finalized_at_response_id".to_string(),
        json!(root
            .get("protocol_meta")
            .and_then(|m| m.get("response_id"))
            .and_then(|v| v.as_i64())),
    );
    pending
        .payload
        .insert("audit_status".to_string(), json!("incomplete"));
    pending
        .payload
        .insert("audit_reason".to_string(), json!(reason));

    let line = Value::Object(pending.payload);
    let _ = writeln!(reward_audit, "{}", line);
    let _ = reward_audit.flush();
    let _ = writeln!(
        log,
        "  [CARD_AUDIT ABANDONED] reason={} offered={}",
        reason,
        pending.offered_signature.join(", ")
    );
}

pub(super) fn human_card_reward_hold_context(root: &Value) -> String {
    let gs = match root.get("game_state") {
        Some(gs) => gs,
        None => return "missing_game_state".to_string(),
    };
    let screen = gs.get("screen_type").and_then(|v| v.as_str()).unwrap_or("");
    let screen_name = gs.get("screen_name").and_then(|v| v.as_str()).unwrap_or("");
    let room_phase = gs.get("room_phase").and_then(|v| v.as_str()).unwrap_or("");
    format!("{screen}|{screen_name}|{room_phase}")
}

fn is_human_card_reward_inspect_context(screen: &str, screen_name: &str) -> bool {
    screen_name == "MASTER_DECK_VIEW" || screen == "MAP" || screen_name == "MAP"
}

pub(super) fn classify_human_card_reward_audit_disposition(
    root: &Value,
) -> HumanCardRewardAuditDisposition {
    if let Some(reward_state) = reward_session_state(root) {
        return match reward_state {
            "active" | "temporarily_offscreen" => HumanCardRewardAuditDisposition::Hold {
                reason: "reward_session_active",
            },
            "closed_without_choice" => HumanCardRewardAuditDisposition::Abandon {
                reason: "reward_session_closed_without_choice",
            },
            "resolved" => HumanCardRewardAuditDisposition::Abandon {
                reason: "reward_session_resolved_without_choice_payload",
            },
            _ => HumanCardRewardAuditDisposition::Hold {
                reason: "reward_session_unknown_state",
            },
        };
    }

    let gs = match root.get("game_state") {
        Some(gs) => gs,
        None => {
            return HumanCardRewardAuditDisposition::Abandon {
                reason: "missing_game_state",
            };
        }
    };
    let screen = gs.get("screen_type").and_then(|v| v.as_str()).unwrap_or("");
    let screen_name = gs.get("screen_name").and_then(|v| v.as_str()).unwrap_or("");
    let room_phase = gs.get("room_phase").and_then(|v| v.as_str()).unwrap_or("");

    if is_human_card_reward_inspect_context(screen, screen_name) {
        return HumanCardRewardAuditDisposition::Hold {
            reason: "temporary_reward_inspect_screen",
        };
    }

    if screen == "COMBAT_REWARD" {
        return HumanCardRewardAuditDisposition::Abandon {
            reason: "reward_context_closed_without_human_choice",
        };
    }

    if screen == "REST"
        || screen == "SHOP_SCREEN"
        || screen == "SHOP_ROOM"
        || screen == "EVENT"
        || screen == "GAME_OVER"
        || screen == "DEATH"
        || (screen == "NONE" && room_phase != "COMBAT")
    {
        return HumanCardRewardAuditDisposition::Abandon {
            reason: "screen_left_without_human_choice",
        };
    }

    HumanCardRewardAuditDisposition::Hold {
        reason: "transient_reward_transition",
    }
}

fn build_human_card_reward_replay_context(
    root: &Value,
    offered_ids: Vec<crate::content::cards::CardId>,
    last_combat_truth: Option<&CombatState>,
) -> (Option<CombatState>, Option<EngineState>) {
    let gs = match root.get("game_state") {
        Some(gs) => gs,
        None => return (None, None),
    };
    if gs.get("screen_type").and_then(|v| v.as_str()) != Some("CARD_REWARD") {
        return (None, None);
    }
    if gs.get("combat_state").is_none_or(|v| v.is_null()) {
        return (None, None);
    }
    if offered_ids.is_empty() {
        return (None, None);
    }

    let rv = gs.get("relics").unwrap_or(&Value::Null);
    let combat_snapshot = build_live_combat_snapshot(gs);
    let mut truth = crate::diff::state_sync::build_combat_state(&combat_snapshot, rv);
    if let Some(prev_truth) = last_combat_truth {
        crate::diff::state_sync::carry_internal_runtime_state(prev_truth, &mut truth);
    }

    let last_command_kind = root
        .get("protocol_meta")
        .and_then(|m| m.get("last_command_kind"))
        .and_then(|v| v.as_str());
    let last_command = root
        .get("protocol_meta")
        .and_then(|m| m.get("last_command"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let engine_state =
        if matches!(last_command_kind, Some("potion")) || last_command.starts_with("POTION USE ") {
            truth.turn.counters.discovery_cost_for_turn = Some(0);
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(offered_ids))
        } else {
            let can_skip = gs
                .get("screen_state")
                .and_then(|ss| ss.get("skip_available"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            EngineState::PendingChoice(PendingChoice::CardRewardSelect {
                cards: offered_ids,
                destination: CardDestination::Hand,
                can_skip,
            })
        };

    (Some(truth), Some(engine_state))
}

fn apply_human_card_reward_to_prediction(
    pending: &mut PendingHumanCardRewardAudit,
    human_choice: &Value,
    last_combat_truth: &mut Option<CombatState>,
    last_input: &mut Option<ClientInput>,
    expected_combat_state: &mut Option<CombatState>,
) -> bool {
    let Some(mut truth) = pending.replay_truth.take() else {
        return false;
    };
    let Some(mut engine_state) = pending.replay_engine_state.take() else {
        return false;
    };

    let input = match human_choice.get("choice_kind").and_then(|v| v.as_str()) {
        Some("card") => {
            let Some(idx) = human_choice.get("choice_index").and_then(|v| v.as_u64()) else {
                return false;
            };
            ClientInput::SubmitDiscoverChoice(idx as usize)
        }
        Some("skip") | Some("bowl") => ClientInput::Cancel,
        _ => return false,
    };

    let alive =
        crate::engine::core::tick_until_stable_turn(&mut engine_state, &mut truth, input.clone());
    if !alive {
        return false;
    }

    *expected_combat_state = Some(truth.clone());
    *last_combat_truth = Some(truth);
    *last_input = Some(input);
    true
}

fn reward_screen_evaluation_to_json(
    evaluation: &crate::bot::reward_heuristics::RewardScreenEvaluation,
) -> Value {
    let cards = evaluation
        .offered_cards
        .iter()
        .map(|card| {
            json!({
                "rust_card_id": format!("{:?}", card.card_id),
                "pick_rate": card.pick_rate,
                "local_score": card.local_score,
                "combined_score": card.combined_score
            })
        })
        .collect::<Vec<_>>();
    json!({
        "cards": cards,
        "recommended_choice": recommended_choice_to_json(evaluation.recommended_choice),
        "best_pick_rate": evaluation.best_pick_rate,
        "best_local_score": evaluation.best_local_score,
        "best_combined_score": evaluation.best_combined_score,
        "skip_probability": evaluation.skip_probability,
        "skip_margin": evaluation.skip_margin,
        "force_pick_in_act1": evaluation.force_pick_in_act1,
        "force_pick_for_shell": evaluation.force_pick_for_shell
    })
}

fn recommended_choice_to_json(recommended_choice: Option<usize>) -> Value {
    match recommended_choice {
        Some(idx) => json!({
            "kind": "card",
            "choice_index": idx
        }),
        None => json!({
            "kind": "skip",
            "choice_index": null
        }),
    }
}

pub(super) fn extract_human_card_reward_choice(root: &Value) -> Option<Value> {
    root.get("protocol_meta")?
        .get("recent_human_card_reward_choice")
        .filter(|v| !v.is_null())
        .cloned()
}

pub(super) fn manual_card_reward_followup_command(root: &Value, screen: &str) -> Option<String> {
    if screen != "COMBAT_REWARD" {
        return None;
    }

    let choice = extract_human_card_reward_choice(root)?;
    let choice_kind = choice.get("choice_kind").and_then(|v| v.as_str())?;
    if !matches!(choice_kind, "skip" | "bowl") {
        return None;
    }

    let rewards = root
        .get("game_state")
        .and_then(|gs| gs.get("screen_state"))
        .and_then(|ss| ss.get("rewards"))
        .and_then(|v| v.as_array())?;

    for (idx, reward) in rewards.iter().enumerate() {
        let reward_type = reward
            .get("reward_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if reward_type != "CARD" {
            return Some(format!("CHOOSE {}", idx));
        }
    }

    Some("PROCEED".to_string())
}

fn compute_human_reward_choice_agreement(
    bot_recommended_choice: Option<usize>,
    human_choice: Option<&Value>,
) -> Option<Value> {
    let human_choice = human_choice?;
    let kind = human_choice.get("choice_kind").and_then(|v| v.as_str())?;
    match (bot_recommended_choice, kind) {
        (Some(bot_idx), "card") => {
            let human_idx = human_choice.get("choice_index").and_then(|v| v.as_u64())?;
            Some(Value::Bool(human_idx as usize == bot_idx))
        }
        (None, "skip") => Some(Value::Bool(true)),
        (Some(_), "skip") | (None, "card") => Some(Value::Bool(false)),
        _ => None,
    }
}

pub(super) fn extract_reward_session(root: &Value) -> Option<&Value> {
    root.get("protocol_meta")?
        .get("reward_session")
        .filter(|v| !v.is_null())
}

pub(super) fn reward_session_state(root: &Value) -> Option<&str> {
    extract_reward_session(root)?
        .get("state")
        .and_then(|v| v.as_str())
}

pub(super) fn reward_session_id(root: &Value) -> Option<&str> {
    extract_reward_session(root)?
        .get("session_id")
        .and_then(|v| v.as_str())
}

pub(super) fn human_choice_session_id(choice: &Value) -> Option<&str> {
    choice.get("session_id").and_then(|v| v.as_str())
}

pub(super) fn reward_choice_matches_pending_session(
    pending: &PendingHumanCardRewardAudit,
    choice: &Value,
) -> bool {
    match (
        pending.session_id.as_deref(),
        human_choice_session_id(choice),
    ) {
        (Some(expected), Some(actual)) => expected == actual,
        _ => true,
    }
}
