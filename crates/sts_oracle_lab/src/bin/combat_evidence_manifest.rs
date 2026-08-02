//! Typed provenance contract for exact combat case/action evidence.

use std::fs;
use std::path::{Path, PathBuf};

use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts_oracle_runtime::sim::combat::CombatTerminal;
use sts_oracle_runtime::state::core::ClientInput;

use super::canonical_launch::{runtime_identity, runtime_source_content_fingerprint};

pub(super) const COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME: &str = "CombatEvidenceManifestV1";
pub(super) const COMBAT_EVIDENCE_MANIFEST_FILE_SUFFIX: &str = "combat-evidence-manifest.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CombatEvidenceProducerV1 {
    PotionExpenditureAudit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CombatEvidenceManifestV1 {
    pub(super) schema_name: String,
    pub(super) schema_version: u32,
    pub(super) producer: CombatEvidenceProducerV1,
    pub(super) runtime: Value,
    pub(super) runtime_source_content_fingerprint: String,
    pub(super) root_exact_state_hash: String,
    pub(super) case_path: PathBuf,
    pub(super) entries: Vec<CombatEvidenceManifestEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CombatEvidenceManifestEntryV1 {
    pub(super) evidence_id: String,
    pub(super) action_paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) trace_paths: Vec<PathBuf>,
    pub(super) action_sequence_blake2b_512: String,
    pub(super) supplied_action_count: usize,
    pub(super) expected_terminal: CombatTerminal,
    pub(super) expected_final_player_hp: Option<i32>,
}

impl CombatEvidenceManifestEntryV1 {
    pub(super) fn from_actions(
        evidence_id: String,
        action_paths: Vec<PathBuf>,
        actions: &[ClientInput],
        expected_terminal: CombatTerminal,
        expected_final_player_hp: Option<i32>,
    ) -> Result<Self, String> {
        if evidence_id.trim().is_empty() {
            return Err("combat evidence manifest evidence_id must not be empty".to_string());
        }
        if action_paths.is_empty() {
            return Err(format!(
                "combat evidence manifest entry '{evidence_id}' has no action paths"
            ));
        }
        Ok(Self {
            evidence_id,
            action_paths,
            trace_paths: Vec::new(),
            action_sequence_blake2b_512: combat_action_sequence_hash(actions)?,
            supplied_action_count: actions.len(),
            expected_terminal,
            expected_final_player_hp,
        })
    }
}

pub(super) fn write_combat_evidence_manifest(
    path: &Path,
    producer: CombatEvidenceProducerV1,
    root_exact_state_hash: String,
    case_path: PathBuf,
    entries: Vec<CombatEvidenceManifestEntryV1>,
) -> Result<(), String> {
    if root_exact_state_hash.trim().is_empty() {
        return Err("combat evidence manifest exact root hash must not be empty".to_string());
    }
    if entries.is_empty() {
        return Err("combat evidence manifest must contain at least one entry".to_string());
    }
    let manifest = CombatEvidenceManifestV1 {
        schema_name: COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME.to_string(),
        schema_version: 1,
        producer,
        runtime: runtime_identity(),
        runtime_source_content_fingerprint: runtime_source_content_fingerprint()?,
        root_exact_state_hash,
        case_path,
        entries,
    };
    validate_combat_evidence_manifest(&manifest)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create combat evidence manifest directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("cannot serialize combat evidence manifest: {error}"))?;
    fs::write(path, bytes).map_err(|error| {
        format!(
            "cannot write combat evidence manifest '{}': {error}",
            path.display()
        )
    })
}

pub(super) fn decode_combat_evidence_manifest(
    path: &Path,
    bytes: &[u8],
) -> Result<CombatEvidenceManifestV1, String> {
    let manifest: CombatEvidenceManifestV1 = serde_json::from_slice(bytes).map_err(|error| {
        format!(
            "cannot decode combat evidence manifest '{}': {error}",
            path.display()
        )
    })?;
    validate_combat_evidence_manifest(&manifest)?;
    Ok(manifest)
}

pub(super) fn combat_action_sequence_hash(actions: &[ClientInput]) -> Result<String, String> {
    let bytes = serde_json::to_vec(actions).map_err(|error| error.to_string())?;
    let mut digest = Blake2b512::new();
    digest.update(bytes);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn validate_combat_evidence_manifest(manifest: &CombatEvidenceManifestV1) -> Result<(), String> {
    if manifest.schema_name != COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME || manifest.schema_version != 1
    {
        return Err(format!(
            "expected {COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME} schema_version 1, got {} schema_version {}",
            manifest.schema_name, manifest.schema_version
        ));
    }
    if manifest.root_exact_state_hash.trim().is_empty() {
        return Err("combat evidence manifest exact root hash must not be empty".to_string());
    }
    if !valid_hex_digest(&manifest.runtime_source_content_fingerprint, 128) {
        return Err("combat evidence manifest runtime fingerprint is invalid".to_string());
    }
    if manifest.case_path.as_os_str().is_empty() {
        return Err("combat evidence manifest case path must not be empty".to_string());
    }
    if manifest.entries.is_empty() {
        return Err("combat evidence manifest must contain at least one entry".to_string());
    }
    for entry in &manifest.entries {
        if entry.evidence_id.trim().is_empty() {
            return Err("combat evidence manifest evidence_id must not be empty".to_string());
        }
        if entry.action_paths.is_empty() {
            return Err(format!(
                "combat evidence manifest entry '{}' has no action paths",
                entry.evidence_id
            ));
        }
        if entry
            .action_paths
            .iter()
            .any(|path| path.as_os_str().is_empty())
        {
            return Err(format!(
                "combat evidence manifest entry '{}' has an empty action path",
                entry.evidence_id
            ));
        }
        if entry
            .trace_paths
            .iter()
            .any(|path| path.as_os_str().is_empty())
        {
            return Err(format!(
                "combat evidence manifest entry '{}' has an empty trace path",
                entry.evidence_id
            ));
        }
        if !valid_hex_digest(&entry.action_sequence_blake2b_512, 128) {
            return Err(format!(
                "combat evidence manifest entry '{}' has an invalid action hash",
                entry.evidence_id
            ));
        }
    }
    Ok(())
}

fn valid_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        combat_action_sequence_hash, decode_combat_evidence_manifest,
        COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME,
    };

    #[test]
    fn manifest_decode_rejects_invalid_action_identity() {
        let runtime_fingerprint = "0".repeat(128);
        let value = serde_json::json!({
            "schema_name": COMBAT_EVIDENCE_MANIFEST_SCHEMA_NAME,
            "schema_version": 1,
            "producer": "potion_expenditure_audit",
            "runtime": {},
            "runtime_source_content_fingerprint": runtime_fingerprint,
            "root_exact_state_hash": "root",
            "case_path": "case.json",
            "entries": [{
                "evidence_id": "lane",
                "action_paths": ["lane.actions.json"],
                "action_sequence_blake2b_512": "not-a-hash",
                "supplied_action_count": 0,
                "expected_terminal": "win",
                "expected_final_player_hp": 1
            }]
        });
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = decode_combat_evidence_manifest(Path::new("manifest.json"), &bytes)
            .expect_err("invalid hash must be rejected");
        assert!(error.contains("invalid action hash"));
    }

    #[test]
    fn action_hash_is_stable_for_typed_empty_sequence() {
        let left = combat_action_sequence_hash(&[]).unwrap();
        let right = combat_action_sequence_hash(&[]).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.len(), 128);
    }
}
