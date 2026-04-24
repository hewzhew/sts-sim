use super::live_comm_admin::{
    ensure_log_dirs, gc_runs, is_clean_label, rewrite_manifest, write_manifest, LiveArtifactRecord,
    LiveLogPaths, LiveProfileMetadata, LiveRetentionFlags, LiveRunArtifacts, LiveRunCounts,
    LiveRunManifest, LiveRunProvenance, LiveRunValidation,
};
use crate::verification::combat::{
    derive_replay_view, load_live_session_replay, verify_replay_view,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const PROFILE_PATH: &str = r"d:\rust\sts_simulator\tools\live_comm\profile.json";

#[derive(Clone, Debug, Default)]
pub(crate) struct FinalizeRunInput {
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
pub(crate) struct FinalizeRunOutcome {
    pub run_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub classification_label: String,
    pub validation_status: String,
    pub gc_summary: String,
}

pub(crate) fn load_profile_metadata() -> LiveProfileMetadata {
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

pub(crate) fn runtime_provenance() -> LiveRunProvenance {
    let profile = load_profile_metadata();
    let git_short = option_env!("LIVE_COMM_GIT_SHORT")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let repo_head_short = env_opt("LIVE_COMM_LAUNCH_REPO_HEAD_SHORT");
    let build_unix = option_env!("LIVE_COMM_BUILD_UNIX").and_then(|s| s.parse::<u64>().ok());
    let binary_matches_head = match (git_short.as_deref(), repo_head_short.as_deref()) {
        (Some(binary), Some(head)) => Some(binary == head),
        _ => None,
    };
    let binary_is_fresh = env_opt("LIVE_COMM_LAUNCH_BINARY_IS_FRESH").and_then(|value| match value
        .to_ascii_lowercase()
        .as_str()
    {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    });

    LiveRunProvenance {
        exe_path: env_opt("LIVE_COMM_LAUNCH_EXE_PATH").or_else(current_exe_path_string),
        exe_mtime_utc: env_opt("LIVE_COMM_LAUNCH_EXE_MTIME_UTC")
            .or_else(current_exe_mtime_fallback),
        git_short_sha: git_short,
        repo_head_short_sha: repo_head_short,
        binary_matches_head,
        binary_is_fresh,
        build_unix,
        build_time_utc: build_unix.map(|secs| format!("unix:{secs}")),
        source_inputs_latest_path: env_opt("LIVE_COMM_LAUNCH_SOURCE_LATEST_PATH"),
        source_inputs_latest_mtime_utc: env_opt("LIVE_COMM_LAUNCH_SOURCE_LATEST_MTIME_UTC"),
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
    let mut multistage_counts = std::collections::BTreeMap::new();
    let mut multistage_has_progress = std::collections::BTreeMap::new();
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
            *multistage_counts
                .entry(event_name.to_string())
                .or_insert(0usize) += 1;
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

#[derive(Clone, Debug, Default)]
struct RunTriageMetrics {
    highest_floor: Option<i64>,
    highest_act: Option<i64>,
    deepest_room_type: Option<String>,
    reached_boss_acts: Vec<i64>,
    terminal_frame: Option<u64>,
    terminal_floor: Option<i64>,
    terminal_act: Option<i64>,
    terminal_room_type: Option<String>,
    terminal_screen: Option<String>,
    terminal_score: Option<i64>,
    terminal_victory: Option<bool>,
    terminal_monsters: Vec<String>,
    slow_search_count: usize,
    slow_search_max_ms: u64,
    exact_turn_disagree_count: usize,
    exact_turn_skip_count: usize,
    exact_turn_takeover_count: usize,
    strict_dominance_disagreement_count: usize,
    high_threat_disagreement_count: usize,
    survival_downgrade_avoided_count: usize,
    regime_counts: BTreeMap<String, usize>,
    search_timeout_count: usize,
    event_fallback_count: usize,
    terminal_snapshot_present: bool,
}

impl RunTriageMetrics {
    fn has_quality_warnings(&self) -> bool {
        self.slow_search_count > 0
            || self.exact_turn_disagree_count > 0
            || self.exact_turn_skip_count > 0
            || self.exact_turn_takeover_count > 0
            || self.strict_dominance_disagreement_count > 0
            || self.high_threat_disagreement_count > 0
            || self.survival_downgrade_avoided_count > 0
            || self.search_timeout_count > 0
            || self.event_fallback_count > 0
    }
}

fn derive_run_triage_metrics(run_dir: &Path, focus_text: &str) -> RunTriageMetrics {
    let mut metrics = RunTriageMetrics::default();
    derive_progression_metrics(&run_dir.join("raw.jsonl"), &mut metrics);
    derive_focus_quality_metrics(focus_text, &mut metrics);
    derive_terminal_outcome_metrics(run_dir, &mut metrics);
    metrics
}

fn derive_progression_metrics(raw_path: &Path, metrics: &mut RunTriageMetrics) {
    let Ok(text) = std::fs::read_to_string(raw_path) else {
        return;
    };

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
        let floor = game_state.get("floor").and_then(Value::as_i64);
        let act = game_state.get("act").and_then(Value::as_i64);
        let room_type = game_state
            .get("room_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        if let (Some(floor), Some(act)) = (floor, act) {
            let should_replace = metrics.highest_floor.is_none()
                || metrics.highest_floor.is_some_and(|current| floor > current)
                || (metrics.highest_floor == Some(floor)
                    && metrics.highest_act.is_some_and(|current| act > current));
            if should_replace {
                metrics.highest_floor = Some(floor);
                metrics.highest_act = Some(act);
                metrics.deepest_room_type = Some(room_type.to_string());
            }
            if room_type == "MonsterRoomBoss" && !metrics.reached_boss_acts.contains(&act) {
                metrics.reached_boss_acts.push(act);
                metrics.reached_boss_acts.sort_unstable();
            }
        }
    }
}

fn derive_focus_quality_metrics(focus_text: &str, metrics: &mut RunTriageMetrics) {
    for line in focus_text.lines() {
        if line.contains("[SLOW SEARCH]") {
            metrics.slow_search_count += 1;
            if let Some(root_ms) = parse_u64_field(line, "root_ms=") {
                metrics.slow_search_max_ms = metrics.slow_search_max_ms.max(root_ms);
            }
        }
        if line.contains("[SEARCH TIMEOUT]") {
            metrics.search_timeout_count += 1;
        }
        if line.contains("[AUDIT]") && line.contains("agrees=false") {
            metrics.exact_turn_disagree_count += 1;
        }
        if line.contains("[AUDIT]") && line.contains("skipped=true") {
            metrics.exact_turn_skip_count += 1;
        }
        if line.contains("[AUDIT]") {
            if let Some(regime) = parse_word_field(line, "regime=") {
                *metrics.regime_counts.entry(regime.to_string()).or_insert(0) += 1;
            }
            if line.contains("takeover=true") {
                metrics.exact_turn_takeover_count += 1;
            }
            if line.contains("dominance=strictly_better_in_window") {
                metrics.strict_dominance_disagreement_count += 1;
            }
            if (line.contains("regime=crisis") || line.contains("regime=fragile"))
                && line.contains("agrees=false")
            {
                metrics.high_threat_disagreement_count += 1;
            }
            if parse_survival_field(line, "frontier_survival=")
                .zip(parse_survival_field(line, "exact_survival="))
                .is_some_and(|(frontier, exact)| exact > frontier)
            {
                metrics.survival_downgrade_avoided_count += 1;
            }
        }
        if line.starts_with("[EVENT]")
            && (line.contains(" fallback ")
                || line.contains("protocol=unsupported_event")
                || line.contains("protocol=unknown_event_name"))
        {
            metrics.event_fallback_count += 1;
        }
    }
}

fn failure_snapshots_only_terminal_loss(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let mut saw_snapshot = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            return false;
        };
        saw_snapshot = true;
        if root
            .get("trigger_kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            != "terminal_loss"
        {
            return false;
        }
    }
    saw_snapshot
}

fn derive_terminal_outcome_metrics(run_dir: &Path, metrics: &mut RunTriageMetrics) {
    let terminal_path = run_dir.join("terminal_snapshot.json");
    if let Ok(text) = std::fs::read_to_string(&terminal_path) {
        if let Ok(root) = serde_json::from_str::<Value>(&text) {
            populate_terminal_metrics_from_value(&root, metrics);
            metrics.terminal_snapshot_present = true;
            return;
        }
    }

    let failure_path = run_dir.join("failure_snapshots.jsonl");
    if !failure_snapshots_only_terminal_loss(&failure_path) {
        return;
    }
    let Ok(text) = std::fs::read_to_string(&failure_path) else {
        return;
    };
    let Some(line) = text.lines().find(|line| !line.trim().is_empty()) else {
        return;
    };
    let Ok(root) = serde_json::from_str::<Value>(line.trim()) else {
        return;
    };
    populate_terminal_metrics_from_value(&root, metrics);
    metrics.terminal_snapshot_present = true;
}

fn populate_terminal_metrics_from_value(root: &Value, metrics: &mut RunTriageMetrics) {
    metrics.terminal_frame = root.get("frame").and_then(Value::as_u64);
    metrics.terminal_screen = root
        .get("screen")
        .and_then(Value::as_str)
        .map(|value| value.to_string());

    let normalized_state = root.get("normalized_state");
    metrics.terminal_floor = normalized_state
        .and_then(|state| state.get("floor"))
        .and_then(Value::as_i64);
    metrics.terminal_act = normalized_state
        .and_then(|state| state.get("act"))
        .and_then(Value::as_i64);
    metrics.terminal_room_type = root
        .get("room_type")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .or_else(|| {
            normalized_state
                .and_then(|state| state.get("room_type"))
                .and_then(Value::as_str)
                .map(|value| value.to_string())
        });
    metrics.terminal_score = root
        .get("decision_context")
        .and_then(|ctx| ctx.get("score"))
        .and_then(Value::as_i64);
    metrics.terminal_victory = root
        .get("decision_context")
        .and_then(|ctx| ctx.get("victory"))
        .and_then(Value::as_bool);
    metrics.terminal_monsters = normalized_state
        .and_then(|state| state.get("monsters"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|monster| {
            monster
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| monster.get("id").and_then(Value::as_str))
        })
        .map(|value| value.to_string())
        .take(3)
        .collect();
}

fn parse_u64_field(line: &str, key: &str) -> Option<u64> {
    let tail = line.split_once(key)?.1;
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

fn parse_word_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let tail = line.split_once(key)?.1;
    let token = tail
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>();
    (!token.is_empty())
        .then_some(tail.get(..token.len())?)
        .or(None)
}

fn parse_survival_field(line: &str, key: &str) -> Option<u8> {
    match parse_word_field(line, key)? {
        "forced_loss" => Some(0),
        "severe_risk" => Some(1),
        "risky_but_playable" => Some(2),
        "stable" => Some(3),
        "safe" => Some(4),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RuntimeFindingSnapshot {
    snapshot_id: String,
    frame: u64,
    screen: String,
    trigger_kind: String,
    reasons: Vec<String>,
    normalized_state: Value,
    decision_context: Value,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeFindingValueExample {
    rust: String,
    java: String,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeFindingFamily {
    category: String,
    key: String,
    count: usize,
    first_frame: u64,
    last_frame: u64,
    example_frames: Vec<u64>,
    example_snapshot_ids: Vec<String>,
    example_rust_java_values: Vec<RuntimeFindingValueExample>,
    combat_labels: Vec<String>,
    event_labels: Vec<String>,
    suggested_artifacts: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeFindingReport {
    run_id: String,
    classification_label: String,
    counts: LiveRunCounts,
    families: Vec<RuntimeFindingFamily>,
}

#[derive(Clone, Debug, Serialize)]
struct BotStrengthSummary {
    run_id: String,
    classification_label: String,
    validation_status: String,
    parity_clean: bool,
    session_exit_reason: String,
    highest_floor: Option<i64>,
    highest_act: Option<i64>,
    deepest_room_type: Option<String>,
    reached_boss_acts: Vec<i64>,
    terminal_outcome: Option<BotTerminalOutcome>,
    slow_search_count: usize,
    slow_search_max_ms: u64,
    exact_turn_disagree_count: usize,
    exact_turn_skip_count: usize,
    exact_turn_takeover_count: usize,
    strict_dominance_disagreement_count: usize,
    high_threat_disagreement_count: usize,
    survival_downgrade_avoided_count: usize,
    regime_counts: BTreeMap<String, usize>,
    search_timeout_count: usize,
    event_fallback_count: usize,
    primary_signals: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct BotTerminalOutcome {
    frame: Option<u64>,
    floor: Option<i64>,
    act: Option<i64>,
    room_type: Option<String>,
    screen: Option<String>,
    score: Option<i64>,
    victory: Option<bool>,
    monsters: Vec<String>,
}

#[derive(Clone, Debug)]
struct RuntimeDiffEntry {
    field: String,
    category: String,
    rust: String,
    java: String,
}

#[derive(Clone, Debug)]
struct RuntimeFindingFamilyBuilder {
    category: String,
    key: String,
    count: usize,
    first_frame: u64,
    last_frame: u64,
    example_frames: Vec<u64>,
    example_snapshot_ids: Vec<String>,
    example_rust_java_values: Vec<RuntimeFindingValueExample>,
    combat_labels: Vec<String>,
    event_labels: Vec<String>,
}

impl RuntimeFindingFamilyBuilder {
    fn new(category: &str, key: &str, frame: u64) -> Self {
        Self {
            category: category.to_string(),
            key: key.to_string(),
            count: 0,
            first_frame: frame,
            last_frame: frame,
            example_frames: Vec::new(),
            example_snapshot_ids: Vec::new(),
            example_rust_java_values: Vec::new(),
            combat_labels: Vec::new(),
            event_labels: Vec::new(),
        }
    }

    fn add_occurrence(
        &mut self,
        frame: u64,
        snapshot_id: &str,
        labels: &RuntimeSnapshotLabels,
        diff_values: Option<RuntimeFindingValueExample>,
    ) {
        self.count += 1;
        self.first_frame = self.first_frame.min(frame);
        self.last_frame = self.last_frame.max(frame);
        push_unique_limited_u64(&mut self.example_frames, frame, 5);
        push_unique_limited_string(&mut self.example_snapshot_ids, snapshot_id, 5);
        for label in &labels.combat_labels {
            push_unique_limited_string(&mut self.combat_labels, label, 5);
        }
        for label in &labels.event_labels {
            push_unique_limited_string(&mut self.event_labels, label, 5);
        }
        if let Some(values) = diff_values {
            push_unique_value_example(&mut self.example_rust_java_values, values, 3);
        }
    }

    fn build(self) -> RuntimeFindingFamily {
        RuntimeFindingFamily {
            category: self.category.clone(),
            key: self.key.clone(),
            count: self.count,
            first_frame: self.first_frame,
            last_frame: self.last_frame,
            example_frames: self.example_frames,
            example_snapshot_ids: self.example_snapshot_ids,
            example_rust_java_values: self.example_rust_java_values,
            combat_labels: self.combat_labels,
            event_labels: self.event_labels,
            suggested_artifacts: suggested_artifacts_for_category(&self.category),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RuntimeSnapshotLabels {
    combat_labels: Vec<String>,
    event_labels: Vec<String>,
}

fn push_unique_limited_string(target: &mut Vec<String>, value: &str, limit: usize) {
    if target.iter().any(|existing| existing == value) {
        return;
    }
    if target.len() < limit {
        target.push(value.to_string());
    }
}

fn push_unique_limited_u64(target: &mut Vec<u64>, value: u64, limit: usize) {
    if target.contains(&value) {
        return;
    }
    if target.len() < limit {
        target.push(value);
    }
}

fn push_unique_value_example(
    target: &mut Vec<RuntimeFindingValueExample>,
    value: RuntimeFindingValueExample,
    limit: usize,
) {
    if target
        .iter()
        .any(|existing| existing.rust == value.rust && existing.java == value.java)
    {
        return;
    }
    if target.len() < limit {
        target.push(value);
    }
}

fn parse_diff_entry(raw: &str) -> Option<RuntimeDiffEntry> {
    let (before_java, java) = raw.split_once(" Java=")?;
    let (before_rust, rust) = before_java.rsplit_once(" Rust=")?;
    let (field, category) = before_rust.rsplit_once(" [")?;
    let category = category.strip_suffix(']')?;
    Some(RuntimeDiffEntry {
        field: field.to_string(),
        category: category.to_ascii_lowercase(),
        rust: rust.to_string(),
        java: java.to_string(),
    })
}

fn load_failure_snapshots(path: &Path) -> Result<Vec<RuntimeFindingSnapshot>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
    let mut snapshots = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let snapshot = serde_json::from_str::<RuntimeFindingSnapshot>(trimmed).map_err(|err| {
            format!(
                "invalid failure snapshot json in '{}' at line {}: {err}",
                path.display(),
                idx + 1
            )
        })?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn derive_snapshot_labels(snapshot: &RuntimeFindingSnapshot) -> RuntimeSnapshotLabels {
    let mut labels = RuntimeSnapshotLabels::default();

    let monsters = snapshot
        .normalized_state
        .get("monsters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !monsters.is_empty() {
        let alive = monsters
            .iter()
            .filter(|monster| {
                monster
                    .get("current_hp")
                    .and_then(Value::as_i64)
                    .is_some_and(|hp| hp > 0)
            })
            .collect::<Vec<_>>();
        let source = if alive.is_empty() {
            monsters.iter().collect::<Vec<_>>()
        } else {
            alive
        };
        let mut names = Vec::new();
        for monster in source {
            let name = monster
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| monster.get("id").and_then(Value::as_str))
                .unwrap_or("Unknown Monster");
            if !names.iter().any(|existing| existing == name) {
                names.push(name.to_string());
            }
        }
        if !names.is_empty() {
            let label = if names.len() == 1 {
                names[0].clone()
            } else {
                names.into_iter().take(3).collect::<Vec<_>>().join(" + ")
            };
            labels.combat_labels.push(label);
        }
    }

    let screen_state = snapshot.normalized_state.get("screen_state");
    let event_label = screen_state
        .and_then(|state| state.get("event_name").and_then(Value::as_str))
        .or_else(|| screen_state.and_then(|state| state.get("event_id").and_then(Value::as_str)))
        .or_else(|| {
            screen_state.and_then(|state| state.get("current_screen_key").and_then(Value::as_str))
        })
        .unwrap_or(&snapshot.screen);
    if !event_label.is_empty() {
        labels.event_labels.push(event_label.to_string());
    }

    labels
}

fn suggested_artifacts_for_category(category: &str) -> Vec<String> {
    match category {
        "engine_bug" | "content_gap" | "timing" => vec![
            "debug.txt".to_string(),
            "failure_snapshots.jsonl".to_string(),
            "replay.json".to_string(),
        ],
        "protocol_screen_action_space" => vec![
            "failure_snapshots.jsonl".to_string(),
            "debug.txt".to_string(),
            "raw.jsonl".to_string(),
        ],
        "validation_failure" => vec!["event_audit.jsonl".to_string(), "debug.txt".to_string()],
        _ => vec!["debug.txt".to_string()],
    }
}

fn classify_validation_reason(snapshot: &RuntimeFindingSnapshot, reason: &str) -> (String, String) {
    if let Some(key) = protocol_screen_action_space_key(snapshot, reason) {
        return ("protocol_screen_action_space".to_string(), key);
    }
    ("validation_failure".to_string(), reason.to_string())
}

fn protocol_screen_action_space_key(
    snapshot: &RuntimeFindingSnapshot,
    reason: &str,
) -> Option<String> {
    let known_kind = [
        "missing_screen_action_space",
        "invalid_screen_action_space",
        "screen_action_space_screen_type_mismatch",
        "missing_screen_action_space_screen_type",
        "missing_screen_action_space_actions",
        "empty_screen_action_space",
        "invalid_screen_action_space_kind",
        "invalid_screen_action_space_command",
        "invalid_screen_action_space_choice_index",
    ]
    .into_iter()
    .find(|kind| reason == *kind || reason.starts_with(&format!("{kind}:")))?;

    let field = snapshot
        .decision_context
        .get("expected_field")
        .and_then(Value::as_str)
        .or_else(|| reason.split(':').nth(1))
        .unwrap_or("unknown_field");
    let screen = snapshot
        .decision_context
        .get("screen")
        .and_then(Value::as_str)
        .unwrap_or(snapshot.screen.as_str());
    Some(format!("{known_kind}:{field}:screen={screen}"))
}

fn build_finding_report(
    run_id: &str,
    classification_label: &str,
    counts: &LiveRunCounts,
    failure_snapshots_path: &Path,
) -> Result<RuntimeFindingReport, String> {
    let snapshots = load_failure_snapshots(failure_snapshots_path)?;
    let mut families = BTreeMap::<(String, String), RuntimeFindingFamilyBuilder>::new();

    for snapshot in snapshots {
        let labels = derive_snapshot_labels(&snapshot);
        match snapshot.trigger_kind.as_str() {
            "engine_bug" => {
                let diffs = snapshot
                    .decision_context
                    .get("diffs")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for diff in diffs {
                    let Some(raw) = diff.as_str() else {
                        continue;
                    };
                    let Some(parsed) = parse_diff_entry(raw) else {
                        continue;
                    };
                    let family_key = (parsed.category.clone(), parsed.field.clone());
                    let builder = families.entry(family_key).or_insert_with(|| {
                        RuntimeFindingFamilyBuilder::new(
                            &parsed.category,
                            &parsed.field,
                            snapshot.frame,
                        )
                    });
                    builder.add_occurrence(
                        snapshot.frame,
                        &snapshot.snapshot_id,
                        &labels,
                        Some(RuntimeFindingValueExample {
                            rust: parsed.rust,
                            java: parsed.java,
                        }),
                    );
                }
            }
            "validation_failure" => {
                for reason in snapshot
                    .reasons
                    .iter()
                    .filter(|reason| !reason.trim().is_empty() && !reason.starts_with("count="))
                {
                    let (category, key) = classify_validation_reason(&snapshot, reason);
                    let family_key = (category.clone(), key.clone());
                    let builder = families.entry(family_key).or_insert_with(|| {
                        RuntimeFindingFamilyBuilder::new(&category, &key, snapshot.frame)
                    });
                    builder.add_occurrence(snapshot.frame, &snapshot.snapshot_id, &labels, None);
                }
            }
            _ => {}
        }
    }

    let mut families = families
        .into_values()
        .map(RuntimeFindingFamilyBuilder::build)
        .collect::<Vec<_>>();
    families.sort_by(|left, right| {
        category_sort_key(&left.category)
            .cmp(&category_sort_key(&right.category))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.key.cmp(&right.key))
    });

    Ok(RuntimeFindingReport {
        run_id: run_id.to_string(),
        classification_label: classification_label.to_string(),
        counts: counts.clone(),
        families,
    })
}

pub fn build_finding_report_json(
    run_id: &str,
    classification_label: &str,
    counts: &LiveRunCounts,
    failure_snapshots_path: &Path,
) -> Result<Value, String> {
    let report =
        build_finding_report(run_id, classification_label, counts, failure_snapshots_path)?;
    serde_json::to_value(report)
        .map_err(|err| format!("failed to convert findings report to json: {err}"))
}

fn category_sort_key(category: &str) -> u8 {
    match category {
        "engine_bug" => 0,
        "content_gap" => 1,
        "timing" => 2,
        "protocol_screen_action_space" => 3,
        "validation_failure" => 4,
        _ => 5,
    }
}

fn findings_for_category<'a>(
    report: &'a RuntimeFindingReport,
    category: &'a str,
) -> impl Iterator<Item = &'a RuntimeFindingFamily> {
    report
        .families
        .iter()
        .filter(move |family| family.category == category)
}

fn build_empty_finding_report(
    run_id: &str,
    classification_label: &str,
    counts: &LiveRunCounts,
) -> RuntimeFindingReport {
    RuntimeFindingReport {
        run_id: run_id.to_string(),
        classification_label: classification_label.to_string(),
        counts: counts.clone(),
        families: Vec::new(),
    }
}

fn build_bot_strength_summary(
    report: &RuntimeFindingReport,
    manifest: &LiveRunManifest,
    validation: &LiveRunValidation,
    metrics: &RunTriageMetrics,
) -> BotStrengthSummary {
    let parity_clean = report.counts.engine_bugs == 0
        && report.counts.content_gaps == 0
        && report.counts.timing_diffs == 0
        && report.counts.replay_failures == 0;
    let mut primary_signals = Vec::new();
    if !parity_clean {
        primary_signals.push("parity_not_clean".to_string());
    }
    if metrics.search_timeout_count > 0 {
        primary_signals.push(format!("search_timeouts={}", metrics.search_timeout_count));
    }
    if metrics.slow_search_count > 0 {
        primary_signals.push(format!(
            "slow_searches={} max_root_ms={}",
            metrics.slow_search_count, metrics.slow_search_max_ms
        ));
    }
    if metrics.exact_turn_disagree_count > 0 || metrics.exact_turn_skip_count > 0 {
        primary_signals.push(format!(
            "exact_turn_disagree={} skip={}",
            metrics.exact_turn_disagree_count, metrics.exact_turn_skip_count
        ));
    }
    if metrics.exact_turn_takeover_count > 0 {
        primary_signals.push(format!(
            "exact_turn_takeovers={}",
            metrics.exact_turn_takeover_count
        ));
    }
    if metrics.high_threat_disagreement_count > 0 {
        primary_signals.push(format!(
            "high_threat_disagreements={}",
            metrics.high_threat_disagreement_count
        ));
    }
    if metrics.event_fallback_count > 0 {
        primary_signals.push(format!("event_fallbacks={}", metrics.event_fallback_count));
    }
    if metrics.terminal_snapshot_present {
        let room = metrics
            .terminal_room_type
            .as_deref()
            .unwrap_or("unknown_room");
        let floor = metrics
            .terminal_floor
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        let act = metrics
            .terminal_act
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        primary_signals.push(format!("terminal_outcome=floor_{floor}_act_{act}_{room}"));
    }
    if primary_signals.is_empty() {
        primary_signals.push("no_primary_strength_warnings".to_string());
    }

    BotStrengthSummary {
        run_id: report.run_id.clone(),
        classification_label: report.classification_label.clone(),
        validation_status: validation.status.clone(),
        parity_clean,
        session_exit_reason: manifest.session_exit_reason.clone(),
        highest_floor: metrics.highest_floor,
        highest_act: metrics.highest_act,
        deepest_room_type: metrics.deepest_room_type.clone(),
        reached_boss_acts: metrics.reached_boss_acts.clone(),
        terminal_outcome: metrics
            .terminal_snapshot_present
            .then(|| BotTerminalOutcome {
                frame: metrics.terminal_frame,
                floor: metrics.terminal_floor,
                act: metrics.terminal_act,
                room_type: metrics.terminal_room_type.clone(),
                screen: metrics.terminal_screen.clone(),
                score: metrics.terminal_score,
                victory: metrics.terminal_victory,
                monsters: metrics.terminal_monsters.clone(),
            }),
        slow_search_count: metrics.slow_search_count,
        slow_search_max_ms: metrics.slow_search_max_ms,
        exact_turn_disagree_count: metrics.exact_turn_disagree_count,
        exact_turn_skip_count: metrics.exact_turn_skip_count,
        exact_turn_takeover_count: metrics.exact_turn_takeover_count,
        strict_dominance_disagreement_count: metrics.strict_dominance_disagreement_count,
        high_threat_disagreement_count: metrics.high_threat_disagreement_count,
        survival_downgrade_avoided_count: metrics.survival_downgrade_avoided_count,
        regime_counts: metrics.regime_counts.clone(),
        search_timeout_count: metrics.search_timeout_count,
        event_fallback_count: metrics.event_fallback_count,
        primary_signals,
    }
}

fn render_triage_summary(
    report: &RuntimeFindingReport,
    manifest: &LiveRunManifest,
    validation: &LiveRunValidation,
    metrics: &RunTriageMetrics,
) -> String {
    let mut out = String::new();
    out.push_str("=== TRIAGE SUMMARY ===\n");
    out.push_str(&format!(
        "Run: {} | Profile: {} | Purpose: {}\n",
        report.run_id,
        manifest
            .profile
            .profile_name
            .as_deref()
            .unwrap_or("unknown"),
        manifest.profile.purpose.as_deref().unwrap_or("unknown"),
    ));
    out.push_str(&format!(
        "Mode: parity={} exit={} classification={}\n",
        manifest.parity_mode, manifest.session_exit_reason, report.classification_label
    ));
    out.push_str(&format!(
        "Combat Parity: {} | Validation: {}\n",
        if report.counts.engine_bugs == 0
            && report.counts.content_gaps == 0
            && report.counts.timing_diffs == 0
            && report.counts.replay_failures == 0
        {
            "clean"
        } else {
            "tainted"
        },
        validation.status
    ));
    if !validation.errors.is_empty() {
        out.push_str("Validation Notes:\n");
        for error in validation.errors.iter().take(3) {
            out.push_str(&format!("- {error}\n"));
        }
    }
    out.push_str(&format!(
        "Build: {} | Binary Fresh: {}\n",
        manifest
            .provenance
            .git_short_sha
            .as_deref()
            .unwrap_or("unknown"),
        manifest
            .provenance
            .binary_is_fresh
            .map(|flag| if flag { "true" } else { "false" })
            .unwrap_or("unknown"),
    ));
    out.push_str(&format!(
        "Counts: engine_bugs={} content_gaps={} timing_diffs={} replay_failures={}\n",
        report.counts.engine_bugs,
        report.counts.content_gaps,
        report.counts.timing_diffs,
        report.counts.replay_failures
    ));
    if let (Some(floor), Some(act)) = (metrics.highest_floor, metrics.highest_act) {
        out.push_str(&format!(
            "Progression: reached floor {} act {} room={}\n",
            floor,
            act,
            metrics.deepest_room_type.as_deref().unwrap_or("unknown")
        ));
    }
    if metrics.terminal_snapshot_present {
        out.push_str(&format!(
            "Terminal Outcome: frame={} floor={} act={} room={} screen={} score={} monsters={}\n",
            metrics
                .terminal_frame
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
            metrics
                .terminal_floor
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
            metrics
                .terminal_act
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
            metrics.terminal_room_type.as_deref().unwrap_or("unknown"),
            metrics.terminal_screen.as_deref().unwrap_or("unknown"),
            metrics
                .terminal_score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
            if metrics.terminal_monsters.is_empty() {
                "n/a".to_string()
            } else {
                metrics.terminal_monsters.join(", ")
            }
        ));
    }
    if !metrics.reached_boss_acts.is_empty() {
        let acts = metrics
            .reached_boss_acts
            .iter()
            .map(|act| format!("Act {act}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("Boss Milestones: reached {acts}\n"));
    }
    if metrics.terminal_snapshot_present {
        out.push_str(
            "Snapshots: terminal_snapshot.json is separate from parity failure snapshots\n",
        );
    }

    let top_engine = findings_for_category(report, "engine_bug")
        .take(5)
        .collect::<Vec<_>>();
    let top_validation = findings_for_category(report, "validation_failure")
        .take(5)
        .collect::<Vec<_>>();
    let top_protocol_screen = findings_for_category(report, "protocol_screen_action_space")
        .take(5)
        .collect::<Vec<_>>();
    let top_gaps = findings_for_category(report, "content_gap")
        .take(5)
        .collect::<Vec<_>>();

    if top_engine.is_empty()
        && top_validation.is_empty()
        && top_protocol_screen.is_empty()
        && top_gaps.is_empty()
        && validation.status.starts_with("ok")
    {
        out.push_str("\nPrimary Signal:\n- no combat parity failures captured in this run\n");
    } else if top_engine.is_empty()
        && top_validation.is_empty()
        && top_protocol_screen.is_empty()
        && top_gaps.is_empty()
    {
        out.push_str(
            "\nPrimary Signal:\n- combat parity is clean, but validation/runtime quality still needs attention\n",
        );
    }

    let _ = write_triage_family_section(&mut out, "Top Engine Bug Families:", top_engine);
    let _ = write_triage_family_section(
        &mut out,
        "Top Protocol Screen Action-Space Families:",
        top_protocol_screen,
    );
    let _ =
        write_triage_family_section(&mut out, "Top Validation Failure Families:", top_validation);
    let _ = write_triage_family_section(&mut out, "Top Content Gap Families:", top_gaps);

    if metrics.has_quality_warnings() {
        out.push_str("\nBot/Runtime Signals:\n");
        if metrics.slow_search_count > 0 {
            out.push_str(&format!(
                "- slow_search_count={} max_root_ms={}\n",
                metrics.slow_search_count, metrics.slow_search_max_ms
            ));
        }
        if metrics.exact_turn_disagree_count > 0 || metrics.exact_turn_skip_count > 0 {
            out.push_str(&format!(
                "- exact_turn_disagree_count={} exact_turn_skip_count={}\n",
                metrics.exact_turn_disagree_count, metrics.exact_turn_skip_count
            ));
        }
        if !metrics.regime_counts.is_empty() {
            out.push_str(&format!(
                "- regime_hotspots={}\n",
                format_regime_hotspots(&metrics.regime_counts)
            ));
        }
        if metrics.exact_turn_takeover_count > 0 {
            out.push_str(&format!(
                "- exact_turn_forced_takeover_count={}\n",
                metrics.exact_turn_takeover_count
            ));
        }
        if metrics.strict_dominance_disagreement_count > 0 {
            out.push_str(&format!(
                "- exact_turn_strict_dominance_count={}\n",
                metrics.strict_dominance_disagreement_count
            ));
        }
        if metrics.high_threat_disagreement_count > 0 {
            out.push_str(&format!(
                "- high_threat_disagreement_count={}\n",
                metrics.high_threat_disagreement_count
            ));
        }
        if metrics.survival_downgrade_avoided_count > 0 {
            out.push_str(&format!(
                "- survival_downgrade_avoided_count={}\n",
                metrics.survival_downgrade_avoided_count
            ));
        }
        if metrics.search_timeout_count > 0 {
            out.push_str(&format!(
                "- search_timeout_count={}\n",
                metrics.search_timeout_count
            ));
        }
        if metrics.event_fallback_count > 0 {
            out.push_str(&format!(
                "- event_fallback_count={}\n",
                metrics.event_fallback_count
            ));
        }
    }

    out.push_str("\nArtifacts To Open First:\n");
    if report.counts.engine_bugs > 0
        || report.counts.content_gaps > 0
        || report.counts.timing_diffs > 0
        || report.counts.replay_failures > 0
        || report
            .families
            .iter()
            .any(|family| family.category == "protocol_screen_action_space")
    {
        out.push_str(
            "- focus.txt\n- findings.json\n- failure_snapshots.jsonl\n- debug.txt\n- raw.jsonl\n",
        );
    } else if validation.trace_incomplete {
        out.push_str("- focus.txt\n- validation.json\n- event_audit.jsonl\n- debug.txt\n");
    } else if metrics.has_quality_warnings() {
        out.push_str("- focus.txt\n- bot_strength.json\n- debug.txt\n");
        if metrics.event_fallback_count > 0 {
            out.push_str("- event_audit.jsonl\n");
        }
        out.push_str(
            "- raw.jsonl (only if you are correlating frames or building a replay/case)\n",
        );
    } else {
        out.push_str(
            "- focus.txt\n- bot_strength.json\n- findings.json\n- raw.jsonl (only if you are generating a replay/case)\n",
        );
    }
    out
}

fn format_regime_hotspots(regime_counts: &BTreeMap<String, usize>) -> String {
    let mut entries = regime_counts
        .iter()
        .map(|(regime, count)| (regime.as_str(), *count))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    entries
        .into_iter()
        .take(4)
        .map(|(regime, count)| format!("{regime}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_focus_dashboard(
    report: &RuntimeFindingReport,
    manifest: &LiveRunManifest,
    validation: &LiveRunValidation,
    metrics: &RunTriageMetrics,
    original_focus: &str,
) -> String {
    let mut out = render_triage_summary(report, manifest, validation, metrics);
    let key_events = select_key_lines(original_focus, |line| line.starts_with("[EVENT]"), 4);
    let runtime_highlights = select_key_lines(
        original_focus,
        |line| {
            line.contains("[SLOW SEARCH]")
                || line.contains("[SEARCH TIMEOUT]")
                || (line.contains("[AUDIT]") && line.contains("agrees=false"))
                || (line.contains("[AUDIT]") && line.contains("skipped=true"))
        },
        8,
    );
    if !key_events.is_empty() {
        out.push_str("\nKey Event Trace:\n");
        for line in key_events {
            out.push_str(&format!("- {line}\n"));
        }
    }
    if !runtime_highlights.is_empty() {
        out.push_str("\nKey Runtime Highlights:\n");
        for line in runtime_highlights {
            out.push_str(&format!("- {line}\n"));
        }
    }
    if let Some(summary_box) = extract_last_combat_summary_box(original_focus) {
        out.push_str("\nLast Combat Summary:\n");
        out.push_str(&summary_box);
        out.push('\n');
    }
    if let Some(terminal_excerpt) = extract_terminal_excerpt(original_focus) {
        out.push_str("\nTerminal Section:\n");
        out.push_str(&terminal_excerpt);
        out.push('\n');
    }
    if original_focus.contains("=== CHRONOLOGICAL APPENDIX ===\n") {
        out.push_str("\nSee also: focus_appendix.txt for the full focused trace.\n");
    }
    out
}

fn select_key_lines<F>(text: &str, predicate: F, limit: usize) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && predicate(line))
        .take(limit)
        .map(|line| line.to_string())
        .collect()
}

fn extract_last_combat_summary_box(text: &str) -> Option<String> {
    let start = text.rfind("╔")?;
    let tail = &text[start..];
    let end = tail.find("╝")?;
    Some(tail[..end + "╝".len()].trim().to_string())
}

fn extract_terminal_excerpt(text: &str) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let idx = lines
        .iter()
        .position(|line| line.contains("=== GAME OVER ===") || line.contains("=== VICTORY ==="))?;
    let excerpt = lines[idx..(idx + 4).min(lines.len())].join("\n");
    Some(excerpt.trim().to_string())
}

fn write_triage_family_section(
    out: &mut String,
    title: &str,
    families: Vec<&RuntimeFindingFamily>,
) -> std::fmt::Result {
    use std::fmt::Write;

    writeln!(out, "\n{title}")?;
    if families.is_empty() {
        writeln!(out, "- none")?;
        return Ok(());
    }
    for family in families {
        let labels = if !family.combat_labels.is_empty() {
            family.combat_labels.join(", ")
        } else if !family.event_labels.is_empty() {
            family.event_labels.join(", ")
        } else {
            "n/a".to_string()
        };
        let frames = family
            .example_frames
            .iter()
            .map(|frame| frame.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "- {} (×{}) labels=[{}] frames=[{}] look_next={}",
            family.key,
            family.count,
            labels,
            frames,
            family.suggested_artifacts.join(", ")
        )?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct FocusRewriteOutcome {
    metrics: RunTriageMetrics,
    appendix_present: bool,
}

fn rewrite_archived_focus_with_triage_summary(
    focus_path: &Path,
    report: &RuntimeFindingReport,
    manifest: &LiveRunManifest,
    validation: &LiveRunValidation,
) -> Result<FocusRewriteOutcome, String> {
    let original = std::fs::read_to_string(focus_path)
        .map_err(|err| format!("failed to read focus '{}': {err}", focus_path.display()))?;
    let run_dir = focus_path.parent().unwrap_or_else(|| Path::new("."));
    let metrics = derive_run_triage_metrics(run_dir, &original);
    let appendix = original
        .split_once("=== CHRONOLOGICAL APPENDIX ===\n")
        .map(|(_, rest)| rest)
        .unwrap_or(&original)
        .trim_start_matches('\n')
        .trim();
    let appendix_present = !appendix.is_empty();
    if appendix_present {
        let appendix_path = run_dir.join("focus_appendix.txt");
        std::fs::write(&appendix_path, appendix).map_err(|err| {
            format!(
                "failed to write focus appendix '{}': {err}",
                appendix_path.display()
            )
        })?;
    }
    let rewritten = build_focus_dashboard(report, manifest, validation, &metrics, &original);
    std::fs::write(focus_path, rewritten)
        .map_err(|err| format!("failed to rewrite focus '{}': {err}", focus_path.display()))?;
    Ok(FocusRewriteOutcome {
        metrics,
        appendix_present,
    })
}

fn write_findings_report(path: &Path, report: &RuntimeFindingReport) -> Result<(), String> {
    let text = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize findings '{}': {err}", path.display()))?;
    std::fs::write(path, text)
        .map_err(|err| format!("failed to write findings '{}': {err}", path.display()))
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

pub(crate) fn finalize_live_run(
    paths: &LiveLogPaths,
    input: FinalizeRunInput,
) -> Result<FinalizeRunOutcome, String> {
    ensure_log_dirs(paths).map_err(|err| format!("failed to ensure log dirs: {err}"))?;
    let run_dir = paths.run_dir(&input.run_id);
    std::fs::create_dir_all(&run_dir)
        .map_err(|err| format!("failed to create run dir '{}': {err}", run_dir.display()))?;

    let classification_label = classify_run(&input);
    let is_clean = is_clean_label(&classification_label);
    let profile = load_profile_metadata();
    let is_engine_profile = profile
        .purpose
        .as_deref()
        .is_some_and(|purpose| purpose.eq_ignore_ascii_case("engine"));
    let retain_replay = matches!(
        classification_label.as_str(),
        "strict_fail" | "survey_tainted" | "loss_tainted" | "victory_tainted"
    );
    let raw_present = copy_if_exists(&paths.current_raw(), &run_dir.join("raw.jsonl"))?;
    let focus_present = copy_if_exists(&paths.current_focus(), &run_dir.join("focus.txt"))?;
    let signatures_present = if is_engine_profile && is_clean {
        false
    } else {
        copy_if_nonempty(
            &paths.current_signatures(),
            &run_dir.join("signatures.jsonl"),
        )?
    };
    let combat_suspects_present = copy_if_nonempty(
        &paths.current_combat_suspects(),
        &run_dir.join("combat_suspects.jsonl"),
    )?;
    let snapshot_split = split_failure_snapshot_artifacts(
        &paths.current_failure_snapshots(),
        &run_dir.join("failure_snapshots.jsonl"),
        &run_dir.join("terminal_snapshot.json"),
    )?;
    let failure_snapshots_present = snapshot_split.failure_snapshots_present;
    let terminal_snapshot_present = snapshot_split.terminal_snapshot_present;
    let debug_present = copy_if_exists(&paths.current_debug(), &run_dir.join("debug.txt"))?;
    let reward_audit_present = copy_if_nonempty(
        &paths.current_reward_audit(),
        &run_dir.join("reward_audit.jsonl"),
    )?;
    let event_audit_present = copy_if_nonempty(
        &paths.current_event_audit(),
        &run_dir.join("event_audit.jsonl"),
    )?;
    let combat_decision_audit_present = copy_if_nonempty(
        &paths.current_combat_decision_audit(),
        &run_dir.join("combat_decision_audit.jsonl"),
    )?;
    let human_noncombat_audit_present = if is_engine_profile {
        false
    } else {
        copy_if_nonempty(
            &paths.current_human_noncombat_audit(),
            &run_dir.join("human_noncombat_audit.jsonl"),
        )?
    };
    let sidecar_shadow_present = if is_engine_profile {
        false
    } else {
        copy_if_nonempty(
            &paths.current_sidecar_shadow(),
            &run_dir.join("sidecar_shadow.jsonl"),
        )?
    };
    let watch_audit_present = if input.watch_enabled && !is_engine_profile {
        copy_if_nonempty(
            &paths.current_watch_audit(),
            &run_dir.join("watch_audit.jsonl"),
        )?
    } else {
        false
    };
    let watch_noncombat_present = if input.watch_enabled && !is_engine_profile {
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
    let findings_path = run_dir.join("findings.json");
    let report = if failure_snapshots_present {
        let report = build_finding_report(
            &input.run_id,
            &classification_label,
            &LiveRunCounts {
                engine_bugs: input.engine_bug_total,
                content_gaps: input.content_gap_total,
                timing_diffs: input.timing_diff_total,
                replay_failures: input.replay_failures,
            },
            &run_dir.join("failure_snapshots.jsonl"),
        )?;
        report
    } else {
        build_empty_finding_report(
            &input.run_id,
            &classification_label,
            &LiveRunCounts {
                engine_bugs: input.engine_bug_total,
                content_gaps: input.content_gap_total,
                timing_diffs: input.timing_diff_total,
                replay_failures: input.replay_failures,
            },
        )
    };
    write_findings_report(&findings_path, &report)?;
    let findings_present = true;

    let manifest = LiveRunManifest {
        run_id: input.run_id.clone(),
        timestamp: input.timestamp.clone(),
        build_tag: input.build_tag,
        parity_mode: input.parity_mode,
        watch_enabled: input.watch_enabled,
        session_exit_reason: input.session_exit_reason,
        classification_label: classification_label.clone(),
        profile,
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
            focus_appendix: Some(LiveArtifactRecord::new("focus_appendix.txt", false)),
            findings: Some(LiveArtifactRecord::new("findings.json", findings_present)),
            bot_strength: Some(LiveArtifactRecord::new("bot_strength.json", false)),
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
            terminal_snapshot: Some(LiveArtifactRecord::new(
                "terminal_snapshot.json",
                terminal_snapshot_present,
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
            combat_decision_audit: Some(LiveArtifactRecord::new(
                "combat_decision_audit.jsonl",
                combat_decision_audit_present,
            )),
            human_noncombat_audit: Some(LiveArtifactRecord::new(
                "human_noncombat_audit.jsonl",
                human_noncombat_audit_present,
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
    let focus_rewrite = if focus_present {
        Some(rewrite_archived_focus_with_triage_summary(
            &run_dir.join("focus.txt"),
            &report,
            &manifest,
            &validation,
        )?)
    } else {
        None
    };
    if let Some(focus_rewrite) = &focus_rewrite {
        manifest.artifacts.focus_appendix = Some(LiveArtifactRecord::new(
            "focus_appendix.txt",
            focus_rewrite.appendix_present,
        ));
    }
    let metrics = if let Some(focus_rewrite) = &focus_rewrite {
        focus_rewrite.metrics.clone()
    } else {
        derive_run_triage_metrics(&run_dir, "")
    };
    let bot_strength = build_bot_strength_summary(&report, &manifest, &validation, &metrics);
    let bot_strength_path = run_dir.join("bot_strength.json");
    let bot_strength_text = serde_json::to_string_pretty(&bot_strength)
        .map_err(|err| format!("failed to serialize bot strength summary: {err}"))?;
    std::fs::write(&bot_strength_path, bot_strength_text).map_err(|err| {
        format!(
            "failed to write bot strength summary '{}': {err}",
            bot_strength_path.display()
        )
    })?;
    manifest.artifacts.bot_strength = Some(LiveArtifactRecord::new("bot_strength.json", true));
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
    if !std::fs::metadata(source)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
    {
        return Ok(false);
    }
    copy_if_exists(source, target)
}

#[derive(Clone, Debug, Default)]
struct ArchivedSnapshotSplit {
    failure_snapshots_present: bool,
    terminal_snapshot_present: bool,
}

fn split_failure_snapshot_artifacts(
    source: &Path,
    failure_target: &Path,
    terminal_target: &Path,
) -> Result<ArchivedSnapshotSplit, String> {
    if !std::fs::metadata(source)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
    {
        return Ok(ArchivedSnapshotSplit::default());
    }

    let text = std::fs::read_to_string(source)
        .map_err(|err| format!("failed to read '{}': {err}", source.display()))?;
    let mut parity_lines = Vec::new();
    let mut terminal_snapshot: Option<Value> = None;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root = serde_json::from_str::<Value>(trimmed).map_err(|err| {
            format!(
                "invalid jsonl in '{}' at line {}: {err}",
                source.display(),
                idx + 1
            )
        })?;
        if root
            .get("trigger_kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "terminal_loss")
        {
            terminal_snapshot = Some(root);
        } else {
            parity_lines.push(trimmed.to_string());
        }
    }

    let mut split = ArchivedSnapshotSplit::default();
    if !parity_lines.is_empty() {
        let text = format!("{}\n", parity_lines.join("\n"));
        std::fs::write(failure_target, text).map_err(|err| {
            format!(
                "failed to write failure snapshots '{}': {err}",
                failure_target.display()
            )
        })?;
        split.failure_snapshots_present = true;
    }
    if let Some(snapshot) = terminal_snapshot {
        let text = serde_json::to_string_pretty(&snapshot)
            .map_err(|err| format!("failed to serialize terminal snapshot: {err}"))?;
        std::fs::write(terminal_target, text).map_err(|err| {
            format!(
                "failed to write terminal snapshot '{}': {err}",
                terminal_target.display()
            )
        })?;
        split.terminal_snapshot_present = true;
    }
    Ok(split)
}

pub(crate) fn verify_replay_counts(replay_path: &Path) -> Result<(usize, usize), String> {
    let replay = load_live_session_replay(replay_path)?;
    let view = derive_replay_view(&replay);
    let report = verify_replay_view(&view, false)?;
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

    #[test]
    fn parse_diff_entry_handles_power_paths() {
        let parsed = parse_diff_entry("player.power[Strength].amount [ENGINE_BUG] Rust=9 Java=5")
            .expect("diff should parse");
        assert_eq!(parsed.field, "player.power[Strength].amount");
        assert_eq!(parsed.category, "engine_bug");
        assert_eq!(parsed.rust, "9");
        assert_eq!(parsed.java, "5");
    }

    #[test]
    fn build_finding_report_groups_engine_and_validation_families() {
        let temp_root = std::env::temp_dir().join(format!(
            "live_comm_findings_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("temp root should be creatable");
        let snapshots_path = temp_root.join("failure_snapshots.jsonl");
        let content = concat!(
            "{\"snapshot_id\":\"snap_engine\",\"frame\":352,\"response_id\":352,\"state_frame_id\":352,\"screen\":\"NONE\",\"room_phase\":\"COMBAT\",\"room_type\":\"MonsterRoom\",\"trigger_kind\":\"engine_bug\",\"reasons\":[\"combat_action_diff\",\"count=1\"],\"normalized_state\":{\"monsters\":[{\"id\":\"Darkling\",\"name\":\"Darkling\",\"current_hp\":50}],\"screen_state\":{}},\"decision_context\":{\"diffs\":[\"player.power[Strength].amount [ENGINE_BUG] Rust=3 Java=2\"]},\"protocol_context\":{}}\n",
            "{\"snapshot_id\":\"snap_validation\",\"frame\":2,\"response_id\":2,\"state_frame_id\":2,\"screen\":\"EVENT\",\"room_phase\":\"EVENT\",\"room_type\":\"EventRoom\",\"trigger_kind\":\"validation_failure\",\"reasons\":[\"compatibility_fallback\",\"event_screen_semantics_incomplete\"],\"normalized_state\":{\"monsters\":[],\"screen_state\":{\"event_name\":\"Neow\"}},\"decision_context\":{},\"protocol_context\":{}}\n",
            "{\"snapshot_id\":\"snap_protocol_screen\",\"frame\":9,\"response_id\":9,\"state_frame_id\":9,\"screen\":\"GRID\",\"room_phase\":\"COMBAT\",\"room_type\":\"MonsterRoom\",\"trigger_kind\":\"validation_failure\",\"reasons\":[\"missing_screen_action_space:combat_action_space\"],\"normalized_state\":{\"monsters\":[],\"screen_state\":{}},\"decision_context\":{\"validation\":\"screen_action_space\",\"expected_field\":\"combat_action_space\",\"screen\":\"GRID\",\"room_phase\":\"COMBAT\",\"available_commands\":[\"choose\"]},\"protocol_context\":{}}\n"
        );
        std::fs::write(&snapshots_path, content).expect("snapshots should write");

        let report = build_finding_report(
            "run_x",
            "survey_tainted",
            &LiveRunCounts {
                engine_bugs: 1,
                content_gaps: 0,
                timing_diffs: 0,
                replay_failures: 0,
            },
            &snapshots_path,
        )
        .expect("report should build");

        let strength = report
            .families
            .iter()
            .find(|family| {
                family.category == "engine_bug" && family.key == "player.power[Strength].amount"
            })
            .expect("strength family should exist");
        assert_eq!(strength.count, 1);
        assert_eq!(strength.combat_labels, vec!["Darkling"]);

        let fallback = report
            .families
            .iter()
            .find(|family| {
                family.category == "validation_failure" && family.key == "compatibility_fallback"
            })
            .expect("compatibility family should exist");
        assert_eq!(fallback.count, 1);
        assert_eq!(fallback.event_labels, vec!["Neow"]);

        let protocol_screen = report
            .families
            .iter()
            .find(|family| {
                family.category == "protocol_screen_action_space"
                    && family.key == "missing_screen_action_space:combat_action_space:screen=GRID"
            })
            .expect("protocol screen action-space family should exist");
        assert_eq!(protocol_screen.count, 1);
        assert_eq!(protocol_screen.event_labels, vec!["GRID"]);
        assert_eq!(
            protocol_screen.suggested_artifacts,
            vec![
                "failure_snapshots.jsonl".to_string(),
                "debug.txt".to_string(),
                "raw.jsonl".to_string()
            ]
        );

        let manifest = LiveRunManifest {
            run_id: "run_x".to_string(),
            timestamp: "20260420_000000".to_string(),
            build_tag: "build".to_string(),
            parity_mode: "Survey".to_string(),
            watch_enabled: false,
            session_exit_reason: "STDIN_EOF".to_string(),
            classification_label: "survey_tainted".to_string(),
            profile: LiveProfileMetadata::default(),
            provenance: LiveRunProvenance::default(),
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts::default(),
            validation: None,
            retention: LiveRetentionFlags::default(),
        };
        let summary = render_triage_summary(
            &report,
            &manifest,
            &LiveRunValidation {
                status: "ok".to_string(),
                ..LiveRunValidation::default()
            },
            &RunTriageMetrics::default(),
        );
        assert!(summary.contains("Top Protocol Screen Action-Space Families:"));
        assert!(summary.contains("missing_screen_action_space:combat_action_space:screen=GRID"));

        let _ = std::fs::remove_file(&snapshots_path);
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn render_triage_summary_surfaces_validation_gaps_for_clean_runs() {
        let report = build_empty_finding_report(
            "run_clean",
            "survey_clean",
            &LiveRunCounts {
                engine_bugs: 0,
                content_gaps: 0,
                timing_diffs: 0,
                replay_failures: 0,
            },
        );
        let manifest = LiveRunManifest {
            run_id: "run_clean".to_string(),
            timestamp: "20260420_000000".to_string(),
            build_tag: "build".to_string(),
            parity_mode: "Strict".to_string(),
            watch_enabled: false,
            session_exit_reason: "STDIN_EOF".to_string(),
            classification_label: "survey_clean".to_string(),
            profile: LiveProfileMetadata {
                profile_name: Some("Ironclad_Engine_Strict.json".to_string()),
                purpose: Some("engine".to_string()),
                capture_policy: Some("strict_stop_on_first_parity_fail".to_string()),
            },
            provenance: LiveRunProvenance {
                git_short_sha: Some("abc1234".to_string()),
                binary_is_fresh: Some(true),
                ..LiveRunProvenance::default()
            },
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts::default(),
            validation: None,
            retention: LiveRetentionFlags::default(),
        };
        let validation = LiveRunValidation {
            status: "trace_incomplete".to_string(),
            trace_incomplete: true,
            errors: vec![
                "event trace present but screen_index/screen_source fields are missing".to_string(),
            ],
            ..LiveRunValidation::default()
        };
        let metrics = RunTriageMetrics::default();

        let summary = render_triage_summary(&report, &manifest, &validation, &metrics);

        assert!(summary.contains("Combat Parity: clean | Validation: trace_incomplete"));
        assert!(summary
            .contains("event trace present but screen_index/screen_source fields are missing"));
        assert!(summary.contains("validation.json"));
        assert!(summary.contains("event_audit.jsonl"));
    }

    #[test]
    fn derive_run_triage_metrics_surfaces_progression_and_quality_signals() {
        let temp_root = std::env::temp_dir().join(format!(
            "live_comm_metrics_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("temp root should be creatable");
        std::fs::write(
            temp_root.join("raw.jsonl"),
            concat!(
                "{\"game_state\":{\"floor\":16,\"act\":1,\"room_type\":\"MonsterRoomBoss\"}}\n",
                "{\"game_state\":{\"floor\":33,\"act\":2,\"room_type\":\"MonsterRoomBoss\"}}\n"
            ),
        )
        .expect("raw should write");
        std::fs::write(
            temp_root.join("failure_snapshots.jsonl"),
            "{\"trigger_kind\":\"terminal_loss\"}\n",
        )
        .expect("failure snapshots should write");
        let focus_text = concat!(
            "[EVENT] frame=1 Neow | choose=Talk score=18 fallback | protocol=unsupported_event\n",
            "[SLOW SEARCH] frame=420 baseline_ms=0 root_ms=709 legal_moves=6 chosen=Play #4 Warcry+\n",
            "[AUDIT] exact_turn best=PlayCard { card_index: 3, target: None } line_len=2 ends=8 nodes=7 prunes=0 cycles=0 truncated=false agrees=false regime=crisis dominance=strictly_better_in_window confidence=exact takeover=true takeover_reason=crisis_strict_dominance frontier_survival=severe_risk exact_survival=stable resources=hp80/blk12/pots0/lost0/exh0\n",
            "[AUDIT] exact_turn skipped=true reason=high_root_branching legal_moves=12 living_monsters=3 filled_potions=0 regime=fragile dominance=incomparable confidence=unavailable takeover=false takeover_reason=no_best_first_input frontier_survival=risky_but_playable exact_survival=risky_but_playable\n",
            "[SEARCH TIMEOUT] frame=435 root_search partial_result budget=1200 elapsed_ms=4\n"
        );

        let metrics = derive_run_triage_metrics(&temp_root, focus_text);

        assert_eq!(metrics.highest_floor, Some(33));
        assert_eq!(metrics.highest_act, Some(2));
        assert_eq!(
            metrics.deepest_room_type.as_deref(),
            Some("MonsterRoomBoss")
        );
        assert_eq!(metrics.reached_boss_acts, vec![1, 2]);
        assert_eq!(metrics.slow_search_count, 1);
        assert_eq!(metrics.slow_search_max_ms, 709);
        assert_eq!(metrics.exact_turn_disagree_count, 1);
        assert_eq!(metrics.exact_turn_skip_count, 1);
        assert_eq!(metrics.exact_turn_takeover_count, 1);
        assert_eq!(metrics.strict_dominance_disagreement_count, 1);
        assert_eq!(metrics.high_threat_disagreement_count, 1);
        assert_eq!(metrics.survival_downgrade_avoided_count, 1);
        assert_eq!(metrics.regime_counts.get("crisis"), Some(&1));
        assert_eq!(metrics.regime_counts.get("fragile"), Some(&1));
        assert_eq!(metrics.search_timeout_count, 1);
        assert_eq!(metrics.event_fallback_count, 1);
        assert!(metrics.terminal_snapshot_present);

        let _ = std::fs::remove_file(temp_root.join("raw.jsonl"));
        let _ = std::fs::remove_file(temp_root.join("failure_snapshots.jsonl"));
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn build_bot_strength_summary_includes_phase1_decision_fields() {
        let report =
            build_empty_finding_report("run_strength", "loss_clean", &LiveRunCounts::default());
        let manifest = LiveRunManifest {
            run_id: "run_strength".to_string(),
            timestamp: "20260420_000000".to_string(),
            build_tag: "build".to_string(),
            parity_mode: "Strict".to_string(),
            watch_enabled: false,
            session_exit_reason: "GAME_OVER".to_string(),
            classification_label: "loss_clean".to_string(),
            profile: LiveProfileMetadata {
                profile_name: Some("Ironclad_Engine_Strict.json".to_string()),
                purpose: Some("engine".to_string()),
                capture_policy: Some("strict_stop_on_first_parity_fail".to_string()),
            },
            provenance: LiveRunProvenance::default(),
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts::default(),
            validation: None,
            retention: LiveRetentionFlags::default(),
        };
        let validation = LiveRunValidation {
            status: "ok".to_string(),
            ..LiveRunValidation::default()
        };
        let mut metrics = RunTriageMetrics::default();
        metrics.exact_turn_takeover_count = 2;
        metrics.strict_dominance_disagreement_count = 3;
        metrics.high_threat_disagreement_count = 1;
        metrics.survival_downgrade_avoided_count = 2;
        metrics.regime_counts.insert("crisis".to_string(), 4);
        metrics.regime_counts.insert("fragile".to_string(), 2);

        let summary = build_bot_strength_summary(&report, &manifest, &validation, &metrics);

        assert_eq!(summary.exact_turn_takeover_count, 2);
        assert_eq!(summary.strict_dominance_disagreement_count, 3);
        assert_eq!(summary.high_threat_disagreement_count, 1);
        assert_eq!(summary.survival_downgrade_avoided_count, 2);
        assert_eq!(summary.regime_counts.get("crisis"), Some(&4));
        assert_eq!(summary.regime_counts.get("fragile"), Some(&2));
    }

    #[test]
    fn split_failure_snapshot_artifacts_separates_terminal_snapshot() {
        let temp_root = std::env::temp_dir().join(format!(
            "live_comm_split_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("temp root should be creatable");
        let source = temp_root.join("source.jsonl");
        std::fs::write(
            &source,
            concat!(
                "{\"trigger_kind\":\"engine_bug\",\"frame\":10}\n",
                "{\"trigger_kind\":\"terminal_loss\",\"frame\":11,\"screen\":\"GAME_OVER\"}\n"
            ),
        )
        .expect("source should write");

        let split = split_failure_snapshot_artifacts(
            &source,
            &temp_root.join("failure_snapshots.jsonl"),
            &temp_root.join("terminal_snapshot.json"),
        )
        .expect("split should succeed");

        assert!(split.failure_snapshots_present);
        assert!(split.terminal_snapshot_present);
        let failure_text = std::fs::read_to_string(temp_root.join("failure_snapshots.jsonl"))
            .expect("failure snapshots should read");
        assert!(failure_text.contains("\"engine_bug\""));
        assert!(!failure_text.contains("\"terminal_loss\""));
        let terminal_text = std::fs::read_to_string(temp_root.join("terminal_snapshot.json"))
            .expect("terminal snapshot should read");
        assert!(terminal_text.contains("\"terminal_loss\""));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn rewrite_archived_focus_moves_appendix_into_separate_file() {
        let temp_root = std::env::temp_dir().join(format!(
            "live_comm_focus_rewrite_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("temp root should be creatable");
        std::fs::write(
            temp_root.join("raw.jsonl"),
            "{\"game_state\":{\"floor\":16,\"act\":1,\"room_type\":\"MonsterRoomBoss\"}}\n",
        )
        .expect("raw should write");
        std::fs::write(
            temp_root.join("terminal_snapshot.json"),
            "{\"trigger_kind\":\"terminal_loss\",\"frame\":99,\"screen\":\"GAME_OVER\",\"room_type\":\"MonsterRoomBoss\",\"normalized_state\":{\"floor\":16,\"act\":1,\"monsters\":[{\"name\":\"Slime Boss\"}]},\"decision_context\":{\"score\":123,\"victory\":false}}\n",
        )
        .expect("terminal snapshot should write");
        let focus_path = temp_root.join("focus.txt");
        std::fs::write(
            &focus_path,
            concat!(
                "=== CHRONOLOGICAL APPENDIX ===\n",
                "[EVENT] frame=2 Neow | choose=Talk score=18 fallback | protocol=unsupported_event\n",
                "[SLOW SEARCH] frame=8 baseline_ms=0 root_ms=709 legal_moves=6 chosen=Play #1 Defend\n",
                "╔══════════════════════════════════════════════════════╗\n",
                "║  COMBAT SUMMARY (F227 ~ F234)                          \n",
                "╚══════════════════════════════════════════════════════╝\n",
                "[F234] === GAME OVER === victory=false score=123\n"
            ),
        )
        .expect("focus should write");

        let report =
            build_empty_finding_report("run_focus", "loss_clean", &LiveRunCounts::default());
        let manifest = LiveRunManifest {
            run_id: "run_focus".to_string(),
            timestamp: "20260420_000000".to_string(),
            build_tag: "build".to_string(),
            parity_mode: "Strict".to_string(),
            watch_enabled: false,
            session_exit_reason: "GAME_OVER".to_string(),
            classification_label: "loss_clean".to_string(),
            profile: LiveProfileMetadata {
                profile_name: Some("Ironclad_Engine_Strict.json".to_string()),
                purpose: Some("engine".to_string()),
                capture_policy: Some("strict_stop_on_first_parity_fail".to_string()),
            },
            provenance: LiveRunProvenance::default(),
            counts: LiveRunCounts::default(),
            artifacts: LiveRunArtifacts::default(),
            validation: None,
            retention: LiveRetentionFlags::default(),
        };
        let validation = LiveRunValidation {
            status: "ok".to_string(),
            ..LiveRunValidation::default()
        };

        let rewrite = rewrite_archived_focus_with_triage_summary(
            &focus_path,
            &report,
            &manifest,
            &validation,
        )
        .expect("focus rewrite should succeed");

        assert!(rewrite.appendix_present);
        let rewritten = std::fs::read_to_string(&focus_path).expect("rewritten focus should read");
        assert!(rewritten.contains("=== TRIAGE SUMMARY ==="));
        assert!(rewritten.contains("See also: focus_appendix.txt"));
        let appendix = std::fs::read_to_string(temp_root.join("focus_appendix.txt"))
            .expect("focus appendix should read");
        assert!(appendix.contains("[EVENT]"));
        assert!(appendix.contains("[F234] === GAME OVER ==="));

        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
