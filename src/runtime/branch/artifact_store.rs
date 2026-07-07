use std::fs;
use std::path::{Path, PathBuf};

use super::PanelSummary;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchArtifactStore {
    capsule_root: PathBuf,
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

    pub fn default_panel_summary_path(&self) -> PathBuf {
        self.capsule_root.join("panel_summary.json")
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
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::runtime::branch::{PanelRow, PanelSummary};

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
    fn store_writes_panel_summary_json() {
        let root = std::env::temp_dir().join("branch_artifact_store_panel_summary");
        let _ = std::fs::remove_dir_all(&root);
        let store = BranchArtifactStore::new(&root);
        let summary = PanelSummary::from_rows(vec![PanelRow {
            seed: 1,
            capsule_path: "capsule".to_string(),
            identity_status: crate::runtime::branch::PanelIdentityStatus::Missing,
            reuse_decision: crate::runtime::branch::PanelReuseDecision::CreateNewCapsule,
            scheduler_action: crate::runtime::branch::PanelSeedAction::StartNew,
            manifest_exists: false,
            result_exists: false,
            frontier_exists: false,
            terminal_exists: false,
            summary_exists: false,
            read_error: None,
        }]);

        let path = store.write_panel_summary(None, &summary).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(path, root.join("panel_summary.json"));
        assert_eq!(value["schema"], json!("branch_panel_summary_v0"));
        assert_eq!(value["total_rows"], 1);

        let _ = std::fs::remove_dir_all(root);
    }
}
