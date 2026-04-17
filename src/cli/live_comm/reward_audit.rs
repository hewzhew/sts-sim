use super::combat::build_live_combat_snapshot;
use super::unix_time_millis;
use crate::cli::live_comm_noncombat::build_live_run_state;
use crate::diff::protocol::card_id_from_java;
use crate::runtime::action::CardDestination;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState, PendingChoice};
use serde_json::{json, Map, Value};
use std::io::Write;

pub(super) fn reward_deck_improvement_summary(
    diagnostics: &crate::bot::RewardDecisionDiagnostics,
    chosen_choice: Option<usize>,
) -> Option<String> {
    let target_idx = chosen_choice
        .or(diagnostics.recommended_choice)
        .or_else(|| diagnostics.candidates.first().map(|card| card.index))?;
    let card = diagnostics
        .candidates
        .iter()
        .find(|card| card.index == target_idx)
        .or_else(|| diagnostics.candidates.first())?;
    Some(format!(
        "{} {} score={} rationale={}",
        if chosen_choice.is_none() {
            "skip_vs"
        } else {
            "pick"
        },
        card.card_id,
        card.score,
        card.rationale_key,
    ))
}

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

pub(super) fn human_card_reward_audit_reason_source(reason: &str) -> &'static str {
    match reason {
        "reward_session_active"
        | "reward_session_closed_without_choice"
        | "reward_session_resolved_without_choice_payload"
        | "reward_session_unknown_state"
        | "reward_session_absent" => "protocol_truth",
        _ => "legacy_fallback",
    }
}

pub(super) fn build_human_card_reward_pending(
    root: &Value,
    last_combat_truth: Option<&CombatState>,
) -> Option<PendingHumanCardRewardAudit> {
    let gs = root.get("game_state")?;
    let protocol_reward_session = reward_session_protocol_supported(root);
    let reward_session = extract_reward_session(root);
    if protocol_reward_session && reward_session.is_none() {
        return None;
    }
    let rs = build_live_run_state(gs)?;
    let cards = gs
        .get("screen_state")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array());

    let mut offered_ids = Vec::new();
    let mut offered_signature = Vec::new();
    let mut offered_cards_json = Vec::new();
    if let Some(cards) = cards {
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
                "upgrades": upgrades,
                "source": "screen_state"
            }));
        }
    } else {
        let reward_session = reward_session?;
        let reward_state = reward_session.get("state").and_then(|v| v.as_str())?;
        if !matches!(reward_state, "active" | "temporarily_offscreen") {
            return None;
        }

        let session_cards = reward_session
            .get("offered_card_ids")
            .and_then(|v| v.as_array())?;
        for card in session_cards {
            let java_id = card.as_str()?;
            let card_id = card_id_from_java(java_id)?;
            let name = crate::content::cards::get_card_definition(card_id).name;
            offered_ids.push(card_id);
            offered_signature.push(format!("{java_id}+session"));
            offered_cards_json.push(json!({
                "java_id": java_id,
                "rust_card_id": format!("{:?}", card_id),
                "name": name,
                "upgrades": 0,
                "source": "reward_session"
            }));
        }
    }
    if offered_ids.is_empty() {
        return None;
    }

    let diagnostics = reward_diagnostics_for_offered_ids(&offered_ids, &rs, true);
    let meta = root.get("protocol_meta");
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
        reward_diagnostics_to_json(&diagnostics),
    );
    payload.insert(
        "bot_recommended_choice".to_string(),
        recommended_choice_to_json(diagnostics.recommended_choice),
    );
    payload.insert(
        "reward_session".to_string(),
        reward_session.cloned().unwrap_or(Value::Null),
    );
    payload.insert(
        "audit_source".to_string(),
        json!(if cards.is_some() {
            "screen_state"
        } else if reward_session.is_some() {
            "reward_session"
        } else {
            "legacy"
        }),
    );
    payload.insert(
        "protocol_reward_session_supported".to_string(),
        json!(protocol_reward_session),
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
        bot_recommended_choice: diagnostics.recommended_choice,
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
    let reason_source = human_card_reward_audit_reason_source(reason);
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
    pending
        .payload
        .insert("audit_reason_source".to_string(), json!(reason_source));

    let line = Value::Object(pending.payload);
    let _ = writeln!(reward_audit, "{}", line);
    let _ = reward_audit.flush();
    let _ = writeln!(
        log,
        "  [CARD_AUDIT ABANDONED] source={} reason={} offered={}",
        reason_source,
        reason,
        pending.offered_signature.join(", ")
    );
}

pub(super) fn emit_bot_card_reward_audit(
    root: &Value,
    frame: u64,
    command: &str,
    reward_audit: &mut std::fs::File,
) {
    let Some(gs) = root.get("game_state") else {
        return;
    };
    let Some(rs) = build_live_run_state(gs) else {
        return;
    };
    let Some(cards) = gs
        .get("screen_state")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())
    else {
        return;
    };

    let mut offered_ids = Vec::new();
    let mut offered_cards_json = Vec::new();
    for card in cards {
        let Some(java_id) = card.get("id").and_then(|v| v.as_str()) else {
            return;
        };
        let Some(card_id) = card_id_from_java(java_id) else {
            return;
        };
        let upgrades = card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let name = card
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::content::cards::get_card_definition(card_id).name);
        offered_ids.push(card_id);
        offered_cards_json.push(json!({
            "java_id": java_id,
            "rust_card_id": format!("{:?}", card_id),
            "name": name,
            "upgrades": upgrades,
            "source": "screen_state"
        }));
    }
    if offered_ids.is_empty() {
        return;
    }

    let diagnostics = reward_diagnostics_for_offered_ids(&offered_ids, &rs, true);
    let chosen_choice = parse_bot_reward_choice(command);
    let payload = json!({
        "kind": "bot_reward_decision",
        "logged_at_unix_ms": unix_time_millis(),
        "frame": frame,
        "response_id": root
            .get("protocol_meta")
            .and_then(|m| m.get("response_id"))
            .and_then(|v| v.as_i64()),
        "state_frame_id": root
            .get("protocol_meta")
            .and_then(|m| m.get("state_frame_id"))
            .and_then(|v| v.as_i64()),
        "floor": gs.get("floor").and_then(|v| v.as_i64()).unwrap_or(0),
        "act": gs.get("act").and_then(|v| v.as_i64()).unwrap_or(0),
        "class": gs.get("class").and_then(|v| v.as_str()).unwrap_or("IRONCLAD"),
        "current_hp": gs.get("current_hp").and_then(|v| v.as_i64()).unwrap_or(0),
        "max_hp": gs.get("max_hp").and_then(|v| v.as_i64()).unwrap_or(0),
        "gold": gs.get("gold").and_then(|v| v.as_i64()).unwrap_or(0),
        "deck_size": rs.master_deck.len(),
        "offered_cards": offered_cards_json,
        "bot_command": command,
        "bot_choice": recommended_choice_to_json(chosen_choice),
        "bot_evaluation": reward_diagnostics_to_json(&diagnostics),
    });
    let _ = writeln!(reward_audit, "{}", payload);
    let _ = reward_audit.flush();
}

fn parse_bot_reward_choice(command: &str) -> Option<usize> {
    let trimmed = command.trim();
    if trimmed.eq_ignore_ascii_case("SKIP") || trimmed.eq_ignore_ascii_case("PROCEED") {
        return None;
    }
    trimmed
        .strip_prefix("CHOOSE ")
        .and_then(|rest| rest.trim().parse::<usize>().ok())
}

pub(super) fn human_card_reward_hold_context(root: &Value) -> String {
    if let Some(reward_session) = extract_reward_session(root) {
        let session_id = reward_session
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let state = reward_session
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let offscreen_screen = reward_session
            .get("offscreen_screen_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return format!("reward_session:{session_id}:{state}:{offscreen_screen}");
    }

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

    if reward_session_protocol_supported(root) {
        return HumanCardRewardAuditDisposition::Abandon {
            reason: "reward_session_absent",
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
    _last_combat_truth: Option<&CombatState>,
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

fn reward_diagnostics_to_json(diagnostics: &crate::bot::RewardDecisionDiagnostics) -> Value {
    let cards = diagnostics
        .candidates
        .iter()
        .map(|card| {
            json!({
                "index": card.index,
                "card_name": card.card_name,
                "card_id": card.card_id,
                "score": card.score,
                "base_score": card.base_score,
                "gap_bonus": card.gap_bonus,
                "survival_bonus": card.survival_bonus,
                "situational_bonus": card.situational_bonus,
                "benefit_score": card.benefit_score,
                "clutter_penalty": card.clutter_penalty,
                "penalty_score": card.penalty_score,
                "rationale_key": card.rationale_key,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "cards": cards,
        "recommended_choice": recommended_choice_to_json(diagnostics.recommended_choice),
        "recommended_rationale_key": diagnostics.recommended_rationale_key,
        "best_score": diagnostics.best_score,
        "skip_score": diagnostics.skip_score,
        "skip_rationale_key": diagnostics.skip_rationale_key,
        "skip_benefit_score": diagnostics.skip_benefit_score,
        "skip_penalty_score": diagnostics.skip_penalty_score,
        "skip_situational_bonus": diagnostics.skip_situational_bonus,
        "force_pick": diagnostics.force_pick,
        "can_skip": diagnostics.can_skip
    })
}

fn reward_diagnostics_for_offered_ids(
    offered_ids: &[crate::content::cards::CardId],
    run_state: &crate::state::run::RunState,
    can_skip: bool,
) -> crate::bot::RewardDecisionDiagnostics {
    let reward_cards = offered_ids
        .iter()
        .copied()
        .map(|card_id| crate::rewards::state::RewardCard::new(card_id, 0))
        .collect::<Vec<_>>();
    crate::bot::reward::decide_cards(run_state, &reward_cards, can_skip).1
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

pub(super) fn reward_session_protocol_supported(root: &Value) -> bool {
    root.get("protocol_meta")
        .and_then(|meta| meta.get("capabilities"))
        .and_then(|caps| caps.get("reward_session"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
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

pub(super) fn reward_session_is_live(root: &Value) -> bool {
    matches!(
        reward_session_state(root),
        Some("active" | "temporarily_offscreen")
    )
}
