use super::combat::{handle_live_combat_frame, CombatFrameOutcome, CombatRuntime};
use super::frame::LiveFrame;
use super::human_noncombat_audit::{
    build_pending_human_noncombat_audit, finalize_human_noncombat_audit,
    human_noncombat_domain_for_frame, mark_human_noncombat_audit_polluted,
    update_human_noncombat_audit, PendingHumanNoncombatAudit,
};
use super::io::LiveCommIo;
use super::noncombat::{maybe_arm_human_card_reward_audit, route_noncombat_command};
use super::reward_audit::{
    classify_human_card_reward_audit_disposition, emit_bot_card_reward_audit,
    extract_human_card_reward_choice, finalize_human_card_reward_audit,
    finalize_human_card_reward_audit_without_choice, human_card_reward_audit_reason_source,
    human_card_reward_hold_context, reward_choice_matches_pending_session,
    reward_deck_improvement_summary, HumanCardRewardAuditDisposition, PendingHumanCardRewardAudit,
};
use super::snapshot::write_failure_snapshot;
use super::watch::{maybe_capture_live_watch, remember_live_record, LiveWatchRuntime};
use super::{
    should_clear_combat_context, LiveCommConfig, LoopExitReason, ENGINE_BUG_SUMMARY_INTERVAL,
    SIG_PATH,
};
use crate::bot::infra::sidecar;
use crate::bot::Agent;
use serde_json::Value;
use std::io::Write;

pub(super) struct LiveCommSession {
    pub(super) config: LiveCommConfig,
    pub(super) consecutive_errors: u32,
    pub(super) last_error_msg: String,
    pub(super) frame_count: u64,
    pub(super) last_sent_cmd: String,
    pub(super) cmd_failed_count: u32,
    pub(super) last_response_id: Option<i64>,
    pub(super) last_state_frame_id: Option<i64>,
    pub(super) last_protocol_command_kind: Option<String>,
    pub(super) pending_human_card_reward_audit: Option<PendingHumanCardRewardAudit>,
    pub(super) pending_human_noncombat_audit: Option<PendingHumanNoncombatAudit>,
    pub(super) combat_handoff_hold_polls: u32,
    pub(super) live_watch_runtime: LiveWatchRuntime,
    pub(super) engine_bug_total: usize,
    pub(super) content_gap_total: usize,
    pub(super) coverage_db: crate::bot::CoverageDb,
    pub(super) combat_runtime: CombatRuntime,
    pub(super) game_over_seen: bool,
    pub(super) final_victory: bool,
    pub(super) noncombat_loop_screen: String,
    pub(super) noncombat_loop_cmd: String,
    pub(super) noncombat_loop_count: u32,
    pub(super) noncombat_polluted: bool,
    pub(super) noncombat_pollution_frame: Option<u64>,
    pub(super) reward_loop_signatures: Vec<String>,
}

fn should_log_card_reward_hold_poll_count(offscreen_hold_polls: u32) -> bool {
    matches!(offscreen_hold_polls, 1 | 50 | 200 | 500) || offscreen_hold_polls % 500 == 0
}

fn should_log_combat_handoff_hold_poll_count(hold_polls: u32) -> bool {
    matches!(hold_polls, 1 | 20 | 100 | 250) || hold_polls % 250 == 0
}

fn reward_audit_hold_command(frame: &LiveFrame) -> &'static str {
    let avail = frame.available_commands();
    if avail.contains(&"wait") {
        "WAIT 30"
    } else {
        "STATE"
    }
}

fn combat_handoff_hold_command(frame: &LiveFrame) -> &'static str {
    let avail = frame.available_commands();
    if avail.contains(&"wait") {
        "WAIT 30"
    } else {
        "STATE"
    }
}

fn is_expected_live_event_compatibility_fallback(event_name: &str) -> bool {
    matches!(
        event_name,
        "Neow" | "Note For Yourself" | "Bonfire Spirits" | "Bonfire Elementals"
    )
}

fn multistage_event_has_meaningful_screen_semantics(trace_audit: &Value) -> bool {
    if trace_audit
        .get("screen_key")
        .is_some_and(|value| !value.is_null())
    {
        return true;
    }

    trace_audit
        .get("screen_index")
        .or_else(|| trace_audit.get("screen"))
        .and_then(Value::as_u64)
        .is_some_and(|screen| screen > 0)
}

fn event_validation_failure_reasons(trace_audit: &Value) -> Vec<String> {
    let family = trace_audit
        .get("family")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let event_name = trace_audit
        .get("event_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let unexpected_fallback = family == "compatibility_fallback"
        && !is_expected_live_event_compatibility_fallback(event_name);
    let suspicious_multistage = trace_audit
        .get("live_event_protocol")
        .and_then(|value| value.get("screen_semantics_incomplete"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || (trace_audit.get("live_event_protocol").is_none()
            && matches!(
                event_name,
                "Neow"
                    | "Shining Light"
                    | "Golden Idol"
                    | "Knowing Skull"
                    | "Living Wall"
                    | "Big Fish"
            )
            && !multistage_event_has_meaningful_screen_semantics(trace_audit));

    let mut reasons = Vec::new();
    if unexpected_fallback {
        reasons.push("compatibility_fallback".to_string());
    }
    if suspicious_multistage {
        reasons.push("event_screen_semantics_incomplete".to_string());
    }
    reasons
}

fn enrich_event_audit_with_screen_state(trace_audit: &Value, game_state: &Value) -> Value {
    let mut enriched = trace_audit.clone();
    let Some(audit) = enriched.as_object_mut() else {
        return enriched;
    };
    let Some(screen_state) = game_state.get("screen_state").and_then(Value::as_object) else {
        return enriched;
    };

    let screen_index_missing = audit
        .get("screen_index")
        .is_none_or(|value| value.is_null());
    if screen_index_missing {
        if let Some(value) = screen_state.get("current_screen_index").cloned() {
            audit.insert("screen_index".to_string(), value);
        }
    }

    let screen_key_missing = audit.get("screen_key").is_none_or(|value| value.is_null());
    if screen_key_missing {
        if let Some(value) = screen_state.get("current_screen_key").cloned() {
            audit.insert("screen_key".to_string(), value);
        }
    }

    let screen_source_missing = audit
        .get("screen_source")
        .is_none_or(|value| value.is_null());
    if screen_source_missing {
        if let Some(value) = screen_state.get("screen_source").cloned() {
            audit.insert("screen_source".to_string(), value);
        }
    }

    enriched
}

fn should_fail_fast_on_snapshot(trigger_kind: &str, reasons: &[String]) -> bool {
    match trigger_kind {
        "validation_failure" | "engine_bug" | "session_polluted" | "protocol_error" => true,
        "high_risk_suspect" => reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "large_sequence_bonus" | "branch_opening_conflict" | "sequencing_conflict"
            )
        }),
        _ => false,
    }
}

fn noncombat_error_loop_fallback_command(avail: &[&str]) -> String {
    if avail.contains(&"skip") {
        "SKIP".to_string()
    } else if avail.contains(&"proceed") {
        "PROCEED".to_string()
    } else if avail.contains(&"choose") {
        "CHOOSE 0".to_string()
    } else if avail.contains(&"leave") {
        "LEAVE".to_string()
    } else if avail.contains(&"cancel") || avail.contains(&"return") {
        "RETURN".to_string()
    } else if avail.contains(&"wait") {
        "WAIT 30".to_string()
    } else {
        "STATE".to_string()
    }
}

fn noncombat_pollution_hold_command(avail: &[&str]) -> &'static str {
    if avail.contains(&"wait") {
        "WAIT 30"
    } else {
        "STATE"
    }
}

fn should_hold_human_noncombat(screen: &str) -> bool {
    matches!(
        screen,
        "CARD_REWARD"
            | "COMBAT_REWARD"
            | "SHOP_ROOM"
            | "SHOP_SCREEN"
            | "EVENT"
            | "REST"
            | "GRID"
            | "MAP"
            | "BOSS_REWARD"
    )
}

fn reward_loop_signature(root: &Value, screen: &str) -> Option<String> {
    let gs = root.get("game_state")?;
    match screen {
        "COMBAT_REWARD" => {
            let rewards = gs.get("screen_state")?.get("rewards")?.as_array()?;
            let active = rewards
                .iter()
                .map(|reward| {
                    let reward_type = reward
                        .get("reward_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN");
                    let choice_index = reward
                        .get("choice_index")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    format!("{reward_type}@{choice_index}")
                })
                .collect::<Vec<_>>();
            Some(format!("COMBAT_REWARD:{}", active.join(",")))
        }
        "CARD_REWARD" => {
            let cards = gs.get("screen_state")?.get("cards")?.as_array()?;
            let offered = cards
                .iter()
                .filter_map(|card| {
                    card.get("id")
                        .and_then(|v| v.as_str())
                        .or_else(|| card.get("name").and_then(|v| v.as_str()))
                })
                .collect::<Vec<_>>();
            Some(format!("CARD_REWARD:{}", offered.join(",")))
        }
        _ => None,
    }
}

fn write_focus_combat_frame_marker(
    frame_count: u64,
    frame: &LiveFrame,
    focus_log: &mut std::fs::File,
) {
    let _ = writeln!(
        focus_log,
        "[FRAME] frame={} response_id={:?} state_frame_id={:?} last_command_kind={:?} {}",
        frame_count,
        frame.response_id(),
        frame.state_frame_id(),
        frame.last_command_kind(),
        frame.brief_summary()
    );
}

impl LiveCommSession {
    pub(super) fn new(config: LiveCommConfig) -> Self {
        Self {
            config,
            consecutive_errors: 0,
            last_error_msg: String::new(),
            frame_count: 0,
            last_sent_cmd: String::new(),
            cmd_failed_count: 0,
            last_response_id: None,
            last_state_frame_id: None,
            last_protocol_command_kind: None,
            pending_human_card_reward_audit: None,
            pending_human_noncombat_audit: None,
            combat_handoff_hold_polls: 0,
            live_watch_runtime: LiveWatchRuntime::default(),
            engine_bug_total: 0,
            content_gap_total: 0,
            coverage_db: crate::bot::CoverageDb::load_or_default(),
            combat_runtime: CombatRuntime::default(),
            game_over_seen: false,
            final_victory: false,
            noncombat_loop_screen: String::new(),
            noncombat_loop_cmd: String::new(),
            noncombat_loop_count: 0,
            noncombat_polluted: false,
            noncombat_pollution_frame: None,
            reward_loop_signatures: Vec::new(),
        }
    }

    pub(super) fn handle_frame<W: Write>(
        &mut self,
        agent: &mut Agent,
        frame: &LiveFrame,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> Option<LoopExitReason> {
        let parsed = frame.root();
        self.last_response_id = frame.response_id();
        self.last_state_frame_id = frame.state_frame_id();
        self.last_protocol_command_kind = frame.last_command_kind().map(|s| s.to_string());
        if let Some(response_id) = self.last_response_id {
            remember_live_record(
                &mut self.live_watch_runtime,
                response_id,
                parsed,
                self.config.watch_capture.window_responses.max(1) + 2,
            );
        }

        if let Some(err) = frame.error() {
            return self.handle_java_error(frame, err, live_io, stdout);
        }
        self.clear_recovered_error_flood(live_io);

        let avail = frame.available_commands();
        let has = |c: &str| avail.contains(&c);

        if !frame.in_game() {
            if has("start") {
                writeln!(live_io.log, "[F{}] Not in game → START", self.frame_count).unwrap();
                live_io.send_line(stdout, "START Ironclad 0");
            } else {
                writeln!(live_io.log, "[F{}] Not in game → STATE", self.frame_count).unwrap();
                live_io.send_line(stdout, "STATE");
            }
            return None;
        }

        if !frame.ready_for_command() {
            if matches!(
                (frame.combat_session_owner(), frame.combat_session_state()),
                (Some("human"), Some("active"))
            ) {
                self.combat_handoff_hold_polls += 1;
                if self.combat_handoff_hold_polls == 1
                    || should_log_combat_handoff_hold_poll_count(self.combat_handoff_hold_polls)
                {
                    writeln!(
                        live_io.log,
                        "[F{}] COMBAT handoff active; waiting_for_command=false session_id={:?} hold_polls={} → STATE",
                        self.frame_count,
                        frame.combat_session_id(),
                        self.combat_handoff_hold_polls
                    )
                    .unwrap();
                }
                live_io.send_line(stdout, "STATE");
                return None;
            }
            writeln!(
                live_io.log,
                "[F{}] In game but not ready → STATE",
                self.frame_count
            )
            .unwrap();
            live_io.send_line(stdout, "STATE");
            return None;
        }

        let gs = frame.game_state();
        let screen = frame.screen();
        let is_combat = frame.is_combat();
        let room_phase = frame.room_phase();

        maybe_capture_live_watch(
            &self.config.watch_capture,
            &mut self.live_watch_runtime,
            parsed,
            self.frame_count,
            &mut live_io.log,
            &mut live_io.watch_audit,
            &mut live_io.watch_noncombat_audit,
        );

        if self.handle_pending_reward_audit(frame, live_io, stdout) {
            return None;
        }

        if self.handle_active_human_combat_handoff(frame, live_io, stdout) {
            return None;
        }

        if self.maybe_arm_human_boss_combat_handoff(frame, live_io, stdout) {
            return None;
        }

        if screen == "GAME_OVER" || screen == "DEATH" {
            self.combat_runtime.flush_summary_on_game_over(
                &mut live_io.log,
                &mut live_io.focus_log,
                self.frame_count,
            );
            let score = frame
                .screen_state()
                .and_then(|s| s["score"].as_i64())
                .unwrap_or(0);
            let victory = frame
                .screen_state()
                .and_then(|s| s["victory"].as_bool())
                .unwrap_or(false);
            writeln!(
                live_io.log,
                "\n[F{}] === GAME OVER === victory={} score={}",
                self.frame_count, victory, score
            )
            .unwrap();
            let _ = writeln!(
                live_io.focus_log,
                "[GAME_OVER] frame={} victory={} score={} floor={} act={} gold={}",
                self.frame_count,
                victory,
                score,
                gs.get("floor").and_then(|v| v.as_i64()).unwrap_or(0),
                gs.get("act").and_then(|v| v.as_i64()).unwrap_or(0),
                gs.get("gold").and_then(|v| v.as_i64()).unwrap_or(0)
            );
            self.game_over_seen = true;
            self.final_victory = victory;
            if !victory {
                let _ = write_failure_snapshot(
                    live_io,
                    self.frame_count,
                    frame,
                    "terminal_loss",
                    vec!["terminal_game_over".to_string(), format!("score={score}")],
                    serde_json::json!({
                        "chosen_command": self.last_sent_cmd,
                        "last_command_kind": self.last_protocol_command_kind,
                        "victory": victory,
                        "score": score,
                    }),
                );
            }
            let _ = live_io.log.flush();
            let _ = live_io.raw.flush();
            return Some(LoopExitReason::GameOver);
        }

        if should_clear_combat_context(is_combat, room_phase, screen) {
            self.combat_runtime.clear_after_combat_if_needed(
                &mut live_io.log,
                &mut live_io.focus_log,
                self.frame_count,
            );
        }

        if is_combat && screen == "NONE" && (has("play") || has("end")) {
            self.reset_noncombat_loop_guard();
            write_focus_combat_frame_marker(self.frame_count, frame, &mut live_io.focus_log);
            let _ = live_io.focus_log.flush();
            let outcome = handle_live_combat_frame(
                frame,
                gs,
                self.frame_count,
                self.config.parity_mode,
                self.config.combat_mode,
                self.config.exact_turn_mode,
                self.config.fail_fast_debug,
                self.config.combat_search_budget,
                self.config.legacy_root_legal_moves,
                &mut self.last_sent_cmd,
                &mut self.cmd_failed_count,
                &mut self.engine_bug_total,
                &mut self.content_gap_total,
                &mut self.coverage_db,
                &mut self.combat_runtime,
                live_io,
                stdout,
                ENGINE_BUG_SUMMARY_INTERVAL,
                SIG_PATH,
            );
            if matches!(outcome, CombatFrameOutcome::StopForParityFailure) {
                self.combat_runtime.flush_summary_on_game_over(
                    &mut live_io.log,
                    &mut live_io.focus_log,
                    self.frame_count,
                );
                return Some(LoopExitReason::ParityFail);
            } else if matches!(outcome, CombatFrameOutcome::StopForFailFast) {
                self.combat_runtime.flush_summary_on_game_over(
                    &mut live_io.log,
                    &mut live_io.focus_log,
                    self.frame_count,
                );
                return Some(LoopExitReason::FailFast);
            }
        } else {
            if let Some(exit_reason) = self
                .handle_noncombat_frame(agent, frame, &avail, screen, room_phase, live_io, stdout)
            {
                return Some(exit_reason);
            }
        }

        let _ = live_io.log.flush();
        let _ = live_io.focus_log.flush();
        let _ = live_io.raw.flush();
        None
    }

    fn handle_active_human_combat_handoff<W: Write>(
        &mut self,
        frame: &LiveFrame,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> bool {
        match (frame.combat_session_owner(), frame.combat_session_state()) {
            (Some("human"), Some("active")) => {
                self.combat_handoff_hold_polls += 1;
                if self.combat_handoff_hold_polls == 1
                    || should_log_combat_handoff_hold_poll_count(self.combat_handoff_hold_polls)
                {
                    writeln!(
                        live_io.log,
                        "[F{}] COMBAT handoff active; session_id={:?} room_type={} hold_polls={} → {}",
                        self.frame_count,
                        frame.combat_session_id(),
                        frame.room_type(),
                        self.combat_handoff_hold_polls,
                        combat_handoff_hold_command(frame)
                    )
                    .unwrap();
                }
                live_io.send_line(stdout, combat_handoff_hold_command(frame));
                let _ = live_io.log.flush();
                true
            }
            (_, Some("resolved")) if self.combat_handoff_hold_polls > 0 => {
                writeln!(
                    live_io.log,
                    "[F{}] COMBAT handoff resolved; session_id={:?}",
                    self.frame_count,
                    frame.combat_session_id()
                )
                .unwrap();
                self.combat_handoff_hold_polls = 0;
                false
            }
            _ => {
                self.combat_handoff_hold_polls = 0;
                false
            }
        }
    }

    fn maybe_arm_human_boss_combat_handoff<W: Write>(
        &mut self,
        frame: &LiveFrame,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> bool {
        if !self.config.human_boss_combat_handoff {
            return false;
        }
        if frame.room_phase() != "COMBAT" || frame.room_type() != "MonsterRoomBoss" {
            return false;
        }
        if frame.combat_session().is_some() {
            return false;
        }
        if !frame.available_commands().contains(&"handoff") {
            return false;
        }
        if self.last_sent_cmd == "HANDOFF HUMAN BOSS_COMBAT" {
            return false;
        }

        writeln!(
            live_io.log,
            "[F{}] COMBAT boss handoff requested → HANDOFF HUMAN BOSS_COMBAT",
            self.frame_count
        )
        .unwrap();
        self.last_sent_cmd = "HANDOFF HUMAN BOSS_COMBAT".to_string();
        self.cmd_failed_count = 0;
        live_io.send_line(stdout, "HANDOFF HUMAN BOSS_COMBAT");
        let _ = live_io.log.flush();
        true
    }

    fn handle_java_error<W: Write>(
        &mut self,
        frame: &LiveFrame,
        err: &str,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> Option<LoopExitReason> {
        self.combat_runtime.on_java_error();
        self.consecutive_errors += 1;
        self.cmd_failed_count += 1;
        if err != self.last_error_msg || self.consecutive_errors <= 2 {
            writeln!(
                live_io.log,
                "[F{}] ERROR #{}: {}",
                self.frame_count, self.consecutive_errors, err
            )
            .unwrap();
            if self
                .combat_runtime
                .last_root_action_source
                .as_deref()
                .is_some_and(|source| source == "protocol")
            {
                writeln!(
                    live_io.focus_log,
                    "[PROTO EXECUTOR BUG] frame={} action_id={:?} command={:?} error={}",
                    self.frame_count,
                    self.combat_runtime.last_root_action_id,
                    self.combat_runtime.last_root_action_command,
                    err
                )
                .unwrap();
            }
            self.last_error_msg = err.to_string();
        } else if self.consecutive_errors == 3 {
            writeln!(live_io.log, "  (suppressing repeated errors...)").unwrap();
        }
        if self.consecutive_errors >= 5 {
            let protocol_root_error = self
                .combat_runtime
                .last_root_action_source
                .as_deref()
                .is_some_and(|source| source == "protocol");
            let mut reasons = vec![
                "java_error_flood".to_string(),
                err.to_string(),
                format!("repeats={}", self.consecutive_errors),
            ];
            if protocol_root_error {
                reasons.push("executor_rejected_protocol_action".to_string());
            }
            writeln!(
                live_io.log,
                "  ERROR FLOOD: {} repeats, sleeping 1s",
                self.consecutive_errors
            )
            .unwrap();
            let _ = write_failure_snapshot(
                live_io,
                self.frame_count,
                frame,
                "protocol_error",
                reasons.clone(),
                serde_json::json!({
                    "chosen_command": self.last_sent_cmd,
                    "last_command_kind": self.last_protocol_command_kind,
                    "error": err,
                    "root_action_source": self.combat_runtime.last_root_action_source,
                    "chosen_action_id": self.combat_runtime.last_root_action_id,
                    "chosen_protocol_command": self.combat_runtime.last_root_action_command,
                    "protocol_root_action_count": self.combat_runtime.last_protocol_root_action_count,
                }),
            );
            if self.config.fail_fast_debug
                && should_fail_fast_on_snapshot("protocol_error", &reasons)
            {
                let _ = writeln!(live_io.log, "  [FAIL_FAST] stopping on protocol_error");
                let _ = writeln!(
                    live_io.focus_log,
                    "[FAIL_FAST] frame={} trigger=protocol_error",
                    self.frame_count
                );
                let _ = live_io.log.flush();
                let _ = live_io.focus_log.flush();
                return Some(LoopExitReason::FailFast);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        live_io.send_line(stdout, "STATE");
        None
    }

    fn clear_recovered_error_flood(&mut self, live_io: &mut LiveCommIo) {
        if self.consecutive_errors > 0 {
            if self.consecutive_errors > 2 {
                writeln!(
                    live_io.log,
                    "  (total {} errors before recovery)",
                    self.consecutive_errors
                )
                .unwrap();
            }
            self.consecutive_errors = 0;
            self.last_error_msg.clear();
        }
    }

    fn reset_noncombat_loop_guard(&mut self) {
        self.noncombat_loop_screen.clear();
        self.noncombat_loop_cmd.clear();
        self.noncombat_loop_count = 0;
        self.noncombat_polluted = false;
        self.noncombat_pollution_frame = None;
        self.reward_loop_signatures.clear();
    }

    pub(super) fn finalize_pending_human_noncombat_audit(
        &mut self,
        live_io: &mut LiveCommIo,
        reason: &str,
    ) {
        if let Some(pending) = self.pending_human_noncombat_audit.take() {
            let status = if pending.polluted {
                "polluted"
            } else {
                "incomplete"
            };
            finalize_human_noncombat_audit(
                pending,
                None,
                self.frame_count,
                &mut live_io.human_noncombat_audit,
                &mut live_io.log,
                status,
                reason,
            );
        }
    }

    fn mark_pending_human_noncombat_polluted(&mut self, reason: impl Into<String>) {
        if let Some(pending) = self.pending_human_noncombat_audit.as_mut() {
            mark_human_noncombat_audit_polluted(pending, reason);
        }
    }

    fn sync_human_noncombat_audit(
        &mut self,
        frame: &LiveFrame,
        bot_recommendation: &str,
        live_io: &mut LiveCommIo,
    ) {
        let current_domain = if self.config.human_noncombat_hold {
            human_noncombat_domain_for_frame(frame, self.pending_human_noncombat_audit.as_ref())
        } else {
            None
        };

        let should_finalize = self
            .pending_human_noncombat_audit
            .as_ref()
            .is_some_and(|pending| current_domain != Some(pending.domain));
        if should_finalize {
            if let Some(pending) = self.pending_human_noncombat_audit.take() {
                let status = if pending.polluted {
                    "polluted"
                } else {
                    "completed"
                };
                let reason = if pending.polluted {
                    "screen_transition_after_pollution"
                } else {
                    "left_human_noncombat_domain"
                };
                finalize_human_noncombat_audit(
                    pending,
                    Some(frame),
                    self.frame_count,
                    &mut live_io.human_noncombat_audit,
                    &mut live_io.log,
                    status,
                    reason,
                );
            }
        }

        let Some(domain) = current_domain else {
            return;
        };
        if domain == "reward_card" {
            return;
        }

        if self.pending_human_noncombat_audit.is_none() {
            let pending = build_pending_human_noncombat_audit(
                frame,
                self.frame_count,
                domain,
                bot_recommendation,
            );
            writeln!(
                live_io.log,
                "[F{}] HUMAN NONCOMBAT ARM session={} domain={} screen={} room_type={} bot_recommendation={}",
                self.frame_count,
                pending.session_id,
                pending.domain,
                frame.screen(),
                frame.room_type(),
                bot_recommendation
            )
            .unwrap();
            self.pending_human_noncombat_audit = Some(pending);
            return;
        }

        if let Some(pending) = self.pending_human_noncombat_audit.as_mut() {
            let previous_screen = pending.last_seen_screen.clone();
            let previous_phase = pending.last_seen_room_phase.clone();
            let previous_bot = pending.last_bot_recommendation.clone();
            let observed_command =
                update_human_noncombat_audit(pending, frame, self.frame_count, bot_recommendation);

            if previous_screen != pending.last_seen_screen {
                writeln!(
                    live_io.log,
                    "[F{}] HUMAN NONCOMBAT TRANSITION session={} {} -> {}",
                    self.frame_count, pending.session_id, previous_screen, pending.last_seen_screen
                )
                .unwrap();
            } else if previous_phase != pending.last_seen_room_phase {
                writeln!(
                    live_io.log,
                    "[F{}] HUMAN NONCOMBAT PHASE session={} {} -> {}",
                    self.frame_count,
                    pending.session_id,
                    previous_phase,
                    pending.last_seen_room_phase
                )
                .unwrap();
            }
            if previous_bot != pending.last_bot_recommendation {
                writeln!(
                    live_io.log,
                    "[F{}] HUMAN NONCOMBAT UPDATE session={} screen={} bot_recommendation={}",
                    self.frame_count,
                    pending.session_id,
                    frame.screen(),
                    pending.last_bot_recommendation
                )
                .unwrap();
            }
            if let Some(command) = observed_command {
                writeln!(
                    live_io.log,
                    "[F{}] HUMAN NONCOMBAT COMMAND session={} cmd={} kind={:?}",
                    self.frame_count,
                    pending.session_id,
                    command,
                    frame.last_command_kind()
                )
                .unwrap();
            }
        }
    }

    fn handle_pending_reward_audit<W: Write>(
        &mut self,
        frame: &LiveFrame,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> bool {
        let parsed = frame.root();
        let screen = frame.screen();
        let room_phase = frame.room_phase();
        if let Some(pending) = self
            .pending_human_card_reward_audit
            .as_mut()
            .filter(|_| screen != "CARD_REWARD")
        {
            if let Some(choice) = extract_human_card_reward_choice(parsed) {
                if reward_choice_matches_pending_session(pending, &choice) {
                    finalize_human_card_reward_audit(
                        self.pending_human_card_reward_audit.take().unwrap(),
                        parsed,
                        &mut live_io.reward_audit,
                        &mut live_io.log,
                        &mut self.combat_runtime.last_combat_truth,
                        &mut self.combat_runtime.last_input,
                        &mut self.combat_runtime.expected_combat_state,
                    );
                } else {
                    writeln!(
                        live_io.log,
                        "[F{}] CARD_REWARD human audit session mismatch pending_session={:?} choice_session={:?} -> {}",
                        self.frame_count,
                        pending.session_id,
                        choice.get("session_id").and_then(|v| v.as_str()),
                        reward_audit_hold_command(frame)
                    )
                    .unwrap();
                    live_io.send_line(stdout, reward_audit_hold_command(frame));
                    let _ = live_io.log.flush();
                    return true;
                }
            } else {
                match classify_human_card_reward_audit_disposition(parsed) {
                    HumanCardRewardAuditDisposition::Abandon { reason } => {
                        finalize_human_card_reward_audit_without_choice(
                            self.pending_human_card_reward_audit.take().unwrap(),
                            parsed,
                            &mut live_io.reward_audit,
                            &mut live_io.log,
                            reason,
                        );
                    }
                    HumanCardRewardAuditDisposition::Hold { reason } => {
                        pending.offscreen_hold_polls += 1;
                        let hold_context = human_card_reward_hold_context(parsed);
                        let hold_cmd = reward_audit_hold_command(frame);
                        let should_log_hold = pending.last_hold_context.as_deref()
                            != Some(hold_context.as_str())
                            || pending.offscreen_hold_polls == 1
                            || should_log_card_reward_hold_poll_count(pending.offscreen_hold_polls);
                        pending.last_hold_context = Some(hold_context);
                        if should_log_hold {
                            writeln!(
                                live_io.log,
                                "[F{}] CARD_REWARD human audit pending; source={} reason={} offscreen_polls={} screen={} screen_name={} room_phase={} → {}",
                                self.frame_count,
                                human_card_reward_audit_reason_source(reason),
                                reason,
                                pending.offscreen_hold_polls,
                                screen,
                                frame.screen_name(),
                                room_phase,
                                hold_cmd
                            )
                            .unwrap();
                        }
                    }
                }
                live_io.send_line(stdout, reward_audit_hold_command(frame));
                let _ = live_io.log.flush();
                return true;
            }
        }
        false
    }

    fn handle_noncombat_frame<W: Write>(
        &mut self,
        agent: &mut Agent,
        frame: &LiveFrame,
        avail: &[&str],
        screen: &str,
        room_phase: &str,
        live_io: &mut LiveCommIo,
        stdout: &mut W,
    ) -> Option<LoopExitReason> {
        let parsed = frame.root();
        let has = |c: &str| avail.contains(&c);

        if maybe_arm_human_card_reward_audit(
            self.config.human_card_reward_audit,
            &mut self.pending_human_card_reward_audit,
            parsed,
            self.combat_runtime.last_combat_truth.as_ref(),
            &mut live_io.log,
            self.frame_count,
        ) {
            let _ = live_io.log.flush();
            return None;
        }

        if self.noncombat_polluted && screen != self.noncombat_loop_screen {
            writeln!(
                live_io.log,
                "[F{}] SESSION POLLUTION CLEARED screen={} last_loop_screen={} last_loop_cmd={}",
                self.frame_count, screen, self.noncombat_loop_screen, self.noncombat_loop_cmd
            )
            .unwrap();
            self.reset_noncombat_loop_guard();
        }

        if self.noncombat_polluted {
            let hold_cmd = noncombat_pollution_hold_command(avail);
            writeln!(
                live_io.log,
                "[F{}] SESSION POLLUTED noncombat_loop since F{:?} screen={} → {}",
                self.frame_count, self.noncombat_pollution_frame, screen, hold_cmd
            )
            .unwrap();
            self.last_sent_cmd = hold_cmd.to_string();
            live_io.send_line(stdout, hold_cmd);
            return None;
        }

        if let Some(signature) = reward_loop_signature(parsed, screen) {
            self.reward_loop_signatures.push(signature);
            if self.reward_loop_signatures.len() > 8 {
                self.reward_loop_signatures
                    .drain(0..self.reward_loop_signatures.len() - 8);
            }
            let n = self.reward_loop_signatures.len();
            if n >= 4
                && self.reward_loop_signatures[n - 1] == self.reward_loop_signatures[n - 3]
                && self.reward_loop_signatures[n - 2] == self.reward_loop_signatures[n - 4]
                && self.reward_loop_signatures[n - 1] != self.reward_loop_signatures[n - 2]
            {
                let hold_cmd = noncombat_pollution_hold_command(avail).to_string();
                self.noncombat_polluted = true;
                self.noncombat_pollution_frame = Some(self.frame_count);
                self.mark_pending_human_noncombat_polluted(format!(
                    "protocol_reward_loop:{}",
                    self.reward_loop_signatures.join(" -> ")
                ));
                writeln!(
                    live_io.log,
                    "[F{}] SESSION POLLUTED protocol_reward_loop screens={:?} → {}",
                    self.frame_count, self.reward_loop_signatures, hold_cmd
                )
                .unwrap();
                let _ = write_failure_snapshot(
                    live_io,
                    self.frame_count,
                    frame,
                    "session_polluted",
                    vec![
                        "protocol_reward_loop".to_string(),
                        format!("screens={}", self.reward_loop_signatures.join(" -> ")),
                    ],
                    serde_json::json!({
                        "chosen_command": hold_cmd,
                        "last_command": self.last_sent_cmd,
                        "reward_session": parsed
                            .get("protocol_meta")
                            .and_then(|v| v.get("reward_session"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    }),
                );
                if self.config.fail_fast_debug
                    && should_fail_fast_on_snapshot(
                        "session_polluted",
                        &[
                            "protocol_reward_loop".to_string(),
                            format!("screens={}", self.reward_loop_signatures.join(" -> ")),
                        ],
                    )
                {
                    let _ = writeln!(
                        live_io.focus_log,
                        "[FAIL_FAST] frame={} trigger=session_polluted",
                        self.frame_count
                    );
                    let _ = live_io.log.flush();
                    let _ = live_io.focus_log.flush();
                    return Some(LoopExitReason::FailFast);
                }
                self.last_sent_cmd = hold_cmd.clone();
                live_io.send_line(stdout, &hold_cmd);
                return None;
            }
        } else {
            self.reward_loop_signatures.clear();
        }

        let mut cmd = route_noncombat_command(agent, parsed, screen, avail);
        if cmd == "STATE" && !has("wait") {
            writeln!(
                live_io.log,
                "  [!] UNKNOWN STATE: avail={:?} screen={}",
                avail, screen
            )
            .unwrap();
        }

        if cmd == self.last_sent_cmd && self.cmd_failed_count > 0 {
            if self.noncombat_loop_screen == screen && self.noncombat_loop_cmd == cmd {
                self.noncombat_loop_count += 1;
            } else {
                self.noncombat_loop_screen = screen.to_string();
                self.noncombat_loop_cmd = cmd.clone();
                self.noncombat_loop_count = 1;
            }

            if self.noncombat_loop_count >= 3 {
                let hold_cmd = noncombat_pollution_hold_command(avail).to_string();
                self.noncombat_polluted = true;
                self.noncombat_pollution_frame = Some(self.frame_count);
                self.mark_pending_human_noncombat_polluted(format!(
                    "repeated_noncombat_command_loop:screen={screen}:cmd={cmd}:repeats={}",
                    self.noncombat_loop_count
                ));
                writeln!(
                    live_io.log,
                    "[F{}] SESSION POLLUTED repeated noncombat command loop screen={} cmd={} repeats={} → {}",
                    self.frame_count,
                    screen,
                    cmd,
                    self.noncombat_loop_count,
                    hold_cmd
                )
                .unwrap();
                let _ = write_failure_snapshot(
                    live_io,
                    self.frame_count,
                    frame,
                    "session_polluted",
                    vec![
                        "repeated_noncombat_command_loop".to_string(),
                        format!("screen={screen}"),
                        format!("cmd={cmd}"),
                        format!("repeats={}", self.noncombat_loop_count),
                    ],
                    serde_json::json!({
                        "chosen_command": hold_cmd,
                        "last_command": self.last_sent_cmd,
                        "loop_screen": screen,
                        "loop_command": cmd,
                    }),
                );
                if self.config.fail_fast_debug
                    && should_fail_fast_on_snapshot(
                        "session_polluted",
                        &[
                            "repeated_noncombat_command_loop".to_string(),
                            format!("screen={screen}"),
                            format!("cmd={cmd}"),
                            format!("repeats={}", self.noncombat_loop_count),
                        ],
                    )
                {
                    let _ = writeln!(
                        live_io.focus_log,
                        "[FAIL_FAST] frame={} trigger=session_polluted",
                        self.frame_count
                    );
                    let _ = live_io.log.flush();
                    let _ = live_io.focus_log.flush();
                    return Some(LoopExitReason::FailFast);
                }
                self.last_sent_cmd = hold_cmd.clone();
                live_io.send_line(stdout, &hold_cmd);
                return None;
            }

            writeln!(live_io.log, "  [!] ERROR LOOP DETECTED, FALLING BACK").unwrap();
            cmd = noncombat_error_loop_fallback_command(avail);
        } else {
            self.noncombat_loop_screen = screen.to_string();
            self.noncombat_loop_cmd = cmd.clone();
            self.noncombat_loop_count = 0;
        }

        if screen == "EVENT" {
            if let Some(gs) = parsed.get("game_state") {
                if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                    if let Some(trace) =
                        crate::cli::live_comm_noncombat::choose_live_event_command_with_trace(
                            gs, &rs,
                        )
                    {
                        if trace.command == cmd {
                            let decision_audit =
                                enrich_event_audit_with_screen_state(&trace.audit, gs);
                            writeln!(
                                live_io.log,
                                "[F{}] EVENT POLICY {}",
                                self.frame_count, trace.detail
                            )
                            .unwrap();
                            writeln!(
                                live_io.focus_log,
                                "[EVENT] frame={} {}",
                                self.frame_count, trace.summary
                            )
                            .unwrap();
                            if let Some(deck_summary) = &trace.deck_improvement_summary {
                                writeln!(
                                    live_io.focus_log,
                                    "[EVENT AUDIT] frame={} deck={}",
                                    self.frame_count, deck_summary
                                )
                                .unwrap();
                            }
                            let encoded = serde_json::to_string(&serde_json::json!({
                                "frame": self.frame_count,
                                "room_phase": room_phase,
                                "screen": screen,
                                "command": cmd,
                                "decision": decision_audit,
                            }))
                            .unwrap();
                            let _ = writeln!(live_io.event_audit, "{}", encoded);
                            let _ = live_io.event_audit.flush();
                            let family = decision_audit
                                .get("family")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let reasons = event_validation_failure_reasons(&decision_audit);
                            if !reasons.is_empty() {
                                let _ = write_failure_snapshot(
                                    live_io,
                                    self.frame_count,
                                    frame,
                                    "validation_failure",
                                    reasons.clone(),
                                    serde_json::json!({
                                        "chosen_command": cmd,
                                        "event_decision": decision_audit.clone(),
                                    }),
                                );
                                if self.config.fail_fast_debug
                                    && should_fail_fast_on_snapshot("validation_failure", &reasons)
                                {
                                    let _ = writeln!(
                                        live_io.focus_log,
                                        "[FAIL_FAST] frame={} trigger=validation_failure reasons={}",
                                        self.frame_count,
                                        reasons.join(",")
                                    );
                                    let _ = live_io.log.flush();
                                    let _ = live_io.focus_log.flush();
                                    return Some(LoopExitReason::FailFast);
                                }
                            }
                            if self.config.sidecar_shadow {
                                let fallback_used = matches!(family, "compatibility_fallback");
                                let meta = crate::bot::DecisionMetadata::new(
                                    crate::bot::DecisionDomain::Event,
                                    "event_policy",
                                    if fallback_used {
                                        Some("compatibility_fallback_adapter")
                                    } else {
                                        None
                                    },
                                    None,
                                    fallback_used,
                                );
                                let shadow = sidecar::noncombat_decision_shadow_json(
                                    self.frame_count,
                                    "live_comm_event",
                                    &meta,
                                    cmd.clone(),
                                    decision_audit,
                                );
                                sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                            }
                        }
                    }
                }
            }
        } else if screen == "CARD_REWARD" {
            emit_bot_card_reward_audit(parsed, self.frame_count, &cmd, &mut live_io.reward_audit);
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        if let Some(cards) = gs
                            .get("screen_state")
                            .and_then(|v| v.get("cards"))
                            .and_then(|v| v.as_array())
                        {
                            let offered_ids = cards
                                .iter()
                                .filter_map(|card| {
                                    card.get("id")
                                        .and_then(|v| v.as_str())
                                        .and_then(crate::protocol::java::card_id_from_java)
                                })
                                .collect::<Vec<_>>();
                            if !offered_ids.is_empty() {
                                let reward_cards = offered_ids
                                    .iter()
                                    .copied()
                                    .map(|card_id| {
                                        crate::rewards::state::RewardCard::new(card_id, 0)
                                    })
                                    .collect::<Vec<_>>();
                                let decision = agent.decide_reward_card_policy(
                                    &rs,
                                    crate::bot::RewardCardDecisionContext {
                                        reward_cards: &reward_cards,
                                        can_skip: true,
                                    },
                                );
                                let chosen_choice = if cmd.trim().eq_ignore_ascii_case("SKIP")
                                    || cmd.trim().eq_ignore_ascii_case("PROCEED")
                                {
                                    None
                                } else {
                                    cmd.trim()
                                        .strip_prefix("CHOOSE ")
                                        .and_then(|rest| rest.trim().parse::<usize>().ok())
                                };
                                if let Some(deck_summary) = reward_deck_improvement_summary(
                                    &decision.diagnostics,
                                    chosen_choice,
                                ) {
                                    writeln!(
                                        live_io.focus_log,
                                        "[REWARD] frame={} cmd={} deck={}",
                                        self.frame_count, cmd, deck_summary
                                    )
                                    .unwrap();
                                }
                                let shadow = sidecar::reward_shadow_json(
                                    self.frame_count,
                                    "live_comm_reward",
                                    &decision.meta,
                                    &decision.diagnostics,
                                    chosen_choice,
                                    None,
                                );
                                sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                            }
                        }
                    }
                }
            }
        } else if screen == "COMBAT_REWARD" {
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        if let Some(reward) =
                            crate::cli::live_comm_noncombat::build_live_reward_state_with_protocol(
                                parsed, gs,
                            )
                        {
                            let blocked_potion_offers =
                                crate::cli::live_comm_noncombat::blocked_replaceable_reward_potion_offers(parsed);
                            let decision = agent.decide_reward_claim_policy(
                                &rs,
                                crate::bot::RewardClaimDecisionContext {
                                    reward: &reward,
                                    blocked_potion_offers: &blocked_potion_offers,
                                },
                            );
                            let action = match decision.action {
                                crate::bot::RewardClaimDecisionAction::Claim(idx) => {
                                    format!("claim:{idx}")
                                }
                                crate::bot::RewardClaimDecisionAction::DiscardPotion(idx) => {
                                    format!("discard_potion:{idx}")
                                }
                                crate::bot::RewardClaimDecisionAction::Proceed => {
                                    "proceed".to_string()
                                }
                            };
                            let shadow = sidecar::noncombat_decision_shadow_json(
                                self.frame_count,
                                "live_comm_reward_claim",
                                &decision.meta,
                                cmd.clone(),
                                serde_json::json!({
                                    "action": action,
                                    "reward_item_count": reward.items.len(),
                                    "blocked_potion_offer_count": blocked_potion_offers.len(),
                                }),
                            );
                            sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                        }
                    }
                }
            }
        } else if screen == "SHOP_SCREEN" {
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        if let Some(shop) =
                            crate::cli::live_comm_noncombat::build_live_shop_state(gs)
                        {
                            let decision = agent.decide_shop_policy(
                                &rs,
                                crate::bot::ShopDecisionContext { shop: &shop },
                            );
                            let action = match decision.action {
                                crate::bot::ShopDecisionAction::BuyCard(idx) => {
                                    format!("buy_card:{idx}")
                                }
                                crate::bot::ShopDecisionAction::BuyRelic(idx) => {
                                    format!("buy_relic:{idx}")
                                }
                                crate::bot::ShopDecisionAction::BuyPotion(idx) => {
                                    format!("buy_potion:{idx}")
                                }
                                crate::bot::ShopDecisionAction::PurgeCard(idx) => {
                                    format!("purge_card:{idx}")
                                }
                                crate::bot::ShopDecisionAction::DiscardPotion(idx) => {
                                    format!("discard_potion:{idx}")
                                }
                                crate::bot::ShopDecisionAction::Leave => "leave".to_string(),
                            };
                            let shadow = sidecar::noncombat_decision_shadow_json(
                                self.frame_count,
                                "live_comm_shop",
                                &decision.meta,
                                cmd.clone(),
                                serde_json::json!({
                                    "action": action,
                                    "card_count": shop.cards.len(),
                                    "relic_count": shop.relics.len(),
                                    "potion_count": shop.potions.len(),
                                    "purge_available": shop.purge_available,
                                }),
                            );
                            sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                        }
                    }
                }
            }
        } else if screen == "MAP" {
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        let decision = agent.decide_map_policy(&rs);
                        let shadow = sidecar::noncombat_decision_shadow_json(
                            self.frame_count,
                            "live_comm_map",
                            &decision.meta,
                            cmd.clone(),
                            serde_json::json!({
                                "chosen_x": decision.chosen_x,
                                "top_option_count": decision.diagnostics.top_options.len(),
                                "top_rationale": decision
                                    .diagnostics
                                    .top_options
                                    .first()
                                    .map(|option| option.rationale_key),
                            }),
                        );
                        sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                    }
                }
            }
        } else if screen == "REST" {
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        let decision = agent.decide_campfire_policy(&rs);
                        let shadow = sidecar::noncombat_decision_shadow_json(
                            self.frame_count,
                            "live_comm_campfire",
                            &decision.meta,
                            cmd.clone(),
                            serde_json::json!({
                                "choice": format!("{:?}", decision.choice),
                                "top_option_count": decision.diagnostics.top_options.len(),
                                "top_rationale": decision
                                    .diagnostics
                                    .top_options
                                    .first()
                                    .map(|option| option.rationale_key),
                            }),
                        );
                        sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                    }
                }
            }
        } else if screen == "BOSS_REWARD" {
            if self.config.sidecar_shadow {
                if let Some(gs) = parsed.get("game_state") {
                    if let Some(rs) = crate::cli::live_comm_noncombat::build_live_run_state(gs) {
                        if let Some(state) =
                            crate::cli::live_comm_noncombat::build_live_boss_relic_state(gs)
                        {
                            let decision = agent.decide_boss_relic_policy(&rs, &state);
                            let shadow = sidecar::noncombat_decision_shadow_json(
                                self.frame_count,
                                "live_comm_boss_relic",
                                &decision.meta,
                                cmd.clone(),
                                serde_json::json!({
                                    "chosen_index": decision.chosen_index,
                                    "option_count": state.relics.len(),
                                    "top_confidence": decision
                                        .diagnostics
                                        .top_candidates
                                        .first()
                                        .map(|candidate| candidate.confidence),
                                    "top_reason": decision
                                        .diagnostics
                                        .top_candidates
                                        .first()
                                        .map(|candidate| candidate.primary_reason),
                                }),
                            );
                            sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
                        }
                    }
                }
            }
        }

        if self.config.human_noncombat_hold && should_hold_human_noncombat(screen) {
            self.sync_human_noncombat_audit(frame, &cmd, live_io);
            let hold_cmd = noncombat_pollution_hold_command(avail);
            self.last_sent_cmd = hold_cmd.to_string();
            self.cmd_failed_count = 0;
            self.combat_runtime.expected_combat_state = None;
            live_io.send_line(stdout, hold_cmd);
            return None;
        }

        if self.pending_human_noncombat_audit.is_some() {
            self.sync_human_noncombat_audit(frame, &cmd, live_io);
        }

        self.last_sent_cmd = cmd.clone();
        self.cmd_failed_count = 0;
        self.combat_runtime.expected_combat_state = None;

        writeln!(
            live_io.log,
            "[F{}] {}  screen={}  → {}",
            self.frame_count, room_phase, screen, cmd
        )
        .unwrap();
        if cmd.trim().is_empty() {
            writeln!(
                live_io.log,
                "  [!] EMPTY NON-COMBAT COMMAND, FALLING BACK TO STATE"
            )
            .unwrap();
            live_io.send_line(stdout, "STATE");
            return None;
        }
        live_io.send_line(stdout, &cmd);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{enrich_event_audit_with_screen_state, event_validation_failure_reasons};
    use serde_json::json;

    #[test]
    fn neow_legacy_fallback_with_screen_progress_is_not_validation_failure() {
        let audit = json!({
            "family": "compatibility_fallback",
            "event_name": "Neow",
            "screen": 3,
            "screen_index": 3,
            "screen_key": null,
        });

        assert!(event_validation_failure_reasons(&audit).is_empty());
    }

    #[test]
    fn unexpected_compatibility_fallback_still_flags_validation_failure() {
        let audit = json!({
            "family": "compatibility_fallback",
            "event_name": "Living Wall",
            "screen": 0,
            "screen_index": 0,
            "screen_key": "INTRO",
        });

        assert_eq!(
            event_validation_failure_reasons(&audit),
            vec!["compatibility_fallback".to_string()]
        );
    }

    #[test]
    fn multistage_intro_without_key_still_flags_incomplete_screen_semantics() {
        let audit = json!({
            "family": "cost_tradeoff",
            "event_name": "Golden Idol",
            "screen": 0,
            "screen_index": 0,
            "screen_key": null,
        });

        assert_eq!(
            event_validation_failure_reasons(&audit),
            vec!["event_screen_semantics_incomplete".to_string()]
        );
    }

    #[test]
    fn enrich_event_audit_backfills_protocol_screen_metadata() {
        let audit = json!({
            "family": "cost_tradeoff",
            "event_name": "Golden Idol",
            "screen": 0,
        });
        let game_state = json!({
            "screen_state": {
                "current_screen_index": 2,
                "current_screen_key": "TAKE",
                "screen_source": "protocol_screen_state"
            }
        });

        let enriched = enrich_event_audit_with_screen_state(&audit, &game_state);

        assert_eq!(
            enriched.get("screen_index").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            enriched.get("screen_key").and_then(|v| v.as_str()),
            Some("TAKE")
        );
        assert_eq!(
            enriched.get("screen_source").and_then(|v| v.as_str()),
            Some("protocol_screen_state")
        );
    }
}
