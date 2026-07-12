use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{json, Value};

use super::run_identity::{current_source_identity, SourceIdentity};
use super::run_slice_result::{
    ArtifactKind, ArtifactRef, ArtifactWriteSummary, RunSliceRequestKind, RunSliceResult, RunStop,
};
use super::{
    accepted_high_loss_diagnostic, combat_gap_case, frontier_checkpoint, run_capsule_format,
    run_capsule_io, trajectory_evidence_store, Args, Branch, BranchStatus,
};
use run_capsule_io::{ensure_dir, read_terminal_entries, remove_if_exists, write_json};

pub(super) struct CapsuleArtifactStore {
    root: PathBuf,
    started_at_ms: u128,
    git_commit: Option<String>,
    source_identity: SourceIdentity,
}

impl CapsuleArtifactStore {
    pub(super) fn new(root: PathBuf) -> Self {
        let source_identity = current_source_identity();
        Self {
            root,
            started_at_ms: now_ms(),
            git_commit: source_identity.git_commit.clone(),
            source_identity,
        }
    }

    pub(super) fn combat_cases_dir(&self) -> PathBuf {
        self.root.join("combat_cases")
    }

    pub(super) fn result_path(&self) -> PathBuf {
        self.root.join("result.json")
    }

    pub(super) fn summary_path(&self) -> PathBuf {
        self.root.join("summary.json")
    }

    pub(super) fn terminal_path(&self) -> PathBuf {
        self.root.join("terminal.json")
    }

    pub(super) fn trajectory_state_path(&self) -> PathBuf {
        self.root.join("trajectory_state.json")
    }

    pub(super) fn running_manifest_summary(&self) -> ArtifactWriteSummary {
        ArtifactWriteSummary::single_ref(self.artifact_ref(
            ArtifactKind::Manifest,
            "manifest.json",
            "branch_tiny_capsule_manifest",
        ))
    }

    pub(super) fn frontier_summary(&self) -> ArtifactWriteSummary {
        let mut summary = ArtifactWriteSummary::default();
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Manifest,
            "manifest.json",
            "branch_tiny_capsule_manifest",
        ));
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Frontier,
            "frontier.json",
            "branch_tiny_frontier_checkpoint",
        ));
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Summary,
            "summary.json",
            "branch_tiny_capsule_summary",
        ));
        summary.record_ref(self.trajectory_evidence_ref());
        summary
    }

    pub(super) fn result_summary(&self) -> ArtifactWriteSummary {
        let mut summary = ArtifactWriteSummary::default();
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Manifest,
            "manifest.json",
            "branch_tiny_capsule_manifest",
        ));
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Result,
            "result.json",
            "branch_tiny_capsule_result",
        ));
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Path,
            "path.json",
            "branch_tiny_capsule_path",
        ));
        summary.record_ref(self.artifact_ref(
            ArtifactKind::Summary,
            "summary.json",
            "branch_tiny_capsule_summary",
        ));
        summary.record_ref(self.trajectory_evidence_ref());
        self.record_accepted_combat_diagnostic_refs(&mut summary);
        summary
    }

    pub(super) fn trajectory_evidence_summary(&self) -> ArtifactWriteSummary {
        ArtifactWriteSummary::single_ref(self.trajectory_evidence_ref())
    }

    pub(super) fn terminal_summary(&self) -> ArtifactWriteSummary {
        let mut summary = ArtifactWriteSummary::single_ref(self.artifact_ref(
            ArtifactKind::Terminal,
            "terminal.json",
            "branch_tiny_terminal_results",
        ));
        summary.record_ref(self.trajectory_evidence_ref());
        self.record_accepted_combat_diagnostic_refs(&mut summary);
        summary
    }

    pub(super) fn write_running_manifest(&self, args: Args) -> Result<(), String> {
        self.write_manifest(args, "running", None)
    }

    pub(super) fn write_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<(), String> {
        self.write_branch_result(
            args,
            generation,
            branch,
            run_capsule_format::terminal_manifest_status(&branch.status),
            None,
        )
    }

    pub(super) fn write_completed_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        reason: &'static str,
    ) -> Result<(), String> {
        self.write_branch_result(args, generation, branch, "completed", Some(reason))
    }

    pub(super) fn write_frontier(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<Option<usize>, String> {
        let running = frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .count();
        if running == 0 {
            return Ok(None);
        }
        trajectory_evidence_store::record_frontier(
            &self.trajectory_state_path(),
            generation,
            frontier,
        )?;
        frontier_checkpoint::save(
            &self.root.join("frontier.json"),
            args,
            generation,
            next_branch_id,
            frontier,
        )?;
        remove_if_exists(&self.root.join("result.json"))?;
        remove_if_exists(&self.root.join("path.json"))?;
        self.write_manifest(args, capsule_status, reason)?;
        self.write_frontier_summary(args, generation, frontier, capsule_status, reason)?;
        Ok(Some(running))
    }

    pub(super) fn append_terminal_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<bool, String> {
        if !matches!(branch.status, BranchStatus::Terminal(_)) {
            return Ok(false);
        }
        ensure_dir(&self.root)?;
        let path = self.terminal_path();
        let mut entries = read_terminal_entries(&path)?;
        if entries
            .iter()
            .any(|entry| entry.get("branch_id").and_then(Value::as_u64) == Some(branch.id as u64))
        {
            return Ok(false);
        }
        let combat_case = self.combat_case_value(args, generation, branch);
        let accepted_high_loss_combat_diagnostics =
            self.accepted_high_loss_combat_diagnostics_value(args, generation, branch)?;
        trajectory_evidence_store::record_branch(
            &self.trajectory_state_path(),
            generation,
            branch,
        )?;
        let trajectory_evaluation = self.current_trajectory_evaluation()?;
        entries.push(run_capsule_format::result_value(
            generation,
            branch,
            combat_case,
            accepted_high_loss_combat_diagnostics,
            &trajectory_evaluation,
        ));
        write_json(&path, run_capsule_format::terminal_results_value(entries))?;
        Ok(true)
    }

    pub(super) fn append_slice_ledger(&self, result: &RunSliceResult) -> Result<(), String> {
        self.append_ledger_event(&CapsuleLedgerEvent::from_result(result))
    }

    pub(super) fn append_slice_started_ledger(
        &self,
        args: Args,
        request_kind: RunSliceRequestKind,
        generation_start: usize,
        artifacts: &ArtifactWriteSummary,
    ) -> Result<(), String> {
        self.append_ledger_event(&CapsuleLedgerEvent::from_slice_start(
            args,
            request_kind,
            generation_start,
            artifacts,
        ))
    }

    fn append_ledger_event(&self, event: &CapsuleLedgerEvent) -> Result<(), String> {
        ensure_dir(&self.root)?;
        let encoded = serde_json::to_string(&event)
            .map_err(|error| format!("serialize capsule ledger event: {error}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("capsule_ledger.jsonl"))
            .map_err(|error| format!("open capsule ledger: {error}"))?;
        writeln!(file, "{encoded}").map_err(|error| format!("write capsule ledger: {error}"))
    }

    fn write_branch_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<(), String> {
        ensure_dir(&self.root)?;
        let combat_case = self.combat_case_value(args, generation, branch);
        let accepted_high_loss_combat_diagnostics =
            self.accepted_high_loss_combat_diagnostics_value(args, generation, branch)?;
        trajectory_evidence_store::record_branch(
            &self.trajectory_state_path(),
            generation,
            branch,
        )?;
        let trajectory_evaluation = self.current_trajectory_evaluation()?;
        write_json(
            &self.root.join("result.json"),
            run_capsule_format::result_value(
                generation,
                branch,
                combat_case.clone(),
                accepted_high_loss_combat_diagnostics.clone(),
                &trajectory_evaluation,
            ),
        )?;
        write_json(
            &self.root.join("path.json"),
            run_capsule_format::path_value(branch),
        )?;
        remove_if_exists(&self.root.join("frontier.json"))?;
        self.write_manifest(args, capsule_status, reason)?;
        self.write_branch_summary(
            args,
            generation,
            branch,
            &combat_case,
            &accepted_high_loss_combat_diagnostics,
            &trajectory_evaluation,
            capsule_status,
            reason,
        )
    }

    fn write_branch_summary(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        combat_case: &Value,
        accepted_high_loss_combat_diagnostics: &Value,
        trajectory_evaluation: &super::trajectory_snapshot::FrontierTrajectoryEvaluation,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<(), String> {
        write_json(
            &self.summary_path(),
            run_capsule_format::branch_summary_value(
                &self.root,
                args,
                generation,
                branch,
                combat_case,
                accepted_high_loss_combat_diagnostics,
                trajectory_evaluation,
                capsule_status,
                reason,
                None,
            ),
        )
    }

    fn write_frontier_summary(
        &self,
        args: Args,
        generation: usize,
        frontier: &VecDeque<Branch>,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<(), String> {
        let running = frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .count();
        let trajectory_evaluation = self.current_trajectory_evaluation()?;
        let frontier_info = run_capsule_format::frontier_trajectory_summary_value(
            frontier.len(),
            running,
            &trajectory_evaluation,
        );
        if let Some(branch) = frontier
            .iter()
            .find(|branch| branch.status.is_resumable())
            .or_else(|| frontier.front())
        {
            return write_json(
                &self.summary_path(),
                run_capsule_format::branch_summary_value(
                    &self.root,
                    args,
                    generation,
                    branch,
                    &Value::Null,
                    &Value::Array(Vec::new()),
                    &trajectory_evaluation,
                    capsule_status,
                    reason,
                    Some(frontier_info),
                ),
            );
        }
        write_json(
            &self.summary_path(),
            run_capsule_format::empty_frontier_summary_value(
                &self.root,
                args,
                generation,
                capsule_status,
                reason,
                frontier_info,
            ),
        )
    }

    fn write_manifest(
        &self,
        args: Args,
        status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<(), String> {
        ensure_dir(&self.root)?;
        write_json(
            &self.root.join("manifest.json"),
            run_capsule_format::manifest_value(
                args,
                status,
                reason,
                self.started_at_ms,
                now_ms(),
                &self.git_commit,
                &self.source_identity,
            ),
        )
    }

    fn combat_case_value(&self, args: Args, generation: usize, branch: &Branch) -> Value {
        if !matches!(
            branch.status,
            BranchStatus::CombatGap { .. }
                | BranchStatus::OperationBudgetExhausted { .. }
                | BranchStatus::BudgetGap { .. }
        ) {
            return Value::Null;
        }
        match combat_gap_case::save_combat_gap_case(
            &self.combat_cases_dir(),
            args,
            generation,
            branch,
        ) {
            Ok(Some(path)) => json!(path.display().to_string()),
            Ok(None) => Value::Null,
            Err(error) => json!({"error": error}),
        }
    }

    fn accepted_high_loss_combat_diagnostics_value(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<Value, String> {
        let dir = self.root.join("accepted_high_loss_combat");
        let mut values = Vec::new();
        for diagnostic in &branch.accepted_high_loss_diagnostics {
            let written = accepted_high_loss_diagnostic::write_diagnostic_pair(
                &dir, args.seed, generation, branch.id, diagnostic,
            )?;
            values.push(written.value());
        }
        Ok(Value::Array(values))
    }

    fn record_accepted_combat_diagnostic_refs(&self, summary: &mut ArtifactWriteSummary) {
        let dir = self.root.join("accepted_high_loss_combat");
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let schema = accepted_combat_diagnostic_schema(&path);
            summary.record_ref(ArtifactRef::new(
                ArtifactKind::AcceptedCombatDiagnostic,
                path,
                schema,
                "owner_audit_runtime",
            ));
        }
    }

    fn artifact_ref(&self, kind: ArtifactKind, file_name: &str, schema: &str) -> ArtifactRef {
        ArtifactRef::new(
            kind,
            self.root.join(file_name),
            schema,
            "owner_audit_runtime",
        )
    }

    fn trajectory_evidence_ref(&self) -> ArtifactRef {
        self.artifact_ref(
            ArtifactKind::TrajectoryEvidence,
            "trajectory_state.json",
            "branch_tiny_trajectory_state_v0",
        )
    }

    fn current_trajectory_evaluation(
        &self,
    ) -> Result<super::trajectory_snapshot::FrontierTrajectoryEvaluation, String> {
        Ok(trajectory_evidence_store::read_state(&self.trajectory_state_path())?.evaluation)
    }

    pub(super) fn record_stopped_trajectory(
        &self,
        generation: usize,
        branch: &Branch,
    ) -> Result<(), String> {
        trajectory_evidence_store::record_branch(
            &self.trajectory_state_path(),
            generation,
            branch,
        )?;
        Ok(())
    }
}

fn accepted_combat_diagnostic_schema(path: &std::path::Path) -> String {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".capture.json"))
    {
        return "CombatCaptureV1".to_string();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| value.get("schema")?.as_str().map(str::to_string))
        .filter(|schema| {
            matches!(
                schema.as_str(),
                "accepted_high_loss_combat_evidence_v1" | "accepted_high_loss_combat_evidence_v2"
            )
        })
        .unwrap_or_else(|| "accepted_high_loss_combat_evidence_v1".to_string())
}

#[derive(Serialize)]
struct CapsuleLedgerEvent {
    schema: &'static str,
    event: &'static str,
    seed: u64,
    request_kind: RunSliceRequestKind,
    generation_start: usize,
    generation_end: Option<usize>,
    stop_kind: Option<&'static str>,
    artifact_refs: Vec<ArtifactRef>,
}

impl CapsuleLedgerEvent {
    fn from_slice_start(
        args: Args,
        request_kind: RunSliceRequestKind,
        generation_start: usize,
        artifacts: &ArtifactWriteSummary,
    ) -> Self {
        Self {
            schema: "branch_tiny_capsule_ledger_event_v0",
            event: "slice_started",
            seed: args.seed,
            request_kind,
            generation_start,
            generation_end: None,
            stop_kind: None,
            artifact_refs: artifacts.refs(),
        }
    }

    fn from_result(result: &RunSliceResult) -> Self {
        Self {
            schema: "branch_tiny_capsule_ledger_event_v0",
            event: "slice_finished",
            seed: result.contract.game.seed,
            request_kind: result.request_kind,
            generation_start: result.generation_start,
            generation_end: Some(result.generation_end),
            stop_kind: Some(stop_kind(&result.stop)),
            artifact_refs: result.artifacts.refs(),
        }
    }
}

fn stop_kind(stop: &RunStop) -> &'static str {
    match stop {
        RunStop::Real(_) => "real",
        RunStop::SoftPause(_) => "soft_pause",
        RunStop::FrontierExhausted(_) => "frontier_exhausted",
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod accepted_diagnostic_schema_tests {
    use super::*;

    #[test]
    fn accepted_diagnostic_schema_reads_v1_and_v2_sidecars() {
        let root = std::env::temp_dir().join("accepted_diagnostic_schema_versions");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let v1 = root.join("old.evidence.json");
        let v2 = root.join("new.evidence.json");
        std::fs::write(&v1, r#"{"schema":"accepted_high_loss_combat_evidence_v1"}"#).unwrap();
        std::fs::write(&v2, r#"{"schema":"accepted_high_loss_combat_evidence_v2"}"#).unwrap();

        assert_eq!(
            accepted_combat_diagnostic_schema(&v1),
            "accepted_high_loss_combat_evidence_v1"
        );
        assert_eq!(
            accepted_combat_diagnostic_schema(&v2),
            "accepted_high_loss_combat_evidence_v2"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod trajectory_artifact_tests {
    use super::*;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    use crate::runtime::branch::owner_audit::branch_model::{BranchStatus, Owner};
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn test_args() -> Args {
        Args {
            seed: 99,
            ascension: 0,
            objective: super::super::run_contract::RunObjective::FirstTerminal,
            generations: 1,
            max_branches: 2,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(1_000),
            checkpoint_before_combat_portfolio: true,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: true,
            wall_capped_boss_budget: true,
        }
    }

    fn test_branch(id: usize, policy_lane: BranchPolicyLane) -> Branch {
        Branch {
            id,
            parent_id: None,
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::Running {
                boundary: "test".to_string(),
                owner: Owner::CardReward,
            },
            policy_lane,
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn written_frontier_summary_keeps_paired_trajectory_evidence() {
        let root = std::env::temp_dir().join("frontier_trajectory_evidence");
        let _ = std::fs::remove_dir_all(&root);
        let store = CapsuleArtifactStore::new(root.clone());
        let frontier = VecDeque::from([
            test_branch(1, BranchPolicyLane::default()),
            test_branch(
                2,
                BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            ),
        ]);

        store
            .write_frontier(test_args(), 0, 3, &frontier, "running", None)
            .unwrap();

        let summary: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("summary.json")).unwrap())
                .unwrap();
        assert_eq!(
            summary["frontier"]["trajectory_evaluation"]["snapshots"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            summary["frontier"]["trajectory_evaluation"]["comparisons"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn final_result_keeps_challenger_that_stopped_in_an_earlier_slice() {
        let root = std::env::temp_dir().join("final_cross_lane_trajectory_evidence");
        let _ = std::fs::remove_dir_all(&root);
        let store = CapsuleArtifactStore::new(root.clone());
        let baseline = test_branch(1, BranchPolicyLane::default());
        let mut challenger = test_branch(
            2,
            BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
        );
        let frontier = VecDeque::from([baseline.clone(), challenger.clone()]);
        store
            .write_frontier(
                test_args(),
                10,
                3,
                &frontier,
                "paused",
                Some("wall_deadline"),
            )
            .unwrap();

        challenger.status = BranchStatus::CombatGap {
            boundary: "boss".to_string(),
            reason: "no win".to_string(),
        };
        challenger.session.run_state.current_hp = 47;
        store.record_stopped_trajectory(40, &challenger).unwrap();

        let mut final_baseline = baseline;
        final_baseline.status = BranchStatus::CombatGap {
            boundary: "boss".to_string(),
            reason: "no win".to_string(),
        };
        final_baseline.session.run_state.current_hp = 42;
        store
            .write_result(test_args(), 42, &final_baseline)
            .unwrap();

        let result: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("result.json")).unwrap())
                .unwrap();
        let summary: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("summary.json")).unwrap())
                .unwrap();
        assert_eq!(
            result["trajectory_evaluation"]["snapshots"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            summary["trajectory_evaluation"]["comparisons"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(root.join("trajectory_state.json").exists());

        let _ = std::fs::remove_dir_all(root);
    }
}
