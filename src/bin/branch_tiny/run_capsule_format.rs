use std::path::Path;

use serde_json::{json, Value};
use sts_simulator::eval::combat_case::combat_summary;
use sts_simulator::eval::run_control::RunControlAutoAppliedKindV1;
use sts_simulator::sim::combat::CombatPosition;

use super::{combat_portfolio_json, run_state_json, Args, Branch, BranchPathStep, BranchStatus};

pub(super) fn branch_summary_value(
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
        BranchStatus::CombatGap { .. }
            | BranchStatus::OperationBudgetExhausted { .. }
            | BranchStatus::BudgetGap { .. }
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

pub(super) fn result_value(generation: usize, branch: &Branch, combat_case: Value) -> Value {
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
            "strategic_deficit": run_state_json::strategic_deficit_value(run),
        },
        "deck": run_state_json::deck_value(run),
        "relics": run_state_json::relics_value(run),
        "potions": run_state_json::potions_value(run),
        "path": path_value(branch),
        "auto": branch.auto_steps.iter()
            .filter(|step| step.kind != RunControlAutoAppliedKindV1::AutoCapture)
            .map(|step| json!({"kind": format!("{:?}", step.kind), "label": step.label}))
            .collect::<Vec<_>>(),
        "combat": active_combat_value(branch),
        "combat_case": combat_case,
        "combat_portfolio": branch.combat_portfolio.as_ref().map(combat_portfolio_json::capsule_value),
        "combat_search_attempts": &branch.combat_search,
        "failed_search": branch.combat_search.last(),
    })
}

fn active_combat_value(branch: &Branch) -> Option<Value> {
    let active = branch.session.active_combat.as_ref()?;
    serde_json::to_value(combat_summary(&CombatPosition::new(
        active.engine_state.clone(),
        active.combat_state.clone(),
    )))
    .ok()
}

pub(super) fn path_value(branch: &Branch) -> Value {
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
        "decision_delta": step.decision_delta.as_ref(),
        "key": serde_json::to_value(&step.key).unwrap_or(Value::Null),
        "label": step.label,
        "annotation": serde_json::to_value(&step.annotation).unwrap_or(Value::Null),
    })
}

pub(super) fn status_value(status: &BranchStatus) -> Value {
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
        BranchStatus::OperationBudgetExhausted { boundary, reason } => {
            json!({"kind": "operation_budget_exhausted", "boundary": boundary, "reason": reason})
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

pub(super) fn terminal_manifest_status(status: &BranchStatus) -> &'static str {
    match status {
        BranchStatus::Terminal(_) => "terminal",
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => "running",
        BranchStatus::AutomationGap { .. }
        | BranchStatus::CombatGap { .. }
        | BranchStatus::OperationBudgetExhausted { .. }
        | BranchStatus::BudgetGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "gap",
    }
}
