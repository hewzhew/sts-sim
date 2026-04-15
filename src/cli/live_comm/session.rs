use super::combat::{handle_live_combat_frame, CombatFrameOutcome, CombatRuntime};
use super::frame::LiveFrame;
use super::io::LiveCommIo;
use super::noncombat::{maybe_arm_human_card_reward_audit, route_noncombat_command};
use super::reward_audit::{
    classify_human_card_reward_audit_disposition, emit_bot_card_reward_audit,
    extract_human_card_reward_choice, finalize_human_card_reward_audit,
    finalize_human_card_reward_audit_without_choice, human_card_reward_audit_reason_source,
    human_card_reward_hold_context, reward_choice_matches_pending_session,
    HumanCardRewardAuditDisposition, PendingHumanCardRewardAudit,
};
use super::snapshot::write_failure_snapshot;
use super::watch::{maybe_capture_live_watch, remember_live_record, LiveWatchRuntime};
use super::{
    should_clear_combat_context, LiveCommConfig, LoopExitReason, ENGINE_BUG_SUMMARY_INTERVAL,
    SIG_PATH,
};
use crate::bot::agent::Agent;
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
    pub(super) combat_handoff_hold_polls: u32,
    pub(super) live_watch_runtime: LiveWatchRuntime,
    pub(super) engine_bug_total: usize,
    pub(super) content_gap_total: usize,
    pub(super) coverage_db: crate::bot::coverage::CoverageDb,
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
            combat_handoff_hold_polls: 0,
            live_watch_runtime: LiveWatchRuntime::default(),
            engine_bug_total: 0,
            content_gap_total: 0,
            coverage_db: crate::bot::coverage::CoverageDb::load_or_default(),
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
            self.handle_java_error(frame, err, live_io, stdout);
            return None;
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
            let outcome = handle_live_combat_frame(
                frame,
                gs,
                self.frame_count,
                self.config.parity_mode,
                self.config.combat_search_budget,
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
            }
        } else {
            self.handle_noncombat_frame(agent, frame, &avail, screen, room_phase, live_io, stdout);
        }

        let _ = live_io.log.flush();
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
    ) {
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
            self.last_error_msg = err.to_string();
        } else if self.consecutive_errors == 3 {
            writeln!(live_io.log, "  (suppressing repeated errors...)").unwrap();
        }
        if self.consecutive_errors >= 5 {
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
                vec![
                    "java_error_flood".to_string(),
                    err.to_string(),
                    format!("repeats={}", self.consecutive_errors),
                ],
                serde_json::json!({
                    "chosen_command": self.last_sent_cmd,
                    "last_command_kind": self.last_protocol_command_kind,
                    "error": err,
                }),
            );
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        live_io.send_line(stdout, "STATE");
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
    ) {
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
            return;
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
            return;
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
                self.last_sent_cmd = hold_cmd.clone();
                live_io.send_line(stdout, &hold_cmd);
                return;
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
                self.last_sent_cmd = hold_cmd.clone();
                live_io.send_line(stdout, &hold_cmd);
                return;
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
                            let encoded = serde_json::to_string(&serde_json::json!({
                                "frame": self.frame_count,
                                "room_phase": room_phase,
                                "screen": screen,
                                "command": cmd,
                                "decision": trace.audit,
                            }))
                            .unwrap();
                            let _ = writeln!(live_io.event_audit, "{}", encoded);
                            let _ = live_io.event_audit.flush();
                            let family = trace
                                .audit
                                .get("family")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let event_name = trace
                                .audit
                                .get("event_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let screen_key = trace.audit.get("screen_key");
                            let suspicious_multistage = matches!(
                                event_name,
                                "Neow"
                                    | "Shining Light"
                                    | "Golden Idol"
                                    | "Knowing Skull"
                                    | "Living Wall"
                                    | "Big Fish"
                            ) && screen_key
                                .is_none_or(|value| value.is_null());
                            if family == "compatibility_fallback" || suspicious_multistage {
                                let mut reasons = Vec::new();
                                if family == "compatibility_fallback" {
                                    reasons.push("compatibility_fallback".to_string());
                                }
                                if suspicious_multistage {
                                    reasons.push("event_screen_semantics_incomplete".to_string());
                                }
                                let _ = write_failure_snapshot(
                                    live_io,
                                    self.frame_count,
                                    frame,
                                    "validation_failure",
                                    reasons,
                                    serde_json::json!({
                                        "chosen_command": cmd,
                                        "event_decision": trace.audit.clone(),
                                    }),
                                );
                            }
                            if self.config.sidecar_shadow {
                                let shadow = serde_json::json!({
                                    "kind": "event_shadow",
                                    "frame": self.frame_count,
                                    "source": "live_comm_event",
                                    "decision": trace.audit,
                                    "suggestion": serde_json::Value::Null,
                                });
                                crate::bot::sidecar::write_shadow_record(
                                    &mut live_io.sidecar_shadow,
                                    &shadow,
                                );
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
                                        .and_then(crate::diff::protocol::mapper::card_id_from_java)
                                })
                                .collect::<Vec<_>>();
                            if !offered_ids.is_empty() {
                                let evaluation = crate::bot::reward_heuristics::evaluate_reward_screen_for_run_detailed(
                                    &offered_ids,
                                    &rs,
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
                                let shadow = crate::bot::sidecar::reward_shadow_json(
                                    self.frame_count,
                                    "live_comm_reward",
                                    &evaluation,
                                    chosen_choice,
                                    None,
                                );
                                crate::bot::sidecar::write_shadow_record(
                                    &mut live_io.sidecar_shadow,
                                    &shadow,
                                );
                            }
                        }
                    }
                }
            }
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
            return;
        }
        live_io.send_line(stdout, &cmd);
    }
}

