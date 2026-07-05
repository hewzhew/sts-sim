use serde_json::{json, Value};

use super::{CombatSearchLaneReport, CombatSearchPortfolioReport, CombatSearchPortfolioStatus};

pub(super) fn capsule_value(report: &CombatSearchPortfolioReport) -> Value {
    json!({
        "status": status_value(&report.status),
        "max_nodes": report.max_nodes,
        "wall_ms": report.wall_ms,
        "action_keys": report.action_keys,
        "attempts": report.attempts.iter().map(capsule_attempt_value).collect::<Vec<_>>(),
    })
}

pub(super) fn trace_value(report: &CombatSearchPortfolioReport) -> Value {
    json!({
        "status": status_value(&report.status),
        "nodes": report.max_nodes,
        "ms": report.wall_ms,
        "actions": report.action_keys,
        "attempts": report.attempts.iter().map(trace_attempt_value).collect::<Vec<_>>(),
    })
}

fn capsule_attempt_value(attempt: &CombatSearchLaneReport) -> Value {
    json!({
        "label": attempt.label,
        "status": status_value(&attempt.status),
        "max_nodes": attempt.max_nodes,
        "wall_ms": attempt.wall_ms,
        "potion_policy": attempt.potion_policy,
        "max_potions_used": attempt.max_potions_used,
        "action_keys": attempt.action_keys,
    })
}

fn trace_attempt_value(attempt: &CombatSearchLaneReport) -> Value {
    json!({
        "label": attempt.label,
        "status": status_value(&attempt.status),
        "nodes": attempt.max_nodes,
        "ms": attempt.wall_ms,
        "potion_policy": attempt.potion_policy,
        "max_potions_used": attempt.max_potions_used,
        "actions": attempt.action_keys,
    })
}

fn status_value(status: &CombatSearchPortfolioStatus) -> Value {
    match status {
        CombatSearchPortfolioStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
        CombatSearchPortfolioStatus::Advanced(boundary) => {
            json!({"kind": "advanced", "boundary": boundary})
        }
        CombatSearchPortfolioStatus::Terminal(result) => {
            json!({"kind": "terminal", "result": result.as_str()})
        }
    }
}
