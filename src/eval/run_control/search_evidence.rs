use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::ai::combat_search_v2::CombatSearchV2Report;

pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME: &str = "CombatSearchEvidenceV1";
pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchEvidenceContextV1 {
    pub source_kind: &'static str,
    pub decision_step: u64,
    pub capture_case_id: Option<String>,
    pub capture_root: Option<String>,
    pub capture_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CombatSearchEvidenceV1<'a> {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub artifact_kind: &'static str,
    pub saved_at_unix_ms: u128,
    pub label_role: &'static str,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub context: CombatSearchEvidenceContextV1,
    pub report: &'a CombatSearchV2Report,
}

pub fn save_combat_search_evidence_v1(
    path: &Path,
    context: CombatSearchEvidenceContextV1,
    report: &CombatSearchV2Report,
) -> Result<(), String> {
    let evidence = CombatSearchEvidenceV1 {
        schema_name: COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME,
        schema_version: COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION,
        artifact_kind: "combat_search_evidence",
        saved_at_unix_ms: unix_ms_now(),
        label_role: "search_evidence_not_human_baseline",
        trainable_as_action_label: false,
        policy_quality_claim: false,
        context,
        report,
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(&evidence).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

pub fn load_combat_search_evidence_v1(path: &Path) -> Result<Value, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let evidence: Value = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    validate_combat_search_evidence_v1(&evidence)?;
    Ok(evidence)
}

pub fn validate_combat_search_evidence_v1(evidence: &Value) -> Result<(), String> {
    let object = evidence
        .as_object()
        .ok_or_else(|| "combat search evidence must be a JSON object".to_string())?;
    expect_string(
        object.get("schema_name"),
        COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME,
        "schema_name",
    )?;
    expect_u64(
        object.get("schema_version"),
        COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION as u64,
        "schema_version",
    )?;
    expect_string(
        object.get("artifact_kind"),
        "combat_search_evidence",
        "artifact_kind",
    )?;
    expect_string(
        object.get("label_role"),
        "search_evidence_not_human_baseline",
        "label_role",
    )?;
    expect_bool(
        object.get("trainable_as_action_label"),
        false,
        "trainable_as_action_label",
    )?;
    expect_bool(
        object.get("policy_quality_claim"),
        false,
        "policy_quality_claim",
    )?;
    validate_context(object.get("context"))?;
    validate_report(object.get("report"))?;
    Ok(())
}

fn validate_context(context: Option<&Value>) -> Result<(), String> {
    let context = context
        .and_then(Value::as_object)
        .ok_or_else(|| "combat search evidence context must be an object".to_string())?;
    let source_kind = context
        .get("source_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "combat search evidence context.source_kind is missing".to_string())?;
    if source_kind.trim().is_empty() {
        return Err("combat search evidence context.source_kind cannot be empty".to_string());
    }
    context
        .get("decision_step")
        .and_then(Value::as_u64)
        .ok_or_else(|| "combat search evidence context.decision_step is missing".to_string())?;
    Ok(())
}

fn validate_report(report: Option<&Value>) -> Result<(), String> {
    let report = report
        .and_then(Value::as_object)
        .ok_or_else(|| "combat search evidence report must be an object".to_string())?;
    expect_string(
        report.get("schema_name"),
        "CombatSearchV2Report",
        "report.schema_name",
    )?;
    expect_u64(report.get("schema_version"), 2, "report.schema_version")?;
    if !report.contains_key("outcome") {
        return Err("combat search evidence report.outcome is missing".to_string());
    }
    if !report.contains_key("budget") {
        return Err("combat search evidence report.budget is missing".to_string());
    }
    Ok(())
}

fn expect_string(value: Option<&Value>, expected: &str, field: &str) -> Result<(), String> {
    let actual = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("combat search evidence {field} is missing"))?;
    if actual != expected {
        return Err(format!(
            "combat search evidence {field} expected '{expected}', got '{actual}'"
        ));
    }
    Ok(())
}

fn expect_u64(value: Option<&Value>, expected: u64, field: &str) -> Result<(), String> {
    let actual = value
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("combat search evidence {field} is missing"))?;
    if actual != expected {
        return Err(format!(
            "combat search evidence {field} expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn expect_bool(value: Option<&Value>, expected: bool, field: &str) -> Result<(), String> {
    let actual = value
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("combat search evidence {field} is missing"))?;
    if actual != expected {
        return Err(format!(
            "combat search evidence {field} expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn unix_ms_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_combat_search_evidence_v1;

    #[test]
    fn search_evidence_validation_rejects_baseline_like_label_role() {
        let mut evidence = json!({
            "schema_name": "CombatSearchEvidenceV1",
            "schema_version": 1,
            "artifact_kind": "combat_search_evidence",
            "saved_at_unix_ms": 1,
            "label_role": "human_baseline",
            "trainable_as_action_label": false,
            "policy_quality_claim": false,
            "context": {
                "source_kind": "run_control_search_combat",
                "decision_step": 3,
                "capture_case_id": null,
                "capture_root": null,
                "capture_path": null
            },
            "report": {
                "schema_name": "CombatSearchV2Report",
                "schema_version": 2,
                "outcome": {},
                "budget": {}
            }
        });

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("search evidence cannot claim baseline label role");

        assert!(err.contains("label_role"));
        evidence["label_role"] = json!("search_evidence_not_human_baseline");
        validate_combat_search_evidence_v1(&evidence)
            .expect("valid search evidence envelope should pass");
    }
}
