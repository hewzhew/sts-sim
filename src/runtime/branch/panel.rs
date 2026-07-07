use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    decide_manifest_reuse, Args, BranchArtifactStore, CapsuleReuseDecision, RunContract,
    SourceIdentity,
};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PanelSeedArtifacts {
    pub manifest: Option<Value>,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelIdentityStatus {
    Missing,
    Exact,
    Unknown,
    Incompatible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelReuseDecision {
    CreateNewCapsule,
    ReuseRealStop,
    ContinueSoftPause,
    RejectUnknownIdentity,
    RejectIncompatibleIdentity,
    RejectIncompleteCapsule,
    RejectMalformedCapsule,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelSeedAction {
    StartNew,
    ContinueCapsule,
    ReuseRealStop,
    RejectCapsule,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelSeedDecision {
    pub identity_status: PanelIdentityStatus,
    pub reuse_decision: PanelReuseDecision,
    pub artifact_facts: PanelArtifactFacts,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelSeedRequest {
    pub seed: u64,
    pub capsule_path: PathBuf,
    pub contract: RunContract,
    pub source_identity: SourceIdentity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelSeedResolution {
    pub seed: u64,
    pub capsule_path: PathBuf,
    pub decision: PanelSeedDecision,
    pub read_error: Option<String>,
}

pub struct PanelScheduler;

#[derive(Clone)]
pub struct PanelInspectConfig {
    pub seeds: Vec<u64>,
    pub artifact_store: BranchArtifactStore,
    pub args_template: Args,
    pub source_identity: SourceIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PanelArtifactFacts {
    pub manifest_exists: bool,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelRow {
    pub seed: u64,
    pub capsule_path: String,
    pub identity_status: PanelIdentityStatus,
    pub reuse_decision: PanelReuseDecision,
    pub scheduler_action: PanelSeedAction,
    pub manifest_exists: bool,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
    pub read_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelSummary {
    pub schema: &'static str,
    pub total_rows: usize,
    pub counts_by_reuse_decision: BTreeMap<String, usize>,
    pub rows: Vec<PanelRow>,
}

pub fn decide_seed_capsule(
    artifacts: PanelSeedArtifacts,
    expected_contract: RunContract,
    expected_source: &SourceIdentity,
) -> PanelSeedDecision {
    let artifact_facts = artifacts.facts();
    let Some(manifest) = artifacts.manifest else {
        return PanelSeedDecision {
            identity_status: PanelIdentityStatus::Missing,
            reuse_decision: PanelReuseDecision::CreateNewCapsule,
            artifact_facts,
        };
    };
    match decide_manifest_reuse(&manifest, expected_contract, expected_source) {
        CapsuleReuseDecision::Exact => exact_identity_decision(artifact_facts),
        CapsuleReuseDecision::UnknownLegacy => PanelSeedDecision {
            identity_status: PanelIdentityStatus::Unknown,
            reuse_decision: PanelReuseDecision::RejectUnknownIdentity,
            artifact_facts,
        },
        CapsuleReuseDecision::Incompatible => PanelSeedDecision {
            identity_status: PanelIdentityStatus::Incompatible,
            reuse_decision: PanelReuseDecision::RejectIncompatibleIdentity,
            artifact_facts,
        },
    }
}

impl PanelSeedRequest {
    pub fn resolve(self) -> PanelSeedResolution {
        let artifacts = match PanelSeedArtifacts::from_capsule_path(&self.capsule_path) {
            Ok(artifacts) => {
                let decision = decide_seed_capsule(artifacts, self.contract, &self.source_identity);
                return PanelSeedResolution {
                    seed: self.seed,
                    capsule_path: self.capsule_path,
                    decision,
                    read_error: None,
                };
            }
            Err(error) => (
                PanelArtifactFacts::from_capsule_path(&self.capsule_path),
                error,
            ),
        };
        PanelSeedResolution {
            seed: self.seed,
            capsule_path: self.capsule_path,
            decision: PanelSeedDecision {
                identity_status: PanelIdentityStatus::Unknown,
                reuse_decision: PanelReuseDecision::RejectMalformedCapsule,
                artifact_facts: artifacts.0,
            },
            read_error: Some(artifacts.1),
        }
    }
}

impl PanelSeedResolution {
    pub fn scheduler_action(&self) -> PanelSeedAction {
        match self.decision.reuse_decision {
            PanelReuseDecision::CreateNewCapsule => PanelSeedAction::StartNew,
            PanelReuseDecision::ReuseRealStop => PanelSeedAction::ReuseRealStop,
            PanelReuseDecision::ContinueSoftPause => PanelSeedAction::ContinueCapsule,
            PanelReuseDecision::RejectUnknownIdentity
            | PanelReuseDecision::RejectIncompatibleIdentity
            | PanelReuseDecision::RejectIncompleteCapsule
            | PanelReuseDecision::RejectMalformedCapsule => PanelSeedAction::RejectCapsule,
        }
    }
}

impl PanelScheduler {
    pub fn resolve_requests(
        requests: impl IntoIterator<Item = PanelSeedRequest>,
    ) -> Vec<PanelSeedResolution> {
        requests
            .into_iter()
            .map(PanelSeedRequest::resolve)
            .collect()
    }

    pub fn summarize_requests(
        requests: impl IntoIterator<Item = PanelSeedRequest>,
    ) -> PanelSummary {
        PanelSummary::from_rows(
            Self::resolve_requests(requests)
                .into_iter()
                .map(PanelRow::from_resolution)
                .collect(),
        )
    }
}

impl PanelInspectConfig {
    pub fn requests(&self) -> Vec<PanelSeedRequest> {
        self.seeds
            .iter()
            .copied()
            .map(|seed| {
                let mut args = self.args_template;
                args.seed = seed;
                PanelSeedRequest {
                    seed,
                    capsule_path: self.artifact_store.capsule_path(seed),
                    contract: RunContract::from_args(args),
                    source_identity: self.source_identity.clone(),
                }
            })
            .collect()
    }

    pub fn summarize(&self) -> PanelSummary {
        PanelScheduler::summarize_requests(self.requests())
    }
}

impl PanelRow {
    pub fn from_resolution(resolution: PanelSeedResolution) -> Self {
        let artifacts = resolution.decision.artifact_facts;
        Self {
            seed: resolution.seed,
            capsule_path: resolution.capsule_path.display().to_string(),
            identity_status: resolution.decision.identity_status,
            reuse_decision: resolution.decision.reuse_decision,
            scheduler_action: resolution.scheduler_action(),
            manifest_exists: artifacts.manifest_exists,
            result_exists: artifacts.result_exists,
            frontier_exists: artifacts.frontier_exists,
            terminal_exists: artifacts.terminal_exists,
            summary_exists: artifacts.summary_exists,
            read_error: resolution.read_error,
        }
    }
}

impl PanelSummary {
    pub fn from_rows(rows: Vec<PanelRow>) -> Self {
        let mut counts_by_reuse_decision = BTreeMap::new();
        for row in &rows {
            *counts_by_reuse_decision
                .entry(reuse_decision_key(row.reuse_decision))
                .or_insert(0) += 1;
        }
        Self {
            schema: "branch_panel_summary_v0",
            total_rows: rows.len(),
            counts_by_reuse_decision,
            rows,
        }
    }
}

fn reuse_decision_key(decision: PanelReuseDecision) -> String {
    serde_json::to_value(decision)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn exact_identity_decision(artifact_facts: PanelArtifactFacts) -> PanelSeedDecision {
    let reuse_decision = if artifact_facts.result_exists {
        PanelReuseDecision::ReuseRealStop
    } else if artifact_facts.frontier_exists {
        PanelReuseDecision::ContinueSoftPause
    } else {
        PanelReuseDecision::RejectIncompleteCapsule
    };
    PanelSeedDecision {
        identity_status: PanelIdentityStatus::Exact,
        reuse_decision,
        artifact_facts,
    }
}

impl PanelSeedArtifacts {
    pub fn from_capsule_path(path: &Path) -> Result<Self, String> {
        let manifest_path = path.join("manifest.json");
        let manifest = if manifest_path.exists() {
            let text = fs::read_to_string(&manifest_path)
                .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
            Some(
                serde_json::from_str::<Value>(&text)
                    .map_err(|err| format!("failed to parse {}: {err}", manifest_path.display()))?,
            )
        } else {
            None
        };
        Ok(Self {
            manifest,
            result_exists: path.join("result.json").exists(),
            frontier_exists: path.join("frontier.json").exists(),
            terminal_exists: path.join("terminal.json").exists(),
            summary_exists: path.join("summary.json").exists(),
        })
    }

    fn facts(&self) -> PanelArtifactFacts {
        PanelArtifactFacts::from_artifacts(self)
    }
}

impl PanelArtifactFacts {
    fn from_artifacts(artifacts: &PanelSeedArtifacts) -> Self {
        Self {
            manifest_exists: artifacts.manifest.is_some(),
            result_exists: artifacts.result_exists,
            frontier_exists: artifacts.frontier_exists,
            terminal_exists: artifacts.terminal_exists,
            summary_exists: artifacts.summary_exists,
        }
    }

    fn from_capsule_path(path: &Path) -> Self {
        Self {
            manifest_exists: path.join("manifest.json").exists(),
            result_exists: path.join("result.json").exists(),
            frontier_exists: path.join("frontier.json").exists(),
            terminal_exists: path.join("terminal.json").exists(),
            summary_exists: path.join("summary.json").exists(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;
    use crate::runtime::branch::{Args, RunObjective};

    fn args(seed: u64) -> Args {
        Args {
            seed,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(1),
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn source_identity() -> SourceIdentity {
        SourceIdentity {
            git_commit: Some("abc123".to_string()),
            git_dirty: Some(false),
        }
    }

    fn exact_manifest(contract: RunContract) -> serde_json::Value {
        json!({
            "run_contract": contract,
            "source_identity": source_identity(),
            "status": "paused",
            "reason": "wall_deadline",
        })
    }

    #[test]
    fn missing_capsule_manifest_starts_new_capsule() {
        let decision = decide_seed_capsule(
            PanelSeedArtifacts::default(),
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::CreateNewCapsule
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Missing);
    }

    #[test]
    fn exact_identity_with_result_reuses_real_stop() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            result_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(decision.reuse_decision, PanelReuseDecision::ReuseRealStop);
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn exact_paused_capsule_with_frontier_continues_soft_pause() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            frontier_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::ContinueSoftPause
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn legacy_capsule_is_not_silently_reused() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(json!({"args": {"seed": 1}, "status": "terminal"})),
            result_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::RejectUnknownIdentity
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Unknown);
    }

    #[test]
    fn exact_paused_capsule_without_frontier_is_incomplete() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::RejectIncompleteCapsule
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn reads_capsule_artifact_presence_from_directory() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_artifacts");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(dir.join("result.json"), "{}").unwrap();
        fs::write(dir.join("summary.json"), "{}").unwrap();

        let artifacts = PanelSeedArtifacts::from_capsule_path(&dir).unwrap();

        assert!(artifacts.manifest.is_some());
        assert!(artifacts.result_exists);
        assert!(!artifacts.frontier_exists);
        assert!(!artifacts.terminal_exists);
        assert!(artifacts.summary_exists);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_manifest_is_a_capsule_read_error() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_bad_manifest");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), "{bad").unwrap();

        let err = PanelSeedArtifacts::from_capsule_path(&dir).unwrap_err();

        assert!(err.contains("manifest.json"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn seed_resolution_preserves_malformed_capsule_as_a_row_decision() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_bad_row");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), "{bad").unwrap();

        let resolution = PanelSeedRequest {
            seed: 1,
            capsule_path: dir.clone(),
            contract: RunContract::from_args(args(1)),
            source_identity: source_identity(),
        }
        .resolve();

        assert_eq!(resolution.seed, 1);
        assert_eq!(
            resolution.decision.reuse_decision,
            PanelReuseDecision::RejectMalformedCapsule
        );
        assert!(resolution.read_error.unwrap().contains("manifest.json"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scheduler_resolution_keeps_one_row_per_seed() {
        let root = std::env::temp_dir().join("runtime_branch_panel_rows");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let good = root.join("good");
        let bad = root.join("bad");
        fs::create_dir_all(&good).unwrap();
        fs::create_dir_all(&bad).unwrap();
        fs::write(
            good.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(good.join("frontier.json"), "{}").unwrap();
        fs::write(bad.join("manifest.json"), "{bad").unwrap();

        let rows = PanelScheduler::resolve_requests(vec![
            PanelSeedRequest {
                seed: 1,
                capsule_path: good,
                contract: RunContract::from_args(args(1)),
                source_identity: source_identity(),
            },
            PanelSeedRequest {
                seed: 2,
                capsule_path: bad,
                contract: RunContract::from_args(args(2)),
                source_identity: source_identity(),
            },
            PanelSeedRequest {
                seed: 3,
                capsule_path: root.join("missing"),
                contract: RunContract::from_args(args(3)),
                source_identity: source_identity(),
            },
        ]);

        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0].decision.reuse_decision,
            PanelReuseDecision::ContinueSoftPause
        );
        assert_eq!(
            rows[1].decision.reuse_decision,
            PanelReuseDecision::RejectMalformedCapsule
        );
        assert_eq!(
            rows[2].decision.reuse_decision,
            PanelReuseDecision::CreateNewCapsule
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn panel_row_serializes_resolution_as_structured_fields() {
        let resolution = PanelSeedResolution {
            seed: 7,
            capsule_path: PathBuf::from("target/example"),
            decision: PanelSeedDecision {
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                artifact_facts: PanelArtifactFacts {
                    manifest_exists: true,
                    result_exists: false,
                    frontier_exists: true,
                    terminal_exists: false,
                    summary_exists: true,
                },
            },
            read_error: None,
        };

        let value = serde_json::to_value(PanelRow::from_resolution(resolution)).unwrap();

        assert_eq!(value["seed"], 7);
        assert_eq!(value["capsule_path"], "target/example");
        assert_eq!(value["identity_status"], "exact");
        assert_eq!(value["reuse_decision"], "continue_soft_pause");
        assert_eq!(value["scheduler_action"], "continue_capsule");
        assert_eq!(value["frontier_exists"], true);
        assert_eq!(value["result_exists"], false);
        assert_eq!(value["read_error"], serde_json::Value::Null);
    }

    #[test]
    fn panel_summary_counts_rows_by_reuse_decision() {
        let rows = vec![
            PanelRow {
                seed: 1,
                capsule_path: "one".to_string(),
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                scheduler_action: PanelSeedAction::ContinueCapsule,
                manifest_exists: true,
                result_exists: false,
                frontier_exists: true,
                terminal_exists: false,
                summary_exists: true,
                read_error: None,
            },
            PanelRow {
                seed: 2,
                capsule_path: "two".to_string(),
                identity_status: PanelIdentityStatus::Missing,
                reuse_decision: PanelReuseDecision::CreateNewCapsule,
                scheduler_action: PanelSeedAction::StartNew,
                manifest_exists: false,
                result_exists: false,
                frontier_exists: false,
                terminal_exists: false,
                summary_exists: false,
                read_error: None,
            },
        ];

        let value = serde_json::to_value(PanelSummary::from_rows(rows)).unwrap();

        assert_eq!(value["total_rows"], 2);
        assert_eq!(value["counts_by_reuse_decision"]["continue_soft_pause"], 1);
        assert_eq!(value["counts_by_reuse_decision"]["create_new_capsule"], 1);
    }

    #[test]
    fn resolution_maps_reuse_decision_to_scheduler_action() {
        let resolution = PanelSeedResolution {
            seed: 1,
            capsule_path: PathBuf::from("target/example"),
            decision: PanelSeedDecision {
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                artifact_facts: PanelArtifactFacts {
                    manifest_exists: true,
                    result_exists: false,
                    frontier_exists: true,
                    terminal_exists: false,
                    summary_exists: true,
                },
            },
            read_error: None,
        };

        assert_eq!(
            resolution.scheduler_action(),
            PanelSeedAction::ContinueCapsule
        );
    }

    #[test]
    fn exact_result_capsule_summarizes_as_reuse_real_stop() {
        let root = std::env::temp_dir().join("runtime_branch_panel_summary");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let capsule = root.join("seed1");
        fs::create_dir_all(&capsule).unwrap();
        fs::write(
            capsule.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(capsule.join("result.json"), "{}").unwrap();

        let summary = PanelScheduler::summarize_requests(vec![PanelSeedRequest {
            seed: 1,
            capsule_path: capsule,
            contract: RunContract::from_args(args(1)),
            source_identity: source_identity(),
        }]);

        assert_eq!(summary.total_rows, 1);
        assert_eq!(
            summary.rows[0].reuse_decision,
            PanelReuseDecision::ReuseRealStop
        );
        assert_eq!(
            summary.rows[0].scheduler_action,
            PanelSeedAction::ReuseRealStop
        );
        assert_eq!(summary.counts_by_reuse_decision["reuse_real_stop"], 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inspect_config_materializes_seed_requests_from_a_template() {
        let mut template = args(999);
        template.ascension = 3;
        template.generations = 9;
        let source = source_identity();
        let config = PanelInspectConfig {
            seeds: vec![7, 8],
            artifact_store: BranchArtifactStore::new("target/panel-config"),
            args_template: template,
            source_identity: source.clone(),
        };

        let requests = config.requests();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].seed, 7);
        assert_eq!(
            requests[0].capsule_path,
            PathBuf::from("target/panel-config/7")
        );
        assert_eq!(requests[0].contract.game.seed, 7);
        assert_eq!(requests[0].contract.game.ascension, 3);
        assert_eq!(requests[0].contract.branching.generations, 9);
        assert_eq!(requests[0].source_identity, source);
        assert_eq!(requests[1].contract.game.seed, 8);
    }
}
