use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod combat;
mod frame;
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
pub(crate) const SIDECAR_SHADOW_AUDIT_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_sidecar_shadow.jsonl";
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
    pub sidecar_shadow: bool,
    pub parity_mode: LiveParityMode,
    pub combat_search_budget: u32,
    pub watch_capture: LiveWatchCaptureConfig,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveParityMode {
    #[default]
    Survey,
    Strict,
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
    StdinError,
    StdinEof,
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

// ─── Main Loop ───────────────────────────────────────────────

pub fn run_live_comm_loop(mut agent: crate::bot::agent::Agent, config: LiveCommConfig) {
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
        if let Some(exit_reason) =
            session.handle_frame(&mut agent, &frame, &mut live_io, &mut stdout)
        {
            break exit_reason;
        }
    };
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
}

fn bootstrap_message() -> String {
    let provenance = crate::cli::live_comm_logs::runtime_provenance();
    format!(
        "{prefix}{payload}",
        prefix = LIVE_COMM_BOOTSTRAP_PREFIX,
        payload = json!({
            "kind": "live_comm_bootstrap",
            "protocol_version": LIVE_COMM_PROTOCOL_VERSION,
            "build_tag": LIVE_COMM_BUILD_TAG,
            "git_short_sha": provenance.git_short_sha,
            "build_unix": provenance.build_unix,
            "exe_path": provenance.exe_path,
            "exe_mtime_utc": provenance.exe_mtime_utc,
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
    // Combat-internal pending screens can temporarily omit combat_state from the protocol,
    // but they still belong to the same combat and must retain internal-only runtime state.
    // Only clear once we have actually left the combat room phase.
    !is_combat && room_phase != "COMBAT"
}

#[cfg(test)]
mod tests {
    use super::{
        combat, reward_audit, should_clear_combat_context, watch, LiveWatchCaptureConfig,
        LiveWatchMatchMode,
    };
    use crate::diff::replay::comparator::{compare_states, ActionContext};
    use crate::state::core::{ClientInput, EngineState};
    use serde_json::Map;
    use std::path::Path;

    #[test]
    fn combat_pending_screens_keep_combat_context() {
        assert!(!should_clear_combat_context(true, "COMBAT", "NONE"));
        assert!(!should_clear_combat_context(true, "COMBAT", "HAND_SELECT"));
        assert!(!should_clear_combat_context(true, "COMBAT", "GRID"));
        assert!(!should_clear_combat_context(true, "COMBAT", "CARD_REWARD"));
        assert!(!should_clear_combat_context(false, "COMBAT", "HAND_SELECT"));
    }

    #[test]
    fn leaving_combat_clears_combat_context() {
        assert!(should_clear_combat_context(false, "COMPLETE", "NONE"));
        assert!(should_clear_combat_context(false, "EVENT", "MAP"));
        assert!(should_clear_combat_context(false, "SHOP", "SHOP_SCREEN"));
    }

    #[test]
    fn live_watch_any_mode_matches_partial_requirement_set() {
        let config = LiveWatchCaptureConfig {
            cards: vec!["Burst".to_string()],
            relics: vec!["Anchor".to_string()],
            match_mode: LiveWatchMatchMode::Any,
            ..Default::default()
        };
        let root = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "screen_type": "NONE",
                "relics": [],
                "combat_state": {
                    "hand": [{"id": "Burst"}],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "player": {"powers": []},
                    "monsters": []
                }
            }
        });

        let matched = watch::collect_live_watch_match(&config, &root).expect("watch should match");
        assert!(matched.tags.iter().any(|tag| tag == "watch_card:Burst"));
    }

    #[test]
    fn live_watch_all_mode_requires_card_relic_and_command_kind() {
        let config = LiveWatchCaptureConfig {
            cards: vec!["Burst".to_string()],
            relics: vec!["Anchor".to_string()],
            command_kinds: vec!["play".to_string()],
            match_mode: LiveWatchMatchMode::All,
            ..Default::default()
        };
        let matching = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "screen_type": "NONE",
                "relics": [{"id": "Anchor"}],
                "combat_state": {
                    "hand": [{"id": "Burst"}],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "player": {"powers": []},
                    "monsters": []
                }
            }
        });
        let missing_relic = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "screen_type": "NONE",
                "relics": [],
                "combat_state": {
                    "hand": [{"id": "Burst"}],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "player": {"powers": []},
                    "monsters": []
                }
            }
        });

        assert!(watch::collect_live_watch_match(&config, &matching).is_some());
        assert!(watch::collect_live_watch_match(&config, &missing_relic).is_none());
    }

    #[test]
    fn live_watch_all_mode_can_require_monster_and_room_phase() {
        let config = LiveWatchCaptureConfig {
            monsters: vec!["JawWorm".to_string()],
            room_phases: vec!["COMBAT".to_string()],
            match_mode: LiveWatchMatchMode::All,
            ..Default::default()
        };
        let matching = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "room_phase": "COMBAT",
                "screen_type": "NONE",
                "relics": [],
                "combat_state": {
                    "hand": [],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "player": {"powers": []},
                    "monsters": [{"id": "JawWorm", "powers": []}]
                }
            }
        });
        let wrong_phase = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "room_phase": "COMPLETE",
                "screen_type": "NONE",
                "relics": [],
                "combat_state": {
                    "hand": [],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "player": {"powers": []},
                    "monsters": [{"id": "JawWorm", "powers": []}]
                }
            }
        });

        let matched =
            watch::collect_live_watch_match(&config, &matching).expect("watch should match");
        assert!(matched
            .tags
            .iter()
            .any(|tag| tag == "watch_monster:JawWorm"));
        assert!(matched
            .tags
            .iter()
            .any(|tag| tag == "watch_room_phase:COMBAT"));
        assert!(matched
            .assertions
            .iter()
            .any(|a| a.field == "monsters.count[JawWorm]"));
        assert!(watch::collect_live_watch_match(&config, &wrong_phase).is_none());
    }

    #[test]
    fn watch_minimize_suggestion_points_to_fixture_and_min_path() {
        let fixture_path =
            Path::new(r"d:\rust\sts_simulator\tests\live_captures\watch_42_burst.json");
        let min_path = watch::derive_watch_minimized_fixture_path(fixture_path);
        assert_eq!(
            min_path,
            Path::new(r"d:\rust\sts_simulator\tests\live_captures\watch_42_burst.min.json")
        );

        let cmd = watch::build_watch_minimize_suggestion(fixture_path, &min_path);
        assert!(cmd.contains("live_regression.py minimize"));
        assert!(cmd.contains(
            r#"--fixture "d:\rust\sts_simulator\tests\live_captures\watch_42_burst.json""#
        ));
        assert!(cmd.contains(
            r#"--out "d:\rust\sts_simulator\tests\live_captures\watch_42_burst.min.json""#
        ));
    }

    #[test]
    fn watch_capture_dedupe_skips_same_signature_within_cooldown() {
        let root = serde_json::json!({
            "protocol_meta": {"last_command_kind": "play"},
            "game_state": {
                "room_phase": "COMBAT",
                "screen_type": "NONE"
            }
        });
        let matched = watch::LiveWatchMatch {
            tags: vec![
                "watch_card:Burst".to_string(),
                "watch_relic:Anchor".to_string(),
            ],
            assertions: vec![],
            notes: vec![],
        };
        let signature = watch::watch_capture_signature(&root, &matched);
        let mut runtime = watch::LiveWatchRuntime::default();

        assert!(!watch::capture_deduped(&mut runtime, &signature, 100, 3));
        assert!(watch::capture_deduped(&mut runtime, &signature, 102, 3));
        assert!(!watch::capture_deduped(&mut runtime, &signature, 104, 3));
    }

    #[test]
    fn human_card_reward_audit_keeps_pending_for_temporary_inspect_screens() {
        let root = serde_json::json!({
            "protocol_meta": { "response_id": 273 },
            "game_state": {
                "screen_type": "NONE",
                "screen_name": "MASTER_DECK_VIEW",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Hold {
                reason: "temporary_reward_inspect_screen"
            }
        );
    }

    #[test]
    fn human_card_reward_audit_prefers_reward_session_hold_state() {
        let root = serde_json::json!({
            "protocol_meta": {
                "response_id": 276,
                "reward_session": {
                    "session_id": "reward-9",
                    "state": "temporarily_offscreen"
                }
            },
            "game_state": {
                "screen_type": "NONE",
                "screen_name": "",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Hold {
                reason: "reward_session_active"
            }
        );
    }

    #[test]
    fn human_card_reward_audit_prefers_reward_session_closed_state() {
        let root = serde_json::json!({
            "protocol_meta": {
                "response_id": 277,
                "reward_session": {
                    "session_id": "reward-10",
                    "state": "closed_without_choice"
                }
            },
            "game_state": {
                "screen_type": "MASTER_DECK_VIEW",
                "screen_name": "MASTER_DECK_VIEW",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Abandon {
                reason: "reward_session_closed_without_choice"
            }
        );
    }

    #[test]
    fn human_card_reward_audit_prefers_reward_session_absence_over_screen_heuristic() {
        let root = serde_json::json!({
            "protocol_meta": {
                "response_id": 278,
                "capabilities": {
                    "reward_session": true
                }
            },
            "game_state": {
                "screen_type": "MAP",
                "screen_name": "MAP",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Abandon {
                reason: "reward_session_absent"
            }
        );
    }

    #[test]
    fn build_human_card_reward_pending_can_use_reward_session_when_offscreen() {
        let root = serde_json::json!({
            "protocol_meta": {
                "response_id": 301,
                "state_frame_id": 77,
                "reward_session": {
                    "session_id": "reward-11",
                    "state": "temporarily_offscreen",
                    "offered_card_ids": ["Shrug It Off", "Warcry", "Pommel Strike"]
                }
            },
            "game_state": {
                "screen_type": "MAP",
                "screen_name": "MAP",
                "room_phase": "COMPLETE",
                "floor": 12,
                "act": 1,
                "class": "IRONCLAD",
                "current_hp": 40,
                "max_hp": 80,
                "gold": 99,
                "deck": []
            }
        });

        let pending = reward_audit::build_human_card_reward_pending(&root, None)
            .expect("reward_session should seed pending audit");

        assert_eq!(pending.session_id.as_deref(), Some("reward-11"));
        assert_eq!(pending.offered_signature.len(), 3);
        assert!(pending
            .offered_signature
            .iter()
            .all(|sig| sig.ends_with("+session")));
    }

    #[test]
    fn maybe_arm_human_card_reward_audit_uses_reward_session_when_offscreen() {
        let parsed = serde_json::json!({
            "protocol_meta": {
                "response_id": 302,
                "state_frame_id": 88,
                "reward_session": {
                    "session_id": "reward-12",
                    "state": "temporarily_offscreen",
                    "offered_card_ids": ["Shrug It Off", "Warcry", "Pommel Strike"]
                }
            },
            "game_state": {
                "screen_type": "MAP",
                "screen_name": "MAP",
                "room_phase": "COMPLETE",
                "floor": 12,
                "act": 1,
                "class": "IRONCLAD",
                "current_hp": 40,
                "max_hp": 80,
                "gold": 99,
                "deck": []
            }
        });

        let path = std::env::temp_dir().join("reward_session_offscreen_arm_test.txt");
        let mut log = std::fs::File::create(path).unwrap();
        let mut pending = None;

        let armed = crate::cli::live_comm::noncombat::maybe_arm_human_card_reward_audit(
            true,
            &mut pending,
            &parsed,
            None,
            &mut log,
            123,
        );

        assert!(armed);
        assert_eq!(
            pending.as_ref().and_then(|p| p.session_id.as_deref()),
            Some("reward-12")
        );
    }

    #[test]
    fn maybe_arm_human_card_reward_audit_does_not_fallback_to_screen_when_protocol_session_is_absent(
    ) {
        let parsed = serde_json::json!({
            "protocol_meta": {
                "response_id": 303,
                "capabilities": {
                    "reward_session": true
                }
            },
            "game_state": {
                "screen_type": "CARD_REWARD",
                "screen_name": "CARD_REWARD",
                "room_phase": "COMPLETE",
                "floor": 12,
                "act": 1,
                "class": "IRONCLAD",
                "current_hp": 40,
                "max_hp": 80,
                "gold": 99,
                "deck": [],
                "screen_state": {
                    "cards": [
                        {"id": "Shrug It Off", "name": "Shrug It Off", "upgrades": 0},
                        {"id": "Warcry", "name": "Warcry", "upgrades": 0},
                        {"id": "Pommel Strike", "name": "Pommel Strike", "upgrades": 0}
                    ]
                }
            }
        });

        let path = std::env::temp_dir().join("reward_session_no_fallback_arm_test.txt");
        let mut log = std::fs::File::create(path).unwrap();
        let mut pending = None;

        let armed = crate::cli::live_comm::noncombat::maybe_arm_human_card_reward_audit(
            true,
            &mut pending,
            &parsed,
            None,
            &mut log,
            124,
        );

        assert!(!armed);
        assert!(pending.is_none());
    }

    #[test]
    fn human_card_reward_audit_keeps_pending_for_map_inspection() {
        let root = serde_json::json!({
            "protocol_meta": { "response_id": 274 },
            "game_state": {
                "screen_type": "MAP",
                "screen_name": "MAP",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Hold {
                reason: "temporary_reward_inspect_screen"
            }
        );
    }

    #[test]
    fn human_card_reward_audit_abandons_when_reward_context_is_closed() {
        let root = serde_json::json!({
            "protocol_meta": { "response_id": 273 },
            "game_state": {
                "screen_type": "COMBAT_REWARD",
                "screen_name": "COMBAT_REWARD",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Abandon {
                reason: "reward_context_closed_without_human_choice"
            }
        );

        let mut payload = Map::new();
        payload.insert("seed".to_string(), serde_json::json!(123));
        let pending = reward_audit::PendingHumanCardRewardAudit {
            session_id: None,
            state_frame_id: Some(10),
            offered_signature: vec!["Reaper+0".to_string(), "Barricade+0".to_string()],
            payload,
            bot_recommended_choice: Some(0),
            replay_truth: None,
            replay_engine_state: None,
            offscreen_hold_polls: 0,
            last_hold_context: None,
        };

        let reward_path = std::env::temp_dir().join("card_reward_audit_test.jsonl");
        let log_path = std::env::temp_dir().join("card_reward_log_test.txt");
        let mut reward_audit_file = std::fs::File::create(&reward_path).unwrap();
        let mut log_file = std::fs::File::create(&log_path).unwrap();
        reward_audit::finalize_human_card_reward_audit_without_choice(
            pending,
            &root,
            &mut reward_audit_file,
            &mut log_file,
            "reward_context_closed_without_human_choice",
        );

        let written = std::fs::read_to_string(&reward_path).unwrap();
        assert!(written.contains("\"audit_status\":\"incomplete\""));
        assert!(written.contains("\"audit_reason\":\"reward_context_closed_without_human_choice\""));
        assert!(written.contains("\"audit_reason_source\":\"legacy_fallback\""));
        assert!(written.contains("\"human_choice\":null"));
    }

    #[test]
    fn human_card_reward_reason_source_marks_protocol_truth_reasons() {
        assert_eq!(
            reward_audit::human_card_reward_audit_reason_source("reward_session_absent"),
            "protocol_truth"
        );
        assert_eq!(
            reward_audit::human_card_reward_audit_reason_source(
                "reward_session_closed_without_choice"
            ),
            "protocol_truth"
        );
    }

    #[test]
    fn human_card_reward_reason_source_marks_legacy_fallback_reasons() {
        assert_eq!(
            reward_audit::human_card_reward_audit_reason_source("temporary_reward_inspect_screen"),
            "legacy_fallback"
        );
        assert_eq!(
            reward_audit::human_card_reward_audit_reason_source("screen_left_without_human_choice"),
            "legacy_fallback"
        );
    }

    #[test]
    fn human_card_reward_audit_abandons_after_plain_complete_transition() {
        let root = serde_json::json!({
            "protocol_meta": { "response_id": 275 },
            "game_state": {
                "screen_type": "NONE",
                "screen_name": "",
                "room_phase": "COMPLETE"
            }
        });
        assert_eq!(
            reward_audit::classify_human_card_reward_audit_disposition(&root),
            reward_audit::HumanCardRewardAuditDisposition::Abandon {
                reason: "screen_left_without_human_choice"
            }
        );
    }

    #[test]
    fn reward_choice_session_match_uses_explicit_session_when_present() {
        let pending = reward_audit::PendingHumanCardRewardAudit {
            session_id: Some("reward-42".to_string()),
            state_frame_id: Some(10),
            offered_signature: vec![],
            payload: Map::new(),
            bot_recommended_choice: Some(0),
            replay_truth: None,
            replay_engine_state: None,
            offscreen_hold_polls: 0,
            last_hold_context: None,
        };

        let matching = serde_json::json!({
            "session_id": "reward-42",
            "choice_kind": "card",
            "choice_index": 0
        });
        let mismatched = serde_json::json!({
            "session_id": "reward-99",
            "choice_kind": "card",
            "choice_index": 0
        });
        let legacy = serde_json::json!({
            "choice_kind": "card",
            "choice_index": 0
        });

        assert!(reward_audit::reward_choice_matches_pending_session(
            &pending, &matching
        ));
        assert!(!reward_audit::reward_choice_matches_pending_session(
            &pending,
            &mismatched
        ));
        assert!(reward_audit::reward_choice_matches_pending_session(
            &pending, &legacy
        ));
    }

    #[test]
    fn default_watch_context_assertions_include_player_monsters_and_piles() {
        let root = serde_json::json!({
            "game_state": {
                "combat_state": {
                    "player": {"current_hp": 70, "block": 4, "energy": 2},
                    "monsters": [
                        {"current_hp": 40, "block": 3},
                        {"current_hp": 12, "block": 0}
                    ],
                    "hand": [{"id": "Strike_G"}],
                    "draw_pile": [{"id": "Defend_G"}, {"id": "Defend_G"}],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": []
                }
            }
        });

        let assertions = watch::default_watch_context_assertions(&root);
        let fields = assertions
            .iter()
            .map(|a| a.field.as_str())
            .collect::<Vec<_>>();
        assert!(fields.contains(&"player.hp"));
        assert!(fields.contains(&"player.block"));
        assert!(fields.contains(&"player.energy"));
        assert!(fields.contains(&"monster_count"));
        assert!(fields.contains(&"monster[0].hp"));
        assert!(fields.contains(&"monster[1].block"));
        assert!(fields.contains(&"hand_size"));
        assert!(fields.contains(&"draw_pile_size"));
    }

    #[test]
    fn build_noncombat_watch_sidecar_keeps_tags_and_screen_payload() {
        let root = serde_json::json!({
            "protocol_meta": {"response_id": 33, "last_command_kind": "choose"},
            "available_commands": ["choose", "proceed"],
            "game_state": {
                "screen_type": "EVENT",
                "class": "IRONCLAD",
                "act": 1,
                "floor": 6,
                "current_hp": 55,
                "max_hp": 80,
                "gold": 120,
                "deck": [
                    {"id": "Strike_R", "type": "ATTACK", "upgrades": 0},
                    {"id": "Barricade", "type": "POWER", "upgrades": 0}
                ],
                "relics": [{"id": "Burning Blood"}],
                "potions": [{"id": "FearPotion"}, {"id": "Potion Slot"}],
                "screen_state": {"event_id": "Golden Shrine"}
            }
        });
        let matched = watch::LiveWatchMatch {
            tags: vec!["watch_screen:EVENT".to_string()],
            assertions: Vec::new(),
            notes: vec!["screen=EVENT".to_string()],
        };
        let sidecar = watch::build_noncombat_watch_sidecar(
            33,
            44,
            &root,
            &matched,
            std::path::Path::new("D:/tmp/watch_noncombat_33.json"),
        );

        assert_eq!(sidecar["kind"], "live_watch_noncombat");
        assert_eq!(sidecar["response_id"], 33);
        assert_eq!(sidecar["frame"], 44);
        assert_eq!(sidecar["tags"][0], "watch_screen:EVENT");
        assert_eq!(sidecar["context_summary"]["screen_type"], "EVENT");
        assert_eq!(sidecar["context_summary"]["deck_summary"]["size"], 2);
        assert_eq!(sidecar["context_summary"]["relic_ids"][0], "Burning Blood");
        assert_eq!(sidecar["context_summary"]["potion_ids"][0], "FearPotion");
    }

    #[test]
    fn noncombat_screen_summary_for_shop_screen_extracts_prices() {
        let root = serde_json::json!({
            "game_state": {
                "screen_type": "SHOP_SCREEN",
                "screen_state": {
                    "purge_available": true,
                    "purge_cost": 75,
                    "cards": [
                        {"id": "Backflip", "name": "Backflip", "upgrades": 0, "price": 54}
                    ],
                    "relics": [
                        {"id": "Anchor", "name": "Anchor", "price": 300}
                    ],
                    "potions": [
                        {"id": "Dexterity Potion", "name": "Dexterity Potion", "price": 66}
                    ]
                }
            }
        });

        let summary = watch::build_noncombat_screen_summary(&root);
        assert_eq!(summary["screen"], "SHOP_SCREEN");
        assert_eq!(summary["purge_available"], true);
        assert_eq!(summary["purge_cost"], 75);
        assert_eq!(summary["cards"][0]["id"], "Backflip");
        assert_eq!(summary["relics"][0]["price"], 300);
        assert_eq!(summary["potions"][0]["name"], "Dexterity Potion");
    }

    #[test]
    fn noncombat_screen_summary_for_card_reward_and_combat_reward_extracts_entries() {
        let card_reward = serde_json::json!({
            "game_state": {
                "screen_type": "CARD_REWARD",
                "screen_state": {
                    "skip_available": true,
                    "cards": [
                        {"id": "Burst", "name": "Burst", "upgrades": 0},
                        {"id": "Backflip", "name": "Backflip", "upgrades": 1}
                    ]
                }
            }
        });
        let combat_reward = serde_json::json!({
            "game_state": {
                "screen_type": "COMBAT_REWARD",
                "screen_state": {
                    "rewards": [
                        {"reward_type": "GOLD", "gold": 25},
                        {"reward_type": "RELIC", "relic": {"id": "Anchor"}},
                        {"reward_type": "CARD", "cards": [{"id": "Burst", "name": "Burst", "upgrades": 0}]}
                    ]
                }
            }
        });

        let card_summary = watch::build_noncombat_screen_summary(&card_reward);
        let reward_summary = watch::build_noncombat_screen_summary(&combat_reward);

        assert_eq!(card_summary["screen"], "CARD_REWARD");
        assert_eq!(card_summary["cards"][1]["upgrades"], 1);
        assert_eq!(reward_summary["screen"], "COMBAT_REWARD");
        assert_eq!(reward_summary["rewards"][0]["gold"], 25);
        assert_eq!(reward_summary["rewards"][1]["id"], "Anchor");
        assert_eq!(reward_summary["rewards"][2]["cards"][0]["id"], "Burst");
    }

    #[test]
    fn live_bloodletting_prediction_keeps_next_turn_block_from_self_forming_clay() {
        let pre = serde_json::json!({
            "room_type": "MonsterRoomBoss",
            "potions": [
                {"id": "LiquidBronze"},
                {"id": "Strength Potion"},
                {"id": "FearPotion"}
            ],
            "relics": [
                {"id": "Burning Blood", "counter": -1},
                {"id": "Kunai", "counter": 0},
                {"id": "Self Forming Clay", "counter": -1}
            ],
            "combat_state": {
                "turn": 7,
                "player": {
                    "current_hp": 25,
                    "max_hp": 80,
                    "block": 0,
                    "energy": 3,
                    "powers": [
                        {"id": "Dexterity", "amount": 1}
                    ]
                },
                "monsters": [
                    {
                        "id": "Hexaghost",
                        "current_hp": 134,
                        "max_hp": 250,
                        "block": 12,
                        "intent": "ATTACK",
                        "move_base_damage": 5,
                        "move_adjusted_damage": 7,
                        "move_hits": 2,
                        "move_id": 2,
                        "powers": [
                            {"id": "Strength", "amount": 2}
                        ]
                    }
                ],
                "hand": [
                    {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
                    {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
                    {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0},
                    {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
                    {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
                ],
                "draw_pile": [],
                "discard_pile": [],
                "exhaust_pile": [
                    {"id": "Defend_R", "uuid": "x1", "upgrades": 0, "cost": 1}
                ],
                "limbo": []
            }
        });

        let post = serde_json::json!({
            "player": {
                "current_hp": 22,
                "max_hp": 80,
                "block": 0,
                "energy": 5,
                "powers": [
                    {"id": "Dexterity", "amount": 1},
                    {"id": "Next Turn Block", "amount": 3}
                ]
            },
            "monsters": [
                {
                    "id": "Hexaghost",
                    "current_hp": 134,
                    "max_hp": 250,
                    "block": 12,
                    "intent": "ATTACK",
                    "move_base_damage": 5,
                    "move_adjusted_damage": 7,
                    "move_hits": 2,
                    "move_id": 2,
                    "powers": [
                        {"id": "Strength", "amount": 2}
                    ]
                }
            ],
            "hand": [
                {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
                {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
                {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
                {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
            ],
            "draw_pile": [],
            "discard_pile": [
                {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0}
            ],
            "exhaust_pile": [
                {"id": "Defend_R", "uuid": "x1", "upgrades": 0, "cost": 1}
            ],
            "limbo": []
        });

        let combat_snapshot = combat::build_live_combat_snapshot(&pre);
        let mut truth =
            crate::diff::state_sync::build_combat_state(&combat_snapshot, &pre["relics"]);
        let mut engine_state = EngineState::CombatPlayerTurn;
        let alive = crate::engine::core::tick_until_stable_turn(
            &mut engine_state,
            &mut truth,
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
        );
        assert!(alive);

        let diffs = compare_states(
            &truth,
            &post,
            false,
            &ActionContext {
                last_command: "PLAY 3".to_string(),
                was_end_turn: false,
                monster_intents: vec!["Attack { damage: 5, hits: 2 }".to_string()],
                monster_names: vec!["Hexaghost".to_string()],
                has_rng_state: false,
            },
        );

        assert!(
            !diffs
                .iter()
                .any(|d| d.field == "player.power[Next Turn Block]"),
            "unexpected diffs: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }
}
