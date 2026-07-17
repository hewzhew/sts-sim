use serde_json::{json, Value};

use super::combat_search_report::{CombatSearchSessionReport, CombatSearchSessionStatus};

pub(super) fn capsule_value(report: &CombatSearchSessionReport) -> Value {
    json!({
        "status": status_value(&report.status),
        "profile_id": report.profile_id,
        "max_nodes": report.max_nodes,
        "wall_ms": report.wall_ms,
        "potion_policy": report.potion_policy,
        "max_potions_used": report.max_potions_used,
        "work_quanta": report.work_quanta.iter().map(quantum_value).collect::<Vec<_>>(),
        "action_keys": report.action_keys,
        "semantics_fingerprint": report.semantics_fingerprint,
        "candidate_tier": report.candidate_tier,
        "applied": report.applied,
        "decision": report.decision,
        "combat_final_hp": report.combat_final_hp,
        "run_hp": report.run_hp,
        "potions_used": report.potions_used,
        "turns": report.turns,
    })
}

pub(super) fn trace_value(report: &CombatSearchSessionReport) -> Value {
    json!({
        "status": status_value(&report.status),
        "profile": report.profile_id,
        "nodes": report.max_nodes,
        "ms": report.wall_ms,
        "potion_policy": report.potion_policy,
        "max_potions_used": report.max_potions_used,
        "work_quanta": report.work_quanta.iter().map(quantum_value).collect::<Vec<_>>(),
        "actions": report.action_keys,
        "semantics_fingerprint": report.semantics_fingerprint,
        "candidate_tier": report.candidate_tier,
        "applied": report.applied,
        "decision": report.decision,
        "combat_final_hp": report.combat_final_hp,
        "run_hp": report.run_hp,
        "potions_used": report.potions_used,
        "turns": report.turns,
    })
}

fn quantum_value(quantum: &super::combat_search_report::CombatSearchQuantumReport) -> Value {
    json!({
        "label": quantum.label,
        "additional_nodes": quantum.additional_nodes,
        "soft_wall_ms": quantum.soft_wall_ms,
    })
}

fn status_value(status: &CombatSearchSessionStatus) -> Value {
    match status {
        CombatSearchSessionStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
        CombatSearchSessionStatus::Advanced(boundary) => {
            json!({"kind": "advanced", "boundary": boundary})
        }
        CombatSearchSessionStatus::Terminal(result) => {
            json!({"kind": "terminal", "result": result.as_str()})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::owner_audit::combat_search_report::CombatSearchQuantumReport;

    #[test]
    fn search_json_exposes_one_session_and_incremental_work() {
        let report = CombatSearchSessionReport {
            status: CombatSearchSessionStatus::Advanced("PostCombat".to_string()),
            profile_id: "canonical_combat_session",
            max_nodes: 100,
            wall_ms: 200,
            potion_policy: "semantic",
            max_potions_used: Some(2),
            work_quanta: vec![CombatSearchQuantumReport {
                label: "initial",
                additional_nodes: 40,
                soft_wall_ms: Some(80),
            }],
            action_keys: vec!["selected-action".to_string()],
            semantics_fingerprint: "engine".to_string(),
            candidate_tier: Some("reserve_compliant_complete_win".to_string()),
            applied: true,
            decision: "accepted_clean_candidate".to_string(),
            combat_final_hp: Some(48),
            run_hp: Some(48),
            potions_used: Some(2),
            turns: Some(5),
        };

        for value in [capsule_value(&report), trace_value(&report)] {
            assert_eq!(value["work_quanta"].as_array().map(Vec::len), Some(1));
            assert!(value.get("attempts").is_none());
            assert_eq!(value["applied"], true);
            assert_eq!(value["semantics_fingerprint"], "engine");
        }
    }
}
