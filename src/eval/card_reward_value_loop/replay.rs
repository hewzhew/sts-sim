use serde::{Deserialize, Serialize};

use super::{CardRewardOutcomeCalibrationV1, CardRewardValueLoopExampleV1};

pub const CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_NAME: &str = "CardRewardCalibrationReplayReportV1";
pub const CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardCalibrationReplayReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub policy_replay_status: String,
    pub total_examples: usize,
    pub replayed_examples: usize,
    pub examples: Vec<CardRewardCalibrationReplayExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardCalibrationReplayExampleV1 {
    pub decision_record_hash: String,
    pub selected_candidate_id: Option<String>,
    pub original_value_count: usize,
    pub candidate_replays: Vec<CardRewardCalibrationReplayCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardCalibrationReplayCandidateV1 {
    pub candidate_id: String,
    pub card_id: Option<String>,
    pub original_value_sources: Vec<String>,
    pub original_value_statuses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_estimate: Option<CardRewardCalibrationReplayEstimateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardCalibrationReplayEstimateV1 {
    pub source: String,
    pub status: String,
    pub survival_delta: f32,
    pub uncertainty: f32,
    pub outcome_sample_count: usize,
    pub usable_for_autopilot_gate: bool,
}

pub fn replay_card_reward_records_with_calibration_v1(
    examples: &[CardRewardValueLoopExampleV1],
    calibration: &CardRewardOutcomeCalibrationV1,
) -> CardRewardCalibrationReplayReportV1 {
    let examples_out = examples
        .iter()
        .map(|example| card_reward_calibration_replay_example(example, calibration))
        .collect::<Vec<_>>();

    CardRewardCalibrationReplayReportV1 {
        schema_name: CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        policy_replay_status: "record_only_no_public_packet".to_string(),
        total_examples: examples.len(),
        replayed_examples: examples_out.len(),
        examples: examples_out,
    }
}

fn card_reward_calibration_replay_example(
    example: &CardRewardValueLoopExampleV1,
    calibration: &CardRewardOutcomeCalibrationV1,
) -> CardRewardCalibrationReplayExampleV1 {
    let candidate_replays = example
        .source_record
        .candidates
        .iter()
        .map(|candidate| {
            let card_id = candidate_card_id_from_candidate_id(&candidate.candidate_id);
            let original_values = example
                .source_record
                .values
                .iter()
                .filter(|value| value.candidate_id == candidate.candidate_id)
                .collect::<Vec<_>>();
            CardRewardCalibrationReplayCandidateV1 {
                candidate_id: candidate.candidate_id.clone(),
                card_id: card_id.clone(),
                original_value_sources: original_values
                    .iter()
                    .flat_map(|value| value_source_components(value))
                    .collect(),
                original_value_statuses: original_values
                    .iter()
                    .flat_map(|value| value_status_components(value))
                    .collect(),
                calibration_estimate: card_id
                    .and_then(|card_id| calibration_replay_estimate(&card_id, calibration)),
            }
        })
        .collect::<Vec<_>>();

    CardRewardCalibrationReplayExampleV1 {
        decision_record_hash: example.decision_record_hash.clone(),
        selected_candidate_id: example.selected_candidate_id.clone(),
        original_value_count: example.source_record.values.len(),
        candidate_replays,
    }
}

fn calibration_replay_estimate(
    card_id: &str,
    calibration: &CardRewardOutcomeCalibrationV1,
) -> Option<CardRewardCalibrationReplayEstimateV1> {
    let bucket = calibration
        .card_id_buckets
        .iter()
        .find(|bucket| bucket.card_id == card_id && bucket.usable_for_value_estimate)?;
    let card_mean = bucket.mean_next_combat_hp_loss?;
    let global_mean = calibration.global.mean_next_combat_hp_loss?;
    Some(CardRewardCalibrationReplayEstimateV1 {
        source: "OutcomeCalibration".to_string(),
        status: "OutcomeCalibrated".to_string(),
        survival_delta: global_mean - card_mean,
        uncertainty: bucket.uncertainty,
        outcome_sample_count: bucket.outcome_attached_count,
        usable_for_autopilot_gate: bucket.usable_for_autopilot_gate,
    })
}

fn candidate_card_id_from_candidate_id(candidate_id: &str) -> Option<String> {
    candidate_id
        .rsplit_once(':')
        .map(|(_, card_id)| card_id.to_string())
}

fn value_source_components(
    value: &crate::ai::noncombat_decision_v1::ValueEstimateV1,
) -> Vec<String> {
    value
        .components
        .iter()
        .filter(|component| component.name.starts_with("value_source_"))
        .map(|component| component.name.clone())
        .collect()
}

fn value_status_components(
    value: &crate::ai::noncombat_decision_v1::ValueEstimateV1,
) -> Vec<String> {
    value
        .components
        .iter()
        .filter(|component| component.name.starts_with("value_status_"))
        .map(|component| component.name.clone())
        .collect()
}
