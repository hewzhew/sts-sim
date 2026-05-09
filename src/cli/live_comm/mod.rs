use serde_json::json;
use std::any::Any;
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod combat;
mod frame;
mod human_noncombat_audit;
mod io;
mod noncombat;
mod reward_audit;
mod session;
mod snapshot;
mod watch;

use frame::LiveFrame;
use io::{LiveCommIo, ProtocolReadFrame};
use session::LiveCommSession;

pub(crate) const CURRENT_LOG_ROOT: &str = r"d:\rust\sts_simulator\logs\current";
pub(crate) const LOG_PATH: &str = r"d:\rust\sts_simulator\logs\current\live_comm_debug.txt";
pub(crate) const RAW_PATH: &str = r"d:\rust\sts_simulator\logs\current\live_comm_raw.jsonl";
pub(crate) const REPLAY_PATH: &str = r"d:\rust\sts_simulator\logs\current\live_comm_replay.json";
pub(crate) const SIG_PATH: &str = r"d:\rust\sts_simulator\logs\current\live_comm_signatures.jsonl";
pub(crate) const FOCUS_LOG_PATH: &str = r"d:\rust\sts_simulator\logs\current\live_comm_focus.txt";
pub(crate) const REWARD_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_reward_audit.jsonl";
pub(crate) const EVENT_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_event_audit.jsonl";
pub(crate) const HUMAN_NONCOMBAT_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_human_noncombat_audit.jsonl";
pub(crate) const SIDECAR_SHADOW_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_sidecar_shadow.jsonl";
pub(crate) const COMBAT_DECISION_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_combat_decision_audit.jsonl";
pub(crate) const WATCH_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_watch_audit.jsonl";
pub(crate) const WATCH_NONCOMBAT_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_watch_noncombat.jsonl";
pub(crate) const COMBAT_SUSPECT_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_combat_suspects.jsonl";
pub(crate) const FAILURE_SNAPSHOT_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_failure_snapshots.jsonl";
const LIVE_COMM_BUILD_TAG: &str = env!("LIVE_COMM_BUILD_TAG");
pub(crate) const LIVE_COMM_PROTOCOL_VERSION: u32 = 2;
pub(crate) const LIVE_COMM_BOOTSTRAP_PREFIX: &str = "__LIVE_COMM_BOOTSTRAP__ ";
const ENGINE_BUG_SUMMARY_INTERVAL: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct LiveCommConfig {
    pub human_card_reward_audit: bool,
    pub human_boss_combat_handoff: bool,
    pub human_noncombat_hold: bool,
    pub fail_fast_debug: bool,
    pub sidecar_shadow: bool,
    pub parity_mode: LiveParityMode,
    pub combat_mode: LiveCombatMode,
    pub exact_turn_mode: LiveExactTurnMode,
    pub combat_search_budget: u32,
    pub legacy_root_legal_moves: bool,
    pub watch_capture: LiveWatchCaptureConfig,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveParityMode {
    #[default]
    Survey,
    Strict,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveCombatMode {
    ChooserOnly,
    #[default]
    ChooserPlusSampledAudit,
    FullDebug,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveExactTurnMode {
    Off,
    #[default]
    Auto,
    Force,
}

#[derive(Clone, Debug, Default)]
pub struct LiveWatchCaptureConfig {
    pub cards: Vec<String>,
    pub relics: Vec<String>,
    pub powers: Vec<String>,
    pub monsters: Vec<String>,
    pub screens: Vec<String>,
    pub room_phases: Vec<String>,
    pub command_kinds: Vec<String>,
    pub match_mode: LiveWatchMatchMode,
    pub window_responses: usize,
    pub dedupe_window_responses: usize,
    pub max_captures: usize,
    pub out_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveWatchMatchMode {
    #[default]
    Any,
    All,
}

impl LiveWatchCaptureConfig {
    fn enabled(&self) -> bool {
        !self.cards.is_empty()
            || !self.relics.is_empty()
            || !self.powers.is_empty()
            || !self.monsters.is_empty()
            || !self.screens.is_empty()
            || !self.room_phases.is_empty()
            || !self.command_kinds.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopExitReason {
    GameOver,
    ParityFail,
    FailFast,
    StdinError,
    StdinEof,
    Panic,
}

fn java_process_status() -> &'static str {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Process java,javaw -ErrorAction SilentlyContinue | Select-Object -First 1 | ForEach-Object { $_.ProcessName }",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let found = stdout.lines().any(|line| {
                let line = line.trim();
                line.eq_ignore_ascii_case("java") || line.eq_ignore_ascii_case("javaw")
            });
            if found {
                "java_alive"
            } else {
                "java_not_found"
            }
        }
        _ => "java_status_unknown",
    }
}

fn hex_prefix(bytes: &[u8], limit: usize) -> String {
    bytes
        .iter()
        .take(limit)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

// ─── Main Loop ───────────────────────────────────────────────

pub fn run_live_comm_loop(mut agent: crate::bot::Agent, config: LiveCommConfig) {
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut stdout = std::io::stdout();
    let mut live_io = LiveCommIo::new(&config);
    let mut session = LiveCommSession::new(config);

    live_io.send_line(&mut stdout, &bootstrap_message());

    let loop_exit_reason = loop {
        let line = match live_io.read_protocol_frame(&mut stdin_lock) {
            ProtocolReadFrame::Line(line) => line,
            ProtocolReadFrame::Eof => break LoopExitReason::StdinEof,
            ProtocolReadFrame::StdinError(err) => {
                live_io.log_stdin_io_error(&err);
                break LoopExitReason::StdinError;
            }
            ProtocolReadFrame::InvalidUtf8(bytes) => {
                live_io.log_stdin_invalid_utf8(&bytes, &hex_prefix(&bytes, 64));
                break LoopExitReason::StdinError;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        session.frame_count += 1;

        // ── Raw JSON dump ──
        live_io.write_raw_line(&line);

        let frame = match LiveFrame::parse(&line) {
            Ok(frame) => frame,
            Err(e) => {
                writeln!(live_io.log, "[F{}] JSON ERR: {}", session.frame_count, e).unwrap();
                continue;
            }
        };
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            session.handle_frame(&mut agent, &frame, &mut live_io, &mut stdout)
        })) {
            Ok(Some(exit_reason)) => break exit_reason,
            Ok(None) => {}
            Err(payload) => {
                let panic_message = panic_payload_message(payload.as_ref());
                let backtrace = std::backtrace::Backtrace::force_capture();
                let _ = writeln!(
                    live_io.log,
                    "[F{}] PANIC: {}",
                    session.frame_count, panic_message
                );
                let _ = writeln!(
                    live_io.log,
                    "[PANIC FRAME] response_id={:?} state_frame_id={:?} last_sent_cmd={} {}",
                    session.last_response_id,
                    session.last_state_frame_id,
                    if session.last_sent_cmd.is_empty() {
                        "<none>"
                    } else {
                        &session.last_sent_cmd
                    },
                    frame.brief_summary()
                );
                let _ = writeln!(live_io.log, "[PANIC BACKTRACE]\n{backtrace}");
                let _ = writeln!(
                    live_io.focus_log,
                    "[PANIC] frame={} response_id={:?} state_frame_id={:?} last_sent_cmd={} last_command_kind={:?} message={} {}",
                    session.frame_count,
                    session.last_response_id,
                    session.last_state_frame_id,
                    if session.last_sent_cmd.is_empty() {
                        "<none>"
                    } else {
                        &session.last_sent_cmd
                    },
                    session.last_protocol_command_kind,
                    panic_message,
                    frame.brief_summary()
                );
                live_io.flush_all();
                break LoopExitReason::Panic;
            }
        }
    };
    session.finalize_pending_human_noncombat_audit(
        &mut live_io,
        match loop_exit_reason {
            LoopExitReason::GameOver => "loop_exit_game_over",
            LoopExitReason::ParityFail => "loop_exit_parity_fail",
            LoopExitReason::FailFast => "loop_exit_fail_fast",
            LoopExitReason::StdinError => "loop_exit_stdin_error",
            LoopExitReason::StdinEof => "loop_exit_stdin_eof",
            LoopExitReason::Panic => "loop_exit_panic",
        },
    );
    live_io.finish_session(
        &session.config,
        &loop_exit_reason,
        &session.last_sent_cmd,
        session.last_response_id,
        session.last_state_frame_id,
        session.last_protocol_command_kind.as_deref(),
        session.engine_bug_total,
        session.content_gap_total,
        session.game_over_seen,
        session.final_victory,
    );

    if matches!(
        loop_exit_reason,
        LoopExitReason::FailFast | LoopExitReason::Panic
    ) {
        std::process::exit(2);
    }
}

fn bootstrap_message() -> String {
    let provenance = crate::cli::live_comm_runtime::runtime_provenance();
    format!(
        "{prefix}{payload}",
        prefix = LIVE_COMM_BOOTSTRAP_PREFIX,
        payload = json!({
            "kind": "live_comm_bootstrap",
            "protocol_version": LIVE_COMM_PROTOCOL_VERSION,
            "build_tag": LIVE_COMM_BUILD_TAG,
            "git_short_sha": provenance.git_short_sha,
            "repo_head_short_sha": provenance.repo_head_short_sha,
            "binary_matches_head": provenance.binary_matches_head,
            "binary_is_fresh": provenance.binary_is_fresh,
            "build_unix": provenance.build_unix,
            "exe_path": provenance.exe_path,
            "exe_mtime_utc": provenance.exe_mtime_utc,
            "source_inputs_latest_path": provenance.source_inputs_latest_path,
            "source_inputs_latest_mtime_utc": provenance.source_inputs_latest_mtime_utc,
            "profile_name": provenance.profile_name,
            "profile_path": provenance.profile_path,
        })
    )
}

fn unix_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn should_clear_combat_context(is_combat: bool, room_phase: &str, _screen: &str) -> bool {
    // Combat-internal pending screens can temporarily omit split combat snapshots,
    // but they still belong to the same combat and must retain internal-only runtime state.
    // Only clear once we have actually left the combat room phase.
    !is_combat && room_phase != "COMBAT"
}
