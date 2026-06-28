use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sts_simulator::eval::branch_campaign::{
    BranchCampaignBranchV1, BranchCampaignCheckpointActiveCombatRecordV1,
    BranchCampaignCheckpointV1, BranchCampaignCombatRetryLedgerV1, BranchCampaignDiscardedBranchV1,
    BranchCampaignReportV1, BranchCampaignRoundSummaryV1, BranchCampaignRouteEvidenceSummaryV1,
    BranchCampaignStrategyRequestV1, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};
use sts_simulator::eval::campaign_journal::CampaignJournalV1;

const CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_NAME: &str = "BranchCampaignStateSidecarV1";
const CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_VERSION: u32 = 1;
const CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_NAME: &str =
    "BranchCampaignCheckpointActiveCombatSidecarV1";
const CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignReportStateSidecarV1 {
    schema_name: String,
    schema_version: u32,
    active: Vec<BranchCampaignBranchV1>,
    frozen: Vec<BranchCampaignBranchV1>,
    victories: Vec<BranchCampaignBranchV1>,
    dead: Vec<BranchCampaignBranchV1>,
    abandoned: Vec<BranchCampaignBranchV1>,
    stuck: Vec<BranchCampaignBranchV1>,
    discarded_count: usize,
    discarded_examples: Vec<String>,
    discarded_branches: Vec<BranchCampaignDiscardedBranchV1>,
    strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    combat_retry_ledger: BranchCampaignCombatRetryLedgerV1,
    rounds: Vec<BranchCampaignRoundSummaryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignCheckpointActiveCombatSidecarV1 {
    schema_name: String,
    schema_version: u32,
    active_combats: Vec<BranchCampaignCheckpointActiveCombatRecordV1>,
}

pub(super) fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--resume")?;
    let mut value: Value = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })?;
    let journal_artifact = value
        .as_object_mut()
        .and_then(|object| remove_string_field_v1(object, "journal_artifact"));
    if let Some(object) = value.as_object_mut() {
        object.remove("journal_event_count");
    }
    let state_artifact = value
        .as_object_mut()
        .and_then(|object| remove_string_field_v1(object, "state_artifact"));
    if let Some(object) = value.as_object_mut() {
        remove_campaign_state_projection_fields_v1(object);
    }
    if let Some(artifact) = state_artifact {
        let state_path = resolve_campaign_artifact_ref_path_v1(path, &artifact);
        let state = read_campaign_report_state_v1(&state_path)?;
        hydrate_campaign_report_state_value_v1(&mut value, state)?;
    }
    let mut report: BranchCampaignReportV1 = serde_json::from_value(value).map_err(|err| {
        format!(
            "failed to decode --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })?;
    if report.journal.is_empty() {
        if let Some(artifact) = journal_artifact {
            let journal_path = resolve_campaign_artifact_ref_path_v1(path, &artifact);
            report.journal = read_campaign_journal_v1(&journal_path)?;
        }
    }
    Ok(report)
}

pub(super) fn read_campaign_checkpoint_v1(
    path: &PathBuf,
) -> Result<BranchCampaignCheckpointV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--resume-checkpoint")?;
    let mut value: Value = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume-checkpoint {} as {BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME}: {err}",
            path.display()
        )
    })?;
    let active_combat_artifact = value
        .as_object_mut()
        .and_then(|object| remove_string_field_v1(object, "active_combat_artifact"));
    if let Some(object) = value.as_object_mut() {
        object.remove("active_combat_count");
    }
    let mut checkpoint: BranchCampaignCheckpointV1 =
        serde_json::from_value(value).map_err(|err| {
            format!(
                "failed to decode --resume-checkpoint {} as {BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME}: {err}",
                path.display()
            )
        })?;
    if checkpoint.schema_name != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME
        || checkpoint.schema_version != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
    {
        return Err(format!(
            "checkpoint {} uses {} v{}; expected {} v{}. Rerun campaign to create a fresh checkpoint.",
            path.display(),
            checkpoint.schema_name,
            checkpoint.schema_version,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
        ));
    }
    if checkpoint.active_combats.is_empty() {
        if let Some(artifact) = active_combat_artifact {
            let active_combat_path = resolve_campaign_artifact_ref_path_v1(path, &artifact);
            let sidecar = read_campaign_checkpoint_active_combats_v1(&active_combat_path)?;
            checkpoint.active_combats = sidecar.active_combats;
        }
    }
    Ok(checkpoint)
}

pub(super) fn write_campaign_report_v1(
    path: &PathBuf,
    report: &BranchCampaignReportV1,
) -> Result<(), String> {
    let mut value = serde_json::to_value(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    if report_has_campaign_state_payload_v1(report) {
        let state_path = campaign_state_sidecar_path_v1(path);
        let state = campaign_report_state_sidecar_from_report_v1(report);
        write_campaign_report_state_v1(&state_path, &state)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| "BranchCampaignV1 report did not serialize as an object".to_string())?;
        remove_campaign_state_payload_fields_v1(object);
        object.insert(
            "state_artifact".to_string(),
            Value::String(campaign_artifact_relative_ref_v1(path, &state_path)),
        );
        object.insert(
            "state_active_count".to_string(),
            Value::Number(report.active.len().into()),
        );
        object.insert(
            "state_frozen_count".to_string(),
            Value::Number(report.frozen.len().into()),
        );
        object.insert(
            "state_round_count".to_string(),
            Value::Number(report.rounds.len().into()),
        );
    }
    if !report.journal.is_empty() {
        let journal_path = campaign_journal_sidecar_path_v1(path);
        write_campaign_journal_v1(&journal_path, &report.journal)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| "BranchCampaignV1 report did not serialize as an object".to_string())?;
        object.remove("journal");
        object.insert(
            "journal_artifact".to_string(),
            Value::String(campaign_artifact_relative_ref_v1(path, &journal_path)),
        );
        object.insert(
            "journal_event_count".to_string(),
            Value::Number(report.journal.events.len().into()),
        );
    }
    let text = serde_json::to_string(&value)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--out")
}

fn read_campaign_report_state_v1(path: &PathBuf) -> Result<CampaignReportStateSidecarV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--state")?;
    let state: CampaignReportStateSidecarV1 = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse campaign state sidecar {}: {err}",
            path.display()
        )
    })?;
    if state.schema_name != CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_NAME
        || state.schema_version != CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_VERSION
    {
        return Err(format!(
            "campaign state sidecar {} uses {} v{}; expected {} v{}",
            path.display(),
            state.schema_name,
            state.schema_version,
            CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_NAME,
            CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_VERSION
        ));
    }
    Ok(state)
}

fn write_campaign_report_state_v1(
    path: &PathBuf,
    state: &CampaignReportStateSidecarV1,
) -> Result<(), String> {
    let text = serde_json::to_string(state)
        .map_err(|err| format!("failed to serialize campaign state sidecar: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--state-out")
}

fn read_campaign_journal_v1(path: &PathBuf) -> Result<CampaignJournalV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--journal")?;
    let mut journal: CampaignJournalV1 = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse campaign journal sidecar {}: {err}",
            path.display()
        )
    })?;
    journal.hydrate_event_ids_v1();
    journal.hydrate_branch_paths_v1();
    journal.hydrate_route_candidate_pools_v1();
    Ok(journal)
}

fn write_campaign_journal_v1(path: &PathBuf, journal: &CampaignJournalV1) -> Result<(), String> {
    let text = serde_json::to_string(journal)
        .map_err(|err| format!("failed to serialize campaign journal sidecar: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--journal-out")
}

pub(super) fn write_campaign_checkpoint_v1(
    path: &PathBuf,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    let mut value = serde_json::to_value(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    if !checkpoint.active_combats.is_empty() {
        let active_combat_path = campaign_checkpoint_active_combat_sidecar_path_v1(path);
        let sidecar = CampaignCheckpointActiveCombatSidecarV1 {
            schema_name: CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_VERSION,
            active_combats: checkpoint.active_combats.clone(),
        };
        write_campaign_checkpoint_active_combats_v1(&active_combat_path, &sidecar)?;
        let object = value.as_object_mut().ok_or_else(|| {
            "BranchCampaignCheckpointV2 did not serialize as an object".to_string()
        })?;
        object.remove("active_combats");
        object.insert(
            "active_combat_artifact".to_string(),
            Value::String(campaign_artifact_relative_ref_v1(path, &active_combat_path)),
        );
        object.insert(
            "active_combat_count".to_string(),
            Value::Number(checkpoint.active_combats.len().into()),
        );
    }
    let text = serde_json::to_string(&value)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--checkpoint-out")
}

fn read_campaign_checkpoint_active_combats_v1(
    path: &PathBuf,
) -> Result<CampaignCheckpointActiveCombatSidecarV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--active-combat")?;
    let sidecar: CampaignCheckpointActiveCombatSidecarV1 =
        serde_json::from_str(&text).map_err(|err| {
            format!(
                "failed to parse campaign active combat sidecar {}: {err}",
                path.display()
            )
        })?;
    if sidecar.schema_name != CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_NAME
        || sidecar.schema_version != CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_VERSION
    {
        return Err(format!(
            "active combat sidecar {} uses {} v{}; expected {} v{}",
            path.display(),
            sidecar.schema_name,
            sidecar.schema_version,
            CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_NAME,
            CAMPAIGN_CHECKPOINT_ACTIVE_COMBAT_SIDECAR_SCHEMA_VERSION
        ));
    }
    Ok(sidecar)
}

fn write_campaign_checkpoint_active_combats_v1(
    path: &PathBuf,
    sidecar: &CampaignCheckpointActiveCombatSidecarV1,
) -> Result<(), String> {
    let text = serde_json::to_string(sidecar)
        .map_err(|err| format!("failed to serialize campaign active combat sidecar: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--active-combat-out")
}

pub(super) fn read_campaign_artifact_text_v1(path: &PathBuf, role: &str) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {role} {}: {err}", path.display()))?;
    if is_gzip_campaign_artifact_path_v1(path) || has_gzip_magic_v1(&bytes) {
        let mut decoder = GzDecoder::new(bytes.as_slice());
        let mut text = String::new();
        decoder
            .read_to_string(&mut text)
            .map_err(|err| format!("failed to decompress {role} {}: {err}", path.display()))?;
        return Ok(text);
    }
    String::from_utf8(bytes)
        .map_err(|err| format!("failed to decode {role} {} as UTF-8: {err}", path.display()))
}

fn write_campaign_artifact_text_v1(path: &PathBuf, text: &str, role: &str) -> Result<(), String> {
    create_campaign_artifact_parent_dir_v1(path, role)?;
    if is_gzip_campaign_artifact_path_v1(path) {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(text.as_bytes()).map_err(|err| {
            format!(
                "failed to compress {role} payload for {}: {err}",
                path.display()
            )
        })?;
        let bytes = encoder.finish().map_err(|err| {
            format!(
                "failed to finish {role} compression for {}: {err}",
                path.display()
            )
        })?;
        return fs::write(path, bytes)
            .map_err(|err| format!("failed to write {role} {}: {err}", path.display()));
    }
    fs::write(path, text).map_err(|err| format!("failed to write {role} {}: {err}", path.display()))
}

fn create_campaign_artifact_parent_dir_v1(path: &PathBuf, role: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create {role} directory {}: {err}",
                parent.display()
            )
        })?;
    }
    Ok(())
}

fn is_gzip_campaign_artifact_path_v1(path: &PathBuf) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".gz"))
        .unwrap_or(false)
}

fn has_gzip_magic_v1(bytes: &[u8]) -> bool {
    bytes.get(0..2) == Some(&[0x1f, 0x8b][..])
}

fn campaign_state_sidecar_path_v1(report_path: &PathBuf) -> PathBuf {
    campaign_sidecar_path_v1(report_path, "state")
}

fn campaign_journal_sidecar_path_v1(report_path: &PathBuf) -> PathBuf {
    campaign_sidecar_path_v1(report_path, "journal")
}

fn campaign_checkpoint_active_combat_sidecar_path_v1(checkpoint_path: &PathBuf) -> PathBuf {
    campaign_sidecar_path_v1(checkpoint_path, "active-combats")
}

fn campaign_sidecar_path_v1(report_path: &PathBuf, sidecar: &str) -> PathBuf {
    let Some(file_name) = report_path.file_name().and_then(|name| name.to_str()) else {
        return report_path.with_extension(format!("{sidecar}.json.gz"));
    };
    let journal_name = if let Some(prefix) = file_name.strip_suffix(".campaign.json.gz") {
        format!("{prefix}.{sidecar}.json.gz")
    } else if let Some(prefix) = file_name.strip_suffix(".campaign.json") {
        format!("{prefix}.{sidecar}.json")
    } else if let Some(prefix) = file_name.strip_suffix(".json.gz") {
        format!("{prefix}.{sidecar}.json.gz")
    } else if let Some(prefix) = file_name.strip_suffix(".json") {
        format!("{prefix}.{sidecar}.json")
    } else if let Some(prefix) = file_name.strip_suffix(".gz") {
        format!("{prefix}.{sidecar}.json.gz")
    } else {
        format!("{file_name}.{sidecar}.json.gz")
    };
    report_path.with_file_name(journal_name)
}

fn campaign_artifact_relative_ref_v1(base_path: &Path, artifact_path: &Path) -> String {
    base_path
        .parent()
        .and_then(|parent| artifact_path.strip_prefix(parent).ok())
        .unwrap_or(artifact_path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resolve_campaign_artifact_ref_path_v1(base_path: &Path, artifact_ref: &str) -> PathBuf {
    let path = PathBuf::from(artifact_ref);
    if path.is_absolute() {
        return path;
    }
    base_path
        .parent()
        .map(|parent| parent.join(&path))
        .unwrap_or(path)
}

fn remove_string_field_v1(object: &mut Map<String, Value>, key: &str) -> Option<String> {
    object
        .remove(key)
        .and_then(|value| value.as_str().map(str::to_string))
}

fn campaign_report_state_sidecar_from_report_v1(
    report: &BranchCampaignReportV1,
) -> CampaignReportStateSidecarV1 {
    let mut rounds = report.rounds.clone();
    if !report.journal.is_empty() {
        for round in &mut rounds {
            round.decision_observations.clear();
            round.combat_performance = Default::default();
        }
    }
    let mut active = report.active.clone();
    let mut frozen = report.frozen.clone();
    let mut victories = report.victories.clone();
    let mut dead = report.dead.clone();
    let mut abandoned = report.abandoned.clone();
    let mut stuck = report.stuck.clone();
    compact_campaign_state_branches_v1(&mut active);
    compact_campaign_state_branches_v1(&mut frozen);
    compact_campaign_state_branches_v1(&mut victories);
    compact_campaign_state_branches_v1(&mut dead);
    compact_campaign_state_branches_v1(&mut abandoned);
    compact_campaign_state_branches_v1(&mut stuck);
    CampaignReportStateSidecarV1 {
        schema_name: CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_NAME.to_string(),
        schema_version: CAMPAIGN_REPORT_STATE_SIDECAR_SCHEMA_VERSION,
        active,
        frozen,
        victories,
        dead,
        abandoned,
        stuck,
        discarded_count: report.discarded_count,
        discarded_examples: report.discarded_examples.clone(),
        discarded_branches: Vec::new(),
        strategy_requests: report.strategy_requests.clone(),
        route_evidence: report.route_evidence.clone(),
        combat_retry_ledger: report.combat_retry_ledger.clone(),
        rounds,
    }
}

fn compact_campaign_state_branches_v1(branches: &mut [BranchCampaignBranchV1]) {
    for branch in branches {
        branch.choice_labels.clear();
        if let Some(summary) = branch.summary.as_mut() {
            compact_campaign_state_branch_summary_v1(summary);
        }
    }
}

fn compact_campaign_state_branch_summary_v1(
    summary: &mut sts_simulator::eval::branch_campaign::BranchCampaignBranchSummaryV1,
) {
    summary.deck_key.clear();
    summary.formation_stage.clear();
    summary.formation_strengths.clear();
    summary.formation_needs.clear();
    summary.trajectory_key.clear();
    summary.boss_pressure.clear();
    summary.run_debt.clear();
    summary.event_boundary = None;
    summary.reward_boundary = None;
}

fn report_has_campaign_state_payload_v1(report: &BranchCampaignReportV1) -> bool {
    !report.active.is_empty()
        || !report.frozen.is_empty()
        || !report.victories.is_empty()
        || !report.dead.is_empty()
        || !report.abandoned.is_empty()
        || !report.stuck.is_empty()
        || report.discarded_count != 0
        || !report.discarded_examples.is_empty()
        || !report.discarded_branches.is_empty()
        || !report.strategy_requests.is_empty()
        || report.route_evidence != BranchCampaignRouteEvidenceSummaryV1::default()
        || report.combat_retry_ledger != BranchCampaignCombatRetryLedgerV1::default()
        || !report.rounds.is_empty()
}

fn hydrate_campaign_report_state_value_v1(
    report_value: &mut Value,
    state: CampaignReportStateSidecarV1,
) -> Result<(), String> {
    let object = report_value
        .as_object_mut()
        .ok_or_else(|| "BranchCampaignV1 report did not parse as an object".to_string())?;
    let state_value = serde_json::to_value(state)
        .map_err(|err| format!("failed to convert campaign state sidecar to JSON: {err}"))?;
    let mut state_object = state_value
        .as_object()
        .ok_or_else(|| "campaign state sidecar did not serialize as an object".to_string())?
        .clone();
    state_object.remove("schema_name");
    state_object.remove("schema_version");
    for (key, value) in state_object {
        object.insert(key, value);
    }
    Ok(())
}

fn remove_campaign_state_payload_fields_v1(object: &mut Map<String, Value>) {
    for key in [
        "active",
        "frozen",
        "victories",
        "dead",
        "abandoned",
        "stuck",
        "discarded_count",
        "discarded_examples",
        "discarded_branches",
        "strategy_requests",
        "route_evidence",
        "combat_retry_ledger",
        "rounds",
    ] {
        object.remove(key);
    }
}

fn remove_campaign_state_projection_fields_v1(object: &mut Map<String, Value>) {
    for key in [
        "state_active_count",
        "state_frozen_count",
        "state_victory_count",
        "state_dead_count",
        "state_abandoned_count",
        "state_stuck_count",
        "state_discarded_count",
        "state_round_count",
    ] {
        object.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::strategic::BranchSignatureCompact;
    use sts_simulator::content::monsters::EnemyId;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
        BranchCampaignCheckpointActiveCombatRecordV1, BranchCampaignCheckpointSessionV1,
        BranchCampaignRoundSummaryV1, BranchCampaignRunDomainV1, BranchCampaignRunPreludeV1,
        BRANCH_CAMPAIGN_SCHEMA_NAME, BRANCH_CAMPAIGN_SCHEMA_VERSION,
    };
    use sts_simulator::eval::campaign_journal::{
        CampaignJournalEventPayloadV1, CampaignJournalEventV1, CampaignJournalV1,
    };

    #[test]
    fn writes_report_as_compact_json() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_report_compact_{}.json",
            std::process::id()
        ));
        let report = BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed: 7,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 0,
            stop_reason: "test".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        };

        write_campaign_report_v1(&path, &report).expect("report should write");
        let text = fs::read_to_string(&path).expect("report readable");
        let parsed = read_campaign_report_v1(&path).expect("compact report should parse");
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.seed, 7);
        assert!(
            !text.contains("\n  "),
            "campaign report artifacts should avoid pretty JSON indentation"
        );
    }

    #[test]
    fn writes_checkpoint_as_compact_json() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_checkpoint_compact_{}.json",
            std::process::id()
        ));
        let checkpoint = BranchCampaignCheckpointV1 {
            schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
            seed: 7,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 0,
            nodes: Vec::new(),
            decision_parent_anchor_commands: Vec::new(),
            decision_parent_anchor_node_ids: Vec::new(),
            run_state_map_graphs: Vec::new(),
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
            run_state_relics: Vec::new(),
            run_state_potions: Vec::new(),
            run_state_schedules: Vec::new(),
            run_state_schedule_components: Default::default(),
            run_state_emitted_events: Vec::new(),
            combat_automation_trajectories: Vec::new(),
            active_combats: Vec::new(),
            sessions: Vec::new(),
        };

        write_campaign_checkpoint_v1(&path, &checkpoint).expect("checkpoint should write");
        let text = fs::read_to_string(&path).expect("checkpoint readable");
        let parsed = read_campaign_checkpoint_v1(&path).expect("compact checkpoint should parse");
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.seed, 7);
        assert!(
            !text.contains("\n  "),
            "checkpoint artifacts should avoid pretty JSON indentation"
        );
    }

    #[test]
    fn writes_and_reads_gzip_report_artifact() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_report_compressed_{}.json.gz",
            std::process::id()
        ));
        let report = BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed: 11,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 0,
            stop_reason: "test".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        };

        write_campaign_report_v1(&path, &report).expect("compressed report should write");
        let bytes = fs::read(&path).expect("compressed report readable");
        let parsed = read_campaign_report_v1(&path).expect("compressed report should parse");
        let _ = fs::remove_file(&path);

        assert_eq!(bytes.get(0..2), Some(&[0x1f, 0x8b][..]));
        assert_eq!(parsed.seed, 11);
    }

    #[test]
    fn writes_report_journal_as_sidecar_and_hydrates_on_read() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_report_with_journal_{}.campaign.json.gz",
            std::process::id()
        ));
        let journal_path = campaign_journal_sidecar_path_v1(&path);
        let mut journal = CampaignJournalV1::new();
        journal.events.push(CampaignJournalEventV1 {
            event_id: "event-1".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Card Reward".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "decision-1".to_string(),
                boundary_title: "Card Reward".to_string(),
                frontier_key: "root".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 3,
                selected_count: 1,
                candidates: Vec::new(),
            },
        });
        let report = BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed: 17,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 1,
            stop_reason: "test".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal,
            rounds: Vec::new(),
        };

        write_campaign_report_v1(&path, &report).expect("report should write");
        let report_text = read_campaign_artifact_text_v1(&path, "--out").expect("report text");
        let journal_text =
            read_campaign_artifact_text_v1(&journal_path, "--journal-out").expect("journal text");
        let parsed = read_campaign_report_v1(&path).expect("report should hydrate sidecar journal");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&journal_path);

        assert!(report_text.contains("\"journal_artifact\""));
        assert!(report_text.contains("\"journal_event_count\":1"));
        assert!(!report_text.contains("\"events\""));
        assert!(journal_text.contains("\"events\""));
        assert_eq!(parsed.journal.events.len(), 1);
        assert_eq!(parsed.journal.events[0].event_id, "event-1");
    }

    #[test]
    fn writes_report_state_as_sidecar_and_hydrates_on_read() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_report_with_state_{}.campaign.json.gz",
            std::process::id()
        ));
        let state_path = campaign_state_sidecar_path_v1(&path);
        let mut report = empty_report_v1(23);
        report.rounds_completed = 1;
        report.active = vec![test_branch_v1(
            "active-1",
            BranchCampaignBranchStatusV1::Scheduled,
        )];
        report.rounds = vec![BranchCampaignRoundSummaryV1 {
            round: 1,
            started_scheduled: 1,
            produced_branches: 2,
            scheduled_after: 1,
            ..BranchCampaignRoundSummaryV1::default()
        }];

        write_campaign_report_v1(&path, &report).expect("report should write");
        let report_text = read_campaign_artifact_text_v1(&path, "--out").expect("report text");
        let state_text =
            read_campaign_artifact_text_v1(&state_path, "--state-out").expect("state text");
        let parsed = read_campaign_report_v1(&path).expect("report should hydrate sidecar state");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&state_path);

        assert!(report_text.contains("\"state_artifact\""));
        assert!(report_text.contains("\"state_active_count\":1"));
        assert!(!report_text.contains("\"active\""));
        assert!(!report_text.contains("\"rounds\""));
        assert!(state_text.contains("\"active\""));
        assert!(state_text.contains("\"rounds\""));
        assert_eq!(parsed.active.len(), 1);
        assert_eq!(parsed.active[0].branch_id, "active-1");
        assert_eq!(parsed.rounds.len(), 1);
    }

    #[test]
    fn writes_and_reads_gzip_checkpoint_artifact() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_checkpoint_compressed_{}.json.gz",
            std::process::id()
        ));
        let checkpoint = BranchCampaignCheckpointV1 {
            schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
            seed: 11,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 0,
            nodes: Vec::new(),
            decision_parent_anchor_commands: Vec::new(),
            decision_parent_anchor_node_ids: Vec::new(),
            run_state_map_graphs: Vec::new(),
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
            run_state_relics: Vec::new(),
            run_state_potions: Vec::new(),
            run_state_schedules: Vec::new(),
            run_state_schedule_components: Default::default(),
            run_state_emitted_events: Vec::new(),
            combat_automation_trajectories: Vec::new(),
            active_combats: Vec::new(),
            sessions: Vec::new(),
        };

        write_campaign_checkpoint_v1(&path, &checkpoint)
            .expect("compressed checkpoint should write");
        let bytes = fs::read(&path).expect("compressed checkpoint readable");
        let parsed =
            read_campaign_checkpoint_v1(&path).expect("compressed checkpoint should parse");
        let _ = fs::remove_file(&path);

        assert_eq!(bytes.get(0..2), Some(&[0x1f, 0x8b][..]));
        assert_eq!(parsed.seed, 11);
    }

    #[test]
    fn writes_checkpoint_active_combats_as_sidecar_and_hydrates_on_read() {
        let path = std::env::temp_dir().join(format!(
            "sts_campaign_checkpoint_active_combat_sidecar_{}.json.gz",
            std::process::id()
        ));
        let active_combat_path = campaign_checkpoint_active_combat_sidecar_path_v1(&path);
        let active_combat = sts_simulator::state::core::ActiveCombat::new(
            sts_simulator::state::core::EngineState::CombatPlayerTurn,
            sts_simulator::test_support::combat_with_monsters(vec![
                sts_simulator::test_support::test_monster(EnemyId::Cultist),
            ]),
            sts_simulator::state::core::CombatContext::Room(
                sts_simulator::state::core::RoomCombatContext {
                    room_type: sts_simulator::state::map::node::RoomType::MonsterRoom,
                },
            ),
        );
        let checkpoint = BranchCampaignCheckpointV1 {
            schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
            seed: 23,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 1,
            nodes: Vec::new(),
            decision_parent_anchor_commands: Vec::new(),
            decision_parent_anchor_node_ids: Vec::new(),
            run_state_map_graphs: Vec::new(),
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
            run_state_relics: Vec::new(),
            run_state_potions: Vec::new(),
            run_state_schedules: Vec::new(),
            run_state_schedule_components: Default::default(),
            run_state_emitted_events: Vec::new(),
            combat_automation_trajectories: Vec::new(),
            active_combats: vec![BranchCampaignCheckpointActiveCombatRecordV1 {
                active_combat_id: "active_combat:0".to_string(),
                active_combat: active_combat.clone(),
            }],
            sessions: vec![BranchCampaignCheckpointSessionV1 {
                node_id: None,
                commands: vec!["combat".to_string()],
                run_state_map_id: None,
                run_state_master_deck_id: None,
                run_state_relics_id: None,
                run_state_potions_id: None,
                run_state_schedule_id: None,
                run_state_emitted_events_id: None,
                active_combat_id: Some("active_combat:0".to_string()),
                session:
                    sts_simulator::eval::run_control::RunControlSessionCheckpointV1::from_session(
                        &sts_simulator::eval::run_control::RunControlSession::new(
                            Default::default(),
                        ),
                    ),
            }],
        };

        write_campaign_checkpoint_v1(&path, &checkpoint).expect("checkpoint should write");
        let checkpoint_text = read_campaign_artifact_text_v1(&path, "--checkpoint-out")
            .expect("checkpoint text should read");
        let active_combat_text =
            read_campaign_artifact_text_v1(&active_combat_path, "--active-combat-out")
                .expect("active combat sidecar should read");
        let parsed = read_campaign_checkpoint_v1(&path).expect("checkpoint should hydrate sidecar");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&active_combat_path);

        assert!(checkpoint_text.contains("\"active_combat_artifact\""));
        assert!(checkpoint_text.contains("\"active_combat_count\":1"));
        assert!(!checkpoint_text.contains("\"active_combats\""));
        assert!(active_combat_text.contains("\"active_combats\""));
        assert_eq!(parsed.active_combats.len(), 1);
        assert_eq!(parsed.active_combats[0].active_combat, active_combat);
    }

    fn empty_report_v1(seed: u64) -> BranchCampaignReportV1 {
        BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: BranchCampaignRunPreludeV1::default(),
            rounds_completed: 0,
            stop_reason: "test".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        }
    }

    fn test_branch_v1(id: &str, status: BranchCampaignBranchStatusV1) -> BranchCampaignBranchV1 {
        BranchCampaignBranchV1 {
            branch_id: id.to_string(),
            commands: vec!["rp 0".to_string()],
            choice_labels: vec!["Test card".to_string()],
            summary: Some(BranchCampaignBranchSummaryV1 {
                act: 1,
                floor: 1,
                hp: 80,
                max_hp: 80,
                gold: 99,
                deck_count: 10,
                deck_key: String::new(),
                formation_stage: "PlanSeeded".to_string(),
                formation_strengths: Vec::new(),
                formation_needs: Vec::new(),
                trajectory_key: String::new(),
                boss: "TheGuardian".to_string(),
                boss_pressure: Vec::new(),
                run_debt: Vec::new(),
                event_boundary: None,
                reward_boundary: None,
            }),
            strategic_summary: BranchSignatureCompact::default(),
            frontier_title: "Reward Screen".to_string(),
            status,
            stop_reason: "test".to_string(),
            continuation_origin: None,
            decision_candidate_axis: None,
            lineage_decision_signal_rank_adjustment: 0,
            rank_key: 0,
            rank_breakdown: None,
            assessment: None,
        }
    }
}
