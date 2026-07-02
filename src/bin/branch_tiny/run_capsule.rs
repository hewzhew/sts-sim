use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use sts_simulator::ai::strategy::deck_strategic_deficit::assess_deck_strategic_deficit;
use sts_simulator::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_simulator::eval::combat_case::combat_summary;
use sts_simulator::eval::run_control::RunControlAutoAppliedKindV1;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::sim::combat::CombatPosition;
use sts_simulator::state::run::RunState;

use super::{
    combat_gap_case, frontier_checkpoint, Args, BossRetryAttemptReport, BossRetryReport,
    BossRetryStatus, Branch, BranchPathStep, BranchStatus, TerminalOutcome,
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
            result_value(generation, branch, combat_case.clone()),
        )?;
        write_json(&self.root.join("path.json"), path_value(branch))?;
        remove_if_exists(&self.root.join("frontier.json"))?;
        self.write_manifest(args, terminal_manifest_status(&branch.status), None)?;
        self.write_branch_summary(
            args,
            generation,
            branch,
            &combat_case,
            terminal_manifest_status(&branch.status),
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
            result_value(generation, branch, combat_case.clone()),
        )?;
        write_json(&self.root.join("path.json"), path_value(branch))?;
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
        entries.push(result_value(generation, branch, combat_case));
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
            branch_summary_value(
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
                branch_summary_value(
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

fn branch_summary_value(
    capsule_path: &Path,
    args: Args,
    generation: usize,
    branch: &Branch,
    combat_case: &Value,
    capsule_status: &'static str,
    reason: Option<&'static str>,
    frontier: Option<Value>,
) -> Value {
    let run = &branch.session.run_state;
    let status = status_value(&branch.status);
    let status_kind = status
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or(capsule_status);
    let blocker_kind = if capsule_status == "paused" {
        reason.unwrap_or("paused")
    } else {
        status_kind
    };
    let combat = active_combat_value(branch);
    let enemies = combat
        .as_ref()
        .and_then(|value| value.get("enemies"))
        .cloned()
        .unwrap_or(Value::Null);
    let subject = enemies
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|text| !text.is_empty());
    let (next_recommended_command, next_recommended_reason) =
        next_recommendation(capsule_path, args, &branch.status, combat_case);
    let mut value = json!({
        "schema": "branch_tiny_capsule_summary",
        "seed": args.seed,
        "ascension": args.ascension,
        "capsule_status": capsule_status,
        "reason": reason,
        "blocker_kind": blocker_kind,
        "generation": generation,
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "status": status,
        "act": run.act_num,
        "floor": run.floor_num,
        "hp": run.current_hp,
        "max_hp": run.max_hp,
        "gold": run.gold,
        "deck_size": run.master_deck.len(),
        "subject": subject,
        "enemies": enemies,
        "capsule_path": capsule_path.display().to_string(),
        "combat_case": combat_case,
        "next_recommended_command": next_recommended_command,
        "next_recommended_reason": next_recommended_reason,
    });
    if let Some(frontier) = frontier {
        value["frontier"] = frontier;
    }
    value
}

fn next_recommendation(
    capsule_path: &Path,
    args: Args,
    status: &BranchStatus,
    combat_case: &Value,
) -> (Option<String>, Option<&'static str>) {
    if matches!(
        status,
        BranchStatus::CombatGap { .. } | BranchStatus::BudgetGap { .. }
    ) {
        return (
            combat_case.as_str().map(|case| {
                format!(
                    ".\\target\\debug\\combat_case_review.exe --case \"{}\" --ladder --compact",
                    case
                )
            }),
            Some("combat_case_review"),
        );
    }
    if status.is_resumable() {
        return (
            args.wall_ms.map(|wall_ms| {
                format!(
                ".\\target\\debug\\branch_tiny.exe --continue-capsule \"{}\" --continue-slices 1 --wall-ms {}",
                capsule_path.display(),
                wall_ms
            )
            }),
            Some("continue_capsule"),
        );
    }
    (None, None)
}

fn result_value(generation: usize, branch: &Branch, combat_case: Value) -> Value {
    let run = &branch.session.run_state;
    json!({
        "schema": "branch_tiny_run_result",
        "generation": generation,
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "status": status_value(&branch.status),
        "state": {
            "act": run.act_num,
            "floor": run.floor_num,
            "hp": run.current_hp,
            "max_hp": run.max_hp,
            "gold": run.gold,
            "deck_size": run.master_deck.len(),
            "strategic_deficit": strategic_deficit_value(run),
        },
        "deck": run.master_deck.iter().map(card_value).collect::<Vec<_>>(),
        "relics": run.relics.iter().map(|relic| {
            let mut value = Map::from_iter([("id".to_string(), json!(relic.id))]);
            if relic.counter != -1 {
                value.insert("counter".to_string(), json!(relic.counter));
            }
            if relic.used_up {
                value.insert("used_up".to_string(), json!(true));
            }
            if relic.amount != 0 {
                value.insert("amount".to_string(), json!(relic.amount));
            }
            Value::Object(value)
        }).collect::<Vec<_>>(),
        "potions": run.potions.iter().map(|slot| {
            slot.as_ref().map(|potion| json!({"id": potion.id, "uuid": potion.uuid}))
        }).collect::<Vec<_>>(),
        "path": path_value(branch),
        "auto": branch.auto_steps.iter()
            .filter(|step| step.kind != RunControlAutoAppliedKindV1::AutoCapture)
            .map(|step| json!({"kind": format!("{:?}", step.kind), "label": step.label}))
            .collect::<Vec<_>>(),
        "combat": active_combat_value(branch),
        "combat_case": combat_case,
        "boss_retry": branch.boss_retry.as_ref().map(boss_retry_value),
        "combat_search_attempts": &branch.combat_search,
        "failed_search": branch.combat_search.last(),
    })
}

fn strategic_deficit_value(run: &RunState) -> Value {
    serde_json::to_value(assess_deck_strategic_deficit(
        &run.master_deck,
        RunStrategicFacts::from_run_state(run),
    ))
    .unwrap_or(Value::Null)
}

fn boss_retry_value(report: &BossRetryReport) -> Value {
    json!({
        "status": boss_retry_status_value(&report.status),
        "max_nodes": report.max_nodes,
        "wall_ms": report.wall_ms,
        "action_keys": report.action_keys,
        "attempts": report.attempts.iter().map(boss_retry_attempt_value).collect::<Vec<_>>(),
    })
}

fn boss_retry_attempt_value(attempt: &BossRetryAttemptReport) -> Value {
    json!({
        "label": attempt.label,
        "status": boss_retry_status_value(&attempt.status),
        "max_nodes": attempt.max_nodes,
        "wall_ms": attempt.wall_ms,
        "potion_policy": attempt.potion_policy,
        "max_potions_used": attempt.max_potions_used,
        "action_keys": attempt.action_keys,
    })
}

fn boss_retry_status_value(status: &BossRetryStatus) -> Value {
    match status {
        BossRetryStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
        BossRetryStatus::Advanced(boundary) => json!({"kind": "advanced", "boundary": boundary}),
        BossRetryStatus::Terminal(result) => {
            json!({"kind": "terminal", "result": result.as_str()})
        }
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
        BranchStatus::CombatGap { .. } | BranchStatus::BudgetGap { .. }
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

fn active_combat_value(branch: &Branch) -> Option<Value> {
    let active = branch.session.active_combat.as_ref()?;
    serde_json::to_value(combat_summary(&CombatPosition::new(
        active.engine_state.clone(),
        active.combat_state.clone(),
    )))
    .ok()
}

fn path_value(branch: &Branch) -> Value {
    json!({
        "schema": "branch_tiny_run_path",
        "branch_id": branch.id,
        "steps": branch.path.iter().enumerate().map(path_step_value).collect::<Vec<_>>(),
    })
}

fn path_step_value((index, step): (usize, &BranchPathStep)) -> Value {
    json!({
        "step": index,
        "state_before": step.state_before.as_ref(),
        "key": serde_json::to_value(&step.key).unwrap_or(Value::Null),
        "label": step.label,
        "annotation": serde_json::to_value(&step.annotation).unwrap_or(Value::Null),
    })
}

fn status_value(status: &BranchStatus) -> Value {
    match status {
        BranchStatus::Running { boundary, owner } => {
            json!({"kind": "running", "boundary": boundary, "owner": format!("{owner:?}")})
        }
        BranchStatus::AwaitingAuto { boundary, reason } => {
            json!({"kind": "awaiting_auto", "boundary": boundary, "reason": reason})
        }
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
        BranchStatus::AutomationGap { boundary, site } => {
            json!({"kind": "automation_gap", "boundary": boundary, "site": format!("{site:?}")})
        }
        BranchStatus::CombatGap { boundary, reason } => {
            json!({"kind": "combat_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::BudgetGap { boundary, reason } => {
            json!({"kind": "budget_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::ApplyFailed(reason) => json!({"kind": "apply_failed", "reason": reason}),
        BranchStatus::AdvanceFailed(reason) => {
            json!({"kind": "advance_failed", "reason": reason})
        }
    }
}

fn terminal_manifest_status(status: &BranchStatus) -> &'static str {
    match status {
        BranchStatus::Terminal(_) => "terminal",
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => "running",
        BranchStatus::AutomationGap { .. }
        | BranchStatus::CombatGap { .. }
        | BranchStatus::BudgetGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "gap",
    }
}

fn card_value(card: &CombatCard) -> Value {
    let mut value = Map::from_iter([
        ("id".to_string(), json!(card.id)),
        ("uuid".to_string(), json!(card.uuid)),
    ]);
    if card.upgrades != 0 {
        value.insert("upgrades".to_string(), json!(card.upgrades));
    }
    if card.misc_value != 0 {
        value.insert("misc".to_string(), json!(card.misc_value));
    }
    Value::Object(value)
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
