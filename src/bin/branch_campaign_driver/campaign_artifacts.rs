use std::fs;
use std::path::PathBuf;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignCheckpointV1, BranchCampaignReportV1, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};

pub(super) fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read --resume {}: {err}", path.display()))?;
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
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --resume-checkpoint {}: {err}",
            path.display()
        )
    })?;
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
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    fs::write(path, text).map_err(|err| format!("failed to write --out {}: {err}", path.display()))
}

pub(super) fn write_campaign_checkpoint_v1(
    path: &PathBuf,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --checkpoint-out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    fs::write(path, text)
        .map_err(|err| format!("failed to write --checkpoint-out {}: {err}", path.display()))
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
            run_state_maps: Vec::new(),
            run_state_master_decks: Vec::new(),
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
}
