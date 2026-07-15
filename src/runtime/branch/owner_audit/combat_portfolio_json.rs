use serde_json::{json, Value};

use super::combat_search_report::{
    CombatSearchLaneReport, CombatSearchPortfolioReport, CombatSearchPortfolioStatus,
};

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
        "engine_fingerprint": attempt.engine_fingerprint,
        "candidate_tier": attempt.candidate_tier,
        "selected": attempt.selected,
        "incumbent_reason": attempt.incumbent_reason,
        "combat_final_hp": attempt.combat_final_hp,
        "run_hp": attempt.run_hp,
        "potions_used": attempt.potions_used,
        "turns": attempt.turns,
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
        "engine_fingerprint": attempt.engine_fingerprint,
        "candidate_tier": attempt.candidate_tier,
        "selected": attempt.selected,
        "incumbent_reason": attempt.incumbent_reason,
        "combat_final_hp": attempt.combat_final_hp,
        "run_hp": attempt.run_hp,
        "potions_used": attempt.potions_used,
        "turns": attempt.turns,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_json_identifies_one_selected_incumbent() {
        let attempt = |label: &'static str, selected: bool| CombatSearchLaneReport {
            label,
            status: CombatSearchPortfolioStatus::Advanced("PostCombat".to_string()),
            max_nodes: 100,
            wall_ms: 200,
            potion_policy: "semantic",
            max_potions_used: Some(2),
            action_keys: vec![format!("{label}-action")],
            engine_fingerprint: format!("engine-{label}"),
            candidate_tier: Some("reserve_compliant_complete_win".to_string()),
            selected,
            incumbent_reason: if selected {
                "strict_resource_dominance".to_string()
            } else {
                "replaced_by_better_candidate".to_string()
            },
            combat_final_hp: Some(if selected { 48 } else { 38 }),
            run_hp: Some(if selected { 48 } else { 38 }),
            potions_used: Some(2),
            turns: Some(5),
        };
        let report = CombatSearchPortfolioReport {
            status: CombatSearchPortfolioStatus::Advanced("PostCombat".to_string()),
            max_nodes: 100,
            wall_ms: 200,
            action_keys: vec!["lazy-action".to_string()],
            attempts: vec![attempt("immediate", false), attempt("lazy", true)],
        };

        for value in [capsule_value(&report), trace_value(&report)] {
            let attempts = value["attempts"].as_array().expect("attempt array");
            assert_eq!(
                attempts
                    .iter()
                    .filter(|attempt| attempt["selected"] == true)
                    .count(),
                1
            );
            assert!(attempts.iter().all(|attempt| attempt["engine_fingerprint"]
                .as_str()
                .is_some_and(|value| !value.is_empty())));
            assert!(attempts.iter().all(|attempt| attempt["incumbent_reason"]
                .as_str()
                .is_some_and(|value| !value.is_empty())));
        }
    }
}
