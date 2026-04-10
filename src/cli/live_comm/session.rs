use super::combat::{handle_live_combat_frame, CombatRuntime};
use super::frame::LiveFrame;
use super::io::LiveCommIo;
use super::noncombat::{maybe_arm_human_card_reward_audit, route_noncombat_command};
use super::reward_audit::{
    classify_human_card_reward_audit_disposition, extract_human_card_reward_choice,
    finalize_human_card_reward_audit, finalize_human_card_reward_audit_without_choice,
    human_card_reward_hold_context, reward_choice_matches_pending_session,
    HumanCardRewardAuditDisposition, PendingHumanCardRewardAudit,
};
use super::watch::{maybe_capture_live_watch, remember_live_record, LiveWatchRuntime};
use super::{
    should_clear_combat_context, LiveCommConfig, LoopExitReason, ENGINE_BUG_SUMMARY_INTERVAL,
    SIG_PATH,
};
use crate::bot::agent::Agent;
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
            self.handle_java_error(err, live_io, stdout);
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
            self.combat_runtime
                .flush_summary_on_game_over(&mut live_io.log, self.frame_count);
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
            self.game_over_seen = true;
            self.final_victory = victory;
            let _ = live_io.log.flush();
            let _ = live_io.raw.flush();
            return Some(LoopExitReason::GameOver);
        }

        if should_clear_combat_context(is_combat, room_phase, screen) {
            self.combat_runtime
                .clear_after_combat_if_needed(&mut live_io.log, self.frame_count);
        }

        if is_combat && screen == "NONE" && (has("play") || has("end")) {
            handle_live_combat_frame(
                frame,
                gs,
                self.frame_count,
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

    fn handle_java_error<W: Write>(&mut self, err: &str, live_io: &mut LiveCommIo, stdout: &mut W) {
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
                                "[F{}] CARD_REWARD human audit pending; reason={} offscreen_polls={} screen={} screen_name={} room_phase={} → {}",
                                self.frame_count,
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
            writeln!(live_io.log, "  [!] ERROR LOOP DETECTED, FALLING BACK").unwrap();
            if has("skip") {
                cmd = "SKIP".to_string();
            } else if has("proceed") {
                cmd = "PROCEED".to_string();
            } else {
                cmd = "RETURN".to_string();
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
