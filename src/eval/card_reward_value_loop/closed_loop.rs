use serde::{Deserialize, Serialize};

use super::{
    calibrate_card_reward_route_risk_v1, CardRewardCalibrationReplayReportV1,
    CardRewardOutcomeCalibrationV1, CardRewardRouteRiskCalibrationV1, CardRewardValueLoopExampleV1,
    HistogramEntryV1,
};

pub const CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_NAME: &str = "CardRewardClosedLoopReportV1";
pub const CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardClosedLoopReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub calibration_source: String,
    pub calibration_total_examples: usize,
    pub calibration_usable_outcome_examples: usize,
    pub calibration_bucket_count: usize,
    pub route_risk_calibration: CardRewardClosedLoopRouteRiskSummaryV1,
    pub summary: CardRewardClosedLoopSummaryV1,
    pub examples: Vec<CardRewardClosedLoopExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardClosedLoopRouteRiskSummaryV1 {
    pub total_examples: usize,
    pub evaluated_examples: usize,
    pub missing_public_packet_examples: usize,
    pub missing_outcome_examples: usize,
    pub missing_selected_route_risk_estimate_examples: usize,
    pub bucket_count: usize,
    pub mean_absolute_error: Option<f32>,
    pub mean_signed_error: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardClosedLoopSummaryV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub total_examples: usize,
    pub full_public_packet_replay_count: usize,
    pub calibration_candidate_estimate_count: usize,
    pub calibration_autopilot_gate_usable_candidate_count: usize,
    pub calibration_autopilot_gate_blocked_candidate_count: usize,
    pub calibration_entered_arbitration_count: usize,
    pub gate_selected_count: usize,
    pub blocked_by_gate_count: usize,
    pub missing_data_count: usize,
    pub status_counts: Vec<HistogramEntryV1>,
    pub gate_blocker_counts: Vec<HistogramEntryV1>,
    pub eligibility_reason_counts: Vec<HistogramEntryV1>,
    pub missing_reason_counts: Vec<HistogramEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardClosedLoopExampleV1 {
    pub decision_record_hash: String,
    pub status: CardRewardClosedLoopStatusV1,
    pub reasons: Vec<String>,
    pub policy_replay_status: String,
    pub policy_selected_candidate_id: Option<String>,
    pub policy_value_sources: Vec<String>,
    pub policy_gate_blockers: Vec<String>,
    pub calibration_candidate_estimate_count: usize,
    pub calibration_autopilot_gate_usable_candidate_count: usize,
    pub calibration_autopilot_gate_blocked_candidate_count: usize,
    pub calibration_eligibility_reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardClosedLoopStatusV1 {
    GateSelected,
    BlockedByGate,
    MissingData,
}

pub fn build_card_reward_closed_loop_report_v1(
    examples: &[CardRewardValueLoopExampleV1],
    calibration: &CardRewardOutcomeCalibrationV1,
    calibration_source: impl Into<String>,
) -> CardRewardClosedLoopReportV1 {
    let replay = super::replay_card_reward_records_with_calibration_v1(examples, calibration);
    let summary = summarize_card_reward_closed_loop_v1(&replay);
    let route_risk_calibration = calibrate_card_reward_route_risk_v1(examples);
    let examples = replay
        .examples
        .iter()
        .map(classify_closed_loop_example_v1)
        .collect::<Vec<_>>();
    CardRewardClosedLoopReportV1 {
        schema_name: CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_CLOSED_LOOP_REPORT_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        calibration_source: calibration_source.into(),
        calibration_total_examples: calibration.total_examples,
        calibration_usable_outcome_examples: calibration.usable_outcome_examples,
        calibration_bucket_count: calibration.card_id_buckets.len(),
        route_risk_calibration: CardRewardClosedLoopRouteRiskSummaryV1::from(
            &route_risk_calibration,
        ),
        summary,
        examples,
    }
}

impl From<&CardRewardRouteRiskCalibrationV1> for CardRewardClosedLoopRouteRiskSummaryV1 {
    fn from(calibration: &CardRewardRouteRiskCalibrationV1) -> Self {
        Self {
            total_examples: calibration.total_examples,
            evaluated_examples: calibration.evaluated_examples,
            missing_public_packet_examples: calibration.missing_public_packet_examples,
            missing_outcome_examples: calibration.missing_outcome_examples,
            missing_selected_route_risk_estimate_examples: calibration
                .missing_selected_route_risk_estimate_examples,
            bucket_count: calibration.buckets.len(),
            mean_absolute_error: calibration.global.mean_absolute_error,
            mean_signed_error: calibration.global.mean_signed_error,
        }
    }
}

pub fn summarize_card_reward_closed_loop_v1(
    replay: &CardRewardCalibrationReplayReportV1,
) -> CardRewardClosedLoopSummaryV1 {
    let examples = replay
        .examples
        .iter()
        .map(classify_closed_loop_example_v1)
        .collect::<Vec<_>>();
    let mut status_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut gate_blocker_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut eligibility_reason_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut missing_reason_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut full_public_packet_replay_count = 0;
    let mut calibration_candidate_estimate_count = 0;
    let mut calibration_autopilot_gate_usable_candidate_count = 0;
    let mut calibration_autopilot_gate_blocked_candidate_count = 0;
    let mut calibration_entered_arbitration_count = 0;
    let mut gate_selected_count = 0;
    let mut blocked_by_gate_count = 0;
    let mut missing_data_count = 0;

    for example in &examples {
        *status_counts
            .entry(closed_loop_status_label(example.status).to_string())
            .or_default() += 1;
        if example.policy_replay_status == "full_public_packet_replay" {
            full_public_packet_replay_count += 1;
        }
        calibration_candidate_estimate_count += example.calibration_candidate_estimate_count;
        calibration_autopilot_gate_usable_candidate_count +=
            example.calibration_autopilot_gate_usable_candidate_count;
        calibration_autopilot_gate_blocked_candidate_count +=
            example.calibration_autopilot_gate_blocked_candidate_count;
        for reason in &example.calibration_eligibility_reasons {
            *eligibility_reason_counts.entry(reason.clone()).or_default() += 1;
        }
        if example
            .policy_value_sources
            .iter()
            .any(|source| source == "OutcomeCalibration")
        {
            calibration_entered_arbitration_count += 1;
        }
        match example.status {
            CardRewardClosedLoopStatusV1::GateSelected => gate_selected_count += 1,
            CardRewardClosedLoopStatusV1::BlockedByGate => {
                blocked_by_gate_count += 1;
                for blocker in &example.policy_gate_blockers {
                    *gate_blocker_counts.entry(blocker.clone()).or_default() += 1;
                }
            }
            CardRewardClosedLoopStatusV1::MissingData => {
                missing_data_count += 1;
                for reason in &example.reasons {
                    *missing_reason_counts.entry(reason.clone()).or_default() += 1;
                }
            }
        }
    }

    CardRewardClosedLoopSummaryV1 {
        schema_name: "CardRewardClosedLoopSummaryV1".to_string(),
        schema_version: 1,
        total_examples: replay.total_examples,
        full_public_packet_replay_count,
        calibration_candidate_estimate_count,
        calibration_autopilot_gate_usable_candidate_count,
        calibration_autopilot_gate_blocked_candidate_count,
        calibration_entered_arbitration_count,
        gate_selected_count,
        blocked_by_gate_count,
        missing_data_count,
        status_counts: histogram_entries(status_counts),
        gate_blocker_counts: histogram_entries(gate_blocker_counts),
        eligibility_reason_counts: histogram_entries(eligibility_reason_counts),
        missing_reason_counts: histogram_entries(missing_reason_counts),
    }
}

fn classify_closed_loop_example_v1(
    example: &super::CardRewardCalibrationReplayExampleV1,
) -> CardRewardClosedLoopExampleV1 {
    let calibration_candidate_estimate_count = example
        .candidate_replays
        .iter()
        .filter(|candidate| candidate.calibration_estimate.is_some())
        .count();
    let calibration_autopilot_gate_usable_candidate_count = example
        .candidate_replays
        .iter()
        .filter_map(|candidate| candidate.calibration_estimate.as_ref())
        .filter(|estimate| estimate.usable_for_autopilot_gate)
        .count();
    let calibration_autopilot_gate_blocked_candidate_count = example
        .candidate_replays
        .iter()
        .filter_map(|candidate| candidate.calibration_estimate.as_ref())
        .filter(|estimate| !estimate.usable_for_autopilot_gate)
        .count();
    let calibration_eligibility_reasons = example
        .candidate_replays
        .iter()
        .filter_map(|candidate| candidate.calibration_estimate.as_ref())
        .flat_map(|estimate| estimate.eligibility_reasons.iter().cloned())
        .collect::<Vec<_>>();
    let calibration_entered_arbitration = example
        .policy_value_sources
        .iter()
        .any(|source| source == "OutcomeCalibration");
    let mut reasons = Vec::new();

    if example.policy_replay_status != "full_public_packet_replay" {
        reasons.push("missing_public_packet".to_string());
    }
    if calibration_candidate_estimate_count == 0 {
        reasons.push("missing_calibration_candidate_estimate".to_string());
    }
    if example.policy_replay_status == "full_public_packet_replay"
        && calibration_candidate_estimate_count > 0
        && !calibration_entered_arbitration
    {
        reasons.push("calibration_not_selected_for_gate".to_string());
    }

    let status = if !reasons.is_empty() {
        CardRewardClosedLoopStatusV1::MissingData
    } else if example.policy_selected_candidate_id.is_some() {
        CardRewardClosedLoopStatusV1::GateSelected
    } else {
        reasons.push("autopilot_gate_blocked_calibrated_estimate".to_string());
        CardRewardClosedLoopStatusV1::BlockedByGate
    };

    CardRewardClosedLoopExampleV1 {
        decision_record_hash: example.decision_record_hash.clone(),
        status,
        reasons,
        policy_replay_status: example.policy_replay_status.clone(),
        policy_selected_candidate_id: example.policy_selected_candidate_id.clone(),
        policy_value_sources: example.policy_value_sources.clone(),
        policy_gate_blockers: example.policy_gate_blockers.clone(),
        calibration_candidate_estimate_count,
        calibration_autopilot_gate_usable_candidate_count,
        calibration_autopilot_gate_blocked_candidate_count,
        calibration_eligibility_reasons,
    }
}

fn closed_loop_status_label(status: CardRewardClosedLoopStatusV1) -> &'static str {
    match status {
        CardRewardClosedLoopStatusV1::GateSelected => "gate_selected",
        CardRewardClosedLoopStatusV1::BlockedByGate => "blocked_by_gate",
        CardRewardClosedLoopStatusV1::MissingData => "missing_data",
    }
}

fn histogram_entries(
    histogram: std::collections::BTreeMap<String, usize>,
) -> Vec<HistogramEntryV1> {
    histogram
        .into_iter()
        .map(|(key, count)| HistogramEntryV1 { key, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::eval::card_reward_value_loop::{
        summarize_card_reward_closed_loop_v1, CardRewardCalibrationReplayCandidateV1,
        CardRewardCalibrationReplayExampleV1, CardRewardCalibrationReplayReportV1,
        CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_NAME, CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_VERSION,
    };

    #[test]
    fn closed_loop_classifies_calibrated_replay_blocked_by_gate() {
        let replay = replay_report(vec![CardRewardCalibrationReplayExampleV1 {
            decision_record_hash: "hash-a".to_string(),
            selected_candidate_id: None,
            policy_replay_status: "full_public_packet_replay".to_string(),
            policy_selected_candidate_id: None,
            policy_input_value_sources: vec!["OutcomeCalibration".to_string()],
            policy_value_sources: vec!["OutcomeCalibration".to_string()],
            policy_gate_blockers: vec!["IneligibleValueSource".to_string()],
            original_value_count: 1,
            candidate_replays: vec![candidate_with_calibration("card_reward:0:TwinStrike")],
        }]);

        let summary = summarize_card_reward_closed_loop_v1(&replay);

        assert_eq!(summary.total_examples, 1);
        assert_eq!(summary.full_public_packet_replay_count, 1);
        assert_eq!(summary.calibration_entered_arbitration_count, 1);
        assert_eq!(summary.blocked_by_gate_count, 1);
        assert_eq!(summary.missing_data_count, 0);
        assert_eq!(
            summary.calibration_autopilot_gate_blocked_candidate_count,
            1
        );
        assert!(summary
            .gate_blocker_counts
            .iter()
            .any(|entry| entry.key == "IneligibleValueSource" && entry.count == 1));
        assert!(summary.eligibility_reason_counts.iter().any(|entry| {
            entry.key == "OutcomeCalibrationBucketNotGateEligible" && entry.count == 1
        }));
    }

    #[test]
    fn closed_loop_classifies_missing_data_without_public_packet() {
        let replay = replay_report(vec![CardRewardCalibrationReplayExampleV1 {
            decision_record_hash: "hash-b".to_string(),
            selected_candidate_id: None,
            policy_replay_status: "record_only_no_public_packet".to_string(),
            policy_selected_candidate_id: None,
            policy_input_value_sources: Vec::new(),
            policy_value_sources: Vec::new(),
            policy_gate_blockers: Vec::new(),
            original_value_count: 1,
            candidate_replays: vec![candidate_with_calibration("card_reward:0:TwinStrike")],
        }]);

        let summary = summarize_card_reward_closed_loop_v1(&replay);

        assert_eq!(summary.total_examples, 1);
        assert_eq!(summary.full_public_packet_replay_count, 0);
        assert_eq!(summary.calibration_entered_arbitration_count, 0);
        assert_eq!(summary.blocked_by_gate_count, 0);
        assert_eq!(summary.missing_data_count, 1);
    }

    fn replay_report(
        examples: Vec<CardRewardCalibrationReplayExampleV1>,
    ) -> CardRewardCalibrationReplayReportV1 {
        CardRewardCalibrationReplayReportV1 {
            schema_name: CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_CALIBRATION_REPLAY_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            policy_replay_status: "mixed_public_packet_and_record".to_string(),
            total_examples: examples.len(),
            replayed_examples: examples.len(),
            examples,
        }
    }

    fn candidate_with_calibration(candidate_id: &str) -> CardRewardCalibrationReplayCandidateV1 {
        CardRewardCalibrationReplayCandidateV1 {
            candidate_id: candidate_id.to_string(),
            card_id: Some("TwinStrike".to_string()),
            original_value_sources: Vec::new(),
            original_value_statuses: Vec::new(),
            policy_value_summary: Vec::new(),
            calibration_estimate: Some(
                crate::eval::card_reward_value_loop::CardRewardCalibrationReplayEstimateV1 {
                    source: "OutcomeCalibration".to_string(),
                    status: "OutcomeCalibrated".to_string(),
                    survival_delta: 2.0,
                    uncertainty: 0.2,
                    outcome_sample_count: 4,
                    usable_for_autopilot_gate: false,
                    eligibility_reasons: vec!["OutcomeCalibrationBucketNotGateEligible".to_string()],
                    horizon: Some("NextCombatHpLoss".to_string()),
                },
            ),
        }
    }
}
