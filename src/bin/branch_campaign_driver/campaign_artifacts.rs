use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use sts_simulator::eval::branch_campaign::{
    BranchCampaignCheckpointV1, BranchCampaignReportV1, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};

pub(super) fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = read_campaign_artifact_text_v1(path, "--resume")?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })
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
    let text = serde_json::to_string(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    write_campaign_artifact_text_v1(path, &text, "--out")
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignRunDomainV1, BranchCampaignRunPreludeV1, BRANCH_CAMPAIGN_SCHEMA_NAME,
        BRANCH_CAMPAIGN_SCHEMA_VERSION,
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
