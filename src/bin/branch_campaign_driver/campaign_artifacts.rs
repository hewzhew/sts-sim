use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde_json::{Map, Value};
use sts_simulator::eval::branch_campaign::{
    BranchCampaignCheckpointV1, BranchCampaignReportV1, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};
use sts_simulator::eval::campaign_journal::CampaignJournalV1;

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
    let checkpoint: BranchCampaignCheckpointV1 = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume-checkpoint {} as {BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME}: {err}",
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
    Ok(checkpoint)
}

pub(super) fn write_campaign_report_v1(
    path: &PathBuf,
    report: &BranchCampaignReportV1,
) -> Result<(), String> {
    let mut value = serde_json::to_value(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
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

fn read_campaign_journal_v1(path: &PathBuf) -> Result<CampaignJournalV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--journal")?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse campaign journal sidecar {}: {err}",
            path.display()
        )
    })
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
    let text = serde_json::to_string(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--checkpoint-out")
}

fn read_campaign_artifact_text_v1(path: &PathBuf, role: &str) -> Result<String, String> {
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

fn campaign_journal_sidecar_path_v1(report_path: &PathBuf) -> PathBuf {
    let Some(file_name) = report_path.file_name().and_then(|name| name.to_str()) else {
        return report_path.with_extension("journal.json.gz");
    };
    let journal_name = if let Some(prefix) = file_name.strip_suffix(".campaign.json.gz") {
        format!("{prefix}.journal.json.gz")
    } else if let Some(prefix) = file_name.strip_suffix(".campaign.json") {
        format!("{prefix}.journal.json")
    } else if let Some(prefix) = file_name.strip_suffix(".json.gz") {
        format!("{prefix}.journal.json.gz")
    } else if let Some(prefix) = file_name.strip_suffix(".json") {
        format!("{prefix}.journal.json")
    } else if let Some(prefix) = file_name.strip_suffix(".gz") {
        format!("{prefix}.journal.json.gz")
    } else {
        format!("{file_name}.journal.json.gz")
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignRunDomainV1, BranchCampaignRunPreludeV1, BRANCH_CAMPAIGN_SCHEMA_NAME,
        BRANCH_CAMPAIGN_SCHEMA_VERSION,
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
            run_state_map_graphs: Vec::new(),
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
            run_state_schedules: Vec::new(),
            run_state_emitted_events: Vec::new(),
            combat_automation_trajectories: Vec::new(),
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
            run_state_map_graphs: Vec::new(),
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
            run_state_schedules: Vec::new(),
            run_state_emitted_events: Vec::new(),
            combat_automation_trajectories: Vec::new(),
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
}
