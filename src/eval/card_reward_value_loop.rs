use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    CardRewardCandidateEvidenceV1, CardRewardValueComponentV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};
use crate::ai::noncombat_decision_v1::{
    noncombat_decision_record_hash_v1, DecisionSiteKindV1, NonCombatDecisionRecordV1,
    NonCombatOutcomeAttachmentV1, PolicySelectionStatusV1,
};
use crate::eval::run_control::{
    RunControlTraceAnnotationV1, SessionTraceBoundaryRecordV1, SessionTraceStepV1, SessionTraceV1,
};

pub const CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME: &str = "CardRewardValueLoopExampleV1";
pub const CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION: u32 = 1;
pub const CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_NAME: &str = "CardRewardValueLoopSummaryV1";
pub const CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME: &str = "CardRewardOutcomeCalibrationV1";
pub const CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardValueLoopReplayStatusV1 {
    RecordOnlyNoPublicPacket,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardValueLoopOutcomeStatusV1 {
    Attached,
    Missing,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardValueLoopExampleV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,

    pub trace_step_index: Option<usize>,
    pub trace_boundary_record_index: Option<usize>,
    pub decision_record_hash: String,
    pub decision_site: DecisionSiteKindV1,

    pub replay_status: CardRewardValueLoopReplayStatusV1,
    pub outcome_status: CardRewardValueLoopOutcomeStatusV1,

    pub selected_candidate_id: Option<String>,
    pub selection_status: PolicySelectionStatusV1,
    pub selection_reason: String,
    pub candidate_count: usize,
    pub value_estimate_count: usize,

    pub source_record: NonCombatDecisionRecordV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<NonCombatOutcomeAttachmentV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistogramEntryV1 {
    pub key: String,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardValueLoopSummaryV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_examples: usize,
    pub attached_outcome_count: usize,
    pub missing_outcome_count: usize,
    pub selection_status_counts: Vec<HistogramEntryV1>,
    pub outcome_status_counts: Vec<HistogramEntryV1>,
    pub replay_status_counts: Vec<HistogramEntryV1>,
    pub value_source_counts: Vec<HistogramEntryV1>,
    pub value_status_counts: Vec<HistogramEntryV1>,
    pub evidence_gap_counts: Vec<HistogramEntryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub estimator_kind: String,
    pub total_examples: usize,
    pub usable_outcome_examples: usize,
    pub missing_outcome_examples: usize,
    pub global: CardRewardOutcomeCalibrationGlobalV1,
    pub card_id_buckets: Vec<CardRewardOutcomeCalibrationBucketV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationGlobalV1 {
    pub selected_count: usize,
    pub outcome_attached_count: usize,
    pub mean_next_combat_hp_loss: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationBucketV1 {
    pub bucket_key: String,
    pub card_id: String,
    pub selected_count: usize,
    pub outcome_attached_count: usize,
    pub missing_outcome_count: usize,
    pub mean_next_combat_hp_loss: Option<f32>,
    pub hp_loss_bucket_counts: Vec<HistogramEntryV1>,
    pub upgraded_count: usize,
    pub removed_count: usize,
    pub confidence: f32,
    pub uncertainty: f32,
    pub usable_for_value_estimate: bool,
    pub usable_for_autopilot_gate: bool,
}

pub fn extract_card_reward_value_loop_examples_v1(
    trace: &SessionTraceV1,
) -> Result<Vec<CardRewardValueLoopExampleV1>, String> {
    let outcomes_by_hash = trace
        .noncombat_outcome_attachments
        .iter()
        .map(|outcome| (outcome.decision_record_hash.clone(), outcome.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut seen_hashes = BTreeSet::new();
    let mut examples = Vec::new();

    for source in card_reward_record_sources(trace) {
        let hash = noncombat_decision_record_hash_v1(source.record)?;
        if !seen_hashes.insert(hash.clone()) {
            continue;
        }
        let outcome = outcomes_by_hash.get(&hash).cloned();
        examples.push(card_reward_value_loop_example(source, hash, outcome));
    }

    Ok(examples)
}

pub fn calibrate_card_reward_outcomes_v1(
    examples: &[CardRewardValueLoopExampleV1],
) -> CardRewardOutcomeCalibrationV1 {
    let mut buckets = BTreeMap::<String, CardRewardOutcomeCalibrationAccumulatorV1>::new();
    let mut global_hp_losses = Vec::new();
    let mut global_selected_count = 0;
    let mut missing_outcome_examples = 0;

    for example in examples {
        if example.selection_status != PolicySelectionStatusV1::Selected {
            continue;
        }
        let Some(card_id) = selected_card_id_from_example(example) else {
            continue;
        };
        global_selected_count += 1;
        let accumulator = buckets.entry(card_id.clone()).or_default();
        accumulator.selected_count += 1;

        let card_reward = example
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.card_reward.as_ref());
        let hp_loss = card_reward.and_then(|card_reward| card_reward.next_combat_hp_loss);

        if let Some(hp_loss) = hp_loss {
            accumulator.hp_losses.push(hp_loss);
            global_hp_losses.push(hp_loss);
            increment(
                &mut accumulator.hp_loss_bucket_counts,
                hp_loss_bucket_label(hp_loss),
            );
        } else {
            accumulator.missing_outcome_count += 1;
            missing_outcome_examples += 1;
        }

        if card_reward
            .and_then(|card_reward| card_reward.picked_card_upgraded_before_boss)
            .unwrap_or(false)
        {
            accumulator.upgraded_count += 1;
        }
        if card_reward
            .and_then(|card_reward| card_reward.picked_card_removed_later)
            .unwrap_or(false)
        {
            accumulator.removed_count += 1;
        }
    }

    let card_id_buckets = buckets
        .into_iter()
        .map(|(card_id, accumulator)| accumulator.into_bucket(card_id))
        .collect::<Vec<_>>();
    let usable_outcome_examples = global_hp_losses.len();

    CardRewardOutcomeCalibrationV1 {
        schema_name: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
        total_examples: examples.len(),
        usable_outcome_examples,
        missing_outcome_examples,
        global: CardRewardOutcomeCalibrationGlobalV1 {
            selected_count: global_selected_count,
            outcome_attached_count: usable_outcome_examples,
            mean_next_combat_hp_loss: mean_i32(&global_hp_losses),
        },
        card_id_buckets,
    }
}

pub fn estimate_card_reward_value_from_calibration_v1(
    candidate: &CardRewardCandidateEvidenceV1,
    calibration: &CardRewardOutcomeCalibrationV1,
) -> Option<CardRewardValueEstimateV1> {
    let card_id = format!("{:?}", candidate.card);
    let bucket = calibration
        .card_id_buckets
        .iter()
        .find(|bucket| bucket.card_id == card_id && bucket.usable_for_value_estimate)?;
    let card_mean = bucket.mean_next_combat_hp_loss?;
    let global_mean = calibration.global.mean_next_combat_hp_loss?;
    let survival_delta = global_mean - card_mean;

    Some(CardRewardValueEstimateV1 {
        index: candidate.index,
        card: candidate.card,
        source: CardRewardValueSourceV1::OutcomeCalibration,
        status: CardRewardValueStatusV1::OutcomeCalibrated,
        survival_delta,
        progress_delta: 0.0,
        deck_consistency_delta: 0.0,
        uncertainty: bucket.uncertainty,
        components: vec![
            CardRewardValueComponentV1 {
                name: "outcome_sample_count",
                value: bucket.outcome_attached_count as f32,
            },
            CardRewardValueComponentV1 {
                name: "mean_next_combat_hp_loss",
                value: card_mean,
            },
            CardRewardValueComponentV1 {
                name: "global_mean_next_combat_hp_loss",
                value: global_mean,
            },
            CardRewardValueComponentV1 {
                name: "survival_delta_from_global",
                value: survival_delta,
            },
            CardRewardValueComponentV1 {
                name: "outcome_calibration_confidence",
                value: bucket.confidence,
            },
            CardRewardValueComponentV1 {
                name: "outcome_calibration_uncertainty",
                value: bucket.uncertainty,
            },
        ],
    })
}

pub fn summarize_card_reward_value_loop_examples_v1(
    examples: &[CardRewardValueLoopExampleV1],
) -> CardRewardValueLoopSummaryV1 {
    let mut selection_status_counts = BTreeMap::<String, usize>::new();
    let mut outcome_status_counts = BTreeMap::<String, usize>::new();
    let mut replay_status_counts = BTreeMap::<String, usize>::new();
    let mut value_source_counts = BTreeMap::<String, usize>::new();
    let mut value_status_counts = BTreeMap::<String, usize>::new();
    let mut evidence_gap_counts = BTreeMap::<String, usize>::new();
    let mut attached_outcome_count = 0;
    let mut missing_outcome_count = 0;

    for example in examples {
        increment(
            &mut selection_status_counts,
            selection_status_label(example.selection_status),
        );
        increment(
            &mut outcome_status_counts,
            outcome_status_label(&example.outcome_status),
        );
        increment(
            &mut replay_status_counts,
            replay_status_label(&example.replay_status),
        );
        match example.outcome_status {
            CardRewardValueLoopOutcomeStatusV1::Attached => attached_outcome_count += 1,
            CardRewardValueLoopOutcomeStatusV1::Missing => missing_outcome_count += 1,
        }
        for value in &example.source_record.values {
            for component in &value.components {
                if component.name.starts_with("value_source_") {
                    increment(&mut value_source_counts, component.name.clone());
                }
                if component.name.starts_with("value_status_") {
                    increment(&mut value_status_counts, component.name.clone());
                }
            }
        }
        for warning in &example.source_record.evidence.warnings {
            increment(&mut evidence_gap_counts, warning.clone());
        }
    }

    CardRewardValueLoopSummaryV1 {
        schema_name: CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_examples: examples.len(),
        attached_outcome_count,
        missing_outcome_count,
        selection_status_counts: histogram_entries(selection_status_counts),
        outcome_status_counts: histogram_entries(outcome_status_counts),
        replay_status_counts: histogram_entries(replay_status_counts),
        value_source_counts: histogram_entries(value_source_counts),
        value_status_counts: histogram_entries(value_status_counts),
        evidence_gap_counts: histogram_entries(evidence_gap_counts),
    }
}

fn card_reward_value_loop_example(
    source: CardRewardDecisionRecordSource<'_>,
    decision_record_hash: String,
    outcome: Option<NonCombatOutcomeAttachmentV1>,
) -> CardRewardValueLoopExampleV1 {
    let outcome_status = if outcome.is_some() {
        CardRewardValueLoopOutcomeStatusV1::Attached
    } else {
        CardRewardValueLoopOutcomeStatusV1::Missing
    };
    CardRewardValueLoopExampleV1 {
        schema_name: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        trace_step_index: source.trace_step_index,
        trace_boundary_record_index: source.trace_boundary_record_index,
        decision_record_hash,
        decision_site: source.record.site,
        replay_status: CardRewardValueLoopReplayStatusV1::RecordOnlyNoPublicPacket,
        outcome_status,
        selected_candidate_id: source.record.selection.selected_candidate_id.clone(),
        selection_status: source.record.selection.status,
        selection_reason: source.record.selection.reason.clone(),
        candidate_count: source.record.candidates.len(),
        value_estimate_count: source.record.values.len(),
        source_record: source.record.clone(),
        outcome,
    }
}

fn selection_status_label(status: PolicySelectionStatusV1) -> &'static str {
    match status {
        PolicySelectionStatusV1::Selected => "selected",
        PolicySelectionStatusV1::Stopped => "stopped",
        PolicySelectionStatusV1::NoCandidates => "no_candidates",
    }
}

fn outcome_status_label(status: &CardRewardValueLoopOutcomeStatusV1) -> &'static str {
    match status {
        CardRewardValueLoopOutcomeStatusV1::Attached => "attached",
        CardRewardValueLoopOutcomeStatusV1::Missing => "missing",
    }
}

fn replay_status_label(status: &CardRewardValueLoopReplayStatusV1) -> &'static str {
    match status {
        CardRewardValueLoopReplayStatusV1::RecordOnlyNoPublicPacket => {
            "record_only_no_public_packet"
        }
    }
}

fn increment(histogram: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *histogram.entry(key.into()).or_default() += 1;
}

fn histogram_entries(histogram: BTreeMap<String, usize>) -> Vec<HistogramEntryV1> {
    histogram
        .into_iter()
        .map(|(key, count)| HistogramEntryV1 { key, count })
        .collect()
}

#[derive(Default)]
struct CardRewardOutcomeCalibrationAccumulatorV1 {
    selected_count: usize,
    missing_outcome_count: usize,
    hp_losses: Vec<i32>,
    hp_loss_bucket_counts: BTreeMap<String, usize>,
    upgraded_count: usize,
    removed_count: usize,
}

impl CardRewardOutcomeCalibrationAccumulatorV1 {
    fn into_bucket(self, card_id: String) -> CardRewardOutcomeCalibrationBucketV1 {
        let outcome_attached_count = self.hp_losses.len();
        let confidence = outcome_attached_count as f32 / (outcome_attached_count as f32 + 3.0);
        let uncertainty = 1.0 - confidence;
        CardRewardOutcomeCalibrationBucketV1 {
            bucket_key: format!("card_id:{card_id}"),
            card_id,
            selected_count: self.selected_count,
            outcome_attached_count,
            missing_outcome_count: self.missing_outcome_count,
            mean_next_combat_hp_loss: mean_i32(&self.hp_losses),
            hp_loss_bucket_counts: histogram_entries(self.hp_loss_bucket_counts),
            upgraded_count: self.upgraded_count,
            removed_count: self.removed_count,
            confidence,
            uncertainty,
            usable_for_value_estimate: outcome_attached_count > 0,
            usable_for_autopilot_gate: false,
        }
    }
}

fn selected_card_id_from_example(example: &CardRewardValueLoopExampleV1) -> Option<String> {
    example
        .selected_candidate_id
        .as_ref()
        .and_then(|candidate_id| candidate_id.rsplit_once(':'))
        .map(|(_, card_id)| card_id.to_string())
}

fn mean_i32(values: &[i32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<i32>() as f32 / values.len() as f32)
}

fn hp_loss_bucket_label(hp_loss: i32) -> &'static str {
    match hp_loss {
        i32::MIN..=-1 => "negative",
        0 => "0",
        1..=5 => "1_5",
        6..=10 => "6_10",
        11..=20 => "11_20",
        _ => "21_plus",
    }
}

#[derive(Clone, Copy)]
struct CardRewardDecisionRecordSource<'a> {
    trace_step_index: Option<usize>,
    trace_boundary_record_index: Option<usize>,
    record: &'a NonCombatDecisionRecordV1,
}

fn card_reward_record_sources(trace: &SessionTraceV1) -> Vec<CardRewardDecisionRecordSource<'_>> {
    let mut sources = Vec::new();
    for step in &trace.steps {
        sources.extend(card_reward_record_sources_from_step(step));
    }
    for boundary in &trace.boundary_records {
        sources.extend(card_reward_record_sources_from_boundary(boundary));
    }
    sources
}

fn card_reward_record_sources_from_step(
    step: &SessionTraceStepV1,
) -> Vec<CardRewardDecisionRecordSource<'_>> {
    step.annotations
        .iter()
        .filter_map(|annotation| card_reward_record_from_annotation(annotation))
        .map(|record| CardRewardDecisionRecordSource {
            trace_step_index: Some(step.step_index),
            trace_boundary_record_index: None,
            record,
        })
        .collect()
}

fn card_reward_record_sources_from_boundary(
    boundary: &SessionTraceBoundaryRecordV1,
) -> Vec<CardRewardDecisionRecordSource<'_>> {
    boundary
        .annotations
        .iter()
        .filter_map(|annotation| card_reward_record_from_annotation(annotation))
        .map(|record| CardRewardDecisionRecordSource {
            trace_step_index: None,
            trace_boundary_record_index: Some(boundary.record_index),
            record,
        })
        .collect()
}

fn card_reward_record_from_annotation(
    annotation: &RunControlTraceAnnotationV1,
) -> Option<&NonCombatDecisionRecordV1> {
    match annotation {
        RunControlTraceAnnotationV1::NonCombatPolicyDecision { record }
        | RunControlTraceAnnotationV1::NonCombatHumanBoundary { record }
            if record.site == DecisionSiteKindV1::CardReward =>
        {
            Some(record)
        }
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: Some(record),
            ..
        } if record.site == DecisionSiteKindV1::CardReward => Some(record),
        RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
        | RunControlTraceAnnotationV1::NonCombatPolicyDecision { .. }
        | RunControlTraceAnnotationV1::NonCombatHumanBoundary { .. }
        | RunControlTraceAnnotationV1::AutoCombatCapture { .. }
        | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. } => None,
    }
}
