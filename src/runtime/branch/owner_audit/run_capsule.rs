use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::run_identity::{current_source_identity, SourceIdentity};
use super::run_slice_result::ArtifactWriteSummary;
use super::{
    combat_gap_case, frontier_checkpoint, run_capsule_format, run_capsule_io, Args, Branch,
    BranchStatus, TerminalOutcome,
};
use run_capsule_io::{ensure_dir, read_terminal_entries, remove_if_exists, write_json};

pub(super) struct RunCapsule {
    root: PathBuf,
    started_at_ms: u128,
    git_commit: Option<String>,
    source_identity: SourceIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RunCapsuleSave {
    None,
    Frontier { running: usize },
    Result,
}

impl RunCapsuleSave {
    pub(super) fn artifact_writes(self) -> ArtifactWriteSummary {
        match self {
            Self::None => ArtifactWriteSummary::default(),
            Self::Frontier { .. } => ArtifactWriteSummary {
                manifest_written: true,
                frontier_written: true,
                summary_written: true,
                ..ArtifactWriteSummary::default()
            },
            Self::Result => ArtifactWriteSummary {
                manifest_written: true,
                result_written: true,
                path_written: true,
                summary_written: true,
                ..ArtifactWriteSummary::default()
            },
        }
    }
}

impl RunCapsule {
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

    pub(super) fn write_running_manifest(&self, args: Args) -> Result<(), String> {
        self.write_manifest(args, "running", None)
    }

    pub(super) fn save_recovery(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<RunCapsuleSave, String> {
        if let Some(branch) = frontier.iter().find(|branch| {
            matches!(
                branch.status,
                BranchStatus::Terminal(TerminalOutcome::Victory)
            )
        }) {
            self.save_completed_result(args, generation, branch, "victory_found")?;
            return Ok(RunCapsuleSave::Result);
        }
        if let Some(save) =
            self.save_frontier(args, generation, next_branch_id, frontier, "running", None)?
        {
            return Ok(save);
        }
        if let Some(branch) = frontier.front() {
            self.save_result(args, generation, branch)?;
            return Ok(RunCapsuleSave::Result);
        }
        Ok(RunCapsuleSave::None)
    }

    pub(super) fn save_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<(), String> {
        self.save_branch_result(
            args,
            generation,
            branch,
            run_capsule_format::terminal_manifest_status(&branch.status),
            None,
        )
    }

    pub(super) fn save_completed_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        reason: &'static str,
    ) -> Result<(), String> {
        self.save_branch_result(args, generation, branch, "completed", Some(reason))
    }

    fn save_branch_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<(), String> {
        ensure_dir(&self.root)?;
        let combat_case = combat_case_value(self, args, generation, branch);
        write_json(
            &self.root.join("result.json"),
            run_capsule_format::result_value(generation, branch, combat_case.clone()),
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
            capsule_status,
            reason,
        )
    }

    pub(super) fn save_paused_recovery(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        reason: &'static str,
    ) -> Result<RunCapsuleSave, String> {
        if let Some(save) = self.save_frontier(
            args,
            generation,
            next_branch_id,
            frontier,
            "paused",
            Some(reason),
        )? {
            return Ok(save);
        }
        self.save_recovery(args, generation, next_branch_id, frontier)
    }

    fn save_frontier(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<Option<RunCapsuleSave>, String> {
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
        Ok(Some(RunCapsuleSave::Frontier { running }))
    }

    pub(super) fn save_terminal_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<ArtifactWriteSummary, String> {
        if !matches!(branch.status, BranchStatus::Terminal(_)) {
            return Ok(ArtifactWriteSummary::default());
        }
        ensure_dir(&self.root)?;
        let path = self.terminal_path();
        let mut entries = read_terminal_entries(&path)?;
        if entries
            .iter()
            .any(|entry| entry.get("branch_id").and_then(Value::as_u64) == Some(branch.id as u64))
        {
            return Ok(ArtifactWriteSummary::default());
        }
        let combat_case = combat_case_value(self, args, generation, branch);
        entries.push(run_capsule_format::result_value(
            generation,
            branch,
            combat_case,
        ));
        write_json(&path, run_capsule_format::terminal_results_value(entries))?;
        Ok(ArtifactWriteSummary {
            terminal_written: true,
            ..ArtifactWriteSummary::default()
        })
    }

    fn write_branch_summary(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        combat_case: &Value,
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
}

fn combat_case_value(
    capsule: &RunCapsule,
    args: Args,
    generation: usize,
    branch: &Branch,
) -> Value {
    if !matches!(
        branch.status,
        BranchStatus::CombatGap { .. }
            | BranchStatus::OperationBudgetExhausted { .. }
            | BranchStatus::BudgetGap { .. }
    ) {
        return Value::Null;
    }
    match combat_gap_case::save_combat_gap_case(
        &capsule.combat_cases_dir(),
        args,
        generation,
        branch,
    ) {
        Ok(Some(path)) => json!(path.display().to_string()),
        Ok(None) => Value::Null,
        Err(error) => json!({"error": error}),
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
