use super::{
    java_process_status, LiveCommConfig, COMBAT_SUSPECT_AUDIT_PATH, CURRENT_LOG_ROOT,
    EVENT_AUDIT_PATH, FAILURE_SNAPSHOT_AUDIT_PATH, FOCUS_LOG_PATH, LIVE_COMM_BUILD_TAG, LOG_PATH,
    RAW_PATH, REPLAY_PATH, REWARD_AUDIT_PATH, SIDECAR_SHADOW_AUDIT_PATH, SIG_PATH,
    WATCH_AUDIT_PATH, WATCH_NONCOMBAT_AUDIT_PATH,
};
use crate::cli::live_comm_admin::{timestamp_string, LiveLogPaths};
use crate::cli::live_comm_runtime::{
    finalize_live_run, runtime_provenance, verify_replay_counts, FinalizeRunInput,
};
use crate::diff::replay::live_comm_replay::generate_live_session_replay_sidecar;
use std::io::{BufRead, Write};

pub(super) enum ProtocolReadFrame {
    Line(String),
    Eof,
    StdinError(std::io::Error),
    InvalidUtf8(Vec<u8>),
}

pub(super) struct LiveCommIo {
    pub(super) log: std::fs::File,
    pub(super) raw: std::fs::File,
    pub(super) signature_log: std::fs::File,
    pub(super) focus_log: std::fs::File,
    pub(super) reward_audit: std::fs::File,
    pub(super) event_audit: std::fs::File,
    pub(super) sidecar_shadow: std::fs::File,
    pub(super) watch_audit: std::fs::File,
    pub(super) watch_noncombat_audit: std::fs::File,
    pub(super) combat_suspects: std::fs::File,
    pub(super) failure_snapshots: std::fs::File,
}

impl LiveCommIo {
    pub(super) fn new(config: &LiveCommConfig) -> Self {
        std::fs::create_dir_all(CURRENT_LOG_ROOT).unwrap();
        let mut log = std::fs::File::create(LOG_PATH).unwrap();
        let raw = std::fs::File::create(RAW_PATH).unwrap();
        let signature_log = std::fs::File::create(SIG_PATH).unwrap();
        let mut focus_log = std::fs::File::create(FOCUS_LOG_PATH).unwrap();
        let reward_audit = create_optional_sidecar(REWARD_AUDIT_PATH, true, "reward_audit");
        let event_audit = std::fs::File::create(EVENT_AUDIT_PATH).unwrap();
        let sidecar_shadow = create_optional_sidecar(
            SIDECAR_SHADOW_AUDIT_PATH,
            config.sidecar_shadow,
            "sidecar_shadow",
        );
        let watch_audit = create_optional_sidecar(
            WATCH_AUDIT_PATH,
            config.watch_capture.enabled(),
            "watch_audit",
        );
        let watch_noncombat_audit = create_optional_sidecar(
            WATCH_NONCOMBAT_AUDIT_PATH,
            config.watch_capture.enabled(),
            "watch_noncombat",
        );
        let combat_suspects = std::fs::File::create(COMBAT_SUSPECT_AUDIT_PATH).unwrap();
        let failure_snapshots = std::fs::File::create(FAILURE_SNAPSHOT_AUDIT_PATH).unwrap();

        writeln!(log, "=== Rust Live-Comm Started ===").unwrap();
        writeln!(focus_log, "=== Rust Live-Comm Focused Debug ===").unwrap();
        writeln!(log, "[BUILD] {}", LIVE_COMM_BUILD_TAG).unwrap();
        writeln!(focus_log, "[BUILD] {}", LIVE_COMM_BUILD_TAG).unwrap();
        let provenance = runtime_provenance();
        writeln!(
            log,
            "[PROVENANCE] exe_path={} exe_mtime_utc={} git_short_sha={} build_unix={:?} build_time_utc={} profile_name={} profile_path={}",
            provenance.exe_path.as_deref().unwrap_or("<unknown>"),
            provenance.exe_mtime_utc.as_deref().unwrap_or("<unknown>"),
            provenance.git_short_sha.as_deref().unwrap_or("<unknown>"),
            provenance.build_unix,
            provenance.build_time_utc.as_deref().unwrap_or("<unknown>"),
            provenance.profile_name.as_deref().unwrap_or("<unknown>"),
            provenance.profile_path.as_deref().unwrap_or("<unknown>")
        )
        .unwrap();
        writeln!(
            focus_log,
            "[PROVENANCE] exe={} git={} profile={}",
            provenance.exe_path.as_deref().unwrap_or("<unknown>"),
            provenance.git_short_sha.as_deref().unwrap_or("<unknown>"),
            provenance.profile_name.as_deref().unwrap_or("<unknown>")
        )
        .unwrap();
        writeln!(
            log,
            "[CONFIG] human_card_reward_audit={} human_boss_combat_handoff={} watch_capture_enabled={} watch_match_mode={:?} watch_cards={:?} watch_relics={:?} watch_powers={:?} watch_monsters={:?} watch_screens={:?} watch_room_phases={:?} watch_command_kinds={:?} watch_window={} watch_dedupe_window={} watch_max={} watch_out_dir={}",
            config.human_card_reward_audit,
            config.human_boss_combat_handoff,
            config.watch_capture.enabled(),
            config.watch_capture.match_mode,
            config.watch_capture.cards,
            config.watch_capture.relics,
            config.watch_capture.powers,
            config.watch_capture.monsters,
            config.watch_capture.screens,
            config.watch_capture.room_phases,
            config.watch_capture.command_kinds,
            config.watch_capture.window_responses,
            config.watch_capture.dedupe_window_responses,
            config.watch_capture.max_captures,
            config.watch_capture.out_dir.display()
        )
        .unwrap();
        writeln!(
            log,
            "[CONFIG] parity_mode={:?} combat_search_budget={} sidecar_shadow={}",
            config.parity_mode, config.combat_search_budget, config.sidecar_shadow
        )
        .unwrap();
        writeln!(
            focus_log,
            "[CONFIG] parity_mode={:?} combat_search_budget={} sidecar_shadow={}",
            config.parity_mode, config.combat_search_budget, config.sidecar_shadow
        )
        .unwrap();
        writeln!(
            focus_log,
            "[CONFIG] focused log keeps only parse diffs, parity failures, flagged combat summaries, and session end markers"
        )
        .unwrap();

        Self {
            log,
            raw,
            signature_log,
            focus_log,
            reward_audit,
            event_audit,
            sidecar_shadow,
            watch_audit,
            watch_noncombat_audit,
            combat_suspects,
            failure_snapshots,
        }
    }

    pub(super) fn send_line<W: Write>(&mut self, stdout: &mut W, line: &str) {
        let _ = writeln!(stdout, "{}", line);
        let _ = stdout.flush();
        if line.starts_with(super::LIVE_COMM_BOOTSTRAP_PREFIX) {
            let _ = writeln!(self.log, "Sent: bootstrap");
        } else if line == "ready" {
            let _ = writeln!(self.log, "Sent: ready");
        }
    }

    pub(super) fn read_protocol_frame<R: BufRead>(
        &mut self,
        stdin_lock: &mut R,
    ) -> ProtocolReadFrame {
        let mut raw_line = Vec::new();
        let read = match stdin_lock.read_until(b'\n', &mut raw_line) {
            Ok(n) => n,
            Err(err) => return ProtocolReadFrame::StdinError(err),
        };
        if read == 0 {
            return ProtocolReadFrame::Eof;
        }
        while matches!(raw_line.last(), Some(b'\n' | b'\r')) {
            raw_line.pop();
        }
        match String::from_utf8(raw_line) {
            Ok(line) => ProtocolReadFrame::Line(line),
            Err(err) => ProtocolReadFrame::InvalidUtf8(err.into_bytes()),
        }
    }

    pub(super) fn write_raw_line(&mut self, line: &str) {
        let _ = writeln!(self.raw, "{}", line);
    }

    pub(super) fn log_stdin_io_error(&mut self, err: &std::io::Error) {
        let _ = writeln!(self.log, "STDIN ERR: {}", err);
    }

    pub(super) fn log_stdin_invalid_utf8(&mut self, bytes: &[u8], hex_prefix: &str) {
        let lossy = String::from_utf8_lossy(bytes);
        let _ = writeln!(self.log, "STDIN ERR: stream did not contain valid UTF-8");
        let _ = writeln!(
            self.log,
            "[STDIN RAW] len={} hex_prefix={}",
            bytes.len(),
            hex_prefix
        );
        let _ = writeln!(self.log, "[STDIN RAW LOSSY] {}", lossy);
    }

    pub(super) fn flush_all(&mut self) {
        let _ = self.log.flush();
        let _ = self.raw.flush();
        let _ = self.signature_log.flush();
        let _ = self.focus_log.flush();
        let _ = self.reward_audit.flush();
        let _ = self.event_audit.flush();
        let _ = self.sidecar_shadow.flush();
        let _ = self.watch_audit.flush();
        let _ = self.watch_noncombat_audit.flush();
        let _ = self.combat_suspects.flush();
        let _ = self.failure_snapshots.flush();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_session(
        &mut self,
        config: &LiveCommConfig,
        loop_exit_reason: &super::LoopExitReason,
        last_sent_cmd: &str,
        last_response_id: Option<i64>,
        last_state_frame_id: Option<i64>,
        last_protocol_command_kind: Option<&str>,
        engine_bug_total: usize,
        content_gap_total: usize,
        game_over_seen: bool,
        final_victory: bool,
    ) {
        let run_id = timestamp_string();
        let java_status = java_process_status();
        match loop_exit_reason {
            super::LoopExitReason::GameOver => {
                let _ = writeln!(self.log, "=== Loop exited: GAME_OVER ===");
                let _ = writeln!(self.focus_log, "=== Loop exited: GAME_OVER ===");
            }
            super::LoopExitReason::ParityFail => {
                let _ = writeln!(self.log, "=== Loop exited: PARITY_FAIL ===");
                let _ = writeln!(self.focus_log, "=== Loop exited: PARITY_FAIL ===");
            }
            super::LoopExitReason::StdinError => {
                let _ = writeln!(self.log, "=== Loop exited: STDIN_ERROR ===");
                let _ = writeln!(self.focus_log, "=== Loop exited: STDIN_ERROR ===");
            }
            super::LoopExitReason::StdinEof => {
                let _ = writeln!(self.log, "=== Loop exited: STDIN_EOF ===");
                let _ = writeln!(self.focus_log, "=== Loop exited: STDIN_EOF ===");
            }
        }
        let _ = writeln!(
            self.log,
            "[DISCONNECT] last_sent_cmd={} last_response_id={:?} last_state_frame_id={:?} last_command_kind={:?} java_status={}",
            if last_sent_cmd.is_empty() {
                "<none>"
            } else {
                last_sent_cmd
            },
            last_response_id,
            last_state_frame_id,
            last_protocol_command_kind,
            java_status
        );
        let _ = writeln!(
            self.focus_log,
            "[DISCONNECT] last_sent_cmd={} last_response_id={:?} last_state_frame_id={:?} last_command_kind={:?} java_status={}",
            if last_sent_cmd.is_empty() {
                "<none>"
            } else {
                last_sent_cmd
            },
            last_response_id,
            last_state_frame_id,
            last_protocol_command_kind,
            java_status
        );
        if *loop_exit_reason == super::LoopExitReason::StdinEof && !game_over_seen {
            let diagnosis = match java_status {
                "java_alive" => "protocol_stream_closed_but_java_still_running",
                "java_not_found" => "game_or_mod_process_exited",
                _ => "protocol_stream_closed_unknown_java_status",
            };
            let _ = writeln!(self.log, "[DISCONNECT] diagnosis={}", diagnosis);
            let _ = writeln!(self.focus_log, "[DISCONNECT] diagnosis={}", diagnosis);
        }
        self.flush_all();
        let replay_generation_result = generate_live_session_replay_sidecar(
            std::path::Path::new(RAW_PATH),
            std::path::Path::new(REPLAY_PATH),
        );
        match &replay_generation_result {
            Ok(replay) => {
                let _ = writeln!(
                    self.log,
                    "[REPLAY] generated structured sidecar {} (frames={} steps={})",
                    REPLAY_PATH,
                    replay.total_frames,
                    replay.steps.len()
                );
                let _ = writeln!(
                    self.focus_log,
                    "[REPLAY] generated structured sidecar {} (frames={} steps={})",
                    REPLAY_PATH,
                    replay.total_frames,
                    replay.steps.len()
                );
            }
            Err(err) => {
                let _ = writeln!(self.log, "[REPLAY] generation failed: {}", err);
                let _ = writeln!(self.focus_log, "[REPLAY] generation failed: {}", err);
            }
        }
        self.flush_all();
        let (replay_failures, timing_diffs) =
            verify_replay_counts(std::path::Path::new(REPLAY_PATH)).unwrap_or((0, 0));
        let finalize = finalize_live_run(
            &LiveLogPaths::default_paths(),
            FinalizeRunInput {
                run_id: run_id.clone(),
                timestamp: run_id,
                build_tag: LIVE_COMM_BUILD_TAG.to_string(),
                parity_mode: format!("{:?}", config.parity_mode),
                watch_enabled: config.watch_capture.enabled(),
                session_exit_reason: loop_exit_reason_string(loop_exit_reason).to_string(),
                engine_bug_total,
                content_gap_total,
                timing_diff_total: timing_diffs,
                replay_failures,
                game_over_seen,
                final_victory,
            },
        );
        match finalize {
            Ok(outcome) => {
                let mut archive_log = std::fs::OpenOptions::new().append(true).open(LOG_PATH).ok();
                if let Some(log_file) = archive_log.as_mut() {
                    let _ = writeln!(
                        log_file,
                        "\n[RUN] id={} label={} manifest={} dir={}",
                        outcome
                            .run_dir
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("<unknown>"),
                        outcome.classification_label,
                        outcome.manifest_path.display(),
                        outcome.run_dir.display()
                    );
                    let _ = writeln!(log_file, "[RUN] {}", outcome.gc_summary);
                }
                let mut focus_archive_log = std::fs::OpenOptions::new()
                    .append(true)
                    .open(FOCUS_LOG_PATH)
                    .ok();
                if let Some(log_file) = focus_archive_log.as_mut() {
                    let _ = writeln!(
                        log_file,
                        "\n[RUN] label={} manifest={}",
                        outcome.classification_label,
                        outcome.manifest_path.display()
                    );
                    let _ = writeln!(log_file, "[RUN] {}", outcome.gc_summary);
                }
                eprintln!(
                    "[live_comm] finalized run {} ({}) validation={}",
                    outcome
                        .run_dir
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("<unknown>"),
                    outcome.classification_label,
                    outcome.validation_status
                );
                eprintln!("[live_comm] {}", outcome.gc_summary);
            }
            Err(err) => {
                eprintln!("[live_comm] finalize failed: {}", err);
            }
        }
    }
}

fn create_optional_sidecar(path: &str, enabled: bool, label: &str) -> std::fs::File {
    if enabled {
        std::fs::File::create(path).unwrap()
    } else {
        let _ = std::fs::remove_file(path);
        let temp = std::env::temp_dir().join(format!(
            "sts_live_comm_{}_{}_{}.jsonl",
            label,
            std::process::id(),
            timestamp_string()
        ));
        std::fs::File::create(temp).unwrap()
    }
}

fn loop_exit_reason_string(reason: &super::LoopExitReason) -> &'static str {
    match reason {
        super::LoopExitReason::GameOver => "GAME_OVER",
        super::LoopExitReason::ParityFail => "PARITY_FAIL",
        super::LoopExitReason::StdinError => "STDIN_ERROR",
        super::LoopExitReason::StdinEof => "STDIN_EOF",
    }
}
