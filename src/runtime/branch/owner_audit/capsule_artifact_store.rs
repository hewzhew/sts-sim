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
    run_capsule_io, Args, Branch, BranchStatus,
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
        self.record_accepted_combat_diagnostic_refs(&mut summary);
        summary
    }

    pub(super) fn terminal_summary(&self) -> ArtifactWriteSummary {
        let mut summary = ArtifactWriteSummary::single_ref(self.artifact_ref(
            ArtifactKind::Terminal,
            "terminal.json",
            "branch_tiny_terminal_results",
        ));
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
        entries.push(run_capsule_format::result_value(
            generation,
            branch,
            combat_case,
            accepted_high_loss_combat_diagnostics,
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
        write_json(
            &self.root.join("result.json"),
            run_capsule_format::result_value(
                generation,
                branch,
                combat_case.clone(),
                accepted_high_loss_combat_diagnostics.clone(),
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
        let frontier_info =
            run_capsule_format::frontier_summary_info_value(frontier.len(), running);
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
            let schema = if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".capture.json"))
            {
                "CombatCaptureV1"
            } else {
                "accepted_high_loss_combat_evidence_v1"
            };
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
