use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    replay_card_reward_decision_v1, CardRewardDecisionContextV1, CardRewardPolicyConfigV1,
    CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1, CardRewardValueEligibilityV1,
    CardRewardValueEstimateV1, CardRewardValueHorizonV1, CardRewardValueSourceV1,
    CardRewardValueStatusV1,
};

use super::CardRewardValueLoopExampleV1;

pub const CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME: &str = "CardRewardRouteRiskCalibrationV1";
pub const CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardRouteRiskCalibrationV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub estimator_kind: String,
    pub total_examples: usize,
    pub evaluated_examples: usize,
    pub missing_public_packet_examples: usize,
    pub missing_outcome_examples: usize,
    pub missing_selected_route_risk_estimate_examples: usize,
    pub global: CardRewardRouteRiskCalibrationGlobalV1,
    pub buckets: Vec<CardRewardRouteRiskCalibrationBucketV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardRouteRiskCalibrationGlobalV1 {
    pub evaluated_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_route_hp_loss: Option<f32>,
    pub mean_actual_next_combat_hp_loss: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_hp_before_next_elite: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_hp_after_next_elite: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_pre_next_elite_route_hp_loss: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_next_elite_combat_hp_loss: Option<f32>,
    pub mean_predicted_route_risk_delta: Option<f32>,
    pub mean_actual_survival_delta: Option<f32>,
    pub mean_signed_error: Option<f32>,
    pub mean_absolute_error: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardRouteRiskCalibrationBucketV1 {
    pub bucket_key: String,
    pub evaluated_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_route_hp_loss: Option<f32>,
    pub mean_actual_next_combat_hp_loss: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_hp_before_next_elite: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_hp_after_next_elite: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_pre_next_elite_route_hp_loss: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_next_elite_combat_hp_loss: Option<f32>,
    pub mean_predicted_route_risk_delta: Option<f32>,
    pub mean_actual_survival_delta: Option<f32>,
    pub mean_signed_error: Option<f32>,
    pub mean_absolute_error: Option<f32>,
    pub confidence: f32,
    pub uncertainty: f32,
    pub usable_for_value_estimate: bool,
    pub usable_for_autopilot_gate: bool,
}

pub fn calibrate_card_reward_route_risk_v1(
    examples: &[CardRewardValueLoopExampleV1],
) -> CardRewardRouteRiskCalibrationV1 {
    let mut rows = Vec::new();
    let mut missing_public_packet_examples = 0;
    let mut missing_outcome_examples = 0;
    let mut missing_selected_route_risk_estimate_examples = 0;

    for example in examples {
        let Some(packet) = example.public_packet.as_ref() else {
            missing_public_packet_examples += 1;
            continue;
        };
        let Some(card_reward) = example
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.card_reward.as_ref())
        else {
            missing_outcome_examples += 1;
            continue;
        };
        let Some(actual_route_hp_loss) = route_risk_actual_hp_loss(example) else {
            missing_outcome_examples += 1;
            continue;
        };
        let Some(selected_candidate_id) = example.selected_candidate_id.as_ref() else {
            missing_selected_route_risk_estimate_examples += 1;
            continue;
        };

        let replay =
            replay_card_reward_decision_v1(packet, &CardRewardPolicyConfigV1::default(), None);
        let Some(estimate) = replay.value_estimates.iter().find(|estimate| {
            estimate.source == CardRewardValueSourceV1::RouteRisk
                && estimate_candidate_id(estimate.index, estimate.card) == *selected_candidate_id
        }) else {
            missing_selected_route_risk_estimate_examples += 1;
            continue;
        };

        rows.push(RouteRiskCalibrationRowV1 {
            decision_start_hp: example
                .outcome
                .as_ref()
                .map(|outcome| outcome.before.current_hp)
                .unwrap_or_default(),
            actual_route_hp_loss,
            next_combat_hp_loss: card_reward.next_combat_hp_loss,
            hp_before_next_elite: card_reward.hp_before_next_elite,
            hp_after_next_elite: card_reward.hp_after_next_elite,
            predicted_route_risk_delta: total_value_delta(estimate),
        });
    }

    let mean_route_hp_loss = mean_i32(rows.iter().map(|row| row.actual_route_hp_loss));
    let mut bucket_accumulators =
        BTreeMap::<String, CardRewardRouteRiskCalibrationAccumulatorV1>::new();
    let mut global = CardRewardRouteRiskCalibrationAccumulatorV1::default();

    for row in &rows {
        let actual_survival_delta = mean_route_hp_loss
            .map(|mean| mean - row.actual_route_hp_loss as f32)
            .unwrap_or(0.0);
        let bucket_key = route_risk_bucket_key(row.predicted_route_risk_delta);
        bucket_accumulators
            .entry(bucket_key.to_string())
            .or_default()
            .push(
                row.actual_route_hp_loss,
                row.next_combat_hp_loss,
                row.hp_before_next_elite,
                row.hp_after_next_elite,
                pre_next_elite_route_hp_loss(row),
                next_elite_combat_hp_loss(row),
                row.predicted_route_risk_delta,
                actual_survival_delta,
            );
        global.push(
            row.actual_route_hp_loss,
            row.next_combat_hp_loss,
            row.hp_before_next_elite,
            row.hp_after_next_elite,
            pre_next_elite_route_hp_loss(row),
            next_elite_combat_hp_loss(row),
            row.predicted_route_risk_delta,
            actual_survival_delta,
        );
    }

    CardRewardRouteRiskCalibrationV1 {
        schema_name: CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        estimator_kind: "route_risk_selected_candidate_route_window_v1".to_string(),
        total_examples: examples.len(),
        evaluated_examples: rows.len(),
        missing_public_packet_examples,
        missing_outcome_examples,
        missing_selected_route_risk_estimate_examples,
        global: global.into_global(),
        buckets: bucket_accumulators
            .into_iter()
            .map(|(bucket_key, accumulator)| accumulator.into_bucket(bucket_key))
            .collect(),
    }
}

pub fn estimate_card_reward_values_from_route_risk_calibration_v1(
    context: &CardRewardDecisionContextV1,
    calibration: &CardRewardRouteRiskCalibrationV1,
) -> Vec<CardRewardValueEstimateV1> {
    let packet =
        crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1::from_context(context);
    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);
    replay
        .value_estimates
        .iter()
        .filter(|estimate| estimate.source == CardRewardValueSourceV1::RouteRisk)
        .filter_map(|estimate| corrected_route_risk_estimate(estimate, calibration))
        .collect()
}

fn corrected_route_risk_estimate(
    estimate: &CardRewardValueEstimateV1,
    calibration: &CardRewardRouteRiskCalibrationV1,
) -> Option<CardRewardValueEstimateV1> {
    let raw_total = total_value_delta(estimate);
    let bucket_key = route_risk_bucket_key(raw_total);
    let bucket = calibration
        .buckets
        .iter()
        .find(|bucket| bucket.bucket_key == bucket_key && bucket.usable_for_value_estimate)?;
    let signed_error = bucket.mean_signed_error.unwrap_or(0.0);
    let corrected_total = raw_total - signed_error;
    Some(CardRewardValueEstimateV1 {
        index: estimate.index,
        card: estimate.card,
        source: CardRewardValueSourceV1::RouteRisk,
        status: CardRewardValueStatusV1::RouteRiskCalibrated,
        survival_delta: corrected_total,
        progress_delta: 0.0,
        deck_consistency_delta: 0.0,
        uncertainty: bucket.uncertainty.max(0.36),
        eligibility: CardRewardValueEligibilityV1 {
            usable_for_value_estimate: true,
            usable_for_autopilot_gate: false,
            reasons: vec![CardRewardValueEligibilityReasonV1::RouteRiskCalibrationNotGateEligible],
            bucket_key: Some(bucket.bucket_key.clone()),
            horizon: Some(CardRewardValueHorizonV1::VisibleRouteRisk),
            outcome_sample_count: Some(bucket.evaluated_count),
        },
        components: vec![
            CardRewardValueComponentV1 {
                name: "route_risk_raw_total_delta".to_string(),
                value: raw_total,
            },
            CardRewardValueComponentV1 {
                name: "route_risk_bucket_mean_signed_error".to_string(),
                value: signed_error,
            },
            CardRewardValueComponentV1 {
                name: "route_risk_corrected_total_delta".to_string(),
                value: corrected_total,
            },
            CardRewardValueComponentV1 {
                name: "route_risk_calibration_bucket_count".to_string(),
                value: bucket.evaluated_count as f32,
            },
        ],
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RouteRiskCalibrationRowV1 {
    decision_start_hp: i32,
    actual_route_hp_loss: i32,
    next_combat_hp_loss: Option<i32>,
    hp_before_next_elite: Option<i32>,
    hp_after_next_elite: Option<i32>,
    predicted_route_risk_delta: f32,
}

#[derive(Default)]
struct CardRewardRouteRiskCalibrationAccumulatorV1 {
    route_hp_losses: Vec<i32>,
    next_combat_hp_losses: Vec<i32>,
    hp_before_next_elites: Vec<i32>,
    hp_after_next_elites: Vec<i32>,
    pre_next_elite_route_hp_losses: Vec<i32>,
    next_elite_combat_hp_losses: Vec<i32>,
    predicted_route_risk_deltas: Vec<f32>,
    actual_survival_deltas: Vec<f32>,
    signed_errors: Vec<f32>,
    absolute_errors: Vec<f32>,
}

impl CardRewardRouteRiskCalibrationAccumulatorV1 {
    fn push(
        &mut self,
        actual_route_hp_loss: i32,
        next_combat_hp_loss: Option<i32>,
        hp_before_next_elite: Option<i32>,
        hp_after_next_elite: Option<i32>,
        pre_next_elite_route_hp_loss: Option<i32>,
        next_elite_combat_hp_loss: Option<i32>,
        predicted_route_risk_delta: f32,
        actual_survival_delta: f32,
    ) {
        let signed_error = predicted_route_risk_delta - actual_survival_delta;
        self.route_hp_losses.push(actual_route_hp_loss);
        if let Some(next_combat_hp_loss) = next_combat_hp_loss {
            self.next_combat_hp_losses.push(next_combat_hp_loss);
        }
        if let Some(hp_before_next_elite) = hp_before_next_elite {
            self.hp_before_next_elites.push(hp_before_next_elite);
        }
        if let Some(hp_after_next_elite) = hp_after_next_elite {
            self.hp_after_next_elites.push(hp_after_next_elite);
        }
        if let Some(pre_next_elite_route_hp_loss) = pre_next_elite_route_hp_loss {
            self.pre_next_elite_route_hp_losses
                .push(pre_next_elite_route_hp_loss);
        }
        if let Some(next_elite_combat_hp_loss) = next_elite_combat_hp_loss {
            self.next_elite_combat_hp_losses
                .push(next_elite_combat_hp_loss);
        }
        self.predicted_route_risk_deltas
            .push(predicted_route_risk_delta);
        self.actual_survival_deltas.push(actual_survival_delta);
        self.signed_errors.push(signed_error);
        self.absolute_errors.push(signed_error.abs());
    }

    fn into_global(self) -> CardRewardRouteRiskCalibrationGlobalV1 {
        CardRewardRouteRiskCalibrationGlobalV1 {
            evaluated_count: self.route_hp_losses.len(),
            mean_actual_route_hp_loss: mean_i32(self.route_hp_losses.into_iter()),
            mean_actual_next_combat_hp_loss: mean_i32(self.next_combat_hp_losses.into_iter()),
            mean_actual_hp_before_next_elite: mean_i32(self.hp_before_next_elites.into_iter()),
            mean_actual_hp_after_next_elite: mean_i32(self.hp_after_next_elites.into_iter()),
            mean_pre_next_elite_route_hp_loss: mean_i32(
                self.pre_next_elite_route_hp_losses.into_iter(),
            ),
            mean_next_elite_combat_hp_loss: mean_i32(self.next_elite_combat_hp_losses.into_iter()),
            mean_predicted_route_risk_delta: mean_f32(self.predicted_route_risk_deltas.into_iter()),
            mean_actual_survival_delta: mean_f32(self.actual_survival_deltas.into_iter()),
            mean_signed_error: mean_f32(self.signed_errors.into_iter()),
            mean_absolute_error: mean_f32(self.absolute_errors.into_iter()),
        }
    }

    fn into_bucket(self, bucket_key: String) -> CardRewardRouteRiskCalibrationBucketV1 {
        let evaluated_count = self.route_hp_losses.len();
        let confidence = evaluated_count as f32 / (evaluated_count as f32 + 3.0);
        let uncertainty = 1.0 - confidence;
        CardRewardRouteRiskCalibrationBucketV1 {
            bucket_key,
            evaluated_count,
            mean_actual_route_hp_loss: mean_i32(self.route_hp_losses.into_iter()),
            mean_actual_next_combat_hp_loss: mean_i32(self.next_combat_hp_losses.into_iter()),
            mean_actual_hp_before_next_elite: mean_i32(self.hp_before_next_elites.into_iter()),
            mean_actual_hp_after_next_elite: mean_i32(self.hp_after_next_elites.into_iter()),
            mean_pre_next_elite_route_hp_loss: mean_i32(
                self.pre_next_elite_route_hp_losses.into_iter(),
            ),
            mean_next_elite_combat_hp_loss: mean_i32(self.next_elite_combat_hp_losses.into_iter()),
            mean_predicted_route_risk_delta: mean_f32(self.predicted_route_risk_deltas.into_iter()),
            mean_actual_survival_delta: mean_f32(self.actual_survival_deltas.into_iter()),
            mean_signed_error: mean_f32(self.signed_errors.into_iter()),
            mean_absolute_error: mean_f32(self.absolute_errors.into_iter()),
            confidence,
            uncertainty,
            usable_for_value_estimate: evaluated_count > 0,
            usable_for_autopilot_gate: false,
        }
    }
}

fn route_risk_actual_hp_loss(example: &CardRewardValueLoopExampleV1) -> Option<i32> {
    let outcome = example.outcome.as_ref()?;
    let card_reward = outcome.card_reward.as_ref()?;
    if let Some(hp_after_next_elite) = card_reward.hp_after_next_elite {
        return Some((outcome.before.current_hp - hp_after_next_elite).max(0));
    }
    card_reward.next_combat_hp_loss
}

fn pre_next_elite_route_hp_loss(row: &RouteRiskCalibrationRowV1) -> Option<i32> {
    row.hp_before_next_elite
        .map(|hp_before_next_elite| (row.decision_start_hp - hp_before_next_elite).max(0))
}

fn next_elite_combat_hp_loss(row: &RouteRiskCalibrationRowV1) -> Option<i32> {
    match (row.hp_before_next_elite, row.hp_after_next_elite) {
        (Some(before), Some(after)) => Some((before - after).max(0)),
        _ => None,
    }
}

fn route_risk_bucket_key(predicted_route_risk_delta: f32) -> &'static str {
    if predicted_route_risk_delta >= 0.25 {
        "route_risk_delta:positive"
    } else if predicted_route_risk_delta <= -0.05 {
        "route_risk_delta:negative"
    } else {
        "route_risk_delta:neutral"
    }
}

fn estimate_candidate_id(index: usize, card: crate::content::cards::CardId) -> String {
    format!("card_reward:{index}:{card:?}")
}

fn total_value_delta(
    estimate: &crate::ai::card_reward_policy_v1::CardRewardValueEstimateV1,
) -> f32 {
    estimate.survival_delta + estimate.progress_delta + estimate.deck_consistency_delta
}

fn mean_i32(values: impl Iterator<Item = i32>) -> Option<f32> {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<i32>() as f32 / values.len() as f32)
}

fn mean_f32(values: impl Iterator<Item = f32>) -> Option<f32> {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f32>() / values.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::card_reward_policy_v1::{
        build_card_reward_decision_context_v1, CardRewardRouteEvidenceV1,
        CardRewardSelectedRouteV1, PublicRewardDecisionPacketV1,
    };
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
    use crate::state::rewards::RewardCard;
    use crate::state::run::RunState;

    #[test]
    fn route_risk_calibration_replays_selected_estimates_against_attached_outcomes() {
        let examples = vec![
            route_risk_example(CardId::TwinStrike, 521, 4),
            route_risk_example(CardId::Warcry, 522, 12),
        ];

        let calibration = calibrate_card_reward_route_risk_v1(&examples);

        assert_eq!(
            calibration.schema_name,
            CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME
        );
        assert_eq!(calibration.total_examples, 2);
        assert_eq!(calibration.evaluated_examples, 2);
        assert_eq!(calibration.global.evaluated_count, 2);
        assert_eq!(
            calibration.global.mean_actual_next_combat_hp_loss,
            Some(8.0)
        );
        assert!(calibration.global.mean_absolute_error.is_some());
        assert!(calibration
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_key == "route_risk_delta:positive"));
        assert!(calibration
            .buckets
            .iter()
            .all(|bucket| !bucket.usable_for_autopilot_gate));
    }

    #[test]
    fn route_risk_calibration_reports_missing_packet_and_outcome_coverage() {
        let mut missing_packet = route_risk_example(CardId::TwinStrike, 521, 4);
        missing_packet.public_packet = None;
        let mut missing_outcome = route_risk_example(CardId::Warcry, 522, 12);
        missing_outcome.outcome = None;

        let calibration = calibrate_card_reward_route_risk_v1(&[missing_packet, missing_outcome]);

        assert_eq!(calibration.total_examples, 2);
        assert_eq!(calibration.evaluated_examples, 0);
        assert_eq!(calibration.missing_public_packet_examples, 1);
        assert_eq!(calibration.missing_outcome_examples, 1);
        assert!(calibration.buckets.is_empty());
    }

    #[test]
    fn route_risk_calibration_uses_next_elite_window_when_next_combat_loss_is_absent() {
        let examples = vec![
            route_risk_next_elite_example(CardId::TwinStrike, 521, 70, 62),
            route_risk_next_elite_example(CardId::Warcry, 522, 62, 50),
        ];

        let calibration = calibrate_card_reward_route_risk_v1(&examples);

        assert_eq!(calibration.evaluated_examples, 2);
        assert_eq!(calibration.missing_outcome_examples, 0);
        assert_eq!(calibration.global.mean_actual_next_combat_hp_loss, None);
        assert_eq!(
            calibration.global.mean_actual_hp_after_next_elite,
            Some(56.0)
        );
        assert_eq!(
            calibration.global.mean_actual_hp_before_next_elite,
            Some(66.0)
        );
        assert_eq!(calibration.global.mean_actual_route_hp_loss, Some(24.0));
        assert_eq!(
            calibration.global.mean_pre_next_elite_route_hp_loss,
            Some(14.0)
        );
        assert_eq!(
            calibration.global.mean_next_elite_combat_hp_loss,
            Some(10.0)
        );
        assert!(calibration
            .buckets
            .iter()
            .any(|bucket| bucket.mean_actual_hp_after_next_elite.is_some()));
    }

    #[test]
    fn route_risk_calibration_generates_corrected_non_gate_estimates() {
        let examples = vec![
            route_risk_example(CardId::TwinStrike, 521, 4),
            route_risk_example(CardId::Warcry, 522, 12),
        ];
        let calibration = calibrate_card_reward_route_risk_v1(&examples);
        let context = examples[0]
            .public_packet
            .as_ref()
            .expect("fixture should include a packet")
            .context
            .clone();

        let estimates =
            estimate_card_reward_values_from_route_risk_calibration_v1(&context, &calibration);

        assert_eq!(estimates.len(), 2);
        assert!(estimates
            .iter()
            .all(|estimate| estimate.status == CardRewardValueStatusV1::RouteRiskCalibrated));
        assert!(estimates
            .iter()
            .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
        assert!(estimates.iter().all(|estimate| estimate.uncertainty > 0.35));
        assert!(estimates.iter().all(|estimate| {
            estimate
                .eligibility
                .reasons
                .contains(&CardRewardValueEligibilityReasonV1::RouteRiskCalibrationNotGateEligible)
        }));
    }

    #[test]
    fn calibrated_route_risk_estimate_is_preferred_over_raw_route_risk_but_still_blocks_gate() {
        let examples = vec![
            route_risk_example(CardId::TwinStrike, 521, 4),
            route_risk_example(CardId::Warcry, 522, 12),
        ];
        let calibration = calibrate_card_reward_route_risk_v1(&examples);
        let context = examples[0]
            .public_packet
            .as_ref()
            .expect("fixture should include a packet")
            .context
            .clone();
        let inputs =
            crate::eval::card_reward_value_loop::build_card_reward_runtime_estimator_inputs_v1(
                &context,
                crate::eval::card_reward_value_loop::CardRewardRuntimeEstimatorCalibrationsV1 {
                    outcome: None,
                    route_risk: Some(&calibration),
                    strategy_package: None,
                },
            );

        let decision =
            crate::ai::card_reward_policy_v1::plan_card_reward_decision_with_estimator_inputs_v1(
                &context,
                &CardRewardPolicyConfigV1::default(),
                &inputs,
            );

        assert!(decision
            .value_arbitration
            .gate_value_estimates
            .iter()
            .all(|estimate| estimate.status == CardRewardValueStatusV1::RouteRiskCalibrated));
        assert!(!decision.autopilot_gate.value_source_eligible);
        assert!(matches!(
            decision.action,
            crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Stop { .. }
        ));
    }

    fn route_risk_example(
        selected_card: CardId,
        seed: u64,
        next_combat_hp_loss: i32,
    ) -> CardRewardValueLoopExampleV1 {
        let run_state = RunState::new(seed, 0, false, "Ironclad");
        let context = build_card_reward_decision_context_v1(
            &run_state,
            vec![
                RewardCard::new(CardId::TwinStrike, 0),
                RewardCard::new(CardId::Warcry, 0),
            ],
            None,
        );
        let mut context = context;
        context.route = Some(route_with_combat_pressure());
        let selected_index = if selected_card == CardId::TwinStrike {
            0
        } else {
            1
        };
        let selected_candidate_id = format!("card_reward:{selected_index}:{selected_card:?}");
        let decision_record_hash = format!("route-risk-hash-{seed}-{selected_card:?}");
        CardRewardValueLoopExampleV1 {
            schema_name: super::super::CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME.to_string(),
            schema_version: super::super::CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            source_trace_schema_name: Some(
                crate::eval::run_control::SESSION_TRACE_SCHEMA_NAME.to_string(),
            ),
            source_trace_schema_version: Some(
                crate::eval::run_control::SESSION_TRACE_SCHEMA_VERSION,
            ),
            source_run_config: Some(super::super::CardRewardValueLoopRunConfigV1 {
                seed,
                ascension_level: 0,
                player_class: "Ironclad".to_string(),
                final_act: false,
            }),
            trace_step_index: Some(0),
            trace_boundary_record_index: None,
            decision_record_hash: decision_record_hash.clone(),
            decision_site: DecisionSiteKindV1::CardReward,
            replay_status: super::super::CardRewardValueLoopReplayStatusV1::FullPublicPacket,
            outcome_status: super::super::CardRewardValueLoopOutcomeStatusV1::Attached,
            selected_candidate_id: Some(selected_candidate_id.clone()),
            selection_status: PolicySelectionStatusV1::Selected,
            selection_reason: "test selected card reward".to_string(),
            candidate_count: 2,
            value_estimate_count: 0,
            source_record: test_record(selected_candidate_id.clone()),
            public_packet: Some(PublicRewardDecisionPacketV1::from_context(&context)),
            outcome: Some(test_outcome(
                decision_record_hash,
                selected_candidate_id,
                next_combat_hp_loss,
            )),
        }
    }

    fn route_risk_next_elite_example(
        selected_card: CardId,
        seed: u64,
        hp_before_next_elite: i32,
        hp_after_next_elite: i32,
    ) -> CardRewardValueLoopExampleV1 {
        let mut example = route_risk_example(selected_card, seed, 0);
        if let Some(outcome) = example.outcome.as_mut() {
            outcome.window = NonCombatOutcomeWindowV1::AfterNextElite;
            outcome.after = test_outcome_snapshot(hp_after_next_elite);
            outcome.metrics.hp_delta = hp_after_next_elite - outcome.before.current_hp;
            outcome.metrics.floor_delta = 5;
            outcome.metrics.combats_completed_delta = 3;
            outcome.metrics.elites_completed_delta = 1;
            if let Some(card_reward) = outcome.card_reward.as_mut() {
                card_reward.next_combat_hp_loss = None;
                card_reward.hp_before_next_elite = Some(hp_before_next_elite);
                card_reward.hp_after_next_elite = Some(hp_after_next_elite);
                card_reward.floor_reached_after_decision = 6;
            }
        }
        example
    }

    fn route_with_combat_pressure() -> CardRewardRouteEvidenceV1 {
        CardRewardRouteEvidenceV1 {
            route_policy: "test_route_policy".to_string(),
            selected_route: Some(CardRewardSelectedRouteV1 {
                next_x: 1,
                next_y: 1,
                min_fires: 1,
                max_fires: 2,
                first_fire_floor: Some(6),
                min_elites: 0,
                max_elites: 1,
                min_early_pressure: 1,
                max_early_pressure: 3,
            }),
            candidate_count: 2,
            need_card_rewards: 0.9,
            need_upgrade: 0.4,
            need_heal: 0.2,
            can_take_elite: 0.2,
            avoid_damage: 0.7,
            warnings: Vec::new(),
        }
    }

    fn test_record(selected_candidate_id: String) -> NonCombatDecisionRecordV1 {
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
                candidate_id: selected_candidate_id.clone(),
                site: DecisionSiteKindV1::CardReward,
                label: selected_candidate_id.clone(),
                action_plan: PublicActionPlanV1 {
                    summary: selected_candidate_id.clone(),
                    command: Some("pick 0".to_string()),
                },
                information_classes: vec![InformationClassV1::PublicObservation],
                uncertainty_notes: Vec::new(),
            }],
            evidence: EvidenceBundleV1::default(),
            values: Vec::new(),
            selection: PolicySelectionV1 {
                status: PolicySelectionStatusV1::Selected,
                selected_candidate_id: Some(selected_candidate_id),
                reason: "test selected card reward".to_string(),
                confidence: 1.0,
                selection_mode: "test".to_string(),
            },
        }
    }

    fn test_outcome(
        decision_record_hash: String,
        selected_candidate_id: String,
        next_combat_hp_loss: i32,
    ) -> NonCombatOutcomeAttachmentV1 {
        NonCombatOutcomeAttachmentV1 {
            schema_name: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            site: DecisionSiteKindV1::CardReward,
            decision_record_hash,
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
                selected_candidate_id,
                picked_card_label: "test picked card".to_string(),
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
