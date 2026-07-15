use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

mod calibration;
mod closed_loop;
mod promotion;
mod replay;
mod route_risk_calibration;
mod runtime_inputs;
mod strategy_package_calibration;
pub use calibration::{
    calibrate_card_reward_outcomes_v1, estimate_card_reward_value_from_calibration_v1,
    estimate_card_reward_values_from_calibration_v1, CardRewardOutcomeCalibrationBucketV1,
    CardRewardOutcomeCalibrationGlobalV1, CardRewardOutcomeCalibrationProvenanceV1,
    CardRewardOutcomeCalibrationV1, CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME,
    CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
};
pub use closed_loop::{
    build_card_reward_closed_loop_report_v1, summarize_card_reward_closed_loop_v1,
    CardRewardClosedLoopExampleV1, CardRewardClosedLoopReportV1, CardRewardClosedLoopStatusV1,
    CardRewardClosedLoopSummaryV1, CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_NAME,
    CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_VERSION,
};
pub use promotion::{
    build_card_reward_runtime_calibration_pipeline_v1, promote_card_reward_outcome_calibration_v1,
    CardRewardOutcomeCalibrationPromotionBlockerV1, CardRewardOutcomeCalibrationPromotionBucketV1,
    CardRewardOutcomeCalibrationPromotionConfigV1, CardRewardOutcomeCalibrationPromotionReportV1,
    CardRewardRuntimeCalibrationPipelineV1, CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_NAME,
    CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_VERSION,
};
pub use replay::{
    replay_card_reward_records_with_calibration_v1,
    replay_card_reward_records_with_runtime_calibrations_v1,
    CardRewardCalibrationReplayCandidateV1, CardRewardCalibrationReplayEstimateV1,
    CardRewardCalibrationReplayExampleV1, CardRewardCalibrationReplayReportV1,
    CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_NAME, CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_VERSION,
};
pub use route_risk_calibration::{
    calibrate_card_reward_route_risk_v1,
    estimate_card_reward_values_from_route_risk_calibration_v1,
    CardRewardRouteRiskCalibrationBucketV1, CardRewardRouteRiskCalibrationGlobalV1,
    CardRewardRouteRiskCalibrationV1, CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME,
    CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION,
};
pub use runtime_inputs::{
    build_card_reward_runtime_estimator_inputs_v1, CardRewardRuntimeEstimatorCalibrationsV1,
};
pub use strategy_package_calibration::{
    calibrate_card_reward_strategy_package_v1,
    estimate_card_reward_values_from_strategy_package_calibration_v1,
    CardRewardStrategyPackageCalibrationBucketV1, CardRewardStrategyPackageCalibrationGlobalV1,
    CardRewardStrategyPackageCalibrationV1, CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME,
    CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION,
};

use crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1;
use crate::ai::noncombat_decision_v1::{
    noncombat_decision_record_hash_v1, CardRewardOutcomeAttachmentV1, DecisionSiteKindV1,
    NonCombatDecisionRecordV1, NonCombatOutcomeAttachmentV1, PolicySelectionStatusV1,
};
use crate::eval::run_control::{
    RunControlTraceAnnotationV1, SessionTraceBoundaryRecordV1, SessionTraceStepV1, SessionTraceV1,
};

pub const CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME: &str = "CardRewardValueLoopExampleV1";
pub const CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION: u32 = 1;
pub const CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_NAME: &str = "CardRewardValueLoopSummaryV1";
pub const CARD_REWARD_VALUE_LOOP_SUMMARY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardValueLoopReplayStatusV1 {
    RecordOnlyNoPublicPacket,
    FullPublicPacket,
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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_trace_schema_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_trace_schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_run_config: Option<CardRewardValueLoopRunConfigV1>,

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
    pub public_packet: Option<PublicRewardDecisionPacketV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<NonCombatOutcomeAttachmentV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardValueLoopRunConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: String,
    pub final_act: bool,
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
    pub picked_card_drawn_observation_count: usize,
    pub picked_card_played_observation_count: usize,
    pub picked_card_upgraded_observation_count: usize,
    pub picked_card_removed_observation_count: usize,
}

pub fn extract_card_reward_value_loop_examples_v1(
    trace: &SessionTraceV1,
) -> Result<Vec<CardRewardValueLoopExampleV1>, String> {
    let outcomes_by_hash = merged_outcomes_by_hash(&trace.noncombat_outcome_attachments);
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

fn merged_outcomes_by_hash(
    outcomes: &[NonCombatOutcomeAttachmentV1],
) -> BTreeMap<String, NonCombatOutcomeAttachmentV1> {
    let mut merged = BTreeMap::new();
    for outcome in outcomes {
        merged
            .entry(outcome.decision_record_hash.clone())
            .and_modify(|existing| merge_noncombat_outcome(existing, outcome))
            .or_insert_with(|| outcome.clone());
    }
    merged
}

fn merge_noncombat_outcome(
    existing: &mut NonCombatOutcomeAttachmentV1,
    incoming: &NonCombatOutcomeAttachmentV1,
) {
    match (&mut existing.card_reward, &incoming.card_reward) {
        (Some(existing_card_reward), Some(incoming_card_reward)) => {
            merge_card_reward_outcome(existing_card_reward, incoming_card_reward);
        }
        (None, Some(incoming_card_reward)) => {
            existing.card_reward = Some(incoming_card_reward.clone());
        }
        (Some(_), None) | (None, None) => {}
    }
}

fn merge_card_reward_outcome(
    existing: &mut CardRewardOutcomeAttachmentV1,
    incoming: &CardRewardOutcomeAttachmentV1,
) {
    fill_option(
        &mut existing.next_combat_hp_loss,
        incoming.next_combat_hp_loss,
    );
    fill_option(
        &mut existing.hp_before_next_elite,
        incoming.hp_before_next_elite,
    );
    fill_option(
        &mut existing.hp_after_next_elite,
        incoming.hp_after_next_elite,
    );
    fill_option(&mut existing.hp_before_boss, incoming.hp_before_boss);
    merge_max_option(
        &mut existing.picked_card_drawn_count,
        incoming.picked_card_drawn_count,
    );
    merge_max_option(
        &mut existing.picked_card_played_count,
        incoming.picked_card_played_count,
    );
    merge_bool_or_option(
        &mut existing.picked_card_upgraded_before_boss,
        incoming.picked_card_upgraded_before_boss,
    );
    merge_bool_or_option(
        &mut existing.picked_card_removed_later,
        incoming.picked_card_removed_later,
    );
}

fn fill_option<T: Copy>(existing: &mut Option<T>, incoming: Option<T>) {
    if existing.is_none() {
        *existing = incoming;
    }
}

fn merge_max_option<T: Copy + Ord>(existing: &mut Option<T>, incoming: Option<T>) {
    match (*existing, incoming) {
        (Some(current), Some(next)) => *existing = Some(current.max(next)),
        (None, Some(next)) => *existing = Some(next),
        (Some(_), None) | (None, None) => {}
    }
}

fn merge_bool_or_option(existing: &mut Option<bool>, incoming: Option<bool>) {
    match (*existing, incoming) {
        (Some(current), Some(next)) => *existing = Some(current || next),
        (None, Some(next)) => *existing = Some(next),
        (Some(_), None) | (None, None) => {}
    }
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
    let mut picked_card_drawn_observation_count = 0;
    let mut picked_card_played_observation_count = 0;
    let mut picked_card_upgraded_observation_count = 0;
    let mut picked_card_removed_observation_count = 0;

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
        if let Some(card_reward) = example
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.card_reward.as_ref())
        {
            if card_reward.picked_card_drawn_count.is_some() {
                picked_card_drawn_observation_count += 1;
            }
            if card_reward.picked_card_played_count.is_some() {
                picked_card_played_observation_count += 1;
            }
            if card_reward.picked_card_upgraded_before_boss.is_some() {
                picked_card_upgraded_observation_count += 1;
            }
            if card_reward.picked_card_removed_later.is_some() {
                picked_card_removed_observation_count += 1;
            }
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
        picked_card_drawn_observation_count,
        picked_card_played_observation_count,
        picked_card_upgraded_observation_count,
        picked_card_removed_observation_count,
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
        source_trace_schema_name: source.source_trace_schema_name.cloned(),
        source_trace_schema_version: source.source_trace_schema_version,
        source_run_config: source.run_config.map(CardRewardValueLoopRunConfigV1::from),
        trace_step_index: source.trace_step_index,
        trace_boundary_record_index: source.trace_boundary_record_index,
        decision_record_hash,
        decision_site: source.record.site,
        replay_status: if source.public_packet.is_some() {
            CardRewardValueLoopReplayStatusV1::FullPublicPacket
        } else {
            CardRewardValueLoopReplayStatusV1::RecordOnlyNoPublicPacket
        },
        outcome_status,
        selected_candidate_id: source.record.selection.selected_candidate_id.clone(),
        selection_status: source.record.selection.status,
        selection_reason: source.record.selection.reason.clone(),
        candidate_count: source.record.candidates.len(),
        value_estimate_count: source.record.values.len(),
        source_record: source.record.clone(),
        public_packet: source.public_packet.cloned(),
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
        CardRewardValueLoopReplayStatusV1::FullPublicPacket => "full_public_packet",
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

#[derive(Clone, Copy)]
struct CardRewardDecisionRecordSource<'a> {
    trace_step_index: Option<usize>,
    trace_boundary_record_index: Option<usize>,
    source_trace_schema_name: Option<&'a String>,
    source_trace_schema_version: Option<u32>,
    run_config: Option<&'a crate::eval::run_control::SessionTraceRunConfigV1>,
    record: &'a NonCombatDecisionRecordV1,
    public_packet: Option<&'a PublicRewardDecisionPacketV1>,
}

impl From<&crate::eval::run_control::SessionTraceRunConfigV1> for CardRewardValueLoopRunConfigV1 {
    fn from(run_config: &crate::eval::run_control::SessionTraceRunConfigV1) -> Self {
        Self {
            seed: run_config.seed,
            ascension_level: run_config.ascension_level,
            player_class: run_config.player_class.clone(),
            final_act: run_config.final_act,
        }
    }
}

fn card_reward_record_sources(trace: &SessionTraceV1) -> Vec<CardRewardDecisionRecordSource<'_>> {
    let mut sources = Vec::new();
    for step in &trace.steps {
        sources.extend(card_reward_record_sources_from_step(trace, step));
    }
    for boundary in &trace.boundary_records {
        sources.extend(card_reward_record_sources_from_boundary(trace, boundary));
    }
    sources
}

fn card_reward_record_sources_from_step<'a>(
    trace: &'a SessionTraceV1,
    step: &'a SessionTraceStepV1,
) -> Vec<CardRewardDecisionRecordSource<'a>> {
    card_reward_records_from_annotations(&step.annotations)
        .into_iter()
        .map(|(record, public_packet)| CardRewardDecisionRecordSource {
            trace_step_index: Some(step.step_index),
            trace_boundary_record_index: None,
            source_trace_schema_name: Some(&trace.schema_name),
            source_trace_schema_version: Some(trace.schema_version),
            run_config: Some(&trace.run_config),
            record,
            public_packet,
        })
        .collect()
}

fn card_reward_record_sources_from_boundary<'a>(
    trace: &'a SessionTraceV1,
    boundary: &'a SessionTraceBoundaryRecordV1,
) -> Vec<CardRewardDecisionRecordSource<'a>> {
    card_reward_records_from_annotations(&boundary.annotations)
        .into_iter()
        .map(|(record, public_packet)| CardRewardDecisionRecordSource {
            trace_step_index: None,
            trace_boundary_record_index: Some(boundary.record_index),
            source_trace_schema_name: Some(&trace.schema_name),
            source_trace_schema_version: Some(trace.schema_version),
            run_config: Some(&trace.run_config),
            record,
            public_packet,
        })
        .collect()
}

fn card_reward_records_from_annotations(
    annotations: &[RunControlTraceAnnotationV1],
) -> Vec<(
    &NonCombatDecisionRecordV1,
    Option<&PublicRewardDecisionPacketV1>,
)> {
    let policy_records = annotations
        .iter()
        .filter_map(card_reward_policy_record_from_annotation)
        .collect::<Vec<_>>();
    if !policy_records.is_empty() {
        return policy_records;
    }

    annotations
        .iter()
        .filter_map(card_reward_record_from_annotation)
        .collect()
}

fn card_reward_policy_record_from_annotation(
    annotation: &RunControlTraceAnnotationV1,
) -> Option<(
    &NonCombatDecisionRecordV1,
    Option<&PublicRewardDecisionPacketV1>,
)> {
    match annotation {
        RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            card_reward_packet,
        } if record.site == DecisionSiteKindV1::CardReward => {
            Some((record, card_reward_packet.as_ref()))
        }
        _ => None,
    }
}

fn card_reward_record_from_annotation(
    annotation: &RunControlTraceAnnotationV1,
) -> Option<(
    &NonCombatDecisionRecordV1,
    Option<&PublicRewardDecisionPacketV1>,
)> {
    match annotation {
        RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            card_reward_packet,
        } if record.site == DecisionSiteKindV1::CardReward => {
            Some((record, card_reward_packet.as_ref()))
        }
        RunControlTraceAnnotationV1::NonCombatHumanBoundary { record }
            if record.site == DecisionSiteKindV1::CardReward =>
        {
            Some((record, None))
        }
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: Some(record),
            ..
        } if record.site == DecisionSiteKindV1::CardReward => Some((record, None)),
        RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
        | RunControlTraceAnnotationV1::RoutePlannerCandidatePool { .. }
        | RunControlTraceAnnotationV1::NonCombatPolicyDecision { .. }
        | RunControlTraceAnnotationV1::NonCombatHumanBoundary { .. }
        | RunControlTraceAnnotationV1::PlannerBehaviorDecision { .. }
        | RunControlTraceAnnotationV1::AutoCombatCapture { .. }
        | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. }
        | RunControlTraceAnnotationV1::CombatSearchPerformance { .. }
        | RunControlTraceAnnotationV1::AcceptedCombatLine { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::calibration::outcome_calibration_eligibility;
    use super::*;
    use crate::ai::noncombat_decision_v1::{
        CandidateDescriptorV1, CardRewardOutcomeAttachmentV1, DataRoleV1, DecisionSiteKindV1,
        EvidenceBundleV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
        NonCombatOutcomeAttachmentV1, NonCombatOutcomeMetricsV1, NonCombatOutcomeSnapshotV1,
        NonCombatOutcomeWindowV1, PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1,
        PublicActionPlanV1, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
        NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION, NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_NAME,
        NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_VERSION,
    };
    use crate::content::cards::CardId;

    #[test]
    fn outcome_calibration_estimate_carries_structured_non_gate_eligibility_metadata() {
        let run_state = crate::state::run::RunState::new(521, 0, false, "Ironclad");
        let context = crate::ai::card_reward_policy_v1::build_card_reward_decision_context_v1(
            &run_state,
            vec![crate::state::rewards::RewardCard::new(
                crate::content::cards::CardId::TwinStrike,
                0,
            )],
            None,
        );
        let mut calibration = test_calibration_with_provenance(CardId::TwinStrike, false, false);
        calibration.provenance = Default::default();

        let estimates = estimate_card_reward_values_from_calibration_v1(&context, &calibration);

        assert_eq!(estimates.len(), 1);
        let eligibility = &estimates[0].eligibility;
        assert!(eligibility.usable_for_value_estimate);
        assert!(!eligibility.usable_for_autopilot_gate);
        assert_eq!(
            eligibility.bucket_key.as_deref(),
            Some("card_id:TwinStrike")
        );
        assert_eq!(
            eligibility.horizon,
            Some(crate::ai::card_reward_policy_v1::CardRewardValueHorizonV1::NextCombatHpLoss)
        );
        assert!(eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::OutcomeCalibrationBucketNotGateEligible,
        ));
        assert!(eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::MissingDistinctSeedCount,
        ));
        assert!(eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::ShortHorizonMetricOnly,
        ));
    }

    #[test]
    fn generated_outcome_calibration_carries_source_provenance() {
        let examples = vec![
            test_card_reward_example(CardId::TwinStrike, 521, 8),
            test_card_reward_example(CardId::Cleave, 522, 4),
        ];

        let calibration = calibrate_card_reward_outcomes_v1(&examples);

        assert_eq!(
            calibration.provenance.source_example_schema_name,
            CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME
        );
        assert_eq!(
            calibration.provenance.source_example_schema_version,
            CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION
        );
        assert_eq!(calibration.provenance.source_run_count, 2);
        assert_eq!(calibration.provenance.distinct_seed_count, Some(2));
        assert_eq!(
            calibration.provenance.data_roles,
            vec!["BehaviorPolicyNotTeacher".to_string()]
        );
        assert!(calibration.provenance.ruleset_version.is_some());
        assert!(!calibration.provenance.short_horizon_autopilot_gate_approved);
    }

    #[test]
    fn generated_outcome_calibration_summarizes_card_usage_observations() {
        let mut examples = vec![
            test_card_reward_example(CardId::TwinStrike, 521, 8),
            test_card_reward_example(CardId::TwinStrike, 522, 4),
        ];
        test_card_reward_outcome_mut(&mut examples[0]).picked_card_played_count = Some(2);
        test_card_reward_outcome_mut(&mut examples[1]).picked_card_played_count = Some(0);
        test_card_reward_outcome_mut(&mut examples[1]).picked_card_drawn_count = Some(1);

        let calibration = calibrate_card_reward_outcomes_v1(&examples);
        let bucket = calibration
            .card_id_buckets
            .iter()
            .find(|bucket| bucket.card_id == "TwinStrike")
            .expect("TwinStrike bucket should be present");

        assert_eq!(calibration.global.picked_card_played_observation_count, 2);
        assert_eq!(calibration.global.mean_picked_card_played_count, Some(1.0));
        assert_eq!(calibration.global.picked_card_drawn_observation_count, 1);
        assert_eq!(calibration.global.mean_picked_card_drawn_count, Some(1.0));
        assert_eq!(bucket.picked_card_played_observation_count, 2);
        assert_eq!(bucket.mean_picked_card_played_count, Some(1.0));
        assert_eq!(bucket.picked_card_drawn_observation_count, 1);
        assert_eq!(bucket.mean_picked_card_drawn_count, Some(1.0));
    }

    #[test]
    fn summary_reports_card_usage_observation_coverage() {
        let mut examples = vec![
            test_card_reward_example(CardId::TwinStrike, 521, 8),
            test_card_reward_example(CardId::Cleave, 522, 4),
        ];
        let first = test_card_reward_outcome_mut(&mut examples[0]);
        first.picked_card_played_count = Some(2);
        first.picked_card_upgraded_before_boss = Some(true);
        let second = test_card_reward_outcome_mut(&mut examples[1]);
        second.picked_card_drawn_count = Some(1);
        second.picked_card_removed_later = Some(false);

        let summary = summarize_card_reward_value_loop_examples_v1(&examples);

        assert_eq!(summary.picked_card_played_observation_count, 1);
        assert_eq!(summary.picked_card_drawn_observation_count, 1);
        assert_eq!(summary.picked_card_upgraded_observation_count, 1);
        assert_eq!(summary.picked_card_removed_observation_count, 1);
    }

    #[test]
    fn complete_provenance_clears_metadata_missing_reasons_without_opening_short_horizon_gate() {
        let calibration = test_calibration_with_provenance(CardId::TwinStrike, true, false);
        let bucket = calibration.card_id_buckets.first().unwrap();

        let eligibility = outcome_calibration_eligibility(&calibration, bucket);

        assert!(!eligibility.usable_for_autopilot_gate);
        assert!(!eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::MissingDistinctSeedCount,
        ));
        assert!(!eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::MissingRulesetVersion,
        ));
        assert!(!eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::MissingDataRoleProvenance,
        ));
        assert!(eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::ShortHorizonMetricOnly,
        ));
    }

    #[test]
    fn bucket_can_be_gate_eligible_only_when_provenance_and_horizon_are_explicitly_approved() {
        let calibration = test_calibration_with_provenance(CardId::TwinStrike, true, true);
        let bucket = calibration.card_id_buckets.first().unwrap();

        let eligibility = outcome_calibration_eligibility(&calibration, bucket);

        assert!(eligibility.usable_for_autopilot_gate);
        assert!(eligibility.reasons.is_empty());
    }

    #[test]
    fn hidden_state_provenance_keeps_outcome_calibration_out_of_autopilot_gate() {
        let mut calibration = test_calibration_with_provenance(CardId::TwinStrike, true, true);
        calibration.provenance.hidden_simulator_state_used = true;
        let bucket = calibration.card_id_buckets.first().unwrap();

        let eligibility = outcome_calibration_eligibility(&calibration, bucket);

        assert!(eligibility.usable_for_value_estimate);
        assert!(!eligibility.usable_for_autopilot_gate);
        assert!(eligibility.reasons.contains(
            &crate::ai::card_reward_policy_v1::CardRewardValueEligibilityReasonV1::HiddenSimulatorStateUsed,
        ));
    }

    #[test]
    fn runtime_calibration_pipeline_builds_promoted_artifact_and_reports_from_examples() {
        let examples = vec![
            test_card_reward_example(CardId::TwinStrike, 521, 4),
            test_card_reward_example(CardId::TwinStrike, 522, 6),
        ];
        let config = CardRewardOutcomeCalibrationPromotionConfigV1 {
            approve_short_horizon_autopilot_gate: true,
            min_distinct_seeds: 2,
            min_bucket_outcome_attached_count: 2,
            min_bucket_confidence: 0.3,
            max_bucket_uncertainty: 0.8,
            reject_hidden_simulator_state: true,
        };

        let pipeline = build_card_reward_runtime_calibration_pipeline_v1(&examples, &config);

        assert!(
            pipeline
                .promoted_calibration
                .provenance
                .short_horizon_autopilot_gate_approved
        );
        assert_eq!(pipeline.promotion_report.promoted_bucket_count, 1);
        assert_eq!(pipeline.route_risk_calibration.total_examples, 2);
        assert_eq!(pipeline.route_risk_calibration.evaluated_examples, 0);
        assert_eq!(pipeline.strategy_package_calibration.total_examples, 2);
        assert_eq!(pipeline.strategy_package_calibration.evaluated_examples, 0);
        assert_eq!(
            pipeline
                .closed_loop_report
                .route_risk_calibration
                .total_examples,
            2
        );
        assert_eq!(
            pipeline
                .closed_loop_report
                .route_risk_calibration
                .missing_public_packet_examples,
            2
        );
        assert_eq!(pipeline.closed_loop_report.calibration_bucket_count, 1);
        assert_eq!(
            pipeline
                .closed_loop_report
                .summary
                .calibration_autopilot_gate_blocked_candidate_count,
            0
        );
    }

    #[test]
    fn extractor_merges_multiple_outcome_windows_for_same_card_reward_decision() {
        let card_id = CardId::TwinStrike;
        let mut trace = crate::eval::run_control::SessionTraceV1::new(
            &crate::eval::run_control::RunControlSession::new(
                crate::eval::run_control::RunControlConfig::default(),
            ),
        );
        let candidate_id = format!("card_reward:0:{card_id:?}");
        let source_record = test_card_reward_record(card_id, &candidate_id);
        let decision_record_hash = noncombat_decision_record_hash_v1(&source_record)
            .expect("test decision record should hash");
        trace
            .boundary_records
            .push(crate::eval::run_control::SessionTraceBoundaryRecordV1 {
                record_index: 0,
                raw_command_line: "0".to_string(),
                decision_step: 1,
                screen_title: "Card Reward".to_string(),
                decision_kind: "card_reward".to_string(),
                boundary: test_boundary_fingerprint(),
                annotations: vec![RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                    record: source_record,
                    card_reward_packet: None,
                }],
            });
        trace.noncombat_outcome_attachments = vec![
            test_outcome_attachment(
                &decision_record_hash,
                &candidate_id,
                NonCombatOutcomeWindowV1::AfterOneFloor,
                Some(9),
                None,
                None,
            ),
            test_outcome_attachment(
                &decision_record_hash,
                &candidate_id,
                NonCombatOutcomeWindowV1::AfterNextElite,
                None,
                Some(70),
                Some(51),
            ),
        ];

        let examples = extract_card_reward_value_loop_examples_v1(&trace)
            .expect("extractor should merge multi-window outcomes");

        assert_eq!(examples.len(), 1);
        let card_reward = examples[0]
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.card_reward.as_ref())
            .expect("card reward outcome should be attached");
        assert_eq!(card_reward.next_combat_hp_loss, Some(9));
        assert_eq!(card_reward.hp_before_next_elite, Some(70));
        assert_eq!(card_reward.hp_after_next_elite, Some(51));
    }

    fn test_calibration_with_provenance(
        card_id: CardId,
        bucket_gate_eligible: bool,
        short_horizon_approved: bool,
    ) -> CardRewardOutcomeCalibrationV1 {
        let mut calibration = calibrate_card_reward_outcomes_v1(&[
            test_card_reward_example(card_id, 521, 5),
            test_card_reward_example(card_id, 522, 3),
        ]);
        calibration.provenance.short_horizon_autopilot_gate_approved = short_horizon_approved;
        calibration.card_id_buckets[0].usable_for_autopilot_gate = bucket_gate_eligible;
        calibration
    }

    fn test_card_reward_outcome_mut(
        example: &mut CardRewardValueLoopExampleV1,
    ) -> &mut CardRewardOutcomeAttachmentV1 {
        example
            .outcome
            .as_mut()
            .and_then(|outcome| outcome.card_reward.as_mut())
            .expect("test outcome should contain card reward")
    }

    fn test_card_reward_record(card_id: CardId, candidate_id: &str) -> NonCombatDecisionRecordV1 {
        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::CardReward,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "test_card_reward_policy".to_string(),
                source_schema_name: "TestCardRewardPolicy".to_string(),
                source_schema_version: 1,
            },
            candidates: vec![CandidateDescriptorV1 {
                candidate_id: candidate_id.to_string(),
                site: DecisionSiteKindV1::CardReward,
                label: format!("{card_id:?}"),
                action_plan: PublicActionPlanV1 {
                    summary: format!("pick {card_id:?}"),
                    command: Some("pick 0".to_string()),
                },
                information_classes: vec![InformationClassV1::PublicObservation],
                uncertainty_notes: Vec::new(),
            }],
            evidence: EvidenceBundleV1::default(),
            values: Vec::new(),
            selection: PolicySelectionV1 {
                status: PolicySelectionStatusV1::Selected,
                selected_candidate_id: Some(candidate_id.to_string()),
                reason: "test selected card reward".to_string(),
                confidence: 1.0,
                selection_mode: "test".to_string(),
            },
        }
    }

    fn test_outcome_attachment(
        decision_record_hash: &str,
        candidate_id: &str,
        window: NonCombatOutcomeWindowV1,
        next_combat_hp_loss: Option<i32>,
        hp_before_next_elite: Option<i32>,
        hp_after_next_elite: Option<i32>,
    ) -> NonCombatOutcomeAttachmentV1 {
        NonCombatOutcomeAttachmentV1 {
            schema_name: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            site: DecisionSiteKindV1::CardReward,
            decision_record_hash: decision_record_hash.to_string(),
            window,
            before: test_outcome_snapshot(80),
            after: test_outcome_snapshot(hp_after_next_elite.unwrap_or(80)),
            metrics: NonCombatOutcomeMetricsV1 {
                act_delta: 0,
                floor_delta: 1,
                hp_delta: -next_combat_hp_loss.unwrap_or(0),
                max_hp_delta: 0,
                gold_delta: 0,
                deck_size_delta: 0,
                relic_count_delta: 0,
                potion_count_delta: 0,
                combats_completed_delta: i32::from(next_combat_hp_loss.is_some()),
                elites_completed_delta: i32::from(hp_after_next_elite.is_some()),
                bosses_completed_delta: 0,
                terminal_changed: false,
            },
            card_reward: Some(CardRewardOutcomeAttachmentV1 {
                selected_candidate_id: candidate_id.to_string(),
                picked_card_label: candidate_id.to_string(),
                floor_reached_after_decision: 2,
                next_combat_hp_loss,
                hp_before_next_elite,
                hp_after_next_elite,
                hp_before_boss: None,
                picked_card_drawn_count: None,
                picked_card_played_count: None,
                picked_card_upgraded_before_boss: None,
                picked_card_removed_later: None,
            }),
        }
    }

    fn test_boundary_fingerprint() -> crate::eval::run_control::SessionTraceBoundaryFingerprintV1 {
        crate::eval::run_control::SessionTraceBoundaryFingerprintV1 {
            decision_step: 1,
            engine_state: "RewardScreen".to_string(),
            active_combat_engine_state: None,
            screen_title: "Card Reward".to_string(),
            decision_kind: "card_reward".to_string(),
            decision_label: "Card Reward".to_string(),
            act: 1,
            floor: 1,
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            boss: "The Guardian".to_string(),
            candidate_count: 1,
            candidate_set_hash: "set".to_string(),
            candidate_order_hash: "order".to_string(),
            combat: None,
        }
    }

    fn test_card_reward_example(
        card_id: CardId,
        seed: u64,
        next_combat_hp_loss: i32,
    ) -> CardRewardValueLoopExampleV1 {
        let candidate_id = format!("card_reward:0:{card_id:?}");
        let source_record = test_card_reward_record(card_id, &candidate_id);
        CardRewardValueLoopExampleV1 {
            schema_name: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            source_trace_schema_name: Some(
                crate::eval::run_control::SESSION_TRACE_SCHEMA_NAME.to_string(),
            ),
            source_trace_schema_version: Some(
                crate::eval::run_control::SESSION_TRACE_SCHEMA_VERSION,
            ),
            source_run_config: Some(CardRewardValueLoopRunConfigV1 {
                seed,
                ascension_level: 0,
                player_class: "Ironclad".to_string(),
                final_act: false,
            }),
            trace_step_index: Some(0),
            trace_boundary_record_index: None,
            decision_record_hash: format!("hash-{seed}-{card_id:?}"),
            decision_site: DecisionSiteKindV1::CardReward,
            replay_status: CardRewardValueLoopReplayStatusV1::FullPublicPacket,
            outcome_status: CardRewardValueLoopOutcomeStatusV1::Attached,
            selected_candidate_id: Some(candidate_id.clone()),
            selection_status: PolicySelectionStatusV1::Selected,
            selection_reason: "test selected card reward".to_string(),
            candidate_count: 1,
            value_estimate_count: 0,
            source_record,
            public_packet: None,
            outcome: Some(NonCombatOutcomeAttachmentV1 {
                schema_name: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_NAME.to_string(),
                schema_version: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_VERSION,
                label_role: "diagnostic_not_teacher_label".to_string(),
                trainable_as_action_label: false,
                policy_quality_claim: false,
                site: DecisionSiteKindV1::CardReward,
                decision_record_hash: format!("hash-{seed}-{card_id:?}"),
                window: NonCombatOutcomeWindowV1::AfterOneFloor,
                before: test_outcome_snapshot(80),
                after: test_outcome_snapshot(80 - next_combat_hp_loss),
                metrics: NonCombatOutcomeMetricsV1 {
                    act_delta: 0,
                    floor_delta: 1,
                    hp_delta: -next_combat_hp_loss,
                    max_hp_delta: 0,
                    gold_delta: 0,
                    deck_size_delta: 0,
                    relic_count_delta: 0,
                    potion_count_delta: 0,
                    combats_completed_delta: 1,
                    elites_completed_delta: 0,
                    bosses_completed_delta: 0,
                    terminal_changed: false,
                },
                card_reward: Some(CardRewardOutcomeAttachmentV1 {
                    selected_candidate_id: candidate_id,
                    picked_card_label: format!("{card_id:?}"),
                    floor_reached_after_decision: 2,
                    next_combat_hp_loss: Some(next_combat_hp_loss),
                    hp_before_next_elite: None,
                    hp_after_next_elite: None,
                    hp_before_boss: None,
                    picked_card_drawn_count: None,
                    picked_card_played_count: None,
                    picked_card_upgraded_before_boss: None,
                    picked_card_removed_later: None,
                }),
            }),
        }
    }

    fn test_outcome_snapshot(current_hp: i32) -> NonCombatOutcomeSnapshotV1 {
        NonCombatOutcomeSnapshotV1 {
            act: 1,
            floor: 1,
            current_hp,
            max_hp: 80,
            gold: 99,
            deck_size: 10,
            relic_count: 1,
            potion_count: 0,
            combats_completed: 0,
            elites_completed: 0,
            bosses_completed: 0,
            run_terminal: None,
        }
    }
}
