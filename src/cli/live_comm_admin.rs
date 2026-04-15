use crate::diff::replay::live_comm_replay::{
    build_live_session_replay_from_raw_path, write_live_session_replay_to_path,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const LOG_ROOT: &str = r"d:\rust\sts_simulator\logs";
pub const CURRENT_ROOT: &str = r"d:\rust\sts_simulator\logs\current";
pub const RUNS_ROOT: &str = r"d:\rust\sts_simulator\logs\runs";
pub const CURRENT_MANIFEST_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_manifest.json";

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
    pub(crate) fn new(relative_path: impl Into<String>, present: bool) -> Self {
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

    pub(crate) fn current_raw(&self) -> PathBuf {
        self.current.join("live_comm_raw.jsonl")
    }

    pub(crate) fn current_debug(&self) -> PathBuf {
        self.current.join("live_comm_debug.txt")
    }

    pub(crate) fn current_focus(&self) -> PathBuf {
        self.current.join("live_comm_focus.txt")
    }

    pub(crate) fn current_signatures(&self) -> PathBuf {
        self.current.join("live_comm_signatures.jsonl")
    }

    pub(crate) fn current_replay(&self) -> PathBuf {
        self.current.join("live_comm_replay.json")
    }

    pub(crate) fn current_reward_audit(&self) -> PathBuf {
        self.current.join("live_comm_reward_audit.jsonl")
    }

    pub(crate) fn current_event_audit(&self) -> PathBuf {
        self.current.join("live_comm_event_audit.jsonl")
    }

    pub(crate) fn current_sidecar_shadow(&self) -> PathBuf {
        self.current.join("live_comm_sidecar_shadow.jsonl")
    }

    pub(crate) fn current_validation(&self) -> PathBuf {
        self.current.join("live_comm_validation.json")
    }

    pub(crate) fn current_watch_audit(&self) -> PathBuf {
        self.current.join("live_comm_watch_audit.jsonl")
    }

    pub(crate) fn current_watch_noncombat(&self) -> PathBuf {
        self.current.join("live_comm_watch_noncombat.jsonl")
    }

    pub(crate) fn current_combat_suspects(&self) -> PathBuf {
        self.current.join("live_comm_combat_suspects.jsonl")
    }

    pub(crate) fn current_failure_snapshots(&self) -> PathBuf {
        self.current.join("live_comm_failure_snapshots.jsonl")
    }

    pub(crate) fn current_manifest(&self) -> PathBuf {
        PathBuf::from(CURRENT_MANIFEST_PATH)
    }

    pub(crate) fn run_dir(&self, run_id: &str) -> PathBuf {
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

pub(crate) fn ensure_log_dirs(paths: &LiveLogPaths) -> std::io::Result<()> {
    std::fs::create_dir_all(&paths.root)?;
    std::fs::create_dir_all(&paths.current)?;
    std::fs::create_dir_all(&paths.runs)?;
    Ok(())
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

pub(crate) fn is_clean_label(label: &str) -> bool {
    matches!(
        label,
        "strict_ok" | "survey_clean" | "loss_clean" | "victory_clean"
    )
}

pub(crate) fn file_nonempty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

pub(crate) fn write_manifest(path: &Path, manifest: &LiveRunManifest) -> Result<PathBuf, String> {
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

pub(crate) fn rewrite_manifest(path: &Path, manifest: &LiveRunManifest) -> Result<(), String> {
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
