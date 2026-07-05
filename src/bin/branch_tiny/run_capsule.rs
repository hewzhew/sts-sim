use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::{
    combat_gap_case, frontier_checkpoint, run_capsule_format, Args, Branch, BranchStatus,
    TerminalOutcome,
};

pub(super) struct RunCapsule {
    root: PathBuf,
    started_at_ms: u128,
    git_commit: Option<String>,
}

pub(super) enum RunCapsuleSave {
    None,
    Frontier { running: usize },
    Result,
}

impl RunCapsule {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            started_at_ms: now_ms(),
            git_commit: current_git_commit(),
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
        let running = frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .count();
        if running > 0 {
            frontier_checkpoint::save(
                &self.root.join("frontier.json"),
                args,
                generation,
                next_branch_id,
                frontier,
            )?;
            remove_if_exists(&self.root.join("result.json"))?;
            remove_if_exists(&self.root.join("path.json"))?;
            self.write_manifest(args, "running", None)?;
            self.write_frontier_summary(args, generation, frontier, "running", None)?;
            return Ok(RunCapsuleSave::Frontier { running });
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
        self.write_manifest(
            args,
            run_capsule_format::terminal_manifest_status(&branch.status),
            None,
        )?;
        self.write_branch_summary(
            args,
            generation,
            branch,
            &combat_case,
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
        self.write_manifest(args, "completed", Some(reason))?;
        self.write_branch_summary(
            args,
            generation,
            branch,
            &combat_case,
            "completed",
            Some(reason),
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
        let running = frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .count();
        if running > 0 {
            frontier_checkpoint::save(
                &self.root.join("frontier.json"),
                args,
                generation,
                next_branch_id,
                frontier,
            )?;
            remove_if_exists(&self.root.join("result.json"))?;
            remove_if_exists(&self.root.join("path.json"))?;
            self.write_manifest(args, "paused", Some(reason))?;
            self.write_frontier_summary(args, generation, frontier, "paused", Some(reason))?;
            return Ok(RunCapsuleSave::Frontier { running });
        }
        self.save_recovery(args, generation, next_branch_id, frontier)
    }

    pub(super) fn save_terminal_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<(), String> {
        if !matches!(branch.status, BranchStatus::Terminal(_)) {
            return Ok(());
        }
        ensure_dir(&self.root)?;
        let path = self.terminal_path();
        let mut entries = read_terminal_entries(&path)?;
        if entries
            .iter()
            .any(|entry| entry.get("branch_id").and_then(Value::as_u64) == Some(branch.id as u64))
        {
            return Ok(());
        }
        let combat_case = combat_case_value(self, args, generation, branch);
        entries.push(run_capsule_format::result_value(
            generation,
            branch,
            combat_case,
        ));
        write_json(
            &path,
            json!({
                "schema": "branch_tiny_terminal_results",
                "terminals": entries,
            }),
        )
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
        let frontier_info = json!({
            "frontier_count": frontier.len(),
            "frontier_running_count": running,
        });
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
            json!({
                "schema": "branch_tiny_capsule_summary",
                "seed": args.seed,
                "ascension": args.ascension,
                "capsule_status": capsule_status,
                "reason": reason,
                "blocker_kind": reason.unwrap_or(capsule_status),
                "generation": generation,
                "capsule_path": self.root.display().to_string(),
                "frontier": frontier_info,
            }),
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
            json!({
                "schema": "branch_tiny_run_capsule",
                "seed": args.seed,
                "ascension": args.ascension,
                "status": status,
                "reason": reason,
                "created_at_epoch_ms": self.started_at_ms,
                "updated_at_epoch_ms": now_ms(),
                "git_commit": self.git_commit,
                "args": args,
            }),
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

fn write_json(path: &Path, value: Value) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, payload).map_err(|err| format!("failed to write {}: {err}", tmp.display()))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "failed to replace {} with {}: {err}",
            path.display(),
            tmp.display()
        )
    })
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove {}: {err}", path.display())),
    }
}

fn read_terminal_entries(path: &Path) -> Result<Vec<Value>, String> {
    let Ok(payload) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    Ok(value
        .get("terminals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| format!("failed to create {}: {err}", path.display()))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn current_git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
}
