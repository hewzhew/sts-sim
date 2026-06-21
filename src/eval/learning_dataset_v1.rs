use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::eval::branch_campaign::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignReportV1,
    BranchCampaignRunDomainV1,
};
use crate::eval::branch_outcome_dataset_v1::{
    BranchOutcomeClassV1, BranchOutcomeRecordV1, BranchOutcomeStateFeaturesV1,
    BranchOutcomeSupervisionStatusV1,
};
use crate::eval::campaign_journal::{
    CampaignJournalCandidateAdmissionReasonCategoryV1,
    CampaignJournalCandidateAdmissionReasonCodeV1, CampaignJournalCandidateAdmissionStatusV1,
    CampaignJournalCandidateAdmissionTraceV1, CampaignJournalCandidateDispositionV1,
    CampaignJournalCandidateV1, CampaignJournalEventPayloadV1, CampaignJournalEventV1,
    CampaignJournalRouteCandidateV1,
};

pub const LEARNING_BRANCH_SAMPLE_SCHEMA_NAME: &str = "LearningBranchSampleV1";
pub const LEARNING_BRANCH_SAMPLE_SCHEMA_VERSION: u32 = 1;
pub const LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_NAME: &str = "LearningDecisionOutcomeSampleV1";
pub const LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_VERSION: u32 = 1;
pub const LEARNING_DECISION_OUTCOME_ANALYSIS_SCHEMA_NAME: &str =
    "LearningDecisionOutcomeAnalysisV1";
pub const LEARNING_DECISION_OUTCOME_ANALYSIS_SCHEMA_VERSION: u32 = 1;
pub const LEARNING_DECISION_CANDIDATE_COVERAGE_SCHEMA_NAME: &str =
    "LearningDecisionCandidateCoverageReportV1";
pub const LEARNING_DECISION_CANDIDATE_COVERAGE_SCHEMA_VERSION: u32 = 1;
pub const LEARNING_READINESS_PROBE_SCHEMA_NAME: &str = "LearningReadinessProbeV1";
pub const LEARNING_READINESS_PROBE_SCHEMA_VERSION: u32 = 1;
pub const TARGETED_CONTINUATION_PLAN_SCHEMA_NAME: &str = "TargetedContinuationPlanV1";
pub const TARGETED_CONTINUATION_PLAN_SCHEMA_VERSION: u32 = 1;
pub const TARGETED_CONTINUATION_EXECUTION_PLAN_SCHEMA_NAME: &str =
    "TargetedContinuationExecutionPlanV1";
pub const TARGETED_CONTINUATION_EXECUTION_PLAN_SCHEMA_VERSION: u32 = 1;
pub const COVERAGE_GAP_CONTINUATION_PLAN_SCHEMA_NAME: &str = "CoverageGapContinuationPlanV1";
pub const COVERAGE_GAP_CONTINUATION_PLAN_SCHEMA_VERSION: u32 = 1;
pub const COVERAGE_GAP_CONTINUATION_EXECUTION_PLAN_SCHEMA_NAME: &str =
    "CoverageGapContinuationExecutionPlanV1";
pub const COVERAGE_GAP_CONTINUATION_EXECUTION_PLAN_SCHEMA_VERSION: u32 = 1;
pub const CONTINUATION_EFFECT_REPORT_SCHEMA_NAME: &str = "ContinuationEffectReportV1";
pub const CONTINUATION_EFFECT_REPORT_SCHEMA_VERSION: u32 = 1;
const CONTINUATION_EFFECT_EXAMPLE_LIMIT: usize = 6;

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
pub struct LearningDecisionHistogramEntryV1 {
    pub key: String,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionOutcomeAnalysisV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_samples: usize,
    pub decision_group_count: usize,
    pub observed_sibling_group_count: usize,
    pub outcome_divergent_group_count: usize,
    pub censored_only_group_count: usize,
    pub command_family_counts: Vec<LearningDecisionHistogramEntryV1>,
    pub outcome_class_counts: Vec<LearningDecisionHistogramEntryV1>,
    pub group_examples: Vec<LearningDecisionGroupExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionCandidateCoverageReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_decisions: usize,
    pub total_candidates: usize,
    pub observed_candidates: usize,
    pub unobserved_candidates: usize,
    pub fully_observed_decisions: usize,
    pub partially_observed_decisions: usize,
    pub unobserved_decisions: usize,
    pub examples: Vec<LearningDecisionCandidateCoverageExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionCandidateCoverageExampleV1 {
    pub decision_id: String,
    pub event_type: String,
    pub parent_branch_id: String,
    pub parent_choices: Vec<String>,
    pub candidate_count: usize,
    pub observed_count: usize,
    pub unobserved_candidates: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapContinuationPlanV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_decisions: usize,
    pub total_candidates: usize,
    pub total_unobserved_candidates: usize,
    #[serde(default)]
    pub kept_unobserved_candidates: usize,
    #[serde(default)]
    pub pruned_unobserved_candidates: usize,
    #[serde(default)]
    pub scheduled_unobserved_candidates: usize,
    #[serde(default)]
    pub unscheduled_unobserved_candidates: usize,
    pub selected_target_count: usize,
    #[serde(default)]
    pub selected_kept_targets: usize,
    #[serde(default)]
    pub selected_pruned_targets: usize,
    #[serde(default)]
    pub selected_scheduled_targets: usize,
    #[serde(default)]
    pub selected_unscheduled_targets: usize,
    pub targets: Vec<CoverageGapContinuationTargetV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapContinuationTargetV1 {
    pub decision_id: String,
    pub event_id: String,
    pub event_type: String,
    pub parent_branch_id: String,
    pub parent_frontier_title: String,
    pub parent_commands: Vec<String>,
    pub parent_choices: Vec<String>,
    pub candidate_index: usize,
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub semantic_class: String,
    #[serde(default)]
    pub admission: CampaignJournalCandidateAdmissionTraceV1,
    pub disposition: CampaignJournalCandidateDispositionV1,
    #[serde(
        default,
        skip_serializing_if = "CoverageGapContinuationTargetOriginV1::is_empty"
    )]
    pub target_origin: CoverageGapContinuationTargetOriginV1,
    pub milestone: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapContinuationTargetOriginV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<CoverageGapRouteTargetOriginV1>,
}

impl CoverageGapContinuationTargetOriginV1 {
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() && self.route.is_none()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapRouteTargetOriginV1 {
    pub legal_candidate_count: usize,
    pub emitted_candidate_count: usize,
    pub complete_legal_pool: bool,
    pub ordering: String,
    pub target_x: i32,
    pub target_y: i32,
    pub room_type: String,
    pub move_kind: String,
    pub action_kind: String,
    pub projection_source: String,
    pub projection_coverage: String,
    pub path_budget: usize,
    pub observed_path_count: usize,
    #[serde(
        default,
        skip_serializing_if = "CoverageGapRoutePathOriginV1::is_empty"
    )]
    pub path: CoverageGapRoutePathOriginV1,
    #[serde(
        default,
        skip_serializing_if = "CoverageGapRouteFirstEliteOriginV1::is_empty"
    )]
    pub first_elite: CoverageGapRouteFirstEliteOriginV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapRoutePathOriginV1 {
    pub path_count: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub path_budget_exhausted: bool,
    pub min_early_pressure: usize,
    pub max_early_pressure: usize,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_shops: usize,
    pub max_shops: usize,
    pub min_fires: usize,
    pub max_fires: usize,
    pub min_unknowns: usize,
    pub max_unknowns: usize,
    pub min_treasures: usize,
    pub max_treasures: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_shop_floor: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_fire_floor: Option<i32>,
    pub min_damage_rooms_before_recovery: usize,
    pub max_damage_rooms_before_recovery: usize,
    pub min_unknowns_before_recovery: usize,
    pub max_unknowns_before_recovery: usize,
    pub paths_with_recovery_before_damage: usize,
}

impl CoverageGapRoutePathOriginV1 {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapRouteFirstEliteOriginV1 {
    pub paths_with_first_elite: usize,
    pub forced: bool,
    pub optional: bool,
    pub min_hallway_fights_before: usize,
    pub max_hallway_fights_before: usize,
    pub min_unknowns_before: usize,
    pub max_unknowns_before: usize,
    pub min_fires_before: usize,
    pub max_fires_before: usize,
    pub min_shops_before: usize,
    pub max_shops_before: usize,
    pub can_bail_to_rest_before: bool,
    pub can_bail_to_shop_before: bool,
}

impl CoverageGapRouteFirstEliteOriginV1 {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageGapContinuationExecutionPlanV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub requested_target_count: usize,
    pub selected_branch_count: usize,
    pub skipped_target_count: usize,
    pub targets: Vec<CoverageGapContinuationTargetV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningDecisionGroupExampleV1 {
    pub sibling_group_id: String,
    pub parent_branch_id: String,
    pub step_index: usize,
    pub command_family: String,
    pub observed_sibling_count: usize,
    pub sample_count: usize,
    pub candidate_summaries: Vec<String>,
    pub outcome_classes: Vec<LearningDecisionHistogramEntryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningReadinessProbeV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_samples: usize,
    pub decision_group_count: usize,
    pub observed_sibling_group_count: usize,
    pub terminal_group_count: usize,
    pub terminal_observed_sibling_group_count: usize,
    pub censored_only_group_count: usize,
    pub branch_scheduling_censored_group_count: usize,
    pub combat_unresolved_group_count: usize,
    pub missing_context_group_count: usize,
    pub missing_context_sample_count: usize,
    pub bottlenecks: Vec<LearningReadinessBottleneckV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningReadinessBottleneckV1 {
    pub key: String,
    pub group_count: usize,
    pub sample_count: usize,
    pub next_action: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedContinuationPlanV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_samples: usize,
    pub total_decision_groups: usize,
    pub selected_target_count: usize,
    pub targets: Vec<TargetedContinuationTargetV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedContinuationTargetV1 {
    pub sibling_group_id: String,
    pub parent_branch_id: String,
    pub step_index: usize,
    pub command_family: String,
    pub priority_bucket: i32,
    pub reason_keys: Vec<String>,
    pub milestone: String,
    pub candidates: Vec<TargetedContinuationCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedContinuationCandidateV1 {
    pub command: String,
    pub choice_label: String,
    pub representative_branch_id: String,
    pub representative_branch_group: String,
    pub observed_branch_count: usize,
    pub best_outcome_class: BranchOutcomeClassV1,
    pub best_supervision_status: BranchOutcomeSupervisionStatusV1,
    pub best_rank_key: i32,
    pub needs_continuation: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedContinuationExecutionPlanV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub requested_target_count: usize,
    pub selected_branch_count: usize,
    pub missing_branch_count: usize,
    pub skipped_candidate_count: usize,
    pub branches: Vec<TargetedContinuationBranchRequestV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedContinuationBranchRequestV1 {
    pub sibling_group_id: String,
    pub target_index: usize,
    pub candidate_index: usize,
    pub milestone: String,
    pub reason_keys: Vec<String>,
    pub command: String,
    pub choice_label: String,
    pub representative_branch_id: String,
    pub representative_branch_group: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationEffectReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub before_samples: usize,
    pub after_samples: usize,
    pub before_groups: usize,
    pub after_groups: usize,
    pub common_groups: usize,
    pub before_censored_only_groups: usize,
    pub after_censored_only_groups: usize,
    pub censored_only_delta: isize,
    pub newly_terminal_groups: usize,
    pub newly_terminal_observed_sibling_groups: usize,
    pub still_censored_target_groups: usize,
    pub expanded_target_groups: usize,
    pub examples: Vec<ContinuationEffectExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationEffectExampleV1 {
    pub sibling_group_id: String,
    pub effect: String,
    pub before_summary: String,
    pub after_summary: String,
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

    for (record_index, record) in records.iter().enumerate() {
        for step_index in 0..record.commands.len() {
            let parent_branch_id = branch_id_from_command_prefix_v1(&record.commands[..step_index]);
            let sibling_group_id = decision_sibling_group_id_v1(record, step_index);
            let draft = LearningDecisionCandidateDraftV1 {
                record_index,
                step_index,
                candidate_id: record.commands[step_index].clone(),
                sibling_group_id: sibling_group_id.clone(),
                parent_branch_id,
                candidate_command: record.commands[step_index].clone(),
                candidate_choice_label: record
                    .choice_labels
                    .get(step_index)
                    .cloned()
                    .unwrap_or_default(),
            };
            drafts.push(draft);
        }
    }

    decision_outcome_samples_from_drafts_v1(records, &context, drafts)
}

pub fn decision_outcome_samples_from_campaign_report_v1(
    report: &BranchCampaignReportV1,
    records: &[BranchOutcomeRecordV1],
    context: LearningDatasetExportContextV1,
) -> Vec<LearningDecisionOutcomeSampleV1> {
    let drafts = journal_decision_candidate_drafts_v1(report, records);
    if drafts.is_empty() {
        return decision_outcome_samples_from_branch_outcomes_v1(records, context);
    }
    decision_outcome_samples_from_drafts_v1(records, &context, drafts)
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

pub fn analyze_learning_decision_outcome_samples_v1(
    samples: &[LearningDecisionOutcomeSampleV1],
) -> LearningDecisionOutcomeAnalysisV1 {
    let mut groups = BTreeMap::<String, Vec<&LearningDecisionOutcomeSampleV1>>::new();
    let mut outcome_class_counts = BTreeMap::<String, usize>::new();
    for sample in samples {
        groups
            .entry(sample.sibling_group_id.clone())
            .or_default()
            .push(sample);
        *outcome_class_counts
            .entry(format!("{:?}", sample.outcome.outcome_class))
            .or_default() += 1;
    }

    let mut command_family_counts = BTreeMap::<String, usize>::new();
    let mut observed_sibling_group_count = 0usize;
    let mut outcome_divergent_group_count = 0usize;
    let mut censored_only_group_count = 0usize;
    let mut group_examples = Vec::new();

    for group_samples in groups.values() {
        if group_samples.is_empty() {
            continue;
        }
        let representative = representative_decision_sample_v1(group_samples);
        let command_family = command_family_v1(&representative.candidate_command);
        *command_family_counts
            .entry(command_family.clone())
            .or_default() += 1;

        let observed_sibling_count = group_samples
            .iter()
            .map(|sample| sample.observed_sibling_count)
            .max()
            .unwrap_or(0);
        if observed_sibling_count > 1 {
            observed_sibling_group_count += 1;
        }

        let outcome_classes = decision_group_outcome_class_counts_v1(group_samples);
        let outcome_divergent = outcome_classes.len() > 1;
        if outcome_divergent {
            outcome_divergent_group_count += 1;
        }
        if group_samples.iter().all(|sample| {
            sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::CensoredOngoing
        }) {
            censored_only_group_count += 1;
        }

        if group_examples.len() < 12 && (outcome_divergent || observed_sibling_count > 1) {
            group_examples.push(LearningDecisionGroupExampleV1 {
                sibling_group_id: representative.sibling_group_id.clone(),
                parent_branch_id: representative.parent_branch_id.clone(),
                step_index: representative.step_index,
                command_family,
                observed_sibling_count,
                sample_count: group_samples.len(),
                candidate_summaries: representative
                    .sibling_candidates
                    .iter()
                    .map(learning_candidate_summary_v1)
                    .collect(),
                outcome_classes,
            });
        }
    }

    LearningDecisionOutcomeAnalysisV1 {
        schema_name: LEARNING_DECISION_OUTCOME_ANALYSIS_SCHEMA_NAME.to_string(),
        schema_version: LEARNING_DECISION_OUTCOME_ANALYSIS_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_samples: samples.len(),
        decision_group_count: groups.len(),
        observed_sibling_group_count,
        outcome_divergent_group_count,
        censored_only_group_count,
        command_family_counts: learning_histogram_entries_by_key_v1(command_family_counts),
        outcome_class_counts: learning_histogram_entries_by_key_v1(outcome_class_counts),
        group_examples,
    }
}

pub fn analyze_journal_decision_candidate_coverage_v1(
    report: &BranchCampaignReportV1,
    records: &[BranchOutcomeRecordV1],
) -> LearningDecisionCandidateCoverageReportV1 {
    const EXAMPLE_LIMIT: usize = 8;

    let mut total_decisions = 0usize;
    let mut total_candidates = 0usize;
    let mut observed_candidates = 0usize;
    let mut fully_observed_decisions = 0usize;
    let mut partially_observed_decisions = 0usize;
    let mut unobserved_decisions = 0usize;
    let mut examples = Vec::new();

    for event in &report.journal.events {
        let Some(decision_id) = journal_decision_id_v1(event) else {
            continue;
        };
        let candidates = journal_decision_candidates_v1(event);
        if candidates.is_empty() {
            continue;
        }

        total_decisions += 1;
        total_candidates += candidates.len();
        let parent_commands = event.branch_commands.as_slice();
        let mut observed = Vec::new();
        let mut unobserved = Vec::new();
        for candidate in candidates {
            if records.iter().any(|record| {
                record_commands_start_with_candidate_v1(
                    &record.commands,
                    parent_commands,
                    &candidate.command,
                )
            }) {
                observed.push(candidate);
            } else {
                unobserved.push(candidate);
            }
        }

        observed_candidates += observed.len();
        match (observed.is_empty(), unobserved.is_empty()) {
            (_, true) => fully_observed_decisions += 1,
            (true, false) => unobserved_decisions += 1,
            (false, false) => partially_observed_decisions += 1,
        }

        if !unobserved.is_empty() && examples.len() < EXAMPLE_LIMIT {
            examples.push(LearningDecisionCandidateCoverageExampleV1 {
                decision_id: decision_id.to_string(),
                event_type: journal_decision_event_kind_v1(event).to_string(),
                parent_branch_id: event.branch_id.clone(),
                parent_choices: event.branch_choices.clone(),
                candidate_count: candidates.len(),
                observed_count: observed.len(),
                unobserved_candidates: unobserved
                    .iter()
                    .take(6)
                    .map(|candidate| {
                        format!(
                            "{} {{{}}}",
                            compact_learning_text_v1(&candidate.label, 44),
                            compact_learning_text_v1(&candidate.command, 28)
                        )
                    })
                    .collect(),
            });
        }
    }

    LearningDecisionCandidateCoverageReportV1 {
        schema_name: LEARNING_DECISION_CANDIDATE_COVERAGE_SCHEMA_NAME.to_string(),
        schema_version: LEARNING_DECISION_CANDIDATE_COVERAGE_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_decisions,
        total_candidates,
        observed_candidates,
        unobserved_candidates: total_candidates.saturating_sub(observed_candidates),
        fully_observed_decisions,
        partially_observed_decisions,
        unobserved_decisions,
        examples,
    }
}

pub fn render_journal_decision_candidate_coverage_v1(
    report: &LearningDecisionCandidateCoverageReportV1,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "DecisionCandidateCoverageV1 decisions={} candidates={} observed={} unobserved={} full={} partial={} none={}",
        report.total_decisions,
        report.total_candidates,
        report.observed_candidates,
        report.unobserved_candidates,
        report.fully_observed_decisions,
        report.partially_observed_decisions,
        report.unobserved_decisions
    ));
    if !report.examples.is_empty() {
        lines.push("Coverage gaps:".to_string());
        for example in &report.examples {
            lines.push(format!(
                "  {} {} observed={}/{} parent={} missing={}",
                example.event_type,
                compact_learning_text_v1(&example.decision_id, 58),
                example.observed_count,
                example.candidate_count,
                compact_learning_text_v1(&example.parent_branch_id, 40),
                if example.unobserved_candidates.is_empty() {
                    "-".to_string()
                } else {
                    example.unobserved_candidates.join("; ")
                }
            ));
        }
    }
    lines.join("\n")
}

pub fn plan_coverage_gap_continuations_v1(
    report: &BranchCampaignReportV1,
    records: &[BranchOutcomeRecordV1],
    max_targets: usize,
    max_candidates_per_decision: usize,
) -> CoverageGapContinuationPlanV1 {
    let mut total_decisions = 0usize;
    let mut total_candidates = 0usize;
    let mut total_unobserved_candidates = 0usize;
    let mut kept_unobserved_candidates = 0usize;
    let mut pruned_unobserved_candidates = 0usize;
    let mut scheduled_unobserved_candidates = 0usize;
    let mut unscheduled_unobserved_candidates = 0usize;
    let mut selected_kept_targets = 0usize;
    let mut selected_pruned_targets = 0usize;
    let mut selected_scheduled_targets = 0usize;
    let mut selected_unscheduled_targets = 0usize;
    let mut target_candidates = Vec::new();

    for event in &report.journal.events {
        let Some(decision_id) = journal_decision_id_v1(event) else {
            continue;
        };
        let candidates = journal_decision_candidates_v1(event);
        if candidates.is_empty() {
            continue;
        }

        total_decisions = total_decisions.saturating_add(1);
        total_candidates = total_candidates.saturating_add(candidates.len());
        let parent_commands = event.branch_commands.as_slice();
        let mut unobserved = Vec::new();
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if records.iter().any(|record| {
                record_commands_start_with_candidate_v1(
                    &record.commands,
                    parent_commands,
                    &candidate.command,
                )
            }) {
                continue;
            }

            total_unobserved_candidates = total_unobserved_candidates.saturating_add(1);
            match candidate.disposition {
                CampaignJournalCandidateDispositionV1::Kept => {
                    kept_unobserved_candidates = kept_unobserved_candidates.saturating_add(1)
                }
                CampaignJournalCandidateDispositionV1::Pruned => {
                    pruned_unobserved_candidates = pruned_unobserved_candidates.saturating_add(1)
                }
            }
            if candidate_admission_is_scheduled_v1(candidate) {
                scheduled_unobserved_candidates = scheduled_unobserved_candidates.saturating_add(1);
            } else {
                unscheduled_unobserved_candidates =
                    unscheduled_unobserved_candidates.saturating_add(1);
            }
            unobserved.push((candidate_index, candidate));
        }

        let mut selected_for_decision = 0usize;
        let ordered_unobserved = unobserved
            .iter()
            .copied()
            .filter(|(_, candidate)| candidate_admission_is_scheduled_v1(candidate))
            .chain(
                unobserved
                    .iter()
                    .copied()
                    .filter(|(_, candidate)| !candidate_admission_is_scheduled_v1(candidate)),
            );
        for (candidate_index, candidate) in ordered_unobserved {
            if selected_for_decision >= max_candidates_per_decision {
                continue;
            }
            target_candidates.push(CoverageGapContinuationTargetV1 {
                decision_id: decision_id.to_string(),
                event_id: event.event_id.clone(),
                event_type: journal_decision_event_kind_v1(event).to_string(),
                parent_branch_id: event.branch_id.clone(),
                parent_frontier_title: event.branch_frontier_title.clone(),
                parent_commands: event.branch_commands.clone(),
                parent_choices: event.branch_choices.clone(),
                candidate_index,
                candidate_id: candidate.candidate_id.clone(),
                command: candidate.command.clone(),
                label: candidate.label.clone(),
                semantic_class: candidate.semantic_class.clone(),
                admission: resolved_candidate_admission_v1(candidate),
                disposition: candidate.disposition,
                target_origin: coverage_gap_target_origin_v1(event, candidate_index),
                milestone: coverage_gap_candidate_milestone_v1(event),
            });
            selected_for_decision = selected_for_decision.saturating_add(1);
        }
    }

    let targets = select_coverage_gap_targets_by_type_v1(
        dedupe_coverage_gap_targets_v1(target_candidates),
        max_targets,
    );
    for target in &targets {
        match target.disposition {
            CampaignJournalCandidateDispositionV1::Kept => {
                selected_kept_targets = selected_kept_targets.saturating_add(1)
            }
            CampaignJournalCandidateDispositionV1::Pruned => {
                selected_pruned_targets = selected_pruned_targets.saturating_add(1)
            }
        }
        if target.admission.status == CampaignJournalCandidateAdmissionStatusV1::Scheduled {
            selected_scheduled_targets = selected_scheduled_targets.saturating_add(1);
        } else {
            selected_unscheduled_targets = selected_unscheduled_targets.saturating_add(1);
        }
    }

    CoverageGapContinuationPlanV1 {
        schema_name: COVERAGE_GAP_CONTINUATION_PLAN_SCHEMA_NAME.to_string(),
        schema_version: COVERAGE_GAP_CONTINUATION_PLAN_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_decisions,
        total_candidates,
        total_unobserved_candidates,
        kept_unobserved_candidates,
        pruned_unobserved_candidates,
        scheduled_unobserved_candidates,
        unscheduled_unobserved_candidates,
        selected_target_count: targets.len(),
        selected_kept_targets,
        selected_pruned_targets,
        selected_scheduled_targets,
        selected_unscheduled_targets,
        targets,
    }
}

fn select_coverage_gap_targets_by_type_v1(
    targets: Vec<CoverageGapContinuationTargetV1>,
    max_targets: usize,
) -> Vec<CoverageGapContinuationTargetV1> {
    if max_targets == 0 || targets.is_empty() {
        return Vec::new();
    }

    let mut buckets = BTreeMap::<String, Vec<CoverageGapContinuationTargetV1>>::new();
    for target in targets {
        buckets
            .entry(coverage_gap_target_bucket_v1(&target).to_string())
            .or_default()
            .push(target);
    }

    let mut bucket_order = coverage_gap_target_bucket_order_v1(&buckets);
    let mut selected = Vec::new();
    while selected.len() < max_targets && !bucket_order.is_empty() {
        let mut retained_order = Vec::new();
        for bucket in bucket_order {
            let Some(items) = buckets.get_mut(&bucket) else {
                continue;
            };
            if !items.is_empty() && selected.len() < max_targets {
                selected.push(items.remove(0));
            }
            if !items.is_empty() {
                retained_order.push(bucket);
            }
        }
        bucket_order = retained_order;
    }
    selected
}

fn dedupe_coverage_gap_targets_v1(
    targets: Vec<CoverageGapContinuationTargetV1>,
) -> Vec<CoverageGapContinuationTargetV1> {
    let mut seen = BTreeSet::<(String, Vec<String>, String, String)>::new();
    targets
        .into_iter()
        .filter(|target| {
            seen.insert((
                target.event_type.clone(),
                target.parent_commands.clone(),
                target.command.clone(),
                target.candidate_id.clone(),
            ))
        })
        .collect()
}

fn coverage_gap_target_bucket_order_v1(
    buckets: &BTreeMap<String, Vec<CoverageGapContinuationTargetV1>>,
) -> Vec<String> {
    let mut order = Vec::new();
    for bucket in [
        "route",
        "boss_relic",
        "shop",
        "shop_branch",
        "event",
        "campfire",
        "reward",
        "other",
    ] {
        if buckets.contains_key(bucket) {
            order.push(bucket.to_string());
        }
    }
    for bucket in buckets.keys() {
        if !order.iter().any(|existing| existing == bucket) {
            order.push(bucket.clone());
        }
    }
    order
}

fn coverage_gap_target_bucket_v1(target: &CoverageGapContinuationTargetV1) -> &'static str {
    match target.event_type.as_str() {
        "route" => "route",
        "boss_relic" => "boss_relic",
        "shop" => "shop",
        "shop_branch" => "shop_branch",
        "event" => "event",
        "campfire" => "campfire",
        "reward" => "reward",
        _ => "other",
    }
}

pub fn coverage_gap_continuation_execution_plan_v1(
    plan: &CoverageGapContinuationPlanV1,
    max_targets: usize,
) -> CoverageGapContinuationExecutionPlanV1 {
    let targets = plan
        .targets
        .iter()
        .take(max_targets)
        .cloned()
        .collect::<Vec<_>>();
    CoverageGapContinuationExecutionPlanV1 {
        schema_name: COVERAGE_GAP_CONTINUATION_EXECUTION_PLAN_SCHEMA_NAME.to_string(),
        schema_version: COVERAGE_GAP_CONTINUATION_EXECUTION_PLAN_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        requested_target_count: max_targets.min(plan.targets.len()),
        selected_branch_count: targets.len(),
        skipped_target_count: plan.targets.len().saturating_sub(targets.len()),
        targets,
    }
}

pub fn render_coverage_gap_continuation_plan_v1(plan: &CoverageGapContinuationPlanV1) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "CoverageGapContinuationPlanV1 decisions={} candidates={} unobserved={} scheduled_unobserved={} unscheduled_unobserved={} kept_unobserved={} pruned_unobserved={} selected={} scheduled_selected={} unscheduled_selected={} kept_selected={} pruned_selected={}",
        plan.total_decisions,
        plan.total_candidates,
        plan.total_unobserved_candidates,
        plan.scheduled_unobserved_candidates,
        plan.unscheduled_unobserved_candidates,
        plan.kept_unobserved_candidates,
        plan.pruned_unobserved_candidates,
        plan.selected_target_count,
        plan.selected_scheduled_targets,
        plan.selected_unscheduled_targets,
        plan.selected_kept_targets,
        plan.selected_pruned_targets
    ));
    if plan.targets.is_empty() {
        lines.push("Targets: none".to_string());
    } else {
        lines.push("Targets:".to_string());
        for (index, target) in plan.targets.iter().take(12).enumerate() {
            lines.push(render_coverage_gap_target_line_v1(index, target));
        }
        if plan.targets.len() > 12 {
            lines.push(format!(
                "  ... {} more target(s)",
                plan.targets.len().saturating_sub(12)
            ));
        }
    }
    lines.join("\n")
}

pub fn render_coverage_gap_execution_plan_v1(
    execution: &CoverageGapContinuationExecutionPlanV1,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "CoverageGapContinuationExecutionPlanV1 requested={} selected={} skipped={}",
        execution.requested_target_count,
        execution.selected_branch_count,
        execution.skipped_target_count
    ));
    if execution.targets.is_empty() {
        lines.push("Executed targets: none".to_string());
    } else {
        lines.push("Executed targets:".to_string());
        for (index, target) in execution.targets.iter().take(12).enumerate() {
            lines.push(render_coverage_gap_target_line_v1(index, target));
        }
        if execution.targets.len() > 12 {
            lines.push(format!(
                "  ... {} more target(s)",
                execution.targets.len().saturating_sub(12)
            ));
        }
    }
    lines.join("\n")
}

fn render_coverage_gap_target_line_v1(
    index: usize,
    target: &CoverageGapContinuationTargetV1,
) -> String {
    format!(
        "  {}. {} {} | parent={} coord={} candidate={} {{{}}} admission={} reason_category={} reason_code={} disposition={} milestone={} origin={} semantic=[{}]",
        index + 1,
        target.event_type,
        compact_learning_text_v1(&target.decision_id, 56),
        compact_learning_text_v1(&target.parent_branch_id, 36),
        compact_learning_text_v1(&render_coverage_gap_parent_coordinate_v1(target), 48),
        compact_learning_text_v1(&target.label, 42),
        compact_learning_text_v1(&target.command, 28),
        render_journal_candidate_admission_status_v1(target.admission.status),
        render_journal_candidate_admission_reason_category_v1(
            target.admission.normalized_reason_category()
        ),
        render_journal_candidate_admission_reason_code_v1(
            target.admission.normalized_reason_code()
        ),
        render_journal_candidate_disposition_v1(target.disposition),
        target.milestone,
        render_coverage_gap_target_origin_v1(&target.target_origin),
        compact_learning_text_v1(&target.semantic_class, 58)
    )
}

fn render_coverage_gap_parent_coordinate_v1(target: &CoverageGapContinuationTargetV1) -> String {
    if target.parent_commands.is_empty() {
        "root".to_string()
    } else {
        target.parent_commands.join(" -> ")
    }
}

fn render_coverage_gap_target_origin_v1(origin: &CoverageGapContinuationTargetOriginV1) -> String {
    let Some(route) = origin.route.as_ref() else {
        if origin.source.is_empty() {
            return "-".to_string();
        }
        return origin.source.clone();
    };
    let first_elite = render_coverage_gap_route_first_elite_origin_v1(&route.first_elite);
    format!(
        "{} route=x{}y{} room={} move={} coverage={} paths={}/{} budget_exhausted={} pool={}/{} complete={} path=elites:{} fires:{} shops:{} unknowns:{} damage_before_recovery:{} first_shop={} first_fire={} first_elite={}",
        origin.source,
        route.target_x,
        route.target_y,
        route.room_type,
        route.move_kind,
        route.projection_coverage,
        route.observed_path_count,
        route.path_budget,
        route.path.path_budget_exhausted,
        route.emitted_candidate_count,
        route.legal_candidate_count,
        route.complete_legal_pool,
        render_usize_range_v1(route.path.min_elites, route.path.max_elites),
        render_usize_range_v1(route.path.min_fires, route.path.max_fires),
        render_usize_range_v1(route.path.min_shops, route.path.max_shops),
        render_usize_range_v1(route.path.min_unknowns, route.path.max_unknowns),
        render_usize_range_v1(
            route.path.min_damage_rooms_before_recovery,
            route.path.max_damage_rooms_before_recovery
        ),
        render_optional_floor_v1(route.path.first_shop_floor),
        render_optional_floor_v1(route.path.first_fire_floor),
        first_elite
    )
}

fn render_coverage_gap_route_first_elite_origin_v1(
    first_elite: &CoverageGapRouteFirstEliteOriginV1,
) -> String {
    if first_elite.paths_with_first_elite == 0 {
        return "none".to_string();
    }
    let mode = if first_elite.forced {
        "forced"
    } else if first_elite.optional {
        "optional"
    } else {
        "seen"
    };
    let mut bailout = Vec::new();
    if first_elite.can_bail_to_rest_before {
        bailout.push("rest");
    }
    if first_elite.can_bail_to_shop_before {
        bailout.push("shop");
    }
    format!(
        "{} hallways:{} fires:{} shops:{} bailout:{}",
        mode,
        render_usize_range_v1(
            first_elite.min_hallway_fights_before,
            first_elite.max_hallway_fights_before
        ),
        render_usize_range_v1(first_elite.min_fires_before, first_elite.max_fires_before),
        render_usize_range_v1(first_elite.min_shops_before, first_elite.max_shops_before),
        if bailout.is_empty() {
            "-".to_string()
        } else {
            bailout.join("+")
        }
    )
}

fn render_usize_range_v1(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

fn render_optional_floor_v1(floor: Option<i32>) -> String {
    floor
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn candidate_admission_is_scheduled_v1(candidate: &CampaignJournalCandidateV1) -> bool {
    resolved_candidate_admission_v1(candidate).status
        == CampaignJournalCandidateAdmissionStatusV1::Scheduled
}

fn resolved_candidate_admission_v1(
    candidate: &CampaignJournalCandidateV1,
) -> CampaignJournalCandidateAdmissionTraceV1 {
    if candidate.admission.status != CampaignJournalCandidateAdmissionStatusV1::Unknown {
        return candidate.admission.clone();
    }
    CampaignJournalCandidateAdmissionTraceV1::from_disposition(
        candidate.disposition,
        "legacy_disposition",
        render_journal_candidate_disposition_v1(candidate.disposition),
    )
}

fn render_journal_candidate_admission_status_v1(
    status: CampaignJournalCandidateAdmissionStatusV1,
) -> &'static str {
    match status {
        CampaignJournalCandidateAdmissionStatusV1::Unknown => "unknown",
        CampaignJournalCandidateAdmissionStatusV1::Scheduled => "scheduled",
        CampaignJournalCandidateAdmissionStatusV1::Deferred => "deferred",
        CampaignJournalCandidateAdmissionStatusV1::Rejected => "rejected",
    }
}

fn render_journal_candidate_admission_reason_category_v1(
    category: CampaignJournalCandidateAdmissionReasonCategoryV1,
) -> &'static str {
    category.as_str()
}

fn render_journal_candidate_admission_reason_code_v1(
    code: CampaignJournalCandidateAdmissionReasonCodeV1,
) -> &'static str {
    code.as_str()
}

fn render_journal_candidate_disposition_v1(
    disposition: CampaignJournalCandidateDispositionV1,
) -> &'static str {
    match disposition {
        CampaignJournalCandidateDispositionV1::Kept => "kept",
        CampaignJournalCandidateDispositionV1::Pruned => "pruned",
    }
}

pub fn render_learning_decision_outcome_analysis_v1(
    analysis: &LearningDecisionOutcomeAnalysisV1,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "LearningDecisionOutcomeAnalysisV1 samples={} groups={} observed_sibling_groups={} outcome_divergent_groups={} censored_only_groups={}",
        analysis.total_samples,
        analysis.decision_group_count,
        analysis.observed_sibling_group_count,
        analysis.outcome_divergent_group_count,
        analysis.censored_only_group_count
    ));
    if !analysis.command_family_counts.is_empty() {
        lines.push(format!(
            "Command families: {}",
            render_learning_histogram_v1(&analysis.command_family_counts)
        ));
    }
    if !analysis.outcome_class_counts.is_empty() {
        lines.push(format!(
            "Outcome classes: {}",
            render_learning_histogram_v1(&analysis.outcome_class_counts)
        ));
    }
    if !analysis.group_examples.is_empty() {
        lines.push(String::new());
        lines.push("Useful sibling group examples:".to_string());
        for example in &analysis.group_examples {
            lines.push(format!(
                "  {} | family={} siblings={} samples={} outcomes={} parent={} step={}",
                compact_learning_text_v1(&example.sibling_group_id, 72),
                example.command_family,
                example.observed_sibling_count,
                example.sample_count,
                render_learning_histogram_v1(&example.outcome_classes),
                compact_learning_text_v1(&example.parent_branch_id, 48),
                example.step_index
            ));
            if !example.candidate_summaries.is_empty() {
                lines.push(format!("    {}", example.candidate_summaries.join("; ")));
            }
        }
    }
    lines.join("\n")
}

pub fn probe_learning_readiness_v1(
    samples: &[LearningDecisionOutcomeSampleV1],
) -> LearningReadinessProbeV1 {
    let mut groups = BTreeMap::<String, Vec<&LearningDecisionOutcomeSampleV1>>::new();
    for sample in samples {
        groups
            .entry(sample.sibling_group_id.clone())
            .or_default()
            .push(sample);
    }

    let mut observed_sibling_group_count = 0usize;
    let mut terminal_group_count = 0usize;
    let mut terminal_observed_sibling_group_count = 0usize;
    let mut censored_only_group_count = 0usize;
    let mut branch_scheduling_censored_group_count = 0usize;
    let mut combat_unresolved_group_count = 0usize;
    let mut missing_context_group_count = 0usize;
    let mut missing_context_sample_count = 0usize;
    let mut no_sibling_sample_count = 0usize;
    let mut censored_only_sample_count = 0usize;
    let mut branch_scheduling_sample_count = 0usize;
    let mut combat_unresolved_sample_count = 0usize;

    for group_samples in groups.values() {
        let observed_sibling = group_observed_sibling_count_v1(group_samples) > 1;
        let terminal = group_samples.iter().any(|sample| {
            sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::TerminalOutcome
        });
        let censored_only = group_samples.iter().all(|sample| {
            sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::CensoredOngoing
        });
        let branch_scheduling_censored = group_samples
            .iter()
            .any(|sample| sample_looks_scheduling_censored_v1(sample));
        let combat_unresolved = group_samples
            .iter()
            .any(|sample| sample_looks_combat_unresolved_v1(sample));
        let missing_context_samples = group_samples
            .iter()
            .filter(|sample| sample_missing_context_v1(sample))
            .count();

        if observed_sibling {
            observed_sibling_group_count += 1;
        } else {
            no_sibling_sample_count += group_samples.len();
        }
        if terminal {
            terminal_group_count += 1;
        }
        if terminal && observed_sibling {
            terminal_observed_sibling_group_count += 1;
        }
        if censored_only {
            censored_only_group_count += 1;
            censored_only_sample_count += group_samples.len();
        }
        if branch_scheduling_censored && !terminal {
            branch_scheduling_censored_group_count += 1;
            branch_scheduling_sample_count += group_samples.len();
        }
        if combat_unresolved && !terminal {
            combat_unresolved_group_count += 1;
            combat_unresolved_sample_count += group_samples.len();
        }
        if missing_context_samples > 0 {
            missing_context_group_count += 1;
            missing_context_sample_count += missing_context_samples;
        }
    }

    let single_candidate_group_count = groups.len().saturating_sub(observed_sibling_group_count);
    let mut bottlenecks = Vec::new();
    push_readiness_bottleneck_v1(
        &mut bottlenecks,
        "missing_context_snapshot",
        missing_context_group_count,
        missing_context_sample_count,
        "next=export with checkpoint/context enrichment",
    );
    push_readiness_bottleneck_v1(
        &mut bottlenecks,
        "no_sibling_alternatives",
        single_candidate_group_count,
        no_sibling_sample_count,
        "next=sample sibling alternatives at the same parent boundary",
    );
    push_readiness_bottleneck_v1(
        &mut bottlenecks,
        "outcome_censored",
        censored_only_group_count,
        censored_only_sample_count,
        "next=run targeted continuation to a milestone",
    );
    push_readiness_bottleneck_v1(
        &mut bottlenecks,
        "branch_scheduling_or_campaign_cutoff",
        branch_scheduling_censored_group_count,
        branch_scheduling_sample_count,
        "next=continue frozen/active siblings before treating them as labels",
    );
    push_readiness_bottleneck_v1(
        &mut bottlenecks,
        "combat_unresolved_or_budget",
        combat_unresolved_group_count,
        combat_unresolved_sample_count,
        "next=inspect combat search budget or combat policy on these groups",
    );

    LearningReadinessProbeV1 {
        schema_name: LEARNING_READINESS_PROBE_SCHEMA_NAME.to_string(),
        schema_version: LEARNING_READINESS_PROBE_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_samples: samples.len(),
        decision_group_count: groups.len(),
        observed_sibling_group_count,
        terminal_group_count,
        terminal_observed_sibling_group_count,
        censored_only_group_count,
        branch_scheduling_censored_group_count,
        combat_unresolved_group_count,
        missing_context_group_count,
        missing_context_sample_count,
        bottlenecks,
    }
}

pub fn render_learning_readiness_probe_v1(probe: &LearningReadinessProbeV1) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "LearningReadinessProbeV1 samples={} groups={} observed_sibling_groups={} terminal_groups={} terminal_observed_sibling_groups={}",
        probe.total_samples,
        probe.decision_group_count,
        probe.observed_sibling_group_count,
        probe.terminal_group_count,
        probe.terminal_observed_sibling_group_count
    ));
    lines.push(format!(
        "Censoring: censored_only_groups={} branch_scheduling_censored_groups={} combat_unresolved_groups={} missing_context_groups={} missing_context_samples={}",
        probe.censored_only_group_count,
        probe.branch_scheduling_censored_group_count,
        probe.combat_unresolved_group_count,
        probe.missing_context_group_count,
        probe.missing_context_sample_count
    ));
    if probe.bottlenecks.is_empty() {
        lines.push("Bottlenecks: none".to_string());
    } else {
        lines.push("Bottlenecks:".to_string());
        for bottleneck in &probe.bottlenecks {
            lines.push(format!(
                "  {} | groups={} samples={} | {}",
                bottleneck.key,
                bottleneck.group_count,
                bottleneck.sample_count,
                bottleneck.next_action
            ));
        }
    }
    lines.join("\n")
}

pub fn plan_targeted_continuations_v1(
    samples: &[LearningDecisionOutcomeSampleV1],
) -> TargetedContinuationPlanV1 {
    let mut groups = BTreeMap::<String, Vec<&LearningDecisionOutcomeSampleV1>>::new();
    for sample in samples {
        groups
            .entry(sample.sibling_group_id.clone())
            .or_default()
            .push(sample);
    }

    let mut targets = Vec::new();
    for group_samples in groups.values() {
        if let Some(target) = targeted_continuation_target_v1(group_samples) {
            targets.push(target);
        }
    }
    targets.sort_by(|left, right| {
        right
            .priority_bucket
            .cmp(&left.priority_bucket)
            .then_with(|| left.command_family.cmp(&right.command_family))
            .then_with(|| left.sibling_group_id.cmp(&right.sibling_group_id))
    });

    TargetedContinuationPlanV1 {
        schema_name: TARGETED_CONTINUATION_PLAN_SCHEMA_NAME.to_string(),
        schema_version: TARGETED_CONTINUATION_PLAN_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_samples: samples.len(),
        total_decision_groups: groups.len(),
        selected_target_count: targets.len(),
        targets,
    }
}

pub fn render_targeted_continuation_plan_v1(plan: &TargetedContinuationPlanV1) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "TargetedContinuationPlanV1 groups={} targets={} samples={}",
        plan.total_decision_groups, plan.selected_target_count, plan.total_samples
    ));
    if plan.targets.is_empty() {
        lines.push("Targets: none".to_string());
    } else {
        lines.push("Targets:".to_string());
        for (index, target) in plan.targets.iter().take(12).enumerate() {
            lines.push(format!(
                "  {}. {} | family={} priority={} reason={} milestone={} candidates={}",
                index + 1,
                compact_learning_text_v1(&target.sibling_group_id, 72),
                target.command_family,
                target.priority_bucket,
                target.reason_keys.join("+"),
                target.milestone,
                target.candidates.len()
            ));
            let shown_candidate_limit = 4;
            let mut candidate_parts = target
                .candidates
                .iter()
                .take(shown_candidate_limit)
                .map(targeted_continuation_candidate_summary_v1)
                .collect::<Vec<_>>();
            if target.candidates.len() > shown_candidate_limit {
                candidate_parts.push(format!(
                    "... {} more candidate(s)",
                    target.candidates.len() - shown_candidate_limit
                ));
            }
            let candidate_line = candidate_parts.join("; ");
            if !candidate_line.is_empty() {
                lines.push(format!("     {candidate_line}"));
            }
        }
    }
    lines.join("\n")
}

pub fn targeted_continuation_execution_plan_v1(
    plan: &TargetedContinuationPlanV1,
    report: &BranchCampaignReportV1,
    max_targets: usize,
    max_candidates_per_target: usize,
) -> TargetedContinuationExecutionPlanV1 {
    let branch_ids = targeted_continuation_report_branch_ids_v1(report);
    let mut branches = Vec::new();
    let mut missing_branch_count = 0usize;
    let mut skipped_candidate_count = 0usize;

    for (target_index, target) in plan.targets.iter().take(max_targets).enumerate() {
        let mut selected_for_target = 0usize;
        for (candidate_index, candidate) in target.candidates.iter().enumerate() {
            if !candidate.needs_continuation {
                skipped_candidate_count = skipped_candidate_count.saturating_add(1);
                continue;
            }
            if selected_for_target >= max_candidates_per_target {
                skipped_candidate_count = skipped_candidate_count.saturating_add(1);
                continue;
            }
            if !branch_ids.contains_key(&candidate.representative_branch_id) {
                missing_branch_count = missing_branch_count.saturating_add(1);
                continue;
            }
            branches.push(TargetedContinuationBranchRequestV1 {
                sibling_group_id: target.sibling_group_id.clone(),
                target_index,
                candidate_index,
                milestone: target.milestone.clone(),
                reason_keys: target.reason_keys.clone(),
                command: candidate.command.clone(),
                choice_label: candidate.choice_label.clone(),
                representative_branch_id: candidate.representative_branch_id.clone(),
                representative_branch_group: candidate.representative_branch_group.clone(),
            });
            selected_for_target = selected_for_target.saturating_add(1);
        }
    }

    TargetedContinuationExecutionPlanV1 {
        schema_name: TARGETED_CONTINUATION_EXECUTION_PLAN_SCHEMA_NAME.to_string(),
        schema_version: TARGETED_CONTINUATION_EXECUTION_PLAN_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        requested_target_count: max_targets.min(plan.targets.len()),
        selected_branch_count: branches.len(),
        missing_branch_count,
        skipped_candidate_count,
        branches,
    }
}

pub fn analyze_continuation_effect_v1(
    before_samples: &[LearningDecisionOutcomeSampleV1],
    after_samples: &[LearningDecisionOutcomeSampleV1],
) -> ContinuationEffectReportV1 {
    let before_groups = learning_samples_by_sibling_group_v1(before_samples);
    let after_groups = learning_samples_by_sibling_group_v1(after_samples);
    let before_censored_only_groups = before_groups
        .values()
        .filter(|group| learning_group_censored_only_v1(group))
        .count();
    let after_censored_only_groups = after_groups
        .values()
        .filter(|group| learning_group_censored_only_v1(group))
        .count();

    let mut common_groups = 0usize;
    let mut newly_terminal_groups = 0usize;
    let mut newly_terminal_observed_sibling_groups = 0usize;
    let mut still_censored_target_groups = 0usize;
    let mut expanded_target_groups = 0usize;
    let mut examples = Vec::new();

    for (group_id, before_group) in &before_groups {
        let Some(after_group) = after_groups.get(group_id) else {
            continue;
        };
        common_groups = common_groups.saturating_add(1);
        expanded_target_groups = expanded_target_groups.saturating_add(1);
        let before_summary = learning_group_effect_summary_v1(before_group);
        let after_summary = learning_group_effect_summary_v1(after_group);
        let before_terminal = learning_group_has_terminal_v1(before_group);
        let after_terminal = learning_group_has_terminal_v1(after_group);
        let after_observed_sibling = learning_group_observed_sibling_count_v1(after_group) > 1;
        if !before_terminal && after_terminal {
            newly_terminal_groups = newly_terminal_groups.saturating_add(1);
            if after_observed_sibling {
                newly_terminal_observed_sibling_groups =
                    newly_terminal_observed_sibling_groups.saturating_add(1);
            }
            if examples.len() < CONTINUATION_EFFECT_EXAMPLE_LIMIT {
                examples.push(ContinuationEffectExampleV1 {
                    sibling_group_id: group_id.clone(),
                    effect: if after_observed_sibling {
                        "new_terminal_observed_sibling".to_string()
                    } else {
                        "new_terminal_single_candidate".to_string()
                    },
                    before_summary,
                    after_summary,
                });
            }
        } else if learning_group_censored_only_v1(after_group) {
            still_censored_target_groups = still_censored_target_groups.saturating_add(1);
            if examples.len() < CONTINUATION_EFFECT_EXAMPLE_LIMIT {
                examples.push(ContinuationEffectExampleV1 {
                    sibling_group_id: group_id.clone(),
                    effect: "still_censored".to_string(),
                    before_summary,
                    after_summary,
                });
            }
        }
    }

    ContinuationEffectReportV1 {
        schema_name: CONTINUATION_EFFECT_REPORT_SCHEMA_NAME.to_string(),
        schema_version: CONTINUATION_EFFECT_REPORT_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        before_samples: before_samples.len(),
        after_samples: after_samples.len(),
        before_groups: before_groups.len(),
        after_groups: after_groups.len(),
        common_groups,
        before_censored_only_groups,
        after_censored_only_groups,
        censored_only_delta: after_censored_only_groups as isize
            - before_censored_only_groups as isize,
        newly_terminal_groups,
        newly_terminal_observed_sibling_groups,
        still_censored_target_groups,
        expanded_target_groups,
        examples,
    }
}

pub fn render_continuation_effect_report_v1(report: &ContinuationEffectReportV1) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "ContinuationEffectReportV1 before_samples={} after_samples={} before_groups={} after_groups={} common_groups={}",
        report.before_samples,
        report.after_samples,
        report.before_groups,
        report.after_groups,
        report.common_groups
    ));
    lines.push(format!(
        "Censoring: before_censored_only_groups={} after_censored_only_groups={} delta={}",
        report.before_censored_only_groups,
        report.after_censored_only_groups,
        report.censored_only_delta
    ));
    lines.push(format!(
        "Continuation effect: newly_terminal_groups={} newly_terminal_observed_sibling_groups={} still_censored_target_groups={} expanded_target_groups={}",
        report.newly_terminal_groups,
        report.newly_terminal_observed_sibling_groups,
        report.still_censored_target_groups,
        report.expanded_target_groups
    ));
    if !report.examples.is_empty() {
        lines.push("Examples:".to_string());
        for example in &report.examples {
            lines.push(format!(
                "  {} | effect={} before=[{}] after=[{}]",
                compact_learning_text_v1(&example.sibling_group_id, 72),
                example.effect,
                example.before_summary,
                example.after_summary
            ));
        }
    }
    lines.join("\n")
}

#[derive(Clone, Debug)]
struct LearningDecisionCandidateDraftV1 {
    record_index: usize,
    step_index: usize,
    candidate_id: String,
    sibling_group_id: String,
    parent_branch_id: String,
    candidate_command: String,
    candidate_choice_label: String,
}

fn decision_outcome_samples_from_drafts_v1(
    records: &[BranchOutcomeRecordV1],
    context: &LearningDatasetExportContextV1,
    drafts: Vec<LearningDecisionCandidateDraftV1>,
) -> Vec<LearningDecisionOutcomeSampleV1> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (draft_index, draft) in drafts.iter().enumerate() {
        groups
            .entry(draft.sibling_group_id.clone())
            .or_default()
            .push(draft_index);
    }

    drafts
        .iter()
        .enumerate()
        .map(|(draft_index, draft)| {
            decision_outcome_sample_from_draft_v1(
                records,
                context,
                &drafts,
                &groups,
                draft_index,
                draft,
            )
        })
        .collect()
}

fn journal_decision_candidate_drafts_v1(
    report: &BranchCampaignReportV1,
    records: &[BranchOutcomeRecordV1],
) -> Vec<LearningDecisionCandidateDraftV1> {
    let mut drafts = Vec::new();
    for event in &report.journal.events {
        let Some(decision_id) = journal_decision_id_v1(event) else {
            continue;
        };
        let candidates = journal_decision_candidates_v1(event);
        if candidates.is_empty() {
            continue;
        }
        let parent_commands = event.branch_commands.as_slice();
        let step_index = parent_commands.len();
        for candidate in candidates {
            for (record_index, record) in records.iter().enumerate() {
                if !record_commands_start_with_candidate_v1(
                    &record.commands,
                    parent_commands,
                    &candidate.command,
                ) {
                    continue;
                }
                drafts.push(LearningDecisionCandidateDraftV1 {
                    record_index,
                    step_index,
                    candidate_id: candidate.candidate_id.clone(),
                    sibling_group_id: format!(
                        "seed={}|domain={}:{}|decision={}",
                        report.seed,
                        report.run_domain.label,
                        report.run_domain.ascension_level,
                        decision_id
                    ),
                    parent_branch_id: event.branch_id.clone(),
                    candidate_command: candidate.command.clone(),
                    candidate_choice_label: candidate.label.clone(),
                });
            }
        }
    }
    drafts
}

fn journal_decision_id_v1(event: &CampaignJournalEventV1) -> Option<&str> {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { decision_id, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { decision_id, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { decision_id, .. } => {
            Some(decision_id)
        }
        CampaignJournalEventPayloadV1::RouteDecision { .. } => None,
    }
}

fn journal_decision_event_kind_v1(event: &CampaignJournalEventV1) -> &'static str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { .. } => "reward",
        CampaignJournalEventPayloadV1::ShopBranchCandidateSet { .. } => "shop_branch",
        CampaignJournalEventPayloadV1::ShopCandidatePool { .. } => "shop",
        CampaignJournalEventPayloadV1::CampfireCandidatePool { .. } => "campfire",
        CampaignJournalEventPayloadV1::EventCandidatePool { .. } => "event",
        CampaignJournalEventPayloadV1::BossRelicCandidatePool { .. } => "boss_relic",
        CampaignJournalEventPayloadV1::RouteCandidatePool { .. } => "route",
        CampaignJournalEventPayloadV1::RouteDecision { .. } => "route",
    }
}

fn coverage_gap_candidate_milestone_v1(event: &CampaignJournalEventV1) -> String {
    match &event.payload {
        CampaignJournalEventPayloadV1::BossRelicCandidatePool { .. } => {
            "next_act_pressure".to_string()
        }
        CampaignJournalEventPayloadV1::ShopBranchCandidateSet { .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { .. } => {
            "resource_conversion_frontier".to_string()
        }
        CampaignJournalEventPayloadV1::CampfireCandidatePool { .. } => {
            "upgrade_rest_mutation_frontier".to_string()
        }
        CampaignJournalEventPayloadV1::EventCandidatePool { .. } => {
            "event_resolution_frontier".to_string()
        }
        CampaignJournalEventPayloadV1::RewardCandidateSet { .. } => {
            "next_major_boundary".to_string()
        }
        CampaignJournalEventPayloadV1::RouteCandidatePool { .. } => "route_frontier".to_string(),
        CampaignJournalEventPayloadV1::RouteDecision { .. } => {
            "route_not_candidate_pool".to_string()
        }
    }
}

fn coverage_gap_target_origin_v1(
    event: &CampaignJournalEventV1,
    candidate_index: usize,
) -> CoverageGapContinuationTargetOriginV1 {
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            candidate_pool_provenance,
            map_decision_packet,
            route_candidates,
            ..
        } => {
            let route = map_decision_packet
                .as_ref()
                .and_then(|packet| {
                    coverage_gap_route_origin_from_map_packet_candidate_v1(
                        packet,
                        candidate_pool_provenance,
                        candidate_index,
                    )
                })
                .or_else(|| {
                    coverage_gap_route_origin_from_journal_candidate_v1(
                        route_candidates,
                        candidate_pool_provenance,
                        candidate_index,
                    )
                });
            CoverageGapContinuationTargetOriginV1 {
                source: coverage_gap_route_origin_source_v1(
                    map_decision_packet.as_ref(),
                    route_candidates,
                    candidate_index,
                    route.is_some(),
                ),
                route,
            }
        }
        _ => CoverageGapContinuationTargetOriginV1 {
            source: "journal_candidate".to_string(),
            route: None,
        },
    }
}

fn coverage_gap_route_origin_source_v1(
    map_decision_packet: Option<&crate::ai::route_planner_v1::MapDecisionPacketV1>,
    route_candidates: &[CampaignJournalRouteCandidateV1],
    candidate_index: usize,
    route_available: bool,
) -> String {
    if !route_available {
        return "route_candidate_pool".to_string();
    }
    if map_decision_packet
        .and_then(|packet| packet.candidates.get(candidate_index))
        .is_some()
    {
        "map_decision_packet".to_string()
    } else if route_candidates.get(candidate_index).is_some() {
        "route_candidate_pool".to_string()
    } else {
        "route_candidate_pool".to_string()
    }
}

fn coverage_gap_route_origin_from_map_packet_candidate_v1(
    packet: &crate::ai::route_planner_v1::MapDecisionPacketV1,
    candidate_pool_provenance: &Option<crate::ai::route_planner_v1::RouteCandidatePoolProvenanceV1>,
    candidate_index: usize,
) -> Option<CoverageGapRouteTargetOriginV1> {
    let candidate = packet.candidates.get(candidate_index)?;
    let pool = candidate_pool_provenance
        .as_ref()
        .unwrap_or(&packet.candidate_pool);
    let path = &candidate.projection.path_summary;
    Some(CoverageGapRouteTargetOriginV1 {
        legal_candidate_count: pool.legal_candidate_count,
        emitted_candidate_count: pool.emitted_candidate_count,
        complete_legal_pool: pool.complete_legal_pool,
        ordering: format!("{:?}", pool.ordering),
        target_x: candidate.target.x,
        target_y: candidate.target.y,
        room_type: candidate
            .target
            .room_type
            .map(|room| format!("{:?}", room))
            .unwrap_or_else(|| "Unknown".to_string()),
        move_kind: format!("{:?}", candidate.target.move_kind),
        action_kind: coverage_gap_route_action_kind_v1(&candidate.action),
        projection_source: format!("{:?}", candidate.projection.metadata.source),
        projection_coverage: format!("{:?}", candidate.projection.metadata.coverage),
        path_budget: candidate.projection.metadata.path_budget,
        observed_path_count: candidate.projection.metadata.observed_path_count,
        path: coverage_gap_route_path_origin_v1(path),
        first_elite: coverage_gap_route_first_elite_origin_from_segment_v1(&path.first_elite),
    })
}

fn coverage_gap_route_origin_from_journal_candidate_v1(
    route_candidates: &[CampaignJournalRouteCandidateV1],
    candidate_pool_provenance: &Option<crate::ai::route_planner_v1::RouteCandidatePoolProvenanceV1>,
    candidate_index: usize,
) -> Option<CoverageGapRouteTargetOriginV1> {
    let candidate = route_candidates.get(candidate_index)?;
    let target = candidate.target_node.as_ref()?;
    let path = candidate.path_summary.as_ref()?;
    Some(CoverageGapRouteTargetOriginV1 {
        legal_candidate_count: candidate_pool_provenance
            .as_ref()
            .map(|pool| pool.legal_candidate_count)
            .unwrap_or(route_candidates.len()),
        emitted_candidate_count: candidate_pool_provenance
            .as_ref()
            .map(|pool| pool.emitted_candidate_count)
            .unwrap_or(route_candidates.len()),
        complete_legal_pool: candidate_pool_provenance
            .as_ref()
            .map(|pool| pool.complete_legal_pool)
            .unwrap_or(false),
        ordering: candidate_pool_provenance
            .as_ref()
            .map(|pool| format!("{:?}", pool.ordering))
            .unwrap_or_else(|| "Unknown".to_string()),
        target_x: target.x,
        target_y: target.y,
        room_type: target
            .room_type
            .map(|room| format!("{:?}", room))
            .unwrap_or_else(|| candidate.room_type.clone()),
        move_kind: format!("{:?}", target.move_kind),
        action_kind: candidate
            .action
            .as_ref()
            .map(coverage_gap_route_action_kind_v1)
            .unwrap_or_else(|| "unknown".to_string()),
        projection_source: candidate
            .projection_source
            .map(|source| format!("{:?}", source))
            .unwrap_or_else(|| "Unknown".to_string()),
        projection_coverage: candidate
            .projection_coverage
            .map(|coverage| format!("{:?}", coverage))
            .unwrap_or_else(|| "Unknown".to_string()),
        path_budget: candidate.path_budget.unwrap_or(0),
        observed_path_count: candidate.observed_path_count.unwrap_or(path.path_count),
        path: coverage_gap_route_path_origin_v1(path),
        first_elite: coverage_gap_route_first_elite_origin_from_evidence_v1(&candidate.first_elite),
    })
}

fn coverage_gap_route_path_origin_v1(
    path: &crate::ai::route_planner_v1::RoutePathSummaryV1,
) -> CoverageGapRoutePathOriginV1 {
    CoverageGapRoutePathOriginV1 {
        path_count: path.path_count,
        path_budget_exhausted: path.path_budget_exhausted,
        min_early_pressure: path.min_early_pressure,
        max_early_pressure: path.max_early_pressure,
        min_elites: path.min_elites,
        max_elites: path.max_elites,
        min_shops: path.min_shops,
        max_shops: path.max_shops,
        min_fires: path.min_fires,
        max_fires: path.max_fires,
        min_unknowns: path.min_unknowns,
        max_unknowns: path.max_unknowns,
        min_treasures: path.min_treasures,
        max_treasures: path.max_treasures,
        first_shop_floor: path.first_shop_floor,
        first_fire_floor: path.first_fire_floor,
        min_damage_rooms_before_recovery: path.min_damage_rooms_before_recovery,
        max_damage_rooms_before_recovery: path.max_damage_rooms_before_recovery,
        min_unknowns_before_recovery: path.min_unknowns_before_recovery,
        max_unknowns_before_recovery: path.max_unknowns_before_recovery,
        paths_with_recovery_before_damage: path.paths_with_recovery_before_damage,
    }
}

fn coverage_gap_route_first_elite_origin_from_segment_v1(
    first_elite: &crate::ai::route_planner_v1::RouteFirstEliteSegmentV1,
) -> CoverageGapRouteFirstEliteOriginV1 {
    CoverageGapRouteFirstEliteOriginV1 {
        paths_with_first_elite: first_elite.paths_with_first_elite,
        forced: first_elite.forced,
        optional: first_elite.optional,
        min_hallway_fights_before: first_elite.min_hallway_fights_before,
        max_hallway_fights_before: first_elite.max_hallway_fights_before,
        min_unknowns_before: first_elite.min_unknowns_before,
        max_unknowns_before: first_elite.max_unknowns_before,
        min_fires_before: first_elite.min_fires_before,
        max_fires_before: first_elite.max_fires_before,
        min_shops_before: first_elite.min_shops_before,
        max_shops_before: first_elite.max_shops_before,
        can_bail_to_rest_before: first_elite.can_bail_to_rest_before,
        can_bail_to_shop_before: first_elite.can_bail_to_shop_before,
    }
}

fn coverage_gap_route_first_elite_origin_from_evidence_v1(
    first_elite: &crate::eval::branch_experiment::BranchExperimentFirstEliteEvidenceV1,
) -> CoverageGapRouteFirstEliteOriginV1 {
    CoverageGapRouteFirstEliteOriginV1 {
        paths_with_first_elite: first_elite.paths_with_first_elite,
        forced: first_elite.forced,
        optional: first_elite.optional,
        min_hallway_fights_before: first_elite.min_hallway_fights_before,
        max_hallway_fights_before: first_elite.max_hallway_fights_before,
        min_unknowns_before: first_elite.min_unknowns_before,
        max_unknowns_before: first_elite.max_unknowns_before,
        min_fires_before: first_elite.min_fires_before,
        max_fires_before: first_elite.max_fires_before,
        min_shops_before: first_elite.min_shops_before,
        max_shops_before: first_elite.max_shops_before,
        can_bail_to_rest_before: first_elite.can_bail_to_rest_before,
        can_bail_to_shop_before: first_elite.can_bail_to_shop_before,
    }
}

fn coverage_gap_route_action_kind_v1(
    action: &crate::ai::route_planner_v1::RouteMapActionV1,
) -> String {
    match action {
        crate::ai::route_planner_v1::RouteMapActionV1::Go { .. } => "go",
        crate::ai::route_planner_v1::RouteMapActionV1::Fly { .. } => "fly",
    }
    .to_string()
}

fn journal_decision_candidates_v1(event: &CampaignJournalEventV1) -> &[CampaignJournalCandidateV1] {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { candidates, .. } => candidates,
        CampaignJournalEventPayloadV1::RouteDecision { .. } => &[],
    }
}

fn record_commands_start_with_candidate_v1(
    record_commands: &[String],
    parent_commands: &[String],
    candidate_command: &str,
) -> bool {
    record_commands.len() > parent_commands.len()
        && record_commands.starts_with(parent_commands)
        && record_commands[parent_commands.len()] == candidate_command
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
            draft.sibling_group_id, draft.candidate_id, record.branch_id
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
                first.candidate_id == draft.candidate_id
                    && first.candidate_command == draft.candidate_command
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

fn learning_samples_by_sibling_group_v1(
    samples: &[LearningDecisionOutcomeSampleV1],
) -> BTreeMap<String, Vec<&LearningDecisionOutcomeSampleV1>> {
    let mut groups = BTreeMap::<String, Vec<&LearningDecisionOutcomeSampleV1>>::new();
    for sample in samples {
        groups
            .entry(continuation_effect_group_key_v1(&sample.sibling_group_id))
            .or_default()
            .push(sample);
    }
    groups
}

fn continuation_effect_group_key_v1(sibling_group_id: &str) -> String {
    let parts = sibling_group_id
        .split('|')
        .filter(|part| !part.starts_with("rounds="))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        sibling_group_id.to_string()
    } else {
        parts.join("|")
    }
}

fn learning_group_censored_only_v1(group: &[&LearningDecisionOutcomeSampleV1]) -> bool {
    !group.is_empty()
        && group.iter().all(|sample| {
            sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::CensoredOngoing
        })
}

fn learning_group_has_terminal_v1(group: &[&LearningDecisionOutcomeSampleV1]) -> bool {
    group.iter().any(|sample| {
        sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::TerminalOutcome
    })
}

fn learning_group_observed_sibling_count_v1(group: &[&LearningDecisionOutcomeSampleV1]) -> usize {
    group
        .iter()
        .map(|sample| sample.observed_sibling_count)
        .max()
        .unwrap_or(0)
}

fn learning_group_effect_summary_v1(group: &[&LearningDecisionOutcomeSampleV1]) -> String {
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut outcome_counts = BTreeMap::<String, usize>::new();
    for sample in group {
        *status_counts
            .entry(format!("{:?}", sample.outcome.supervision_status))
            .or_default() += 1;
        *outcome_counts
            .entry(format!("{:?}", sample.outcome.outcome_class))
            .or_default() += 1;
    }
    format!(
        "samples={} siblings={} status={} outcome={}",
        group.len(),
        learning_group_observed_sibling_count_v1(group),
        render_learning_histogram_v1(&learning_histogram_entries_by_key_v1(status_counts)),
        render_learning_histogram_v1(&learning_histogram_entries_by_key_v1(outcome_counts))
    )
}

fn representative_decision_sample_v1<'a>(
    group_samples: &[&'a LearningDecisionOutcomeSampleV1],
) -> &'a LearningDecisionOutcomeSampleV1 {
    group_samples
        .iter()
        .max_by_key(|sample| sample.observed_sibling_count)
        .copied()
        .unwrap_or_else(|| group_samples[0])
}

fn command_family_v1(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn decision_group_outcome_class_counts_v1(
    group_samples: &[&LearningDecisionOutcomeSampleV1],
) -> Vec<LearningDecisionHistogramEntryV1> {
    let mut counts = BTreeMap::<String, usize>::new();
    for sample in group_samples {
        *counts
            .entry(format!("{:?}", sample.outcome.outcome_class))
            .or_default() += 1;
    }
    learning_histogram_entries_by_key_v1(counts)
}

fn learning_candidate_summary_v1(candidate: &LearningSiblingCandidateV1) -> String {
    format!(
        "{} | best={:?} rank={} observed={}",
        candidate.choice_label,
        candidate.best_outcome_class,
        candidate.best_rank_key,
        candidate.observed_branch_count
    )
}

fn learning_histogram_entries_by_key_v1(
    counts: BTreeMap<String, usize>,
) -> Vec<LearningDecisionHistogramEntryV1> {
    counts
        .into_iter()
        .map(|(key, count)| LearningDecisionHistogramEntryV1 { key, count })
        .collect()
}

fn render_learning_histogram_v1(entries: &[LearningDecisionHistogramEntryV1]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.key, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn push_readiness_bottleneck_v1(
    bottlenecks: &mut Vec<LearningReadinessBottleneckV1>,
    key: &str,
    group_count: usize,
    sample_count: usize,
    next_action: &str,
) {
    if group_count == 0 {
        return;
    }
    bottlenecks.push(LearningReadinessBottleneckV1 {
        key: key.to_string(),
        group_count,
        sample_count,
        next_action: next_action.to_string(),
    });
}

fn group_observed_sibling_count_v1(group_samples: &[&LearningDecisionOutcomeSampleV1]) -> usize {
    group_samples
        .iter()
        .map(|sample| sample.observed_sibling_count)
        .max()
        .unwrap_or(0)
}

fn sample_looks_scheduling_censored_v1(sample: &LearningDecisionOutcomeSampleV1) -> bool {
    if sample.outcome.supervision_status != BranchOutcomeSupervisionStatusV1::CensoredOngoing {
        return false;
    }
    matches!(
        sample.outcome.outcome_class,
        BranchOutcomeClassV1::OngoingActive | BranchOutcomeClassV1::OngoingFrozen
    ) || matches!(
        sample.outcome.report_stop_reason.as_str(),
        "max_rounds" | "victory_found"
    )
}

fn sample_looks_combat_unresolved_v1(sample: &LearningDecisionOutcomeSampleV1) -> bool {
    if sample.outcome.frontier_title != "Combat" {
        return false;
    }
    matches!(
        sample.outcome.outcome_class,
        BranchOutcomeClassV1::Abandoned | BranchOutcomeClassV1::Stuck
    ) || sample
        .outcome
        .stop_reason
        .to_ascii_lowercase()
        .contains("combat")
}

fn sample_missing_context_v1(sample: &LearningDecisionOutcomeSampleV1) -> bool {
    !sample.outcome.checkpoint_enriched || sample.outcome.state_features.is_none()
}

fn targeted_continuation_target_v1(
    group_samples: &[&LearningDecisionOutcomeSampleV1],
) -> Option<TargetedContinuationTargetV1> {
    if group_samples.is_empty() {
        return None;
    }
    let representative = representative_decision_sample_v1(group_samples);
    if representative.observed_sibling_count <= 1 {
        return None;
    }
    let has_terminal = group_samples.iter().any(|sample| {
        sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::TerminalOutcome
    });
    let has_censored = group_samples.iter().any(|sample| {
        sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::CensoredOngoing
    });
    let censored_only = group_samples.iter().all(|sample| {
        sample.outcome.supervision_status == BranchOutcomeSupervisionStatusV1::CensoredOngoing
    });
    let mut reason_keys = Vec::new();
    let priority_bucket = if has_terminal && has_censored {
        reason_keys.push("partial_terminal_siblings".to_string());
        300
    } else if censored_only {
        reason_keys.push("censored_sibling_group".to_string());
        200
    } else if decision_group_outcome_class_counts_v1(group_samples).len() > 1 {
        reason_keys.push("outcome_divergent_siblings".to_string());
        100
    } else {
        return None;
    };

    let candidates = representative
        .sibling_candidates
        .iter()
        .map(targeted_continuation_candidate_v1)
        .collect::<Vec<_>>();
    if candidates
        .iter()
        .all(|candidate| !candidate.needs_continuation)
    {
        return None;
    }

    Some(TargetedContinuationTargetV1 {
        sibling_group_id: representative.sibling_group_id.clone(),
        parent_branch_id: representative.parent_branch_id.clone(),
        step_index: representative.step_index,
        command_family: command_family_v1(&representative.candidate_command),
        priority_bucket,
        reason_keys,
        milestone: "next_major_milestone_or_terminal".to_string(),
        candidates,
    })
}

fn targeted_continuation_candidate_v1(
    candidate: &LearningSiblingCandidateV1,
) -> TargetedContinuationCandidateV1 {
    TargetedContinuationCandidateV1 {
        command: candidate.command.clone(),
        choice_label: candidate.choice_label.clone(),
        representative_branch_id: candidate.representative_branch_id.clone(),
        representative_branch_group: candidate.representative_branch_group.clone(),
        observed_branch_count: candidate.observed_branch_count,
        best_outcome_class: candidate.best_outcome_class.clone(),
        best_supervision_status: candidate.best_supervision_status.clone(),
        best_rank_key: candidate.best_rank_key,
        needs_continuation: candidate.best_supervision_status
            == BranchOutcomeSupervisionStatusV1::CensoredOngoing,
    }
}

fn targeted_continuation_report_branch_ids_v1(
    report: &BranchCampaignReportV1,
) -> BTreeMap<String, ()> {
    report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .map(|branch| (branch.branch_id.clone(), ()))
        .collect()
}

fn targeted_continuation_candidate_summary_v1(
    candidate: &TargetedContinuationCandidateV1,
) -> String {
    format!(
        "{}:{}:{}:r{}",
        if candidate.needs_continuation {
            "continue"
        } else {
            "observed"
        },
        compact_learning_text_v1(&candidate.choice_label, 36),
        targeted_continuation_outcome_label_v1(&candidate.best_outcome_class),
        candidate.best_rank_key
    )
}

fn targeted_continuation_outcome_label_v1(outcome: &BranchOutcomeClassV1) -> &'static str {
    match outcome {
        BranchOutcomeClassV1::OngoingActive => "active",
        BranchOutcomeClassV1::OngoingFrozen => "frozen",
        BranchOutcomeClassV1::TerminalVictory => "win",
        BranchOutcomeClassV1::TerminalDefeat => "loss",
        BranchOutcomeClassV1::Abandoned => "abandoned",
        BranchOutcomeClassV1::Stuck => "stuck",
    }
}

fn compact_learning_text_v1(text: &str, max_len: usize) -> String {
    if text.len() <= max_len || max_len < 12 {
        return text.to_string();
    }
    let head_len = (max_len - 3) / 2;
    let tail_len = max_len - 3 - head_len;
    format!("{}...{}", &text[..head_len], &text[text.len() - tail_len..])
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
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
        BranchCampaignRunDomainV1,
    };
    use crate::eval::branch_outcome_dataset_v1::{
        BranchOutcomeClassV1, BranchOutcomeDeckFeaturesV1, BranchOutcomeFormationFeaturesV1,
        BranchOutcomeRecordV1, BranchOutcomeStartupFeaturesV1, BranchOutcomeStateFeaturesV1,
        BranchOutcomeSupervisionStatusV1, BRANCH_OUTCOME_RECORD_SCHEMA_NAME,
        BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
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
    fn decision_outcome_samples_use_journal_decision_identity_when_available() {
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

        let records = vec![clothesline, shrug];
        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-reward0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "journal-reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "reward-frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 2,
                selected_count: 2,
                candidates: vec![
                    sample_journal_candidate("rp 0", "Clothesline"),
                    sample_journal_candidate("rp 1", "Shrug It Off"),
                ],
            },
        });

        let samples = decision_outcome_samples_from_campaign_report_v1(
            &report,
            &records,
            LearningDatasetExportContextV1::default(),
        );

        assert_eq!(samples.len(), 2);
        assert!(samples
            .iter()
            .all(|sample| sample.sibling_group_id.contains("decision=journal-reward0")));
        assert_eq!(samples[0].parent_branch_id, "root");
        assert_eq!(
            samples[0].candidate_set_status,
            LearningCandidateSetStatusV1::ObservedSiblings
        );
        assert_eq!(samples[0].observed_sibling_count, 2);
    }

    #[test]
    fn journal_candidate_coverage_reports_uncontinued_candidates() {
        let mut clothesline = sample_branch_outcome_record();
        clothesline.branch_id = "root.rp 0".to_string();
        clothesline.commands = vec!["rp 0".to_string()];
        clothesline.choice_labels = vec!["Clothesline".to_string()];

        let mut shrug = sample_branch_outcome_record();
        shrug.branch_index = 1;
        shrug.branch_id = "root.rp 1".to_string();
        shrug.commands = vec!["rp 1".to_string()];
        shrug.choice_labels = vec!["Shrug It Off".to_string()];

        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-reward0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "journal-reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "reward-frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 3,
                selected_count: 2,
                candidates: vec![
                    sample_journal_candidate("rp 0", "Clothesline"),
                    sample_journal_candidate("rp 1", "Shrug It Off"),
                    sample_journal_candidate("rp 2", "Carnage"),
                ],
            },
        });
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-route0:route".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Map".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RouteDecision {
                decision_id: "route0".to_string(),
                route_branch_id: "root:route".to_string(),
                selected_index: None,
                selected_candidate_id: None,
                selected_candidate_rank: None,
                selected_target_node: None,
                selected_route_candidate: None,
                target: "x=1 Monster".to_string(),
                move_kind: "Monster".to_string(),
                safety_flag: None,
                safety: "ok".to_string(),
                candidate_pool_provenance: None,
                command: "go 1".to_string(),
                elite_prep_bp: 50,
                first_elite: Default::default(),
            },
        });

        let coverage =
            analyze_journal_decision_candidate_coverage_v1(&report, &[clothesline, shrug]);
        let rendered = render_journal_decision_candidate_coverage_v1(&coverage);

        assert_eq!(coverage.total_decisions, 1);
        assert_eq!(coverage.total_candidates, 3);
        assert_eq!(coverage.observed_candidates, 2);
        assert_eq!(coverage.unobserved_candidates, 1);
        assert_eq!(coverage.partially_observed_decisions, 1);
        assert!(rendered.contains("Carnage"));
    }

    #[test]
    fn coverage_gap_continuation_plan_targets_unobserved_journal_candidates() {
        let mut clothesline = sample_branch_outcome_record();
        clothesline.branch_id = "root.rp 0".to_string();
        clothesline.commands = vec!["rp 0".to_string()];
        clothesline.choice_labels = vec!["Clothesline".to_string()];

        let mut shrug = sample_branch_outcome_record();
        shrug.branch_index = 1;
        shrug.branch_id = "root.rp 1".to_string();
        shrug.commands = vec!["rp 1".to_string()];
        shrug.choice_labels = vec!["Shrug It Off".to_string()];

        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-reward0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "journal-reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "reward-frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 3,
                selected_count: 2,
                candidates: vec![
                    sample_journal_candidate("rp 0", "Clothesline"),
                    sample_journal_candidate("rp 1", "Shrug It Off"),
                    sample_journal_candidate("rp 2", "Carnage"),
                ],
            },
        });
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-route0:route".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Map".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RouteDecision {
                decision_id: "route0".to_string(),
                route_branch_id: "root:route".to_string(),
                selected_index: None,
                selected_candidate_id: None,
                selected_candidate_rank: None,
                selected_target_node: None,
                selected_route_candidate: None,
                target: "x=1 Monster".to_string(),
                move_kind: "Monster".to_string(),
                safety_flag: None,
                safety: "ok".to_string(),
                candidate_pool_provenance: None,
                command: "go 1".to_string(),
                elite_prep_bp: 50,
                first_elite: Default::default(),
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[clothesline, shrug], 8, 2);
        let rendered = render_coverage_gap_continuation_plan_v1(&plan);

        assert_eq!(plan.total_decisions, 1);
        assert_eq!(plan.total_unobserved_candidates, 1);
        assert_eq!(plan.selected_target_count, 1);
        assert_eq!(plan.targets[0].decision_id, "journal-reward0");
        assert_eq!(plan.targets[0].command, "rp 2");
        assert_eq!(plan.targets[0].label, "Carnage");
        assert_eq!(plan.targets[0].candidate_index, 2);
        assert_eq!(plan.targets[0].parent_commands, Vec::<String>::new());
        assert!(rendered.contains("Carnage"));
        assert!(!rendered.contains("route0"));
    }

    #[test]
    fn coverage_gap_continuation_plan_targets_unobserved_route_candidates() {
        let mut run = crate::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = crate::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &crate::state::core::EngineState::MapNavigation,
            crate::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet =
            crate::ai::route_planner_v1::MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(packet.candidates.len() >= 2);
        let observed_route = &packet.candidates[0];
        let unobserved_route = &packet.candidates[1];
        let mut route_one = sample_branch_outcome_record();
        route_one.branch_id = format!("root.{}", observed_route.command);
        route_one.commands = vec![observed_route.command.clone()];
        route_one.choice_labels = vec![route_candidate_test_label_v1(observed_route)];

        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-route-pool0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Map".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                decision_id: "journal-route-pool0".to_string(),
                boundary_title: "Map".to_string(),
                frontier_key: "map-frontier".to_string(),
                depth: 0,
                candidate_count: packet.candidates.len(),
                selected_index: Some(0),
                candidate_pool_provenance: Some(packet.candidate_pool.clone()),
                map_decision_packet: Some(packet.clone()),
                route_candidates: packet
                    .candidates
                    .iter()
                    .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                    .collect(),
                candidates: packet
                    .candidates
                    .iter()
                    .map(|candidate| {
                        sample_journal_candidate(
                            &candidate.command,
                            &route_candidate_test_label_v1(candidate),
                        )
                    })
                    .collect(),
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[route_one], 8, 2);
        let rendered = render_coverage_gap_continuation_plan_v1(&plan);

        assert_eq!(plan.total_decisions, 1);
        assert_eq!(plan.total_candidates, packet.candidates.len());
        assert_eq!(
            plan.total_unobserved_candidates,
            packet.candidates.len() - 1
        );
        assert_eq!(plan.targets[0].decision_id, "journal-route-pool0");
        assert_eq!(plan.targets[0].command, unobserved_route.command);
        assert_eq!(
            plan.targets[0].label,
            route_candidate_test_label_v1(unobserved_route)
        );
        assert_eq!(plan.targets[0].milestone, "route_frontier");
        assert_eq!(plan.targets[0].target_origin.source, "map_decision_packet");
        let route_origin = plan.targets[0]
            .target_origin
            .route
            .as_ref()
            .expect("route candidate should carry typed route origin");
        assert_eq!(route_origin.target_x, unobserved_route.target.x);
        assert_eq!(route_origin.target_y, unobserved_route.target.y);
        assert_eq!(
            route_origin.observed_path_count,
            unobserved_route.projection.metadata.observed_path_count
        );
        assert!(rendered.contains("origin=map_decision_packet"));
    }

    #[test]
    fn coverage_gap_continuation_uses_typed_route_candidates_without_map_packet() {
        let mut run = crate::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = crate::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &crate::state::core::EngineState::MapNavigation,
            crate::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet =
            crate::ai::route_planner_v1::MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(packet.candidates.len() >= 2);
        let observed_route = &packet.candidates[0];
        let unobserved_route = &packet.candidates[1];
        let mut route_one = sample_branch_outcome_record();
        route_one.branch_id = format!("root.{}", observed_route.command);
        route_one.commands = vec![observed_route.command.clone()];
        route_one.choice_labels = vec![route_candidate_test_label_v1(observed_route)];

        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-route-pool0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Map".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                decision_id: "journal-route-pool0".to_string(),
                boundary_title: "Map".to_string(),
                frontier_key: "map-frontier".to_string(),
                depth: 0,
                candidate_count: packet.candidates.len(),
                selected_index: Some(0),
                candidate_pool_provenance: Some(packet.candidate_pool.clone()),
                map_decision_packet: None,
                route_candidates: packet
                    .candidates
                    .iter()
                    .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                    .collect(),
                candidates: packet
                    .candidates
                    .iter()
                    .map(|candidate| {
                        sample_journal_candidate(
                            &candidate.command,
                            &route_candidate_test_label_v1(candidate),
                        )
                    })
                    .collect(),
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[route_one], 8, 2);

        assert_eq!(plan.targets[0].target_origin.source, "route_candidate_pool");
        let route_origin = plan.targets[0]
            .target_origin
            .route
            .as_ref()
            .expect("typed route candidates should recover route origin");
        assert_eq!(route_origin.target_x, unobserved_route.target.x);
        assert_eq!(route_origin.target_y, unobserved_route.target.y);
        assert_eq!(
            route_origin.path.min_elites,
            unobserved_route.projection.path_summary.min_elites
        );
        assert_eq!(
            route_origin.first_elite.paths_with_first_elite,
            unobserved_route
                .projection
                .path_summary
                .first_elite
                .paths_with_first_elite
        );
    }

    #[test]
    fn coverage_gap_continuation_balances_targets_across_decision_types() {
        let mut report = sample_campaign_report_with_branches(Vec::new());
        for reward_index in 0..3 {
            report.journal.events.push(CampaignJournalEventV1 {
                event_id: format!("journal-reward{reward_index}:candidate_set"),
                round: 1,
                branch_id: "root".to_string(),
                branch_index: 0,
                branch_frontier_title: "Reward Screen".to_string(),
                act: 1,
                floor: 1,
                branch_choices: Vec::new(),
                branch_commands: Vec::new(),
                combat_budget_retry_used: false,
                payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                    decision_id: format!("journal-reward{reward_index}"),
                    boundary_title: "Reward Screen".to_string(),
                    frontier_key: "reward-frontier".to_string(),
                    depth: 0,
                    max_reward_options_per_branch: 3,
                    original_count: 3,
                    selected_count: 2,
                    candidates: vec![
                        sample_journal_candidate("rp 0", "Reward A"),
                        sample_journal_candidate("rp 1", "Reward B"),
                        sample_journal_candidate("rp 2", "Reward C"),
                    ],
                },
            });
        }
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-route-pool0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Map".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: vec!["__route_decision:0:go_1".to_string()],
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                decision_id: "journal-route-pool0".to_string(),
                boundary_title: "Map".to_string(),
                frontier_key: "map-frontier".to_string(),
                depth: 0,
                candidate_count: 2,
                selected_index: Some(0),
                candidate_pool_provenance: None,
                map_decision_packet: None,
                route_candidates: Vec::new(),
                candidates: vec![
                    sample_journal_candidate("go 1", "Route selected"),
                    sample_journal_candidate("go 3", "Route alternative"),
                ],
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[], 2, 2);
        let event_types = plan
            .targets
            .iter()
            .map(|target| target.event_type.as_str())
            .collect::<Vec<_>>();

        assert_eq!(plan.selected_target_count, 2);
        assert!(event_types.contains(&"route"));
        assert!(event_types.contains(&"reward"));
    }

    #[test]
    fn coverage_gap_continuation_dedupes_repeated_target_coordinates() {
        let mut report = sample_campaign_report_with_branches(Vec::new());
        for event_index in 0..2 {
            report.journal.events.push(CampaignJournalEventV1 {
                event_id: format!("journal-route-pool{event_index}:candidate_set"),
                round: 1,
                branch_id: "root.rp 1".to_string(),
                branch_index: 0,
                branch_frontier_title: "Map".to_string(),
                act: 1,
                floor: 2,
                branch_choices: vec!["Seeing Red".to_string()],
                branch_commands: vec!["rp 1".to_string(), "__route_decision:0:go_2".to_string()],
                combat_budget_retry_used: false,
                payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                    decision_id: format!("journal-route-pool{event_index}"),
                    boundary_title: "Map".to_string(),
                    frontier_key: "map-frontier".to_string(),
                    depth: 0,
                    candidate_count: 1,
                    selected_index: None,
                    candidate_pool_provenance: None,
                    map_decision_packet: None,
                    route_candidates: Vec::new(),
                    candidates: vec![sample_journal_candidate("go 3", "Route duplicate")],
                },
            });
        }

        let plan = plan_coverage_gap_continuations_v1(&report, &[], 8, 2);

        assert_eq!(plan.total_unobserved_candidates, 2);
        assert_eq!(plan.selected_target_count, 1);
        assert_eq!(plan.targets[0].command, "go 3");
        assert_eq!(
            plan.targets[0].parent_commands,
            vec!["rp 1".to_string(), "__route_decision:0:go_2".to_string()]
        );
    }

    #[test]
    fn coverage_gap_continuation_prioritizes_kept_candidates_before_pruned_candidates() {
        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-reward0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "journal-reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "reward-frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 3,
                selected_count: 1,
                candidates: vec![
                    sample_pruned_journal_candidate("rp 0", "Pruned first"),
                    sample_journal_candidate("rp 1", "Kept second"),
                    sample_pruned_journal_candidate("rp 2", "Pruned third"),
                ],
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[], 1, 1);
        let rendered = render_coverage_gap_continuation_plan_v1(&plan);

        assert_eq!(plan.total_unobserved_candidates, 3);
        assert_eq!(plan.kept_unobserved_candidates, 1);
        assert_eq!(plan.pruned_unobserved_candidates, 2);
        assert_eq!(plan.selected_kept_targets, 1);
        assert_eq!(plan.selected_pruned_targets, 0);
        assert_eq!(plan.targets[0].command, "rp 1");
        assert_eq!(
            plan.targets[0].disposition,
            CampaignJournalCandidateDispositionV1::Kept
        );
        assert!(rendered.contains("kept_unobserved=1"));
        assert!(rendered.contains("pruned_unobserved=2"));
        assert!(rendered.contains("disposition=kept"));
    }

    #[test]
    fn coverage_gap_continuation_uses_admission_trace_before_legacy_disposition() {
        let mut report = sample_campaign_report_with_branches(Vec::new());
        report.journal.events.push(CampaignJournalEventV1 {
            event_id: "journal-reward0:candidate_set".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: Vec::new(),
            combat_budget_retry_used: false,
            payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "journal-reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "reward-frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 2,
                selected_count: 1,
                candidates: vec![
                    sample_journal_candidate_with_admission(
                        "rp 0",
                        "Legacy kept but deferred",
                        CampaignJournalCandidateDispositionV1::Kept,
                        CampaignJournalCandidateAdmissionStatusV1::Deferred,
                    ),
                    sample_journal_candidate_with_admission(
                        "rp 1",
                        "Legacy pruned but scheduled",
                        CampaignJournalCandidateDispositionV1::Pruned,
                        CampaignJournalCandidateAdmissionStatusV1::Scheduled,
                    ),
                ],
            },
        });

        let plan = plan_coverage_gap_continuations_v1(&report, &[], 1, 1);
        let rendered = render_coverage_gap_continuation_plan_v1(&plan);

        assert_eq!(plan.scheduled_unobserved_candidates, 1);
        assert_eq!(plan.unscheduled_unobserved_candidates, 1);
        assert_eq!(plan.selected_scheduled_targets, 1);
        assert_eq!(plan.selected_unscheduled_targets, 0);
        assert_eq!(plan.targets[0].command, "rp 1");
        assert_eq!(
            plan.targets[0].admission.status,
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        );
        assert!(rendered.contains("admission=scheduled"));
        assert!(rendered.contains("reason_category=unknown"));
        assert!(rendered.contains("reason_code=unknown"));
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

    #[test]
    fn decision_outcome_analysis_identifies_useful_sibling_groups() {
        let mut win = sample_decision_outcome_sample("rp 0", "Clothesline");
        win.sibling_group_id = "group-a".to_string();
        win.outcome.outcome_class = BranchOutcomeClassV1::TerminalVictory;
        win.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::TerminalOutcome;
        win.sibling_candidates = vec![
            sample_sibling_candidate("rp 0", "Clothesline", BranchOutcomeClassV1::TerminalVictory),
            sample_sibling_candidate("rp 1", "Shrug It Off", BranchOutcomeClassV1::Abandoned),
        ];
        win.observed_sibling_count = win.sibling_candidates.len();

        let mut abandoned = sample_decision_outcome_sample("rp 1", "Shrug It Off");
        abandoned.sibling_group_id = "group-a".to_string();
        abandoned.outcome.outcome_class = BranchOutcomeClassV1::Abandoned;
        abandoned.outcome.supervision_status =
            BranchOutcomeSupervisionStatusV1::InterventionOrFailure;
        abandoned.sibling_candidates = win.sibling_candidates.clone();
        abandoned.observed_sibling_count = abandoned.sibling_candidates.len();

        let analysis = analyze_learning_decision_outcome_samples_v1(&[win, abandoned]);

        assert_eq!(analysis.total_samples, 2);
        assert_eq!(analysis.decision_group_count, 1);
        assert_eq!(analysis.observed_sibling_group_count, 1);
        assert_eq!(analysis.outcome_divergent_group_count, 1);
        assert_eq!(
            analysis.command_family_counts,
            vec![LearningDecisionHistogramEntryV1 {
                key: "rp".to_string(),
                count: 1
            }]
        );
        assert_eq!(analysis.group_examples.len(), 1);
        assert_eq!(analysis.group_examples[0].sibling_group_id, "group-a");
        assert_eq!(
            analysis.group_examples[0].candidate_summaries,
            vec![
                "Clothesline | best=TerminalVictory rank=42 observed=1".to_string(),
                "Shrug It Off | best=Abandoned rank=42 observed=1".to_string(),
            ]
        );
    }

    #[test]
    fn decision_outcome_analysis_render_reports_censored_groups() {
        let mut sample = sample_decision_outcome_sample("buy card 0", "Buy Clothesline");
        sample.sibling_group_id = format!("{}{}", "x".repeat(80), "y".repeat(80));
        sample.parent_branch_id = format!("root.{}", "rp 0.".repeat(50));
        sample.outcome.outcome_class = BranchOutcomeClassV1::OngoingFrozen;
        sample.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;
        sample.sibling_candidates = vec![
            sample_sibling_candidate(
                "buy card 0",
                "Buy Clothesline",
                BranchOutcomeClassV1::OngoingFrozen,
            ),
            sample_sibling_candidate("leave", "Leave shop", BranchOutcomeClassV1::OngoingFrozen),
        ];
        sample.observed_sibling_count = 2;

        let analysis = analyze_learning_decision_outcome_samples_v1(&[sample]);
        let rendered = render_learning_decision_outcome_analysis_v1(&analysis);

        assert!(rendered.contains("LearningDecisionOutcomeAnalysisV1 samples=1 groups=1"));
        assert!(rendered.contains("Command families: buy:1"));
        assert!(rendered.contains("censored_only_groups=1"));
        assert!(rendered.lines().all(|line| line.len() <= 220));
    }

    #[test]
    fn learning_readiness_probe_separates_signal_from_censoring() {
        let mut terminal = sample_decision_outcome_sample("rp 0", "Burning Pact");
        terminal.sibling_group_id = "terminal-group".to_string();
        terminal.observed_sibling_count = 2;
        terminal.sibling_candidates = vec![
            sample_sibling_candidate(
                "rp 0",
                "Burning Pact",
                BranchOutcomeClassV1::TerminalVictory,
            ),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::Abandoned),
        ];
        terminal.outcome.outcome_class = BranchOutcomeClassV1::TerminalVictory;
        terminal.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::TerminalOutcome;
        terminal.outcome.checkpoint_enriched = true;
        terminal.outcome.state_features = Some(sample_state_features());

        let mut censored = sample_decision_outcome_sample("smith 1", "Smith Bash");
        censored.sibling_group_id = "censored-group".to_string();
        censored.observed_sibling_count = 1;
        censored.outcome.outcome_class = BranchOutcomeClassV1::OngoingFrozen;
        censored.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;
        censored.outcome.report_stop_reason = "victory_found".to_string();

        let mut combat = sample_decision_outcome_sample("rp 2", "Carnage");
        combat.sibling_group_id = "combat-group".to_string();
        combat.outcome.outcome_class = BranchOutcomeClassV1::Abandoned;
        combat.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::InterventionOrFailure;
        combat.outcome.frontier_title = "Combat".to_string();
        combat.outcome.stop_reason = "combat search did not find executable win".to_string();

        let probe = probe_learning_readiness_v1(&[terminal, censored, combat]);

        assert_eq!(probe.total_samples, 3);
        assert_eq!(probe.decision_group_count, 3);
        assert_eq!(probe.observed_sibling_group_count, 1);
        assert_eq!(probe.terminal_group_count, 1);
        assert_eq!(probe.terminal_observed_sibling_group_count, 1);
        assert_eq!(probe.censored_only_group_count, 1);
        assert_eq!(probe.branch_scheduling_censored_group_count, 1);
        assert_eq!(probe.combat_unresolved_group_count, 1);
        assert_eq!(probe.missing_context_group_count, 2);
        assert_eq!(
            probe
                .bottlenecks
                .iter()
                .map(|entry| (entry.key.as_str(), entry.group_count))
                .collect::<Vec<_>>(),
            vec![
                ("missing_context_snapshot", 2),
                ("no_sibling_alternatives", 2),
                ("outcome_censored", 1),
                ("branch_scheduling_or_campaign_cutoff", 1),
                ("combat_unresolved_or_budget", 1),
            ]
        );
    }

    #[test]
    fn learning_readiness_probe_render_names_next_actions() {
        let mut censored = sample_decision_outcome_sample("skip", "Skip potion reward");
        censored.outcome.outcome_class = BranchOutcomeClassV1::OngoingFrozen;
        censored.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;
        censored.outcome.report_stop_reason = "max_rounds".to_string();

        let probe = probe_learning_readiness_v1(&[censored]);
        let rendered = render_learning_readiness_probe_v1(&probe);

        assert!(rendered.contains("LearningReadinessProbeV1 samples=1 groups=1"));
        assert!(rendered.contains("terminal_observed_sibling_groups=0"));
        assert!(rendered.contains("outcome_censored"));
        assert!(rendered.contains("next=run targeted continuation"));
    }

    #[test]
    fn targeted_continuation_plan_selects_partial_terminal_sibling_groups() {
        let mut terminal = sample_decision_outcome_sample("rp 0", "Burning Pact");
        terminal.sibling_group_id = "reward-group".to_string();
        terminal.observed_sibling_count = 2;
        terminal.sibling_candidates = vec![
            sample_sibling_candidate(
                "rp 0",
                "Burning Pact",
                BranchOutcomeClassV1::TerminalVictory,
            ),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::OngoingFrozen),
        ];
        terminal.outcome.outcome_class = BranchOutcomeClassV1::TerminalVictory;
        terminal.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::TerminalOutcome;

        let mut frozen = sample_decision_outcome_sample("rp 1", "Skip");
        frozen.sibling_group_id = "reward-group".to_string();
        frozen.observed_sibling_count = 2;
        frozen.sibling_candidates = terminal.sibling_candidates.clone();
        frozen.outcome.outcome_class = BranchOutcomeClassV1::OngoingFrozen;
        frozen.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;

        let mut singleton = sample_decision_outcome_sample("smith 1", "Smith Bash");
        singleton.sibling_group_id = "single-group".to_string();
        singleton.observed_sibling_count = 1;

        let plan = plan_targeted_continuations_v1(&[terminal, frozen, singleton]);

        assert_eq!(plan.total_decision_groups, 2);
        assert_eq!(plan.selected_target_count, 1);
        assert_eq!(plan.targets[0].sibling_group_id, "reward-group");
        assert_eq!(plan.targets[0].command_family, "rp");
        assert_eq!(
            plan.targets[0].reason_keys,
            vec!["partial_terminal_siblings".to_string()]
        );
        assert_eq!(
            plan.targets[0].milestone,
            "next_major_milestone_or_terminal"
        );
        assert_eq!(
            plan.targets[0]
                .candidates
                .iter()
                .map(|candidate| (
                    candidate.choice_label.as_str(),
                    candidate.needs_continuation
                ))
                .collect::<Vec<_>>(),
            vec![("Burning Pact", false), ("Skip", true)]
        );
    }

    #[test]
    fn targeted_continuation_candidate_does_not_continue_intervention_failures() {
        let abandoned =
            sample_sibling_candidate("rp 2", "Sentinel", BranchOutcomeClassV1::Abandoned);

        let candidate = targeted_continuation_candidate_v1(&abandoned);

        assert!(!candidate.needs_continuation);
    }

    #[test]
    fn targeted_continuation_execution_plan_selects_existing_censored_branches() {
        let mut frozen_sample = sample_decision_outcome_sample("rp 1", "Skip");
        frozen_sample.sibling_group_id = "reward-group".to_string();
        frozen_sample.observed_sibling_count = 2;
        frozen_sample.sibling_candidates = vec![
            sample_sibling_candidate(
                "rp 0",
                "Burning Pact",
                BranchOutcomeClassV1::TerminalVictory,
            ),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::OngoingFrozen),
            sample_sibling_candidate("rp 2", "Sentinel", BranchOutcomeClassV1::Abandoned),
        ];
        let plan = plan_targeted_continuations_v1(&[frozen_sample]);
        let report = sample_campaign_report_with_branches(vec![
            sample_report_branch("root.rp 1", BranchCampaignBranchStatusV1::Frozen),
            sample_report_branch("root.rp 2", BranchCampaignBranchStatusV1::Abandoned),
        ]);

        let execution = targeted_continuation_execution_plan_v1(&plan, &report, 4, 2);

        assert_eq!(execution.selected_branch_count, 1);
        assert_eq!(execution.missing_branch_count, 0);
        assert_eq!(execution.branches[0].representative_branch_id, "root.rp 1");
        assert_eq!(execution.branches[0].choice_label, "Skip");
    }

    #[test]
    fn targeted_continuation_execution_plan_reports_missing_branches() {
        let mut frozen_sample = sample_decision_outcome_sample("rp 1", "Skip");
        frozen_sample.sibling_group_id = "reward-group".to_string();
        frozen_sample.observed_sibling_count = 2;
        frozen_sample.sibling_candidates = vec![
            sample_sibling_candidate(
                "rp 0",
                "Burning Pact",
                BranchOutcomeClassV1::TerminalVictory,
            ),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::OngoingFrozen),
        ];
        let plan = plan_targeted_continuations_v1(&[frozen_sample]);
        let report = sample_campaign_report_with_branches(Vec::new());

        let execution = targeted_continuation_execution_plan_v1(&plan, &report, 4, 2);

        assert_eq!(execution.selected_branch_count, 0);
        assert_eq!(execution.missing_branch_count, 1);
    }

    #[test]
    fn continuation_effect_report_detects_new_terminal_sibling_progress() {
        let mut before = sample_decision_outcome_sample("rp 1", "Skip");
        before.sibling_group_id = "reward-group".to_string();
        before.observed_sibling_count = 2;
        before.sibling_candidates = vec![
            sample_sibling_candidate("rp 0", "Burning Pact", BranchOutcomeClassV1::OngoingFrozen),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::OngoingFrozen),
        ];
        before.outcome.outcome_class = BranchOutcomeClassV1::OngoingFrozen;
        before.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;

        let mut after_win = sample_decision_outcome_sample("rp 0", "Burning Pact");
        after_win.sibling_group_id = "reward-group".to_string();
        after_win.observed_sibling_count = 2;
        after_win.sibling_candidates = vec![
            sample_sibling_candidate(
                "rp 0",
                "Burning Pact",
                BranchOutcomeClassV1::TerminalVictory,
            ),
            sample_sibling_candidate("rp 1", "Skip", BranchOutcomeClassV1::OngoingFrozen),
        ];
        after_win.outcome.outcome_class = BranchOutcomeClassV1::TerminalVictory;
        after_win.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::TerminalOutcome;

        let report = analyze_continuation_effect_v1(&[before], &[after_win]);

        assert_eq!(report.common_groups, 1);
        assert_eq!(report.before_censored_only_groups, 1);
        assert_eq!(report.after_censored_only_groups, 0);
        assert_eq!(report.censored_only_delta, -1);
        assert_eq!(report.newly_terminal_groups, 1);
        assert_eq!(report.newly_terminal_observed_sibling_groups, 1);
        assert_eq!(report.still_censored_target_groups, 0);
        assert_eq!(report.expanded_target_groups, 1);
        assert_eq!(report.examples[0].effect, "new_terminal_observed_sibling");
    }

    #[test]
    fn continuation_effect_report_detects_still_censored_targets() {
        let mut before = sample_decision_outcome_sample("buy card 0", "Buy Warcry");
        before.sibling_group_id = "shop-group".to_string();
        before.observed_sibling_count = 2;
        before.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;

        let mut after = before.clone();
        after.branch_id = "root.buy card 0.rp 0".to_string();
        after.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;

        let report = analyze_continuation_effect_v1(&[before], &[after]);
        let rendered = render_continuation_effect_report_v1(&report);

        assert_eq!(report.common_groups, 1);
        assert_eq!(report.still_censored_target_groups, 1);
        assert_eq!(report.expanded_target_groups, 1);
        assert!(rendered.contains("ContinuationEffectReportV1"));
        assert!(rendered.contains("still_censored_target_groups=1"));
    }

    #[test]
    fn continuation_effect_report_ignores_campaign_round_in_group_key() {
        let mut before = sample_decision_outcome_sample("rp 1", "Skip");
        before.sibling_group_id =
            "seed=1|domain=debug_a0:0|rounds=7|parent=root.rp 0|step=3".to_string();
        before.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::CensoredOngoing;

        let mut after = sample_decision_outcome_sample("rp 1", "Skip");
        after.sibling_group_id =
            "seed=1|domain=debug_a0:0|rounds=8|parent=root.rp 0|step=3".to_string();
        after.outcome.outcome_class = BranchOutcomeClassV1::TerminalVictory;
        after.outcome.supervision_status = BranchOutcomeSupervisionStatusV1::TerminalOutcome;

        let report = analyze_continuation_effect_v1(&[before], &[after]);

        assert_eq!(report.common_groups, 1);
        assert_eq!(report.newly_terminal_groups, 1);
        assert_eq!(report.after_censored_only_groups, 0);
    }

    #[test]
    fn targeted_continuation_plan_render_summarizes_targets() {
        let mut sample = sample_decision_outcome_sample("buy card 0", "Buy Burning Pact");
        sample.sibling_group_id = "shop-group".to_string();
        sample.observed_sibling_count = 2;
        sample.sibling_candidates = vec![
            sample_sibling_candidate(
                "buy card 0",
                "Buy Burning Pact",
                BranchOutcomeClassV1::OngoingFrozen,
            ),
            sample_sibling_candidate("leave", "Leave shop", BranchOutcomeClassV1::OngoingFrozen),
            sample_sibling_candidate(
                "buy card 1",
                "Buy Dark Embrace",
                BranchOutcomeClassV1::OngoingFrozen,
            ),
            sample_sibling_candidate(
                "buy card 2",
                "Buy Shrug It Off",
                BranchOutcomeClassV1::OngoingFrozen,
            ),
            sample_sibling_candidate(
                "buy combo",
                "Purge Strike 50g then Buy Dark Embrace 20g then Buy FrozenEye 72g",
                BranchOutcomeClassV1::OngoingFrozen,
            ),
        ];

        let plan = plan_targeted_continuations_v1(&[sample]);
        let rendered = render_targeted_continuation_plan_v1(&plan);

        assert!(rendered.contains("TargetedContinuationPlanV1 groups=1 targets=1"));
        assert!(rendered.contains("reason=censored_sibling_group"));
        assert!(rendered.contains("next_major_milestone_or_terminal"));
        assert!(rendered.contains("... 1 more candidate(s)"));
        assert!(!rendered.contains("then Buy FrozenEye"));
    }

    fn sample_decision_outcome_sample(
        command: &str,
        choice_label: &str,
    ) -> LearningDecisionOutcomeSampleV1 {
        LearningDecisionOutcomeSampleV1 {
            schema_name: LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_NAME.to_string(),
            schema_version: LEARNING_DECISION_OUTCOME_SAMPLE_SCHEMA_VERSION,
            label_role: "campaign_observation_not_teacher".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            provenance: LearningDatasetProvenanceV1 {
                exporter_git_commit: None,
                exporter_git_dirty: None,
                source_report_path: None,
                source_checkpoint_path: None,
                source_record_schema_name: BRANCH_OUTCOME_RECORD_SCHEMA_NAME.to_string(),
                source_record_schema_version: BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
            },
            seed: 521,
            run_domain: BranchCampaignRunDomainV1::default(),
            report_rounds_completed: 3,
            decision_id: format!("group|candidate={command}"),
            sibling_group_id: "group".to_string(),
            parent_branch_id: "root".to_string(),
            step_index: 0,
            candidate_command: command.to_string(),
            candidate_choice_label: choice_label.to_string(),
            candidate_set_status: LearningCandidateSetStatusV1::ChosenOnly,
            observed_candidate_index: 0,
            observed_sibling_count: 1,
            sibling_candidates: Vec::new(),
            branch_group: "active".to_string(),
            branch_index: 0,
            branch_id: format!("root.{command}"),
            strategic_summary: BranchSignatureCompact::default(),
            outcome: LearningBranchOutcomeV1 {
                branch_status: BranchCampaignBranchStatusV1::Active,
                outcome_class: BranchOutcomeClassV1::OngoingActive,
                supervision_status: BranchOutcomeSupervisionStatusV1::CensoredOngoing,
                report_stop_reason: "max_rounds".to_string(),
                stop_reason: "card reward requires human choice".to_string(),
                frontier_title: "Card Reward".to_string(),
                rank_key: 42,
                report_summary: None,
                checkpoint_enriched: false,
                state_features: None,
            },
        }
    }

    fn sample_sibling_candidate(
        command: &str,
        choice_label: &str,
        outcome_class: BranchOutcomeClassV1,
    ) -> LearningSiblingCandidateV1 {
        LearningSiblingCandidateV1 {
            command: command.to_string(),
            choice_label: choice_label.to_string(),
            observed_branch_count: 1,
            representative_branch_group: "active".to_string(),
            representative_branch_index: 0,
            representative_branch_id: format!("root.{command}"),
            best_outcome_class: outcome_class.clone(),
            best_supervision_status: match outcome_class {
                BranchOutcomeClassV1::TerminalVictory | BranchOutcomeClassV1::TerminalDefeat => {
                    BranchOutcomeSupervisionStatusV1::TerminalOutcome
                }
                BranchOutcomeClassV1::OngoingActive | BranchOutcomeClassV1::OngoingFrozen => {
                    BranchOutcomeSupervisionStatusV1::CensoredOngoing
                }
                BranchOutcomeClassV1::Abandoned | BranchOutcomeClassV1::Stuck => {
                    BranchOutcomeSupervisionStatusV1::InterventionOrFailure
                }
            },
            best_rank_key: 42,
            best_frontier_title: "Card Reward".to_string(),
            outcome_class_counts: vec![LearningOutcomeClassCountV1 {
                outcome_class,
                count: 1,
            }],
        }
    }

    fn sample_journal_candidate(command: &str, label: &str) -> CampaignJournalCandidateV1 {
        CampaignJournalCandidateV1 {
            candidate_id: command.to_string(),
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: "test".to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Scheduled,
                "test",
                "kept",
            ),
            disposition: crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Kept,
        }
    }

    fn route_candidate_test_label_v1(
        candidate: &crate::ai::route_planner_v1::RouteMoveCandidateV1,
    ) -> String {
        format!(
            "x={} y={} {}",
            candidate.target.x,
            candidate.target.y,
            candidate
                .target
                .room_type
                .map(|room| format!("{:?}", room))
                .unwrap_or_else(|| "Unknown".to_string())
        )
    }

    fn sample_pruned_journal_candidate(command: &str, label: &str) -> CampaignJournalCandidateV1 {
        CampaignJournalCandidateV1 {
            candidate_id: command.to_string(),
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: "test".to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Deferred,
                "test",
                "pruned",
            ),
            disposition:
                crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Pruned,
        }
    }

    fn sample_journal_candidate_with_admission(
        command: &str,
        label: &str,
        disposition: CampaignJournalCandidateDispositionV1,
        status: CampaignJournalCandidateAdmissionStatusV1,
    ) -> CampaignJournalCandidateV1 {
        CampaignJournalCandidateV1 {
            candidate_id: command.to_string(),
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: "test".to_string(),
            disposition,
            admission: CampaignJournalCandidateAdmissionTraceV1::new(status, "test", "test"),
        }
    }

    fn sample_campaign_report_with_branches(
        branches: Vec<BranchCampaignBranchV1>,
    ) -> BranchCampaignReportV1 {
        let mut active = Vec::new();
        let mut frozen = Vec::new();
        let mut abandoned = Vec::new();
        for branch in branches {
            match branch.status {
                BranchCampaignBranchStatusV1::Active => active.push(branch),
                BranchCampaignBranchStatusV1::Frozen => frozen.push(branch),
                BranchCampaignBranchStatusV1::Abandoned => abandoned.push(branch),
                _ => frozen.push(branch),
            }
        }

        BranchCampaignReportV1 {
            schema_name: "BranchCampaignV1".to_string(),
            schema_version: 1,
            seed: 521,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: Default::default(),
            rounds_completed: 3,
            stop_reason: "max_rounds".to_string(),
            active,
            frozen,
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned,
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        }
    }

    fn sample_report_branch(
        branch_id: &str,
        status: BranchCampaignBranchStatusV1,
    ) -> BranchCampaignBranchV1 {
        BranchCampaignBranchV1 {
            branch_id: branch_id.to_string(),
            commands: branch_id
                .strip_prefix("root.")
                .map(|suffix| suffix.split('.').map(str::to_string).collect())
                .unwrap_or_default(),
            choice_labels: vec![branch_id.to_string()],
            summary: None,
            strategic_summary: BranchSignatureCompact::default(),
            frontier_title: "Card Reward".to_string(),
            status,
            stop_reason: String::new(),
            continuation_origin: None,
            lineage_decision_signal_rank_adjustment: 0,
            rank_key: 42,
            final_boss_combat_record: None,
            combat_lab_probes: Vec::new(),
        }
    }

    fn sample_state_features() -> BranchOutcomeStateFeaturesV1 {
        BranchOutcomeStateFeaturesV1 {
            engine_state: "RewardScreen".to_string(),
            act: 1,
            floor: 4,
            hp: 70,
            max_hp: 80,
            gold: 120,
            ascension_level: 0,
            player_class: "Ironclad".to_string(),
            boss: Some("TheGuardian".to_string()),
            boss_pressure: Vec::new(),
            deck: BranchOutcomeDeckFeaturesV1 {
                deck_count: 12,
                grouped_cards: Vec::new(),
                attacks: 6,
                skills: 5,
                powers: 0,
                curses: 0,
                statuses: 0,
                starter_strikes: 4,
                starter_defends: 4,
                upgraded: 1,
            },
            relics: Vec::new(),
            potions: Vec::new(),
            formation: BranchOutcomeFormationFeaturesV1 {
                stage: "PlanSeeded".to_string(),
                needs: vec!["Frontload".to_string()],
                strengths: Vec::new(),
            },
            startup: BranchOutcomeStartupFeaturesV1 {
                setup_debt: 0,
                setup_payment: 0,
                effective_setup_payment: 0,
                immediate_survival: 1,
                payoff_engine: 0,
                combat_shape_risk: 0,
                strong_draw_count: 0,
                effective_strong_draw_count: 0,
                exhaust_engine_count: 0,
                exhaust_payoff_count: 0,
                status_generator_count: 0,
                status_digest_count: 0,
                persistent_strength_source_count: 0,
                temporary_strength_burst_count: 0,
                strength_converter_count: 0,
                convertible_strength_source_count: 0,
                strength_payoff_count: 0,
                zero_cost_card_count: 0,
                low_cost_card_count: 10,
                high_cost_card_count: 1,
                has_snecko_eye: false,
                snecko_random_cost_debt: 0,
                liabilities: Vec::new(),
            },
            last_combat: None,
        }
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
