use crate::diff::replay::live_comm_replay::{
    build_live_session_replay_from_raw_path, derive_combat_replay_view, verify_combat_replay_view,
    write_live_session_replay_to_path,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const LOG_ROOT: &str = r"d:\rust\sts_simulator\logs";
pub const CURRENT_ROOT: &str = r"d:\rust\sts_simulator\logs\current";
pub const RUNS_ROOT: &str = r"d:\rust\sts_simulator\logs\runs";
pub const CURRENT_MANIFEST_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_manifest.json";

const PROFILE_PATH: &str = r"d:\rust\sts_simulator\tools\live_comm\profile.json";
const MAX_CLEAN_CANONICAL_RUNS: usize = 20;
const MAX_CLEAN_DEBUG_RUNS: usize = 10;
const MAX_WATCH_SIDECAR_RUNS: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveProfileMetadata {
    pub profile_name: Option<String>,
    pub purpose: Option<String>,
    pub capture_policy: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRunCounts {
    pub engine_bugs: usize,
    pub content_gaps: usize,
    pub timing_diffs: usize,
    pub replay_failures: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveArtifactRecord {
    pub relative_path: String,
    pub present: bool,
}

impl LiveArtifactRecord {
    fn new(relative_path: impl Into<String>, present: bool) -> Self {
        Self {
            relative_path: relative_path.into(),
            present,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LiveRunArtifacts {
    pub raw: Option<LiveArtifactRecord>,
    pub focus: Option<LiveArtifactRecord>,
    pub signatures: Option<LiveArtifactRecord>,
    pub combat_suspects: Option<LiveArtifactRecord>,
    pub failure_snapshots: Option<LiveArtifactRecord>,
    pub debug: Option<LiveArtifactRecord>,
    pub replay: Option<LiveArtifactRecord>,
    pub reward_audit: Option<LiveArtifactRecord>,
    pub event_audit: Option<LiveArtifactRecord>,
    pub sidecar_shadow: Option<LiveArtifactRecord>,
    pub validation: Option<LiveArtifactRecord>,
    pub watch_audit: Option<LiveArtifactRecord>,
    pub watch_noncombat: Option<LiveArtifactRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRunProvenance {
    pub exe_path: Option<String>,
    pub exe_mtime_utc: Option<String>,
    pub git_short_sha: Option<String>,
    pub build_unix: Option<u64>,
    pub build_time_utc: Option<String>,
    pub profile_path: Option<String>,
    pub profile_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRunValidation {
    pub status: String,
    pub event_frames_present: bool,
    pub focus_has_event_trace: bool,
    pub debug_has_event_policy: bool,
    pub event_audit_present: bool,
    pub event_audit_json_lines: usize,
    pub manifest_lists_event_audit: bool,
    pub reward_loop_detected: bool,
    pub bootstrap_protocol_ok: bool,
    pub event_screen_fields_present: bool,
    pub event_screen_nonzero_or_keyed_for_multistage_events: bool,
    pub trace_incomplete: bool,
    pub latest_failure_snapshot_frame: Option<u64>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRetentionFlags {
    pub pinned: bool,
    pub cache_only: bool,
    pub retain_debug: bool,
    pub retain_replay: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveRunManifest {
    pub run_id: String,
    pub timestamp: String,
    pub build_tag: String,
    pub parity_mode: String,
    pub watch_enabled: bool,
    pub session_exit_reason: String,
    pub classification_label: String,
    pub profile: LiveProfileMetadata,
    #[serde(default)]
    pub provenance: LiveRunProvenance,
    pub counts: LiveRunCounts,
    pub artifacts: LiveRunArtifacts,
    #[serde(default)]
    pub validation: Option<LiveRunValidation>,
    pub retention: LiveRetentionFlags,
}

#[derive(Clone, Debug, Default)]
pub struct FinalizeRunInput {
    pub run_id: String,
    pub timestamp: String,
    pub build_tag: String,
    pub parity_mode: String,
    pub watch_enabled: bool,
    pub session_exit_reason: String,
    pub engine_bug_total: usize,
    pub content_gap_total: usize,
    pub timing_diff_total: usize,
    pub replay_failures: usize,
    pub game_over_seen: bool,
    pub final_victory: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FinalizeRunOutcome {
    pub run_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub classification_label: String,
    pub validation_status: String,
    pub gc_summary: String,
}

#[derive(Clone, Debug, Default)]
pub struct LiveLogsStatus {
    pub total_runs: usize,
    pub pinned_runs: usize,
    pub clean_runs: usize,
    pub tainted_runs: usize,
    pub labels: BTreeMap<String, usize>,
    pub latest_run_id: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct GcSummary {
    pub pruned_run_artifacts: usize,
    pub pruned_debug: usize,
    pub pruned_replay: usize,
    pub pruned_watch: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LiveLogPaths {
    pub root: PathBuf,
    pub current: PathBuf,
    pub runs: PathBuf,
}

impl LiveLogPaths {
    pub fn default_paths() -> Self {
        Self {
            root: PathBuf::from(LOG_ROOT),
            current: PathBuf::from(CURRENT_ROOT),
            runs: PathBuf::from(RUNS_ROOT),
        }
    }

    pub fn current_raw(&self) -> PathBuf {
        self.current.join("live_comm_raw.jsonl")
    }

    pub fn current_debug(&self) -> PathBuf {
        self.current.join("live_comm_debug.txt")
    }

    pub fn current_focus(&self) -> PathBuf {
        self.current.join("live_comm_focus.txt")
    }

    pub fn current_signatures(&self) -> PathBuf {
        self.current.join("live_comm_signatures.jsonl")
    }

    pub fn current_replay(&self) -> PathBuf {
        self.current.join("live_comm_replay.json")
    }

    pub fn current_reward_audit(&self) -> PathBuf {
        self.current.join("live_comm_reward_audit.jsonl")
    }

    pub fn current_event_audit(&self) -> PathBuf {
        self.current.join("live_comm_event_audit.jsonl")
    }

    pub fn current_sidecar_shadow(&self) -> PathBuf {
        self.current.join("live_comm_sidecar_shadow.jsonl")
    }

    pub fn current_validation(&self) -> PathBuf {
        self.current.join("live_comm_validation.json")
    }

    pub fn current_watch_audit(&self) -> PathBuf {
        self.current.join("live_comm_watch_audit.jsonl")
    }

    pub fn current_watch_noncombat(&self) -> PathBuf {
        self.current.join("live_comm_watch_noncombat.jsonl")
    }

    pub fn current_combat_suspects(&self) -> PathBuf {
        self.current.join("live_comm_combat_suspects.jsonl")
    }

    pub fn current_failure_snapshots(&self) -> PathBuf {
        self.current.join("live_comm_failure_snapshots.jsonl")
    }

    pub fn current_manifest(&self) -> PathBuf {
        PathBuf::from(CURRENT_MANIFEST_PATH)
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs.join(run_id)
    }
}

pub fn timestamp_string() -> String {
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "Get-Date -Format yyyyMMdd_HHmmss"])
        .output();
    match out {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown_time".to_string(),
    }
}

pub fn ensure_log_dirs(paths: &LiveLogPaths) -> std::io::Result<()> {
    std::fs::create_dir_all(&paths.root)?;
    std::fs::create_dir_all(&paths.current)?;
    std::fs::create_dir_all(&paths.runs)?;
    Ok(())
}

pub fn load_profile_metadata() -> LiveProfileMetadata {
    let Ok(text) = std::fs::read_to_string(PROFILE_PATH) else {
        return LiveProfileMetadata::default();
    };
    let Ok(root) = serde_json::from_str::<Value>(&text) else {
        return LiveProfileMetadata::default();
    };
    LiveProfileMetadata {
        profile_name: root
            .get("activated_profile")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        purpose: root
            .get("purpose")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        capture_policy: root
            .get("capture_policy")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn current_exe_path_string() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
}

fn current_exe_mtime_fallback() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("unix:{secs}"))
}

pub fn runtime_provenance() -> LiveRunProvenance {
    let profile = load_profile_metadata();
    let git_short = option_env!("LIVE_COMM_GIT_SHORT")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env_opt("LIVE_COMM_LAUNCH_GIT_SHORT"));
    let build_unix = option_env!("LIVE_COMM_BUILD_UNIX").and_then(|s| s.parse::<u64>().ok());

    LiveRunProvenance {
        exe_path: env_opt("LIVE_COMM_LAUNCH_EXE_PATH").or_else(current_exe_path_string),
        exe_mtime_utc: env_opt("LIVE_COMM_LAUNCH_EXE_MTIME_UTC")
            .or_else(current_exe_mtime_fallback),
        git_short_sha: git_short,
        build_unix,
        build_time_utc: build_unix.map(|secs| format!("unix:{secs}")),
        profile_path: env_opt("LIVE_COMM_LAUNCH_PROFILE_PATH")
            .or_else(|| Some(PROFILE_PATH.to_string())),
        profile_name: env_opt("LIVE_COMM_LAUNCH_PROFILE_NAME").or(profile.profile_name),
    }
}

fn file_contains(path: &Path, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|text| text.contains(needle))
        .unwrap_or(false)
}

fn count_valid_jsonl_records(path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
    let mut count = 0usize;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(trimmed).map_err(|err| {
            format!(
                "invalid jsonl in '{}' at line {}: {err}",
                path.display(),
                idx + 1
            )
        })?;
        count += 1;
    }
    Ok(count)
}

fn raw_contains_event_frames(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let screen_type = root
            .get("game_state")
            .and_then(|state| {
                state
                    .get("screen_type")
                    .or_else(|| state.get("screen_name"))
            })
            .and_then(Value::as_str);
        if screen_type.is_some_and(|screen| screen.eq_ignore_ascii_case("EVENT")) {
            return true;
        }
    }
    false
}

fn debug_bootstrap_protocol_ok(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    !text.contains("Invalid command: ready")
        && !text.contains("Invalid command: __LIVE_COMM_BOOTSTRAP__")
}

fn detect_reward_loop(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let mut signatures: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(game_state) = root.get("game_state") else {
            continue;
        };
        let screen = game_state
            .get("screen_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let signature = match screen {
            "COMBAT_REWARD" => {
                let rewards = game_state
                    .get("screen_state")
                    .and_then(|v| v.get("rewards"))
                    .and_then(Value::as_array);
                let active = rewards
                    .into_iter()
                    .flatten()
                    .map(|reward| {
                        let reward_type = reward
                            .get("reward_type")
                            .and_then(Value::as_str)
                            .unwrap_or("UNKNOWN");
                        let choice_index = reward
                            .get("choice_index")
                            .and_then(Value::as_u64)
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        format!("{reward_type}@{choice_index}")
                    })
                    .collect::<Vec<_>>();
                format!("COMBAT_REWARD:{}", active.join(","))
            }
            "CARD_REWARD" => {
                let cards = game_state
                    .get("screen_state")
                    .and_then(|v| v.get("cards"))
                    .and_then(Value::as_array);
                let offered = cards
                    .into_iter()
                    .flatten()
                    .filter_map(|card| {
                        card.get("id")
                            .and_then(Value::as_str)
                            .or_else(|| card.get("name").and_then(Value::as_str))
                    })
                    .collect::<Vec<_>>();
                format!("CARD_REWARD:{}", offered.join(","))
            }
            _ => {
                signatures.clear();
                continue;
            }
        };
        signatures.push(signature);
        if signatures.len() >= 6 {
            let n = signatures.len();
            if signatures[n - 1] == signatures[n - 3]
                && signatures[n - 2] == signatures[n - 4]
                && signatures[n - 3] == signatures[n - 5]
                && signatures[n - 4] == signatures[n - 6]
                && signatures[n - 1] != signatures[n - 2]
            {
                return true;
            }
        }
        if signatures.len() > 8 {
            signatures.drain(0..signatures.len() - 8);
        }
        if signatures.len() >= 4
            && signatures[signatures.len() - 1] == signatures[signatures.len() - 3]
            && signatures[signatures.len() - 2] == signatures[signatures.len() - 4]
            && signatures[signatures.len() - 1] != signatures[signatures.len() - 2]
        {
            return true;
        }
    }
    false
}

fn event_screen_validation(path: &Path) -> (bool, bool) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (false, false);
    };
    let mut fields_present = true;
    let mut multistage_ok = true;
    let mut multistage_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut multistage_has_progress: BTreeMap<String, bool> = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            fields_present = false;
            continue;
        };
        let Some(decision) = root.get("decision") else {
            fields_present = false;
            continue;
        };
        let has_index = decision.get("screen_index").is_some();
        let has_key = decision.get("screen_key").is_some_and(|v| !v.is_null());
        let has_source = decision.get("screen_source").is_some_and(|v| !v.is_null());
        if !has_index || !has_source {
            fields_present = false;
        }
        let event_name = decision
            .get("event_name")
            .and_then(Value::as_str)
            .unwrap_or("");
        if matches!(
            event_name,
            "Neow" | "Shining Light" | "Golden Idol" | "Knowing Skull" | "Living Wall" | "Big Fish"
        ) {
            *multistage_counts.entry(event_name.to_string()).or_default() += 1;
            let progressed = decision
                .get("screen_index")
                .and_then(Value::as_u64)
                .is_some_and(|v| v > 0)
                || has_key;
            let entry = multistage_has_progress
                .entry(event_name.to_string())
                .or_insert(false);
            *entry |= progressed;
        }
    }
    for (event_name, count) in multistage_counts {
        if count > 1
            && !multistage_has_progress
                .get(&event_name)
                .copied()
                .unwrap_or(false)
        {
            multistage_ok = false;
        }
    }
    (fields_present, multistage_ok)
}

fn last_failure_snapshot_frame(path: &Path) -> Option<u64> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return None;
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .filter_map(|root| root.get("frame").and_then(Value::as_u64))
        .last()
}

fn validate_run_artifacts(run_dir: &Path, manifest: &LiveRunManifest) -> LiveRunValidation {
    let focus_path = run_dir.join("focus.txt");
    let debug_path = run_dir.join("debug.txt");
    let raw_path = run_dir.join("raw.jsonl");
    let event_audit_path = run_dir.join("event_audit.jsonl");
    let failure_snapshots_path = run_dir.join("failure_snapshots.jsonl");

    let focus_has_event_trace = file_contains(&focus_path, "[EVENT]");
    let debug_has_event_policy = file_contains(&debug_path, "EVENT POLICY");
    let event_frames_present = raw_contains_event_frames(&raw_path);
    let event_audit_present = event_audit_path.exists();
    let event_audit_json_lines = count_valid_jsonl_records(&event_audit_path).unwrap_or(0);
    let reward_loop_detected = detect_reward_loop(&raw_path);
    let bootstrap_protocol_ok = debug_bootstrap_protocol_ok(&debug_path);
    let (event_screen_fields_present, event_screen_nonzero_or_keyed_for_multistage_events) =
        event_screen_validation(&event_audit_path);
    let manifest_lists_event_audit = manifest
        .artifacts
        .event_audit
        .as_ref()
        .is_some_and(|artifact| artifact.relative_path == "event_audit.jsonl");

    let trace_incomplete = event_frames_present
        && (!focus_has_event_trace
            || !debug_has_event_policy
            || !event_audit_present
            || event_audit_json_lines == 0
            || reward_loop_detected
            || !bootstrap_protocol_ok
            || !event_screen_fields_present
            || !event_screen_nonzero_or_keyed_for_multistage_events
            || !manifest_lists_event_audit);

    let mut errors = Vec::new();
    if event_frames_present && !focus_has_event_trace {
        errors.push("event frames present but focus.txt has no [EVENT] trace".to_string());
    }
    if event_frames_present && !debug_has_event_policy {
        errors.push("event frames present but debug.txt has no EVENT POLICY trace".to_string());
    }
    if !manifest_lists_event_audit {
        errors.push("manifest is missing event_audit artifact record".to_string());
    }
    if event_frames_present && !event_audit_present {
        errors.push("event frames present but event_audit.jsonl is missing".to_string());
    }
    if event_frames_present && event_audit_present && event_audit_json_lines == 0 {
        errors.push("event frames present but event_audit.jsonl has no valid records".to_string());
    }
    if reward_loop_detected {
        errors.push("reward loop detected in raw protocol stream".to_string());
    }
    if !bootstrap_protocol_ok {
        errors.push("bootstrap protocol leaked into normal command stream".to_string());
    }
    if event_frames_present && !event_screen_fields_present {
        errors.push(
            "event trace present but screen_index/screen_source fields are missing".to_string(),
        );
    }
    if event_frames_present && !event_screen_nonzero_or_keyed_for_multistage_events {
        errors.push("multistage events never reported a keyed or advanced screen".to_string());
    }

    let status = if trace_incomplete || reward_loop_detected || !bootstrap_protocol_ok {
        "trace_incomplete"
    } else if event_frames_present {
        "ok"
    } else {
        "ok_no_events"
    };

    LiveRunValidation {
        status: status.to_string(),
        event_frames_present,
        focus_has_event_trace,
        debug_has_event_policy,
        event_audit_present,
        event_audit_json_lines,
        manifest_lists_event_audit,
        reward_loop_detected,
        bootstrap_protocol_ok,
        event_screen_fields_present,
        event_screen_nonzero_or_keyed_for_multistage_events,
        trace_incomplete,
        latest_failure_snapshot_frame: last_failure_snapshot_frame(&failure_snapshots_path),
        errors,
    }
}

pub fn finalize_live_run(
    paths: &LiveLogPaths,
    input: FinalizeRunInput,
) -> Result<FinalizeRunOutcome, String> {
    ensure_log_dirs(paths).map_err(|err| format!("failed to ensure log dirs: {err}"))?;
    let run_dir = paths.run_dir(&input.run_id);
    std::fs::create_dir_all(&run_dir)
        .map_err(|err| format!("failed to create run dir '{}': {err}", run_dir.display()))?;

    let classification_label = classify_run(&input);
    let is_clean = is_clean_label(&classification_label);
    let retain_replay = matches!(
        classification_label.as_str(),
        "strict_fail" | "survey_tainted" | "loss_tainted" | "victory_tainted"
    );
    let raw_present = copy_if_exists(&paths.current_raw(), &run_dir.join("raw.jsonl"))?;
    let focus_present = copy_if_exists(&paths.current_focus(), &run_dir.join("focus.txt"))?;
    let signatures_present = copy_if_exists(
        &paths.current_signatures(),
        &run_dir.join("signatures.jsonl"),
    )?;
    let combat_suspects_present = copy_if_nonempty(
        &paths.current_combat_suspects(),
        &run_dir.join("combat_suspects.jsonl"),
    )?;
    let failure_snapshots_present = copy_if_exists(
        &paths.current_failure_snapshots(),
        &run_dir.join("failure_snapshots.jsonl"),
    )?;
    let debug_present = copy_if_exists(&paths.current_debug(), &run_dir.join("debug.txt"))?;
    let reward_audit_present = copy_if_nonempty(
        &paths.current_reward_audit(),
        &run_dir.join("reward_audit.jsonl"),
    )?;
    let event_audit_present = copy_if_exists(
        &paths.current_event_audit(),
        &run_dir.join("event_audit.jsonl"),
    )?;
    let sidecar_shadow_present = copy_if_nonempty(
        &paths.current_sidecar_shadow(),
        &run_dir.join("sidecar_shadow.jsonl"),
    )?;
    let watch_audit_present = if input.watch_enabled {
        copy_if_nonempty(
            &paths.current_watch_audit(),
            &run_dir.join("watch_audit.jsonl"),
        )?
    } else {
        false
    };
    let watch_noncombat_present = if input.watch_enabled {
        copy_if_nonempty(
            &paths.current_watch_noncombat(),
            &run_dir.join("watch_noncombat.jsonl"),
        )?
    } else {
        false
    };
    let replay_present = if retain_replay {
        copy_if_exists(&paths.current_replay(), &run_dir.join("replay.json"))?
    } else {
        false
    };

    let manifest = LiveRunManifest {
        run_id: input.run_id.clone(),
        timestamp: input.timestamp.clone(),
        build_tag: input.build_tag,
        parity_mode: input.parity_mode,
        watch_enabled: input.watch_enabled,
        session_exit_reason: input.session_exit_reason,
        classification_label: classification_label.clone(),
        profile: load_profile_metadata(),
        provenance: runtime_provenance(),
        counts: LiveRunCounts {
            engine_bugs: input.engine_bug_total,
            content_gaps: input.content_gap_total,
            timing_diffs: input.timing_diff_total,
            replay_failures: input.replay_failures,
        },
        artifacts: LiveRunArtifacts {
            raw: Some(LiveArtifactRecord::new("raw.jsonl", raw_present)),
            focus: Some(LiveArtifactRecord::new("focus.txt", focus_present)),
            signatures: Some(LiveArtifactRecord::new(
                "signatures.jsonl",
                signatures_present,
            )),
            combat_suspects: Some(LiveArtifactRecord::new(
                "combat_suspects.jsonl",
                combat_suspects_present,
            )),
            failure_snapshots: Some(LiveArtifactRecord::new(
                "failure_snapshots.jsonl",
                failure_snapshots_present,
            )),
            debug: Some(LiveArtifactRecord::new("debug.txt", debug_present)),
            replay: Some(LiveArtifactRecord::new("replay.json", replay_present)),
            reward_audit: Some(LiveArtifactRecord::new(
                "reward_audit.jsonl",
                reward_audit_present,
            )),
            event_audit: Some(LiveArtifactRecord::new(
                "event_audit.jsonl",
                event_audit_present,
            )),
            sidecar_shadow: Some(LiveArtifactRecord::new(
                "sidecar_shadow.jsonl",
                sidecar_shadow_present,
            )),
            validation: Some(LiveArtifactRecord::new("validation.json", false)),
            watch_audit: Some(LiveArtifactRecord::new(
                "watch_audit.jsonl",
                watch_audit_present,
            )),
            watch_noncombat: Some(LiveArtifactRecord::new(
                "watch_noncombat.jsonl",
                watch_noncombat_present,
            )),
        },
        retention: LiveRetentionFlags {
            pinned: false,
            cache_only: is_clean,
            retain_debug: debug_present,
            retain_replay,
        },
        validation: None,
    };

    let mut manifest = manifest;
    let manifest_path = write_manifest(&run_dir.join("manifest.json"), &manifest)?;
    let validation = validate_run_artifacts(&run_dir, &manifest);
    let validation_path = run_dir.join("validation.json");
    let validation_text = serde_json::to_string_pretty(&validation)
        .map_err(|err| format!("failed to serialize validation: {err}"))?;
    std::fs::write(&validation_path, validation_text).map_err(|err| {
        format!(
            "failed to write validation '{}': {err}",
            validation_path.display()
        )
    })?;
    let _ = std::fs::copy(&validation_path, paths.current_validation());
    manifest.validation = Some(validation.clone());
    manifest.artifacts.validation = Some(LiveArtifactRecord::new("validation.json", true));
    rewrite_manifest(&manifest_path, &manifest)?;
    let _ = write_manifest(&paths.current_manifest(), &manifest);
    let gc_summary = gc_runs(paths)?;

    Ok(FinalizeRunOutcome {
        run_dir,
        manifest_path,
        classification_label,
        validation_status: validation.status,
        gc_summary: format!(
            "pruned_runs={} pruned_debug={} pruned_replay={} pruned_watch={}",
            gc_summary.pruned_run_artifacts,
            gc_summary.pruned_debug,
            gc_summary.pruned_replay,
            gc_summary.pruned_watch
        ),
    })
}

pub fn gc_runs(paths: &LiveLogPaths) -> Result<GcSummary, String> {
    ensure_log_dirs(paths).map_err(|err| format!("failed to ensure log dirs: {err}"))?;
    let mut entries = list_run_manifests(paths)?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));

    let mut summary = GcSummary::default();
    let clean_indices = entries
        .iter()
        .enumerate()
        .filter(|(_, (_, manifest))| is_clean_label(&manifest.classification_label))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut watch_ranks = BTreeMap::new();
    let mut watch_rank = 0usize;
    for (idx, (_, manifest)) in entries.iter().enumerate() {
        if manifest.watch_enabled {
            watch_ranks.insert(idx, watch_rank);
            watch_rank += 1;
        }
    }

    for &idx in clean_indices.iter().skip(MAX_CLEAN_DEBUG_RUNS) {
        let (manifest_path, manifest) = &mut entries[idx];
        if manifest.retention.pinned {
            continue;
        }
        if artifact_present(&manifest.artifacts.debug) {
            remove_run_artifact(manifest_path, &mut manifest.artifacts.debug);
            manifest.retention.retain_debug = false;
            summary.pruned_debug += 1;
            rewrite_manifest(manifest_path, manifest)?;
        }
    }

    for (rank, &idx) in clean_indices.iter().enumerate() {
        let (manifest_path, manifest) = &mut entries[idx];
        if manifest.retention.pinned {
            continue;
        }

        if artifact_present(&manifest.artifacts.replay) {
            remove_run_artifact(manifest_path, &mut manifest.artifacts.replay);
            manifest.retention.retain_replay = false;
            summary.pruned_replay += 1;
        }
        if rank >= MAX_CLEAN_CANONICAL_RUNS {
            let mut changed = false;
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.raw);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.focus);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.signatures);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.combat_suspects);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.reward_audit);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.debug);
            changed |= remove_run_artifact(manifest_path, &mut manifest.artifacts.replay);
            if changed {
                summary.pruned_run_artifacts += 1;
            }
        }
        rewrite_manifest(manifest_path, manifest)?;
    }

    for (idx, (manifest_path, manifest)) in entries.iter_mut().enumerate() {
        if manifest.retention.pinned {
            continue;
        }
        if watch_ranks.get(&idx).copied().unwrap_or(usize::MAX) >= MAX_WATCH_SIDECAR_RUNS {
            let mut watch_changed = false;
            watch_changed |=
                remove_run_artifact(manifest_path, &mut manifest.artifacts.watch_audit);
            watch_changed |=
                remove_run_artifact(manifest_path, &mut manifest.artifacts.watch_noncombat);
            if watch_changed {
                summary.pruned_watch += 1;
                rewrite_manifest(manifest_path, manifest)?;
            }
        }
    }

    Ok(summary)
}

pub fn logs_status(paths: &LiveLogPaths) -> Result<LiveLogsStatus, String> {
    let entries = list_run_manifests(paths)?;
    let mut status = LiveLogsStatus::default();
    status.total_runs = entries.len();
    status.latest_run_id = entries
        .iter()
        .map(|(_, manifest)| manifest.run_id.clone())
        .max();
    for (_, manifest) in entries {
        *status
            .labels
            .entry(manifest.classification_label.clone())
            .or_insert(0) += 1;
        if manifest.retention.pinned {
            status.pinned_runs += 1;
        }
        if is_clean_label(&manifest.classification_label) {
            status.clean_runs += 1;
        } else {
            status.tainted_runs += 1;
        }
    }
    Ok(status)
}

pub fn list_run_manifests_for_audit(
    paths: &LiveLogPaths,
) -> Result<Vec<(PathBuf, LiveRunManifest)>, String> {
    list_run_manifests(paths)
}

pub fn set_run_pin(paths: &LiveLogPaths, run_id: &str, pinned: bool) -> Result<PathBuf, String> {
    let manifest_path = manifest_path_for_run(paths, run_id)?;
    let mut manifest = load_manifest(&manifest_path)?;
    manifest.retention.pinned = pinned;
    rewrite_manifest(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

pub fn regenerate_run_replay(paths: &LiveLogPaths, run_id: &str) -> Result<PathBuf, String> {
    let manifest_path = manifest_path_for_run(paths, run_id)?;
    let mut manifest = load_manifest(&manifest_path)?;
    let Some(raw) = artifact_absolute_path(&manifest_path, &manifest.artifacts.raw) else {
        return Err(format!(
            "run '{}' has no raw.jsonl to regenerate replay from",
            run_id
        ));
    };
    let replay_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("replay.json");
    let replay = build_live_session_replay_from_raw_path(&raw)?;
    write_live_session_replay_to_path(&replay, &replay_path)?;
    manifest.artifacts.replay = Some(LiveArtifactRecord::new("replay.json", true));
    rewrite_manifest(&manifest_path, &manifest)?;
    Ok(replay_path)
}

pub fn latest_run_artifact_path(
    paths: &LiveLogPaths,
    label: Option<&str>,
    artifact: &str,
) -> Option<PathBuf> {
    let mut entries = list_run_manifests(paths).ok()?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));
    for (manifest_path, manifest) in entries {
        if let Some(label) = label {
            if manifest.classification_label != label {
                continue;
            }
        }
        let candidate = match artifact {
            "raw" => artifact_absolute_path(&manifest_path, &manifest.artifacts.raw),
            "focus" => artifact_absolute_path(&manifest_path, &manifest.artifacts.focus),
            "signatures" => artifact_absolute_path(&manifest_path, &manifest.artifacts.signatures),
            "combat_suspects" => {
                artifact_absolute_path(&manifest_path, &manifest.artifacts.combat_suspects)
            }
            "debug" => artifact_absolute_path(&manifest_path, &manifest.artifacts.debug),
            "replay" => artifact_absolute_path(&manifest_path, &manifest.artifacts.replay),
            _ => None,
        };
        if candidate.is_some() {
            return candidate;
        }
    }
    None
}

pub fn latest_raw_path(paths: &LiveLogPaths) -> Option<PathBuf> {
    let current = paths.current_raw();
    if current.exists() {
        return Some(current);
    }
    latest_run_artifact_path(paths, None, "raw")
        .or_else(|| latest_legacy_path(paths, "raw", "live_comm_raw_", ".jsonl"))
}

pub fn latest_valid_raw_path(paths: &LiveLogPaths) -> Option<PathBuf> {
    let current_manifest = paths.current_manifest();
    if current_manifest.exists() && current_validation_is_ok(paths) && paths.current_raw().exists()
    {
        return Some(paths.current_raw());
    }

    let mut entries = list_run_manifests(paths).ok()?;
    entries.reverse();
    for (manifest_path, manifest) in entries {
        let validation_ok = manifest
            .validation
            .as_ref()
            .is_some_and(|validation| validation.status.starts_with("ok"));
        if !validation_ok {
            continue;
        }
        if let Some(path) = artifact_absolute_path(&manifest_path, &manifest.artifacts.raw) {
            return Some(path);
        }
    }
    None
}

fn current_validation_is_ok(paths: &LiveLogPaths) -> bool {
    let validation_path = paths.current_validation();
    let Ok(text) = std::fs::read_to_string(validation_path) else {
        return false;
    };
    let Ok(validation) = serde_json::from_str::<LiveRunValidation>(&text) else {
        return false;
    };
    validation.status.starts_with("ok")
}

pub fn latest_combat_suspect_path(paths: &LiveLogPaths) -> Option<PathBuf> {
    let current = paths.current_combat_suspects();
    if current.exists() && file_nonempty(&current) {
        return Some(current);
    }
    latest_run_artifact_path(paths, None, "combat_suspects").or_else(|| {
        latest_legacy_path(
            paths,
            "combat_suspects",
            "live_comm_combat_suspects_",
            ".jsonl",
        )
    })
}

fn latest_legacy_path(
    paths: &LiveLogPaths,
    subdir: &str,
    prefix: &str,
    suffix: &str,
) -> Option<PathBuf> {
    let dir = paths.root.join(subdir);
    let mut files = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(suffix))
        })
        .collect::<Vec<_>>();
    files.sort();
    files.pop()
}

fn classify_run(input: &FinalizeRunInput) -> String {
    let tainted = input.engine_bug_total > 0
        || input.content_gap_total > 0
        || input.timing_diff_total > 0
        || input.replay_failures > 0
        || input.session_exit_reason == "PARITY_FAIL";

    if input.final_victory {
        return if tainted {
            "victory_tainted".to_string()
        } else {
            "victory_clean".to_string()
        };
    }
    if input.game_over_seen {
        return if tainted {
            "loss_tainted".to_string()
        } else {
            "loss_clean".to_string()
        };
    }
    if input.parity_mode.eq_ignore_ascii_case("strict") {
        if tainted {
            "strict_fail".to_string()
        } else {
            "strict_ok".to_string()
        }
    } else if tainted {
        "survey_tainted".to_string()
    } else {
        "survey_clean".to_string()
    }
}

fn is_clean_label(label: &str) -> bool {
    matches!(
        label,
        "strict_ok" | "survey_clean" | "loss_clean" | "victory_clean"
    )
}

fn copy_if_exists(source: &Path, target: &Path) -> Result<bool, String> {
    if !source.exists() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create '{}': {err}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|err| {
        format!(
            "failed to copy '{}' to '{}': {err}",
            source.display(),
            target.display()
        )
    })?;
    Ok(true)
}

fn copy_if_nonempty(source: &Path, target: &Path) -> Result<bool, String> {
    if !file_nonempty(source) {
        return Ok(false);
    }
    copy_if_exists(source, target)
}

fn file_nonempty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

fn write_manifest(path: &Path, manifest: &LiveRunManifest) -> Result<PathBuf, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create '{}': {err}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|err| format!("failed to serialize manifest: {err}"))?;
    std::fs::write(path, text)
        .map_err(|err| format!("failed to write manifest '{}': {err}", path.display()))?;
    Ok(path.to_path_buf())
}

fn rewrite_manifest(path: &Path, manifest: &LiveRunManifest) -> Result<(), String> {
    write_manifest(path, manifest).map(|_| ())
}

fn load_manifest(path: &Path) -> Result<LiveRunManifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read manifest '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse manifest '{}': {err}", path.display()))
}

fn list_run_manifests(paths: &LiveLogPaths) -> Result<Vec<(PathBuf, LiveRunManifest)>, String> {
    let mut manifests = Vec::new();
    if !paths.runs.exists() {
        return Ok(manifests);
    }
    for entry in std::fs::read_dir(&paths.runs)
        .map_err(|err| format!("failed to read runs dir '{}': {err}", paths.runs.display()))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest = match load_manifest(&manifest_path) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        manifests.push((manifest_path, manifest));
    }
    Ok(manifests)
}

fn manifest_path_for_run(paths: &LiveLogPaths, run_id: &str) -> Result<PathBuf, String> {
    let path = paths.run_dir(run_id).join("manifest.json");
    if !path.exists() {
        return Err(format!(
            "run '{}' not found under '{}'",
            run_id,
            paths.runs.display()
        ));
    }
    Ok(path)
}

fn artifact_absolute_path(
    manifest_path: &Path,
    record: &Option<LiveArtifactRecord>,
) -> Option<PathBuf> {
    let record = record.as_ref()?;
    if !record.present {
        return None;
    }
    let run_dir = manifest_path.parent()?;
    let path = run_dir.join(&record.relative_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn artifact_present(record: &Option<LiveArtifactRecord>) -> bool {
    record.as_ref().is_some_and(|record| record.present)
}

fn remove_run_artifact(manifest_path: &Path, record: &mut Option<LiveArtifactRecord>) -> bool {
    let Some(entry) = record.as_mut() else {
        return false;
    };
    if !entry.present {
        return false;
    }
    let Some(run_dir) = manifest_path.parent() else {
        return false;
    };
    let path = run_dir.join(&entry.relative_path);
    let existed = path.exists();
    let _ = std::fs::remove_file(&path);
    entry.present = false;
    existed
}

pub fn verify_replay_counts(replay_path: &Path) -> Result<(usize, usize), String> {
    let replay = crate::diff::replay::live_comm_replay::load_live_session_replay_path(replay_path)?;
    let view = derive_combat_replay_view(&replay);
    let report = verify_combat_replay_view(&view, false)?;
    let mut timing = 0;
    for failure in &report.failures {
        for diff in &failure.diffs {
            if diff.category.to_string() == "TIMING" {
                timing += 1;
            }
        }
    }
    Ok((report.failures.len(), timing))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths(name: &str) -> LiveLogPaths {
        let root = std::env::temp_dir().join(format!(
            "sts_live_logs_{}_{}_{}",
            name,
            std::process::id(),
            timestamp_string()
        ));
        LiveLogPaths {
            current: root.join("current"),
            runs: root.join("runs"),
            root,
        }
    }

    fn write_text(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    fn clean_input(run_id: &str) -> FinalizeRunInput {
        FinalizeRunInput {
            run_id: run_id.to_string(),
            timestamp: run_id.to_string(),
            build_tag: "test".to_string(),
            parity_mode: "Strict".to_string(),
            watch_enabled: false,
            session_exit_reason: "GAME_OVER".to_string(),
            game_over_seen: true,
            final_victory: false,
            ..Default::default()
        }
    }

    #[test]
    fn gc_trims_replay_for_clean_runs_and_preserves_manifest() {
        let paths = temp_paths("gc");
        ensure_log_dirs(&paths).unwrap();
        for idx in 0..3 {
            let run_id = format!("20260413_00000{}", idx);
            let run_dir = paths.run_dir(&run_id);
            std::fs::create_dir_all(&run_dir).unwrap();
            let manifest = LiveRunManifest {
                run_id: run_id.clone(),
                timestamp: run_id.clone(),
                build_tag: "test".to_string(),
                parity_mode: "Strict".to_string(),
                watch_enabled: false,
                session_exit_reason: "GAME_OVER".to_string(),
                classification_label: "loss_clean".to_string(),
                profile: LiveProfileMetadata::default(),
                provenance: LiveRunProvenance::default(),
                counts: LiveRunCounts::default(),
                artifacts: LiveRunArtifacts {
                    replay: Some(LiveArtifactRecord::new("replay.json", true)),
                    raw: Some(LiveArtifactRecord::new("raw.jsonl", true)),
                    focus: Some(LiveArtifactRecord::new("focus.txt", true)),
                    signatures: Some(LiveArtifactRecord::new("signatures.jsonl", true)),
                    ..Default::default()
                },
                retention: LiveRetentionFlags {
                    cache_only: true,
                    ..Default::default()
                },
                validation: None,
            };
            write_text(&run_dir.join("manifest.json"), "{}");
            rewrite_manifest(&run_dir.join("manifest.json"), &manifest).unwrap();
            write_text(&run_dir.join("replay.json"), "{}");
            write_text(&run_dir.join("raw.jsonl"), "{}");
            write_text(&run_dir.join("focus.txt"), "{}");
            write_text(&run_dir.join("signatures.jsonl"), "{}");
        }

        let summary = gc_runs(&paths).unwrap();
        assert_eq!(summary.pruned_replay, 3);
        for idx in 0..3 {
            let manifest_path = paths
                .run_dir(&format!("20260413_00000{}", idx))
                .join("manifest.json");
            assert!(manifest_path.exists());
        }
    }

    #[test]
    fn finalize_creates_run_manifest_and_keeps_clean_replay_cache_only() {
        let paths = temp_paths("finalize");
        ensure_log_dirs(&paths).unwrap();
        write_text(&paths.current_raw(), "{}\n");
        write_text(&paths.current_focus(), "focus\n");
        write_text(&paths.current_signatures(), "{}\n");
        write_text(&paths.current_debug(), "debug\n");
        write_text(&paths.current_replay(), "{}\n");

        let outcome = finalize_live_run(&paths, clean_input("20260413_010101")).unwrap();
        assert_eq!(outcome.classification_label, "loss_clean");
        assert!(outcome.manifest_path.exists());
        assert!(outcome.run_dir.join("raw.jsonl").exists());
        assert!(!outcome.run_dir.join("replay.json").exists());
    }

    #[test]
    fn finalize_writes_validation_and_preserves_empty_event_audit_artifact() {
        let paths = temp_paths("validation");
        ensure_log_dirs(&paths).unwrap();
        write_text(
            &paths.current_raw(),
            "{\"game_state\":{\"screen_type\":\"EVENT\"}}\n",
        );
        write_text(
            &paths.current_focus(),
            "[EVENT] frame=7 Golden Idol s1 -> Fight | family=cost_tradeoff rationale=test\n",
        );
        write_text(
            &paths.current_debug(),
            "[F7] EVENT POLICY Golden Idol family=cost_tradeoff rationale=test score=1.0\n",
        );
        write_text(
            &paths.current_event_audit(),
            "{\"frame\":7,\"screen\":\"EVENT\",\"decision\":{\"event_name\":\"Golden Idol\",\"screen\":1,\"screen_index\":1,\"screen_key\":\"TRAP\",\"screen_source\":\"GoldenIdolEvent.screenNum\",\"family\":\"cost_tradeoff\",\"rationale_key\":\"test\"}}\n",
        );
        write_text(
            &paths.current_sidecar_shadow(),
            "{\"kind\":\"event_shadow\",\"frame\":7}\n",
        );
        write_text(
            &paths.current_failure_snapshots(),
            "{\"snapshot_id\":\"f7_r1_s1_validation_failure\",\"frame\":7,\"trigger_kind\":\"validation_failure\",\"reasons\":[\"event_screen_semantics_incomplete\"]}\n",
        );

        let outcome = finalize_live_run(&paths, clean_input("20260413_020202")).unwrap();
        assert_eq!(outcome.validation_status, "ok");
        assert!(outcome.run_dir.join("event_audit.jsonl").exists());
        assert!(outcome.run_dir.join("sidecar_shadow.jsonl").exists());
        assert!(outcome.run_dir.join("failure_snapshots.jsonl").exists());
        assert!(outcome.run_dir.join("validation.json").exists());

        let manifest = load_manifest(&outcome.manifest_path).unwrap();
        assert!(manifest
            .artifacts
            .event_audit
            .as_ref()
            .is_some_and(|artifact| artifact.present));
        assert!(manifest
            .artifacts
            .sidecar_shadow
            .as_ref()
            .is_some_and(|artifact| artifact.present));
        assert!(manifest
            .artifacts
            .failure_snapshots
            .as_ref()
            .is_some_and(|artifact| artifact.present));
        assert_eq!(
            manifest.validation.as_ref().map(|v| v.status.as_str()),
            Some("ok")
        );
        assert_eq!(
            manifest
                .validation
                .as_ref()
                .and_then(|validation| validation.latest_failure_snapshot_frame),
            Some(7)
        );
    }

    #[test]
    fn finalize_validation_flags_protocol_reward_loop() {
        let paths = temp_paths("reward_loop");
        ensure_log_dirs(&paths).unwrap();
        write_text(
            &paths.current_raw(),
            concat!(
                "{\"game_state\":{\"screen_type\":\"COMBAT_REWARD\",\"screen_state\":{\"rewards\":[{\"reward_type\":\"CARD\",\"choice_index\":0}]}}}\n",
                "{\"game_state\":{\"screen_type\":\"CARD_REWARD\",\"screen_state\":{\"cards\":[{\"id\":\"Combust\"}]}}}\n",
                "{\"game_state\":{\"screen_type\":\"COMBAT_REWARD\",\"screen_state\":{\"rewards\":[{\"reward_type\":\"CARD\",\"choice_index\":0}]}}}\n",
                "{\"game_state\":{\"screen_type\":\"CARD_REWARD\",\"screen_state\":{\"cards\":[{\"id\":\"Combust\"}]}}}\n",
            ),
        );
        write_text(&paths.current_focus(), "focus\n");
        write_text(&paths.current_debug(), "debug\n");

        let outcome = finalize_live_run(&paths, clean_input("20260413_020303")).unwrap();
        let manifest = load_manifest(&outcome.manifest_path).unwrap();
        let validation = manifest.validation.expect("validation");
        assert!(validation.reward_loop_detected);
        assert_eq!(validation.status, "trace_incomplete");
    }

    #[test]
    fn pinned_run_survives_gc() {
        let paths = temp_paths("pinned");
        ensure_log_dirs(&paths).unwrap();
        let run_dir = paths.run_dir("20260413_030303");
        std::fs::create_dir_all(&run_dir).unwrap();
        write_text(&run_dir.join("raw.jsonl"), "{}\n");
        write_text(&run_dir.join("focus.txt"), "focus\n");
        write_text(&run_dir.join("signatures.jsonl"), "{}\n");
        let manifest = LiveRunManifest {
            run_id: "20260413_030303".to_string(),
            timestamp: "20260413_030303".to_string(),
            build_tag: "test".to_string(),
            parity_mode: "Strict".to_string(),
            watch_enabled: false,
            session_exit_reason: "GAME_OVER".to_string(),
            classification_label: "loss_clean".to_string(),
            profile: LiveProfileMetadata::default(),
            provenance: LiveRunProvenance::default(),
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts {
                raw: Some(LiveArtifactRecord::new("raw.jsonl", true)),
                focus: Some(LiveArtifactRecord::new("focus.txt", true)),
                signatures: Some(LiveArtifactRecord::new("signatures.jsonl", true)),
                ..Default::default()
            },
            retention: LiveRetentionFlags {
                pinned: true,
                cache_only: true,
                ..Default::default()
            },
            validation: None,
        };
        rewrite_manifest(&run_dir.join("manifest.json"), &manifest).unwrap();

        gc_runs(&paths).unwrap();

        assert!(run_dir.join("manifest.json").exists());
        assert!(run_dir.join("raw.jsonl").exists());
        assert!(run_dir.join("focus.txt").exists());
    }

    #[test]
    fn latest_raw_prefers_run_manifest_before_legacy() {
        let paths = temp_paths("latest");
        ensure_log_dirs(&paths).unwrap();
        let run_dir = paths.run_dir("20260413_020202");
        std::fs::create_dir_all(&run_dir).unwrap();
        write_text(&run_dir.join("raw.jsonl"), "raw\n");
        let manifest = LiveRunManifest {
            run_id: "20260413_020202".to_string(),
            timestamp: "20260413_020202".to_string(),
            build_tag: "test".to_string(),
            parity_mode: "Survey".to_string(),
            watch_enabled: false,
            session_exit_reason: "GAME_OVER".to_string(),
            classification_label: "survey_clean".to_string(),
            profile: LiveProfileMetadata::default(),
            provenance: LiveRunProvenance::default(),
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts {
                raw: Some(LiveArtifactRecord::new("raw.jsonl", true)),
                ..Default::default()
            },
            retention: LiveRetentionFlags::default(),
            validation: None,
        };
        rewrite_manifest(&run_dir.join("manifest.json"), &manifest).unwrap();

        let latest = latest_raw_path(&paths).unwrap();
        assert_eq!(latest, run_dir.join("raw.jsonl"));
    }
}
