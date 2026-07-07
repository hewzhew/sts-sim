use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{ArtifactRef, PanelRunMode, PanelSeedAction, PanelSeedArtifacts, PanelSummary};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchArtifactStore {
    capsule_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PanelLedgerEvent {
    pub schema: &'static str,
    pub seed: u64,
    pub scheduler_action: PanelSeedAction,
    pub event: String,
    pub run_mode: Option<PanelRunMode>,
    pub slice_index: Option<usize>,
    pub error: Option<String>,
    pub artifact_refs: Vec<ArtifactRef>,
}

impl PanelLedgerEvent {
    pub fn new(seed: u64, scheduler_action: PanelSeedAction, event: impl Into<String>) -> Self {
        Self {
            schema: "branch_panel_ledger_event_v0",
            seed,
            scheduler_action,
            event: event.into(),
            run_mode: None,
            slice_index: None,
            error: None,
            artifact_refs: Vec::new(),
        }
    }

    pub fn for_slice(
        seed: u64,
        scheduler_action: PanelSeedAction,
        event: impl Into<String>,
        run_mode: PanelRunMode,
        slice_index: usize,
        error: Option<String>,
        artifact_refs: Vec<ArtifactRef>,
    ) -> Self {
        Self {
            schema: "branch_panel_ledger_event_v0",
            seed,
            scheduler_action,
            event: event.into(),
            run_mode: Some(run_mode),
            slice_index: Some(slice_index),
            error,
            artifact_refs,
        }
    }
}

impl BranchArtifactStore {
    pub fn new(capsule_root: impl Into<PathBuf>) -> Self {
        Self {
            capsule_root: capsule_root.into(),
        }
    }

    pub fn capsule_path(&self, seed: u64) -> PathBuf {
        self.capsule_root.join(seed.to_string())
    }

    pub fn compare_profile_root(&self, profile: &str) -> PathBuf {
        self.capsule_root.join("_compare").join(profile)
    }

    pub fn compare_profile_capsule_path(&self, profile: &str, seed: u64) -> PathBuf {
        self.compare_profile_root(profile).join(seed.to_string())
    }

    pub fn read_seed_artifacts(&self, seed: u64) -> Result<PanelSeedArtifacts, String> {
        self.read_capsule_artifacts(&self.capsule_path(seed))
    }

    pub fn read_compare_seed_artifacts(
        &self,
        profile: &str,
        seed: u64,
    ) -> Result<PanelSeedArtifacts, String> {
        self.read_capsule_artifacts(&self.compare_profile_capsule_path(profile, seed))
    }

    pub fn read_capsule_artifacts(&self, path: &Path) -> Result<PanelSeedArtifacts, String> {
        PanelSeedArtifacts::from_capsule_path(path)
    }

    pub fn default_panel_summary_path(&self) -> PathBuf {
        self.capsule_root.join("panel_summary.json")
    }

    pub fn default_panel_ledger_path(&self) -> PathBuf {
        self.capsule_root.join("panel_ledger.jsonl")
    }

    pub fn default_panel_archive_root(&self) -> PathBuf {
        self.capsule_root.join("_archive")
    }

    pub fn archive_capsule(&self, seed: u64) -> Result<Option<PathBuf>, String> {
        let capsule_path = self.capsule_path(seed);
        if !capsule_path.exists() {
            return Ok(None);
        }
        let archive_root = self.default_panel_archive_root();
        fs::create_dir_all(&archive_root)
            .map_err(|err| format!("failed to create {}: {err}", archive_root.display()))?;
        let base_name = format!("{seed}-{}", archive_timestamp_ms()?);
        for suffix in 0..1000 {
            let name = if suffix == 0 {
                base_name.clone()
            } else {
                format!("{base_name}-{suffix}")
            };
            let archive_path = archive_root.join(name);
            if archive_path.exists() {
                continue;
            }
            fs::rename(&capsule_path, &archive_path).map_err(|err| {
                format!(
                    "failed to archive {} to {}: {err}",
                    capsule_path.display(),
                    archive_path.display()
                )
            })?;
            return Ok(Some(archive_path));
        }
        Err(format!("failed to choose archive path for seed {seed}"))
    }

    pub fn write_panel_summary(
        &self,
        path: Option<&Path>,
        summary: &PanelSummary,
    ) -> Result<PathBuf, String> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.default_panel_summary_path());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(summary)
            .map_err(|err| format!("failed to serialize panel summary: {err}"))?;
        fs::write(&path, text)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        Ok(path)
    }

    pub fn append_panel_ledger_event(
        &self,
        path: Option<&Path>,
        event: &PanelLedgerEvent,
    ) -> Result<PathBuf, String> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.default_panel_ledger_path());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        let text = serde_json::to_string(event)
            .map_err(|err| format!("failed to serialize panel ledger event: {err}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
        writeln!(file, "{text}")
            .map_err(|err| format!("failed to append {}: {err}", path.display()))?;
        Ok(path)
    }
}

fn archive_timestamp_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|err| format!("system clock before unix epoch: {err}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::runtime::branch::{PanelRow, PanelSeedAction, PanelSummary};

    #[test]
    fn store_owns_seed_capsule_paths() {
        let store = BranchArtifactStore::new("target/panel-root");

        assert_eq!(
            store.capsule_path(123),
            std::path::PathBuf::from("target/panel-root/123")
        );
        assert_eq!(
            store.default_panel_summary_path(),
            std::path::PathBuf::from("target/panel-root/panel_summary.json")
        );
    }

    #[test]
    fn store_owns_compare_profile_capsule_paths() {
        let store = BranchArtifactStore::new("target/panel-root");

        assert_eq!(
            store.compare_profile_capsule_path("double-search", 123),
            std::path::PathBuf::from("target/panel-root/_compare/double-search/123")
        );
    }

    #[test]
    fn store_writes_panel_summary_json() {
        let root = std::env::temp_dir().join("branch_artifact_store_panel_summary");
        let _ = std::fs::remove_dir_all(&root);
        let store = BranchArtifactStore::new(&root);
        let summary = PanelSummary::from_rows(vec![PanelRow {
            profile: None,
            seed: 1,
            capsule_path: "capsule".to_string(),
            row_status: crate::runtime::branch::PanelRowStatus::Scheduled,
            identity_status: crate::runtime::branch::PanelIdentityStatus::Missing,
            reuse_decision: crate::runtime::branch::PanelReuseDecision::CreateNewCapsule,
            scheduler_action: crate::runtime::branch::PanelSeedAction::StartNew,
            manifest_exists: false,
            result_exists: false,
            frontier_exists: false,
            terminal_exists: false,
            summary_exists: false,
            artifact_refs: Vec::new(),
            read_error: None,
            tool_error: None,
            archived_capsule_path: None,
        }]);

        let path = store.write_panel_summary(None, &summary).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(path, root.join("panel_summary.json"));
        assert_eq!(value["schema"], json!("branch_panel_summary_v0"));
        assert_eq!(value["total_rows"], 1);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn store_appends_panel_ledger_jsonl() {
        let root = std::env::temp_dir().join("branch_artifact_store_panel_ledger");
        let _ = std::fs::remove_dir_all(&root);
        let store = BranchArtifactStore::new(&root);

        let first = PanelLedgerEvent::new(1, PanelSeedAction::StartNew, "executed");
        let second = PanelLedgerEvent::new(2, PanelSeedAction::ReuseRealStop, "skipped");
        let path = store.append_panel_ledger_event(None, &first).unwrap();
        let second_path = store.append_panel_ledger_event(None, &second).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let lines = text.lines().collect::<Vec<_>>();

        assert_eq!(path, root.join("panel_ledger.jsonl"));
        assert_eq!(second_path, path);
        assert_eq!(lines.len(), 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(lines[0]).unwrap()["event"],
            "executed"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(lines[1]).unwrap()["scheduler_action"],
            "reuse_real_stop"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn store_reads_seed_artifact_presence() {
        let root = std::env::temp_dir().join("branch_artifact_store_seed_artifacts");
        let _ = std::fs::remove_dir_all(&root);
        let store = BranchArtifactStore::new(&root);
        let capsule = store.capsule_path(7);
        std::fs::create_dir_all(&capsule).unwrap();
        std::fs::write(capsule.join("manifest.json"), "{}").unwrap();
        std::fs::write(capsule.join("frontier.json"), "{}").unwrap();
        std::fs::write(capsule.join("summary.json"), "{}").unwrap();

        let artifacts = store.read_seed_artifacts(7).unwrap();

        assert!(artifacts.manifest.is_some());
        assert!(artifacts.frontier_exists);
        assert!(!artifacts.result_exists);
        assert!(!artifacts.terminal_exists);
        assert!(artifacts.summary_exists);

        let _ = std::fs::remove_dir_all(root);
    }
}
