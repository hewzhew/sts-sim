use crate::diff::replay::live_comm_replay::{
    build_live_session_replay_from_raw_path, write_live_session_replay_to_path,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::live_comm_manifest::{
    artifact_absolute_path, artifact_present, ensure_log_dirs, is_clean_label, list_run_manifests,
    load_manifest_for_run, manifest_path_for_run, remove_run_artifact, rewrite_manifest,
    LiveArtifactRecord, LiveLogPaths, LiveRunManifest,
};

const MAX_CLEAN_CANONICAL_RUNS: usize = 20;
const MAX_CLEAN_DEBUG_RUNS: usize = 10;
const MAX_WATCH_SIDECAR_RUNS: usize = 5;

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
    let mut manifest = load_manifest_for_run(&manifest_path)?;
    manifest.retention.pinned = pinned;
    rewrite_manifest(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

pub fn regenerate_run_replay(paths: &LiveLogPaths, run_id: &str) -> Result<PathBuf, String> {
    let manifest_path = manifest_path_for_run(paths, run_id)?;
    let mut manifest = load_manifest_for_run(&manifest_path)?;
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
