use super::live_comm_admin::{
    ensure_log_dirs, gc_runs, is_clean_label, rewrite_manifest, write_manifest, LiveArtifactRecord,
    LiveLogPaths, LiveProfileMetadata, LiveRetentionFlags, LiveRunArtifacts, LiveRunCounts,
    LiveRunManifest, LiveRunProvenance, LiveRunValidation,
};
use crate::diff::replay::{derive_combat_replay_view, verify_combat_replay_view};
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
            .and_then(|state| state.get("screen_type").or_else(|| state.get("screen_name")))
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
            *multistage_counts.entry(event_name.to_string()).or_insert(0usize) += 1;
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
        "validation_failure" => vec!["event_audit.jsonl".to_string(), "debug.txt".to_string()],
        _ => vec!["debug.txt".to_string()],
    }
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
                    let family_key = ("validation_failure".to_string(), reason.clone());
                    let builder = families.entry(family_key).or_insert_with(|| {
                        RuntimeFindingFamilyBuilder::new(
                            "validation_failure",
                            reason,
                            snapshot.frame,
                        )
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
    let report = build_finding_report(
        run_id,
        classification_label,
        counts,
        failure_snapshots_path,
    )?;
    serde_json::to_value(report)
        .map_err(|err| format!("failed to convert findings report to json: {err}"))
}

fn category_sort_key(category: &str) -> u8 {
    match category {
        "engine_bug" => 0,
        "content_gap" => 1,
        "timing" => 2,
        "validation_failure" => 3,
        _ => 4,
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

fn render_triage_summary(report: &RuntimeFindingReport) -> String {
    let mut out = String::new();
    out.push_str("=== TRIAGE SUMMARY ===\n");
    out.push_str(&format!(
        "Run: {} | Classification: {}\n",
        report.run_id, report.classification_label
    ));
    out.push_str(&format!(
        "Counts: engine_bugs={} content_gaps={} timing_diffs={} replay_failures={}\n",
        report.counts.engine_bugs,
        report.counts.content_gaps,
        report.counts.timing_diffs,
        report.counts.replay_failures
    ));

    let mut write_family_section =
        |title: &str, families: Vec<&RuntimeFindingFamily>| -> std::fmt::Result {
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
        };

    let top_engine = findings_for_category(report, "engine_bug")
        .take(5)
        .collect::<Vec<_>>();
    let top_validation = findings_for_category(report, "validation_failure")
        .take(5)
        .collect::<Vec<_>>();
    let top_gaps = findings_for_category(report, "content_gap")
        .take(5)
        .collect::<Vec<_>>();

    let _ = write_family_section("Top Engine Bug Families:", top_engine);
    let _ = write_family_section("Top Validation Failure Families:", top_validation);
    let _ = write_family_section("Top Content Gap Families:", top_gaps);

    out.push_str(
        "\nWhere to look next:\n- combat engine bugs: debug.txt, failure_snapshots.jsonl, replay.json\n- validation failures: event_audit.jsonl, debug.txt\n",
    );
    out.push_str("\n=== CHRONOLOGICAL APPENDIX ===\n");
    out
}

fn rewrite_archived_focus_with_triage_summary(
    focus_path: &Path,
    report: &RuntimeFindingReport,
) -> Result<(), String> {
    let original = std::fs::read_to_string(focus_path)
        .map_err(|err| format!("failed to read focus '{}': {err}", focus_path.display()))?;
    let appendix = original
        .split_once("=== CHRONOLOGICAL APPENDIX ===\n")
        .map(|(_, rest)| rest)
        .unwrap_or(&original)
        .trim_start_matches('\n');
    let rewritten = format!("{}\n{}", render_triage_summary(report), appendix);
    std::fs::write(focus_path, rewritten)
        .map_err(|err| format!("failed to rewrite focus '{}': {err}", focus_path.display()))
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
    let findings_path = run_dir.join("findings.json");
    let findings_present = if failure_snapshots_present {
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
        write_findings_report(&findings_path, &report)?;
        if focus_present {
            rewrite_archived_focus_with_triage_summary(&run_dir.join("focus.txt"), &report)?;
        }
        true
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
            findings: Some(LiveArtifactRecord::new("findings.json", findings_present)),
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

pub(crate) fn verify_replay_counts(replay_path: &Path) -> Result<(usize, usize), String> {
    let replay = crate::diff::replay::load_live_session_replay_path(replay_path)?;
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

    #[test]
    fn parse_diff_entry_handles_power_paths() {
        let parsed = parse_diff_entry(
            "player.power[Strength].amount [ENGINE_BUG] Rust=9 Java=5",
        )
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
            "{\"snapshot_id\":\"snap_validation\",\"frame\":2,\"response_id\":2,\"state_frame_id\":2,\"screen\":\"EVENT\",\"room_phase\":\"EVENT\",\"room_type\":\"EventRoom\",\"trigger_kind\":\"validation_failure\",\"reasons\":[\"compatibility_fallback\",\"event_screen_semantics_incomplete\"],\"normalized_state\":{\"monsters\":[],\"screen_state\":{\"event_name\":\"Neow\"}},\"decision_context\":{},\"protocol_context\":{}}\n"
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
                family.category == "engine_bug"
                    && family.key == "player.power[Strength].amount"
            })
            .expect("strength family should exist");
        assert_eq!(strength.count, 1);
        assert_eq!(strength.combat_labels, vec!["Darkling"]);

        let fallback = report
            .families
            .iter()
            .find(|family| {
                family.category == "validation_failure"
                    && family.key == "compatibility_fallback"
            })
            .expect("compatibility family should exist");
        assert_eq!(fallback.count, 1);
        assert_eq!(fallback.event_labels, vec!["Neow"]);

        let _ = std::fs::remove_file(&snapshots_path);
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
