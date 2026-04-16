use std::path::PathBuf;

use super::live_comm_manifest::{
    artifact_absolute_path, file_nonempty, list_run_manifests, LiveLogPaths, LiveRunValidation,
};

pub const LOG_ROOT: &str = r"d:\rust\sts_simulator\logs";
pub const CURRENT_ROOT: &str = r"d:\rust\sts_simulator\logs\current";
pub const RUNS_ROOT: &str = r"d:\rust\sts_simulator\logs\runs";
pub const CURRENT_MANIFEST_PATH: &str =
    r"d:\rust\sts_simulator\logs\current\live_comm_manifest.json";

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
            "findings" => artifact_absolute_path(&manifest_path, &manifest.artifacts.findings),
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
