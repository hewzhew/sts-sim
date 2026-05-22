use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::ai::combat_search_v2::CombatSearchV2Report;

pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME: &str = "CombatSearchEvidenceV1";
pub const COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchEvidenceContextV1 {
    pub source_kind: &'static str,
    pub decision_step: u64,
    pub capture_case_id: Option<String>,
    pub capture_root: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CombatSearchEvidenceV1<'a> {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub artifact_kind: &'static str,
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
