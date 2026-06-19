use serde::{Deserialize, Serialize};

use crate::eval::branch_campaign::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignRunDomainV1,
};
use crate::eval::branch_outcome_dataset_v1::{
    BranchOutcomeClassV1, BranchOutcomeRecordV1, BranchOutcomeStateFeaturesV1,
    BranchOutcomeSupervisionStatusV1,
};

pub const LEARNING_BRANCH_SAMPLE_SCHEMA_NAME: &str = "LearningBranchSampleV1";
pub const LEARNING_BRANCH_SAMPLE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDatasetExportContextV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exporter_git_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exporter_git_dirty: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_checkpoint_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningCandidateSetStatusV1 {
    ChosenOnly,
    NoDecisionRecorded,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningBranchSampleV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,

    pub provenance: LearningDatasetProvenanceV1,
    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
    pub report_rounds_completed: usize,

    pub branch_group: String,
    pub branch_index: usize,
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_branch_id: Option<String>,

    pub candidate_set_status: LearningCandidateSetStatusV1,
    pub decision_events: Vec<LearningDecisionEventV1>,
    pub strategic_summary: crate::ai::strategic::BranchSignatureCompact,
    pub outcome: LearningBranchOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDatasetProvenanceV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exporter_git_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exporter_git_dirty: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_checkpoint_path: Option<String>,
    pub source_record_schema_name: String,
    pub source_record_schema_version: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionEventV1 {
    pub step_index: usize,
    pub command: String,
    pub choice_label: String,
    pub candidate_set_status: LearningCandidateSetStatusV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningBranchOutcomeV1 {
    pub branch_status: BranchCampaignBranchStatusV1,
    pub outcome_class: BranchOutcomeClassV1,
    pub supervision_status: BranchOutcomeSupervisionStatusV1,
    pub report_stop_reason: String,
    pub stop_reason: String,
    pub frontier_title: String,
    pub rank_key: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_summary: Option<BranchCampaignBranchSummaryV1>,
    pub checkpoint_enriched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_features: Option<BranchOutcomeStateFeaturesV1>,
}

pub fn learning_records_from_branch_outcomes_v1(
    records: &[BranchOutcomeRecordV1],
    context: LearningDatasetExportContextV1,
) -> Vec<LearningBranchSampleV1> {
    records
        .iter()
        .map(|record| learning_record_from_branch_outcome_v1(record, &context))
        .collect()
}

pub fn serialize_learning_branch_samples_jsonl_v1(
    samples: &[LearningBranchSampleV1],
) -> Result<String, String> {
    let mut text = String::new();
    for sample in samples {
        let line = serde_json::to_string(sample)
            .map_err(|err| format!("failed to serialize LearningBranchSampleV1: {err}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    Ok(text)
}

pub fn parse_learning_branch_samples_jsonl_v1(
    text: &str,
) -> Result<Vec<LearningBranchSampleV1>, String> {
    let mut samples = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let sample = serde_json::from_str(line).map_err(|err| {
            format!(
                "failed to parse LearningBranchSampleV1 JSONL line {}: {err}",
                index + 1
            )
        })?;
        samples.push(sample);
    }
    Ok(samples)
}

fn learning_record_from_branch_outcome_v1(
    record: &BranchOutcomeRecordV1,
    context: &LearningDatasetExportContextV1,
) -> LearningBranchSampleV1 {
    let decision_events = learning_decision_events_v1(record);
    let candidate_set_status = if decision_events.is_empty() {
        LearningCandidateSetStatusV1::NoDecisionRecorded
    } else {
        LearningCandidateSetStatusV1::ChosenOnly
    };

    LearningBranchSampleV1 {
        schema_name: LEARNING_BRANCH_SAMPLE_SCHEMA_NAME.to_string(),
        schema_version: LEARNING_BRANCH_SAMPLE_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        provenance: LearningDatasetProvenanceV1 {
            exporter_git_commit: context.exporter_git_commit.clone(),
            exporter_git_dirty: context.exporter_git_dirty,
            source_report_path: context.source_report_path.clone(),
            source_checkpoint_path: context.source_checkpoint_path.clone(),
            source_record_schema_name: record.schema_name.clone(),
            source_record_schema_version: record.schema_version,
        },
        seed: record.seed,
        run_domain: record.run_domain.clone(),
        report_rounds_completed: record.report_rounds_completed,
        branch_group: record.branch_group.clone(),
        branch_index: record.branch_index,
        branch_id: record.branch_id.clone(),
        parent_branch_id: parent_branch_id_from_commands_v1(&record.commands),
        candidate_set_status,
        decision_events,
        strategic_summary: record.strategic_summary,
        outcome: LearningBranchOutcomeV1 {
            branch_status: record.branch_status.clone(),
            outcome_class: record.outcome_class.clone(),
            supervision_status: record.supervision_status.clone(),
            report_stop_reason: record.report_stop_reason.clone(),
            stop_reason: record.stop_reason.clone(),
            frontier_title: record.frontier_title.clone(),
            rank_key: record.rank_key,
            report_summary: record.report_summary.clone(),
            checkpoint_enriched: record.checkpoint_enriched,
            state_features: record.state_features.clone(),
        },
    }
}

fn learning_decision_events_v1(record: &BranchOutcomeRecordV1) -> Vec<LearningDecisionEventV1> {
    record
        .commands
        .iter()
        .enumerate()
        .map(|(step_index, command)| LearningDecisionEventV1 {
            step_index,
            command: command.clone(),
            choice_label: record
                .choice_labels
                .get(step_index)
                .cloned()
                .unwrap_or_default(),
            candidate_set_status: LearningCandidateSetStatusV1::ChosenOnly,
        })
        .collect()
}

fn parent_branch_id_from_commands_v1(commands: &[String]) -> Option<String> {
    if commands.is_empty() {
        return None;
    }
    let prefix = commands[..commands.len().saturating_sub(1)].join(".");
    if prefix.is_empty() {
        Some("root".to_string())
    } else {
        Some(format!("root.{prefix}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategic::BranchSignatureCompact;
    use crate::eval::branch_campaign::{
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignRunDomainV1,
    };
    use crate::eval::branch_outcome_dataset_v1::{
        BranchOutcomeClassV1, BranchOutcomeRecordV1, BranchOutcomeSupervisionStatusV1,
        BRANCH_OUTCOME_RECORD_SCHEMA_NAME, BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
    };

    #[test]
    fn learning_dataset_preserves_branch_decision_path_without_teacher_label_claim() {
        let records = vec![sample_branch_outcome_record()];
        let context = LearningDatasetExportContextV1 {
            exporter_git_commit: Some("abc123".to_string()),
            exporter_git_dirty: Some(true),
            source_report_path: Some("latest.campaign.json".to_string()),
            source_checkpoint_path: Some("latest.checkpoint.json".to_string()),
        };

        let samples = learning_records_from_branch_outcomes_v1(&records, context);

        assert_eq!(samples.len(), 1);
        let sample = &samples[0];
        assert_eq!(sample.schema_name, LEARNING_BRANCH_SAMPLE_SCHEMA_NAME);
        assert_eq!(sample.schema_version, LEARNING_BRANCH_SAMPLE_SCHEMA_VERSION);
        assert_eq!(sample.label_role, "campaign_observation_not_teacher");
        assert!(!sample.trainable_as_action_label);
        assert!(!sample.policy_quality_claim);
        assert_eq!(
            sample.provenance.exporter_git_commit.as_deref(),
            Some("abc123")
        );
        assert_eq!(
            sample.candidate_set_status,
            LearningCandidateSetStatusV1::ChosenOnly
        );
        assert_eq!(
            sample
                .decision_events
                .iter()
                .map(|event| (
                    event.step_index,
                    event.command.as_str(),
                    event.choice_label.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![(0, "rp 0", "Clothesline"), (1, "smith 1", "Smith Bash")]
        );
        assert_eq!(
            sample.outcome.outcome_class,
            BranchOutcomeClassV1::TerminalVictory
        );
    }

    #[test]
    fn learning_dataset_jsonl_round_trips() {
        let records = vec![sample_branch_outcome_record()];
        let samples = learning_records_from_branch_outcomes_v1(
            &records,
            LearningDatasetExportContextV1::default(),
        );

        let text = serialize_learning_branch_samples_jsonl_v1(&samples).expect("serialize");
        let parsed = parse_learning_branch_samples_jsonl_v1(&text).expect("parse");

        assert_eq!(parsed, samples);
    }

    fn sample_branch_outcome_record() -> BranchOutcomeRecordV1 {
        BranchOutcomeRecordV1 {
            schema_name: BRANCH_OUTCOME_RECORD_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
            label_role: "campaign_observation_not_teacher".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            seed: 521,
            run_domain: BranchCampaignRunDomainV1::default(),
            report_rounds_completed: 3,
            report_stop_reason: "victory_found".to_string(),
            branch_group: "victories".to_string(),
            branch_index: 0,
            branch_id: "root.rp 0.smith 1".to_string(),
            branch_status: BranchCampaignBranchStatusV1::TerminalVictory,
            outcome_class: BranchOutcomeClassV1::TerminalVictory,
            supervision_status: BranchOutcomeSupervisionStatusV1::TerminalOutcome,
            rank_key: 42,
            strategic_summary: BranchSignatureCompact::default(),
            stop_reason: "victory".to_string(),
            frontier_title: "Game Over Victory".to_string(),
            commands: vec!["rp 0".to_string(), "smith 1".to_string()],
            choice_labels: vec!["Clothesline".to_string(), "Smith Bash".to_string()],
            report_summary: Some(BranchCampaignBranchSummaryV1 {
                act: 3,
                floor: 48,
                hp: 55,
                max_hp: 90,
                gold: 102,
                deck_count: 20,
                deck_key: "Clothesline+0x1".to_string(),
                formation_stage: "PlanCommitted".to_string(),
                formation_strengths: vec!["StrengthScaling".to_string()],
                formation_needs: vec!["Consistency".to_string()],
                trajectory_key: "test".to_string(),
                boss: "TimeEater".to_string(),
                boss_pressure: vec!["pressure:time_warp_counter_control".to_string()],
                run_debt: Vec::new(),
                event_boundary: None,
            }),
            checkpoint_enriched: false,
            state_features: None,
        }
    }
}
