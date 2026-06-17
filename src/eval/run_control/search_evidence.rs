use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::ai::combat_search_v2::{
    CombatSearchV2HiddenInformationRisk, CombatSearchV2InformationAccess, CombatSearchV2Report,
};

pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME: &str = "CombatSearchEvidenceV1";
pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const COMBAT_SEARCH_REPORT_SCHEMA_VERSION: u64 = 9;

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
    expect_u64_range(
        report.get("schema_version"),
        7,
        COMBAT_SEARCH_REPORT_SCHEMA_VERSION,
        "report.schema_version",
    )?;
    if !report.contains_key("outcome") {
        return Err("combat search evidence report.outcome is missing".to_string());
    }
    if !report.contains_key("budget") {
        return Err("combat search evidence report.budget is missing".to_string());
    }
    validate_policy_evidence(report.get("policy_evidence"))?;
    Ok(())
}

fn validate_policy_evidence(policy_evidence: Option<&Value>) -> Result<(), String> {
    let policy_evidence = policy_evidence
        .and_then(Value::as_object)
        .ok_or_else(|| "combat search evidence report.policy_evidence is missing".to_string())?;
    let information_access_label = policy_evidence
        .get("information_access")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "combat search evidence report.policy_evidence.information_access is missing"
                .to_string()
        })?;
    let information_access =
        CombatSearchV2InformationAccess::from_label(information_access_label).ok_or_else(|| {
            format!(
                "combat search evidence report.policy_evidence.information_access unknown label '{information_access_label}'"
            )
        })?;
    if information_access != CombatSearchV2InformationAccess::PrivilegedSimulator {
        return Err(format!(
            "combat search evidence report.policy_evidence.information_access expected '{}', got '{information_access_label}'",
            CombatSearchV2InformationAccess::PrivilegedSimulator.label()
        ));
    }
    expect_bool(
        policy_evidence.get("public_safe"),
        false,
        "report.policy_evidence.public_safe",
    )?;
    let risks = policy_evidence
        .get("hidden_information_risks")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "combat search evidence report.policy_evidence.hidden_information_risks is missing"
                .to_string()
        })?;
    let mut parsed_risks = Vec::new();
    for risk in risks {
        let risk_label = risk.as_str().ok_or_else(|| {
            "combat search evidence report.policy_evidence.hidden_information_risks must contain strings"
                .to_string()
        })?;
        let risk = CombatSearchV2HiddenInformationRisk::from_label(risk_label).ok_or_else(|| {
            format!(
                "combat search evidence report.policy_evidence.hidden_information_risks unknown label '{risk_label}'"
            )
        })?;
        if parsed_risks.contains(&risk) {
            return Err(format!(
                "combat search evidence report.policy_evidence.hidden_information_risks duplicate label '{risk_label}'"
            ));
        }
        parsed_risks.push(risk);
    }
    for required in [
        CombatSearchV2HiddenInformationRisk::PrivilegedSimulatorState,
        CombatSearchV2HiddenInformationRisk::ExactRngState,
    ] {
        if !parsed_risks.contains(&required) {
            return Err(format!(
                "combat search evidence report.policy_evidence.hidden_information_risks must include {}",
                required.label()
            ));
        }
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

fn expect_u64_range(value: Option<&Value>, min: u64, max: u64, field: &str) -> Result<(), String> {
    let actual = value
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("combat search evidence {field} is missing"))?;
    if !(min..=max).contains(&actual) {
        return Err(format!(
            "combat search evidence {field} expected {min}..={max}, got {actual}"
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
    use serde_json::Value;

    use super::validate_combat_search_evidence_v1;

    fn valid_search_evidence() -> Value {
        json!({
            "schema_name": "CombatSearchEvidenceV1",
            "schema_version": 1,
            "artifact_kind": "combat_search_evidence",
            "saved_at_unix_ms": 1,
            "label_role": "search_evidence_not_human_baseline",
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
                "schema_version": 8,
                "policy_evidence": {
                    "information_access": "privileged_simulator",
                    "public_safe": false,
                    "hidden_information_risks": [
                        "privileged_simulator_state",
                        "exact_rng_state"
                    ]
                },
                "outcome": {},
                "budget": {}
            }
        })
    }

    #[test]
    fn search_evidence_validation_rejects_baseline_like_label_role() {
        let mut evidence = valid_search_evidence();
        evidence["label_role"] = json!("human_baseline");

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("search evidence cannot claim baseline label role");

        assert!(err.contains("label_role"));
        evidence["label_role"] = json!("search_evidence_not_human_baseline");
        validate_combat_search_evidence_v1(&evidence)
            .expect("valid search evidence envelope should pass");
    }

    #[test]
    fn search_evidence_validation_accepts_previous_search_report_schema() {
        let mut evidence = valid_search_evidence();
        evidence["report"]["schema_version"] = json!(7);

        validate_combat_search_evidence_v1(&evidence)
            .expect("v7 search reports should remain readable after adding performance fields");
    }

    #[test]
    fn search_evidence_validation_requires_policy_evidence_boundary() {
        let mut evidence = valid_search_evidence();
        evidence["report"]
            .as_object_mut()
            .expect("report should be an object")
            .remove("policy_evidence");

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("search report must declare its policy evidence boundary");

        assert!(err.contains("report.policy_evidence"));
    }

    #[test]
    fn search_evidence_validation_rejects_unknown_policy_risk_label() {
        let mut evidence = valid_search_evidence();
        evidence["report"]["policy_evidence"]["hidden_information_risks"] = json!([
            "privileged_simulator_state",
            "exact_rng_state",
            "unknown_future_risk"
        ]);

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("policy evidence risk labels must be from the known registry");

        assert!(err.contains("unknown_future_risk"));
    }

    #[test]
    fn search_evidence_validation_rejects_duplicate_policy_risk_label() {
        let mut evidence = valid_search_evidence();
        evidence["report"]["policy_evidence"]["hidden_information_risks"] = json!([
            "privileged_simulator_state",
            "exact_rng_state",
            "exact_rng_state"
        ]);

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("policy evidence risk labels must not be duplicated");

        assert!(err.contains("duplicate"));
    }

    #[test]
    fn search_evidence_validation_requires_exact_rng_risk_for_privileged_search() {
        let mut evidence = valid_search_evidence();
        evidence["report"]["policy_evidence"]["hidden_information_risks"] =
            json!(["privileged_simulator_state"]);

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("privileged simulator search must declare exact RNG access");

        assert!(err.contains("exact_rng_state"));
    }

    #[test]
    fn search_evidence_validation_rejects_public_safe_privileged_search_claim() {
        let mut evidence = valid_search_evidence();
        evidence["report"]["policy_evidence"]["public_safe"] = json!(true);

        let err = validate_combat_search_evidence_v1(&evidence)
            .expect_err("privileged simulator search evidence cannot claim public safety");

        assert!(err.contains("public_safe"));
    }
}
