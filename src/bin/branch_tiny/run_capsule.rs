use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use sts_simulator::ai::strategy::decision_pipeline::candidate_lane_label;
use sts_simulator::eval::combat_case::combat_summary;
use sts_simulator::eval::run_control::RunControlAutoAppliedKindV1;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::sim::combat::CombatPosition;

use super::owners::ChoiceAnnotation;
use super::{combat_gap_case, frontier_checkpoint, Args, Branch, BranchPathStep, BranchStatus};

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

    pub(super) fn write_running_manifest(&self, args: Args) -> Result<(), String> {
        self.write_manifest(args, "running")
    }

    pub(super) fn save_recovery(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<RunCapsuleSave, String> {
        let running = frontier
            .iter()
            .filter(|branch| matches!(branch.status, BranchStatus::Running { .. }))
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
            self.write_manifest(args, "running")?;
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
            result_value(generation, branch, combat_case),
        )?;
        write_json(&self.root.join("path.json"), path_value(branch))?;
        remove_if_exists(&self.root.join("frontier.json"))?;
        self.write_manifest(args, terminal_manifest_status(&branch.status))
    }

    fn write_manifest(&self, args: Args, status: &'static str) -> Result<(), String> {
        ensure_dir(&self.root)?;
        write_json(
            &self.root.join("manifest.json"),
            json!({
                "schema": "branch_tiny_run_capsule",
                "seed": args.seed,
                "ascension": args.ascension,
                "status": status,
                "created_at_epoch_ms": self.started_at_ms,
                "updated_at_epoch_ms": now_ms(),
                "git_commit": self.git_commit,
                "args": args,
            }),
        )
    }
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
        "auto": branch.auto_steps.iter()
            .filter(|step| step.kind != RunControlAutoAppliedKindV1::AutoCapture)
            .map(|step| json!({"kind": format!("{:?}", step.kind), "label": step.label}))
            .collect::<Vec<_>>(),
        "combat": active_combat_value(branch),
        "combat_case": combat_case,
        "combat_search_attempts": &branch.combat_search,
        "failed_search": branch.combat_search.last(),
    })
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
        "key": serde_json::to_value(&step.key).unwrap_or(Value::Null),
        "label": step.label,
        "annotation": annotation_value(&step.annotation),
    })
}

fn annotation_value(annotation: &ChoiceAnnotation) -> Value {
    match annotation {
        ChoiceAnnotation::None => Value::Null,
        ChoiceAnnotation::Candidate(decision) => json!({
            "kind": "candidate",
            "lane": candidate_lane_label(decision.evaluation.lane),
            "score": decision.evaluation.total_score(),
            "admission": decision.admission.as_ref().map(|admission| json!({
                "card": admission.card,
                "class": format!("{:?}", admission.class),
            })),
        }),
        ChoiceAnnotation::BossRelic(admission) => json!({
            "kind": "boss_relic",
            "relic": admission.relic,
            "lane": format!("{:?}", admission.lane),
            "class": format!("{:?}", admission.class),
        }),
    }
}

fn status_value(status: &BranchStatus) -> Value {
    match status {
        BranchStatus::Running { boundary, owner } => {
            json!({"kind": "running", "boundary": boundary, "owner": format!("{owner:?}")})
        }
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result}),
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
        BranchStatus::Running { .. } => "running",
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
