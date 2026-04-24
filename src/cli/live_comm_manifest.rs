use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::live_comm_paths::{CURRENT_MANIFEST_PATH, CURRENT_ROOT, LOG_ROOT, RUNS_ROOT};

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
    pub focus_appendix: Option<LiveArtifactRecord>,
    pub findings: Option<LiveArtifactRecord>,
    pub bot_strength: Option<LiveArtifactRecord>,
    pub signatures: Option<LiveArtifactRecord>,
    pub combat_suspects: Option<LiveArtifactRecord>,
    pub failure_snapshots: Option<LiveArtifactRecord>,
    pub terminal_snapshot: Option<LiveArtifactRecord>,
    pub debug: Option<LiveArtifactRecord>,
    pub replay: Option<LiveArtifactRecord>,
    pub reward_audit: Option<LiveArtifactRecord>,
    pub event_audit: Option<LiveArtifactRecord>,
    pub combat_decision_audit: Option<LiveArtifactRecord>,
    pub human_noncombat_audit: Option<LiveArtifactRecord>,
    pub sidecar_shadow: Option<LiveArtifactRecord>,
    pub validation: Option<LiveArtifactRecord>,
    pub watch_audit: Option<LiveArtifactRecord>,
    pub watch_noncombat: Option<LiveArtifactRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LiveRunProvenance {
    pub exe_path: Option<String>,
    pub exe_mtime_utc: Option<String>,
    pub git_short_sha: Option<String>,
    pub repo_head_short_sha: Option<String>,
    pub binary_matches_head: Option<bool>,
    pub binary_is_fresh: Option<bool>,
    pub build_unix: Option<u64>,
    pub build_time_utc: Option<String>,
    pub source_inputs_latest_path: Option<String>,
    pub source_inputs_latest_mtime_utc: Option<String>,
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

    pub(crate) fn current_combat_decision_audit(&self) -> PathBuf {
        self.current.join("live_comm_combat_decision_audit.jsonl")
    }

    pub(crate) fn current_human_noncombat_audit(&self) -> PathBuf {
        self.current.join("live_comm_human_noncombat_audit.jsonl")
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

pub(crate) fn ensure_log_dirs(paths: &LiveLogPaths) -> std::io::Result<()> {
    std::fs::create_dir_all(&paths.root)?;
    std::fs::create_dir_all(&paths.current)?;
    std::fs::create_dir_all(&paths.runs)?;
    Ok(())
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

pub(crate) fn list_run_manifests(
    paths: &LiveLogPaths,
) -> Result<Vec<(PathBuf, LiveRunManifest)>, String> {
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

pub(crate) fn manifest_path_for_run(paths: &LiveLogPaths, run_id: &str) -> Result<PathBuf, String> {
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

pub(crate) fn load_manifest_for_run(path: &Path) -> Result<LiveRunManifest, String> {
    load_manifest(path)
}

pub(crate) fn artifact_absolute_path(
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

pub(crate) fn artifact_present(record: &Option<LiveArtifactRecord>) -> bool {
    record.as_ref().is_some_and(|record| record.present)
}

pub(crate) fn remove_run_artifact(
    manifest_path: &Path,
    record: &mut Option<LiveArtifactRecord>,
) -> bool {
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
