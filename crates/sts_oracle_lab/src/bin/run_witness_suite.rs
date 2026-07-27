use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_oracle_runtime::eval::run_control::{
    exact_audit_run_progress_journal_policy_v1, exact_replay_run_progress_journal_v1,
    ExactRunProgressReplayReportV1,
};
use sts_oracle_runtime::runtime::branch::{
    current_oracle_candidate_order_v1, load_oracle_run_continuation_v1,
};

const MANIFEST_SCHEMA_NAME: &str = "OracleRunWitnessSuiteManifestV1";
const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Args)]
pub struct RunWitnessSuiteArgs {
    /// Compact manifest whose paths are resolved relative to the manifest.
    #[arg(long)]
    pub manifest: PathBuf,
    /// Also compare every committed strategic choice with today's owner.
    /// Divergences are diagnostics and do not invalidate an exact witness.
    #[arg(long)]
    pub audit_policy: bool,
    /// Include every policy divergence instead of only the audit summary.
    #[arg(long, requires = "audit_policy")]
    pub details: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunWitnessSuiteManifestV1 {
    schema_name: String,
    schema_version: u32,
    witnesses: Vec<RunWitnessSuiteEntryV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunWitnessSuiteEntryV1 {
    id: String,
    continuation: PathBuf,
    expected: ExactRunProgressReplayReportV1,
}

#[derive(Debug, Serialize)]
struct RunWitnessSuiteReportV1 {
    schema_name: &'static str,
    schema_version: u32,
    manifest: PathBuf,
    exact_witnesses_verified: usize,
    witnesses: Vec<RunWitnessSuiteWitnessReportV1>,
}

#[derive(Debug, Serialize)]
struct RunWitnessSuiteWitnessReportV1 {
    id: String,
    continuation: PathBuf,
    replay: ExactRunProgressReplayReportV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_audit: Option<Value>,
}

pub fn verify_run_witness_suite(args: RunWitnessSuiteArgs) -> Result<Value, String> {
    let bytes = fs::read(&args.manifest)
        .map_err(|error| format!("failed to read {}: {error}", args.manifest.display()))?;
    let manifest = serde_json::from_slice::<RunWitnessSuiteManifestV1>(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", args.manifest.display()))?;
    if manifest.schema_name != MANIFEST_SCHEMA_NAME
        || manifest.schema_version != MANIFEST_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported run-witness suite schema '{}'/{}",
            manifest.schema_name, manifest.schema_version
        ));
    }
    if manifest.witnesses.is_empty() {
        return Err("run-witness suite manifest contains no witnesses".to_string());
    }

    let root = args.manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut reports = Vec::with_capacity(manifest.witnesses.len());
    for entry in manifest.witnesses {
        let continuation_path = if entry.continuation.is_absolute() {
            entry.continuation.clone()
        } else {
            root.join(&entry.continuation)
        };
        let continuation = load_oracle_run_continuation_v1(&continuation_path)?;
        let expected_final = continuation.session.into_session()?;
        let replay = exact_replay_run_progress_journal_v1(
            continuation.seed,
            continuation.ascension,
            &continuation.journal,
            &expected_final,
        )?;
        if replay != entry.expected {
            return Err(format!(
                "run-witness '{}' replayed exactly but no longer identifies the manifest witness:\nexpected {}\nactual   {}",
                entry.id,
                serde_json::to_string(&entry.expected)
                    .map_err(|error| format!("failed to encode expected replay: {error}"))?,
                serde_json::to_string(&replay)
                    .map_err(|error| format!("failed to encode actual replay: {error}"))?,
            ));
        }

        let policy_audit = if args.audit_policy {
            let audit = exact_audit_run_progress_journal_policy_v1(
                continuation.seed,
                continuation.ascension,
                &continuation.journal,
                &expected_final,
                current_oracle_candidate_order_v1,
            )?;
            Some(if args.details {
                serde_json::to_value(audit)
                    .map_err(|error| format!("failed to encode policy audit: {error}"))?
            } else {
                json!({
                    "decisions_with_owner_preferences": audit.decisions_with_owner_preferences,
                    "decisions_without_owner_preferences": audit.decisions_without_owner_preferences,
                    "rank_zero_agreements": audit.rank_zero_agreements,
                    "nonzero_rank_choices": audit.nonzero_rank_choices,
                    "choices_absent_from_owner_preferences": audit.choices_absent_from_owner_preferences,
                    "discrepancy_sum": audit.discrepancy_sum,
                    "max_owner_rank": audit.max_owner_rank,
                    "first_divergence": audit.first_divergence,
                    "combat_sources": audit.combat_sources,
                })
            })
        } else {
            None
        };

        reports.push(RunWitnessSuiteWitnessReportV1 {
            id: entry.id,
            continuation: continuation_path,
            replay,
            policy_audit,
        });
    }

    serde_json::to_value(RunWitnessSuiteReportV1 {
        schema_name: "OracleRunWitnessSuiteReportV1",
        schema_version: 1,
        manifest: args.manifest,
        exact_witnesses_verified: reports.len(),
        witnesses: reports,
    })
    .map_err(|error| format!("failed to encode run-witness suite report: {error}"))
}
