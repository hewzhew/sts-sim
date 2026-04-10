use super::{
    java_process_status, ArchiveOutcome, LiveCommConfig, LIVE_COMM_BUILD_TAG, LOG_PATH,
    MAX_DEBUG_ARCHIVES, MAX_RAW_ARCHIVES, MAX_SIGNATURE_ARCHIVES, RAW_PATH, REWARD_AUDIT_PATH,
    SIG_PATH, WATCH_AUDIT_PATH, WATCH_NONCOMBAT_AUDIT_PATH,
};
use crate::cli::live_comm_archive::maybe_archive_live_comm_logs;
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
    pub(super) reward_audit: std::fs::File,
    pub(super) watch_audit: std::fs::File,
    pub(super) watch_noncombat_audit: std::fs::File,
}

impl LiveCommIo {
    pub(super) fn new(config: &LiveCommConfig) -> Self {
        let mut log = std::fs::File::create(LOG_PATH).unwrap();
        let raw = std::fs::File::create(RAW_PATH).unwrap();
        let signature_log = std::fs::File::create(SIG_PATH).unwrap();
        let reward_audit = std::fs::File::create(REWARD_AUDIT_PATH).unwrap();
        let watch_audit = std::fs::File::create(WATCH_AUDIT_PATH).unwrap();
        let watch_noncombat_audit = std::fs::File::create(WATCH_NONCOMBAT_AUDIT_PATH).unwrap();

        writeln!(log, "=== Rust Live-Comm Started ===").unwrap();
        writeln!(log, "[BUILD] {}", LIVE_COMM_BUILD_TAG).unwrap();
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

        Self {
            log,
            raw,
            signature_log,
            reward_audit,
            watch_audit,
            watch_noncombat_audit,
        }
    }

    pub(super) fn send_line<W: Write>(&mut self, stdout: &mut W, line: &str) {
        let _ = writeln!(stdout, "{}", line);
        let _ = stdout.flush();
        if line == "ready" {
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
        let _ = self.reward_audit.flush();
        let _ = self.watch_audit.flush();
        let _ = self.watch_noncombat_audit.flush();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_session(
        &mut self,
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
        let java_status = java_process_status();
        match loop_exit_reason {
            super::LoopExitReason::GameOver => {
                let _ = writeln!(self.log, "=== Loop exited: GAME_OVER ===");
            }
            super::LoopExitReason::StdinError => {
                let _ = writeln!(self.log, "=== Loop exited: STDIN_ERROR ===");
            }
            super::LoopExitReason::StdinEof => {
                let _ = writeln!(self.log, "=== Loop exited: STDIN_EOF ===");
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
        if *loop_exit_reason == super::LoopExitReason::StdinEof && !game_over_seen {
            let diagnosis = match java_status {
                "java_alive" => "protocol_stream_closed_but_java_still_running",
                "java_not_found" => "game_or_mod_process_exited",
                _ => "protocol_stream_closed_unknown_java_status",
            };
            let _ = writeln!(self.log, "[DISCONNECT] diagnosis={}", diagnosis);
        }
        self.flush_all();

        let archive = maybe_archive_live_comm_logs(
            engine_bug_total,
            content_gap_total,
            game_over_seen,
            final_victory,
        )
        .unwrap_or_else(|err| ArchiveOutcome {
            should_archive: false,
            reason: format!("archive_failed: {}", err),
            archived: Vec::new(),
        });
        if archive.should_archive {
            let mut archive_log = std::fs::OpenOptions::new().append(true).open(LOG_PATH).ok();
            if let Some(log_file) = archive_log.as_mut() {
                let _ = writeln!(log_file, "\n[ARCHIVE] reason={}", archive.reason);
                for path in &archive.archived {
                    let _ = writeln!(log_file, "[ARCHIVE] saved {}", path.display());
                }
            }
            eprintln!(
                "[live_comm] archived {} logs; caps raw/debug/signatures = {}/{}/{}",
                archive.archived.len(),
                MAX_RAW_ARCHIVES,
                MAX_DEBUG_ARCHIVES,
                MAX_SIGNATURE_ARCHIVES
            );
        } else {
            eprintln!("[live_comm] no archive: {}", archive.reason);
        }
    }
}
