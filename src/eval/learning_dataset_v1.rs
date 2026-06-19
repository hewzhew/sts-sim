use std::collections::BTreeMap;

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
pub const LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_NAME: &str = "LearningDecisionOutcomeSampleV1";
pub const LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_VERSION: u32 = 1;

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
    ObservedSiblings,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionOutcomeSampleV1 {
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

    pub decision_id: String,
    pub sibling_group_id: String,
    pub parent_branch_id: String,
    pub step_index: usize,
    pub candidate_command: String,
    pub candidate_choice_label: String,
    pub candidate_set_status: LearningCandidateSetStatusV1,
    pub observed_candidate_index: usize,
    pub observed_sibling_count: usize,
    pub sibling_candidates: Vec<LearningSiblingCandidateV1>,

    pub branch_group: String,
    pub branch_index: usize,
    pub branch_id: String,
    pub strategic_summary: crate::ai::strategic::BranchSignatureCompact,
    pub outcome: LearningBranchOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningSiblingCandidateV1 {
    pub command: String,
    pub choice_label: String,
    pub observed_branch_count: usize,
    pub representative_branch_group: String,
    pub representative_branch_index: usize,
    pub representative_branch_id: String,
    pub best_outcome_class: BranchOutcomeClassV1,
    pub best_supervision_status: BranchOutcomeSupervisionStatusV1,
    pub best_rank_key: i32,
    pub best_frontier_title: String,
    pub outcome_class_counts: Vec<LearningOutcomeClassCountV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningOutcomeClassCountV1 {
    pub outcome_class: BranchOutcomeClassV1,
    pub count: usize,
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

pub fn decision_outcome_samples_from_branch_outcomes_v1(
    records: &[BranchOutcomeRecordV1],
    context: LearningDatasetExportContextV1,
) -> Vec<LearningDecisionOutcomeSampleV1> {
    let mut drafts = Vec::new();
    let mut groups = BTreeMap::<String, Vec<usize>>::new();

    for (record_index, record) in records.iter().enumerate() {
        for step_index in 0..record.commands.len() {
            let parent_branch_id = branch_id_from_command_prefix_v1(&record.commands[..step_index]);
            let sibling_group_id = decision_sibling_group_id_v1(record, step_index);
            let draft = LearningDecisionCandidateDraftV1 {
                record_index,
                step_index,
                sibling_group_id: sibling_group_id.clone(),
                parent_branch_id,
                candidate_command: record.commands[step_index].clone(),
                candidate_choice_label: record
                    .choice_labels
                    .get(step_index)
                    .cloned()
                    .unwrap_or_default(),
            };
            groups
                .entry(sibling_group_id)
                .or_default()
                .push(drafts.len());
            drafts.push(draft);
        }
    }

    drafts
        .iter()
        .enumerate()
        .map(|(draft_index, draft)| {
            decision_outcome_sample_from_draft_v1(
                records,
                &context,
                &drafts,
                &groups,
                draft_index,
                draft,
            )
        })
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

pub fn serialize_learning_decision_outcome_samples_jsonl_v1(
    samples: &[LearningDecisionOutcomeSampleV1],
) -> Result<String, String> {
    let mut text = String::new();
    for sample in samples {
        let line = serde_json::to_string(sample)
            .map_err(|err| format!("failed to serialize LearningDecisionOutcomeSampleV1: {err}"))?;
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

pub fn parse_learning_decision_outcome_samples_jsonl_v1(
    text: &str,
) -> Result<Vec<LearningDecisionOutcomeSampleV1>, String> {
    let mut samples = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let sample = serde_json::from_str(line).map_err(|err| {
            format!(
                "failed to parse LearningDecisionOutcomeSampleV1 JSONL line {}: {err}",
                index + 1
            )
        })?;
        samples.push(sample);
    }
    Ok(samples)
}

#[derive(Clone, Debug)]
struct LearningDecisionCandidateDraftV1 {
    record_index: usize,
    step_index: usize,
    sibling_group_id: String,
    parent_branch_id: String,
    candidate_command: String,
    candidate_choice_label: String,
}

fn decision_outcome_sample_from_draft_v1(
    records: &[BranchOutcomeRecordV1],
    context: &LearningDatasetExportContextV1,
    drafts: &[LearningDecisionCandidateDraftV1],
    groups: &BTreeMap<String, Vec<usize>>,
    draft_index: usize,
    draft: &LearningDecisionCandidateDraftV1,
) -> LearningDecisionOutcomeSampleV1 {
    let record = &records[draft.record_index];
    let sibling_indexes = groups
        .get(&draft.sibling_group_id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let sibling_candidate_groups = sibling_candidate_groups_v1(drafts, sibling_indexes);
    let observed_candidate_index = sibling_candidate_groups
        .iter()
        .position(|indexes| indexes.contains(&draft_index))
        .unwrap_or(0);
    let sibling_candidates = sibling_candidate_groups
        .iter()
        .map(|candidate_indexes| learning_sibling_candidate_v1(records, drafts, candidate_indexes))
        .collect::<Vec<_>>();
    let observed_sibling_count = sibling_candidates.len();
    let candidate_set_status = if observed_sibling_count > 1 {
        LearningCandidateSetStatusV1::ObservedSiblings
    } else {
        LearningCandidateSetStatusV1::ChosenOnly
    };

    LearningDecisionOutcomeSampleV1 {
        schema_name: LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_NAME.to_string(),
        schema_version: LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        provenance: learning_provenance_v1(record, context),
        seed: record.seed,
        run_domain: record.run_domain.clone(),
        report_rounds_completed: record.report_rounds_completed,
        decision_id: format!(
            "{}|candidate={}|branch={}",
            draft.sibling_group_id, draft.candidate_command, record.branch_id
        ),
        sibling_group_id: draft.sibling_group_id.clone(),
        parent_branch_id: draft.parent_branch_id.clone(),
        step_index: draft.step_index,
        candidate_command: draft.candidate_command.clone(),
        candidate_choice_label: draft.candidate_choice_label.clone(),
        candidate_set_status,
        observed_candidate_index,
        observed_sibling_count,
        sibling_candidates,
        branch_group: record.branch_group.clone(),
        branch_index: record.branch_index,
        branch_id: record.branch_id.clone(),
        strategic_summary: record.strategic_summary,
        outcome: learning_branch_outcome_v1(record),
    }
}

fn learning_sibling_candidate_v1(
    records: &[BranchOutcomeRecordV1],
    drafts: &[LearningDecisionCandidateDraftV1],
    candidate_indexes: &[usize],
) -> LearningSiblingCandidateV1 {
    let representative_index = candidate_indexes
        .iter()
        .copied()
        .max_by_key(|index| records[drafts[*index].record_index].rank_key)
        .unwrap_or(0);
    let representative_draft = &drafts[representative_index];
    let representative_record = &records[representative_draft.record_index];
    LearningSiblingCandidateV1 {
        command: representative_draft.candidate_command.clone(),
        choice_label: representative_draft.candidate_choice_label.clone(),
        observed_branch_count: candidate_indexes.len(),
        representative_branch_group: representative_record.branch_group.clone(),
        representative_branch_index: representative_record.branch_index,
        representative_branch_id: representative_record.branch_id.clone(),
        best_outcome_class: representative_record.outcome_class.clone(),
        best_supervision_status: representative_record.supervision_status.clone(),
        best_rank_key: representative_record.rank_key,
        best_frontier_title: representative_record.frontier_title.clone(),
        outcome_class_counts: learning_outcome_class_counts_v1(records, drafts, candidate_indexes),
    }
}

fn sibling_candidate_groups_v1(
    drafts: &[LearningDecisionCandidateDraftV1],
    sibling_indexes: &[usize],
) -> Vec<Vec<usize>> {
    let mut candidate_groups: Vec<Vec<usize>> = Vec::new();
    for sibling_index in sibling_indexes {
        let draft = &drafts[*sibling_index];
        if let Some(group) = candidate_groups.iter_mut().find(|group| {
            group.first().is_some_and(|first_index| {
                let first = &drafts[*first_index];
                first.candidate_command == draft.candidate_command
                    && first.candidate_choice_label == draft.candidate_choice_label
            })
        }) {
            group.push(*sibling_index);
        } else {
            candidate_groups.push(vec![*sibling_index]);
        }
    }
    candidate_groups
}

fn learning_outcome_class_counts_v1(
    records: &[BranchOutcomeRecordV1],
    drafts: &[LearningDecisionCandidateDraftV1],
    candidate_indexes: &[usize],
) -> Vec<LearningOutcomeClassCountV1> {
    let mut counts = Vec::<LearningOutcomeClassCountV1>::new();
    for index in candidate_indexes {
        let record = &records[drafts[*index].record_index];
        if let Some(entry) = counts
            .iter_mut()
            .find(|entry| entry.outcome_class == record.outcome_class)
        {
            entry.count += 1;
        } else {
            counts.push(LearningOutcomeClassCountV1 {
                outcome_class: record.outcome_class.clone(),
                count: 1,
            });
        }
    }
    counts
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
        provenance: learning_provenance_v1(record, context),
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
        outcome: learning_branch_outcome_v1(record),
    }
}

fn learning_provenance_v1(
    record: &BranchOutcomeRecordV1,
    context: &LearningDatasetExportContextV1,
) -> LearningDatasetProvenanceV1 {
    LearningDatasetProvenanceV1 {
        exporter_git_commit: context.exporter_git_commit.clone(),
        exporter_git_dirty: context.exporter_git_dirty,
        source_report_path: context.source_report_path.clone(),
        source_checkpoint_path: context.source_checkpoint_path.clone(),
        source_record_schema_name: record.schema_name.clone(),
        source_record_schema_version: record.schema_version,
    }
}

fn learning_branch_outcome_v1(record: &BranchOutcomeRecordV1) -> LearningBranchOutcomeV1 {
    LearningBranchOutcomeV1 {
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
    Some(branch_id_from_command_prefix_v1(
        &commands[..commands.len().saturating_sub(1)],
    ))
}

fn branch_id_from_command_prefix_v1(commands: &[String]) -> String {
    if commands.is_empty() {
        "root".to_string()
    } else {
        format!("root.{}", commands.join("."))
    }
}

fn decision_sibling_group_id_v1(record: &BranchOutcomeRecordV1, step_index: usize) -> String {
    let parent_branch_id = branch_id_from_command_prefix_v1(&record.commands[..step_index]);
    format!(
        "seed={}|domain={}:{}|rounds={}|parent={}|step={}",
        record.seed,
        record.run_domain.label,
        record.run_domain.ascension_level,
        record.report_rounds_completed,
        parent_branch_id,
        step_index
    )
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

    #[test]
    fn decision_outcome_samples_group_observed_sibling_candidates() {
        let mut clothesline = sample_branch_outcome_record();
        clothesline.branch_id = "root.rp 0".to_string();
        clothesline.commands = vec!["rp 0".to_string()];
        clothesline.choice_labels = vec!["Clothesline".to_string()];
        clothesline.rank_key = 20;

        let mut shrug = sample_branch_outcome_record();
        shrug.branch_index = 1;
        shrug.branch_id = "root.rp 1".to_string();
        shrug.commands = vec!["rp 1".to_string()];
        shrug.choice_labels = vec!["Shrug It Off".to_string()];
        shrug.rank_key = 35;

        let samples = decision_outcome_samples_from_branch_outcomes_v1(
            &[clothesline, shrug],
            LearningDatasetExportContextV1::default(),
        );

        assert_eq!(samples.len(), 2);
        assert_eq!(
            samples[0].candidate_set_status,
            LearningCandidateSetStatusV1::ObservedSiblings
        );
        assert_eq!(samples[0].sibling_group_id, samples[1].sibling_group_id);
        assert_eq!(samples[0].parent_branch_id, "root");
        assert_eq!(samples[0].observed_sibling_count, 2);
        assert_eq!(samples[0].observed_candidate_index, 0);
        assert_eq!(samples[1].observed_candidate_index, 1);
        assert_eq!(
            samples[0]
                .sibling_candidates
                .iter()
                .map(|candidate| candidate.choice_label.as_str())
                .collect::<Vec<_>>(),
            vec!["Clothesline", "Shrug It Off"]
        );
        assert!(!samples[0].trainable_as_action_label);
        assert!(!samples[0].policy_quality_claim);
    }

    #[test]
    fn decision_outcome_samples_jsonl_round_trips() {
        let mut clothesline = sample_branch_outcome_record();
        clothesline.branch_id = "root.rp 0".to_string();
        clothesline.commands = vec!["rp 0".to_string()];
        clothesline.choice_labels = vec!["Clothesline".to_string()];

        let samples = decision_outcome_samples_from_branch_outcomes_v1(
            &[clothesline],
            LearningDatasetExportContextV1::default(),
        );

        let text =
            serialize_learning_decision_outcome_samples_jsonl_v1(&samples).expect("serialize");
        let parsed = parse_learning_decision_outcome_samples_jsonl_v1(&text).expect("parse");

        assert_eq!(parsed, samples);
        assert_eq!(
            parsed[0].schema_name,
            LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_NAME
        );
        assert_eq!(parsed[0].schema_version, 1);
    }

    #[test]
    fn decision_outcome_samples_collapse_descendants_of_same_candidate() {
        let mut clothesline_one = sample_branch_outcome_record();
        clothesline_one.branch_id = "root.rp 0.rp 2".to_string();
        clothesline_one.commands = vec!["rp 0".to_string(), "rp 2".to_string()];
        clothesline_one.choice_labels = vec!["Clothesline".to_string(), "Iron Wave".to_string()];
        clothesline_one.rank_key = 10;

        let mut clothesline_two = sample_branch_outcome_record();
        clothesline_two.branch_index = 1;
        clothesline_two.branch_id = "root.rp 0.rp 1".to_string();
        clothesline_two.commands = vec!["rp 0".to_string(), "rp 1".to_string()];
        clothesline_two.choice_labels =
            vec!["Clothesline".to_string(), "Pommel Strike".to_string()];
        clothesline_two.rank_key = 30;

        let mut shrug = sample_branch_outcome_record();
        shrug.branch_index = 2;
        shrug.branch_id = "root.rp 1".to_string();
        shrug.commands = vec!["rp 1".to_string()];
        shrug.choice_labels = vec!["Shrug It Off".to_string()];
        shrug.rank_key = 20;

        let samples = decision_outcome_samples_from_branch_outcomes_v1(
            &[clothesline_one, clothesline_two, shrug],
            LearningDatasetExportContextV1::default(),
        );
        let root_clothesline = samples
            .iter()
            .find(|sample| sample.parent_branch_id == "root" && sample.candidate_command == "rp 0")
            .expect("root clothesline sample");

        assert_eq!(root_clothesline.observed_sibling_count, 2);
        assert_eq!(
            root_clothesline
                .sibling_candidates
                .iter()
                .map(|candidate| (
                    candidate.choice_label.as_str(),
                    candidate.observed_branch_count,
                    candidate.best_rank_key
                ))
                .collect::<Vec<_>>(),
            vec![("Clothesline", 2, 30), ("Shrug It Off", 1, 20)]
        );
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
                reward_boundary: None,
            }),
            checkpoint_enriched: false,
            state_features: None,
        }
    }
}
