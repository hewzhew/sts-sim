use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    replay_card_reward_decision_v1, CardRewardPolicyConfigV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1,
};
use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;
use crate::content::cards::CardId;

use super::CardRewardValueLoopExampleV1;

pub const CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME: &str =
    "CardRewardStrategyPackageCalibrationV1";
pub const CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardStrategyPackageCalibrationV1 {
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
    pub missing_selected_strategy_package_estimate_examples: usize,
    pub global: CardRewardStrategyPackageCalibrationGlobalV1,
    pub buckets: Vec<CardRewardStrategyPackageCalibrationBucketV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardStrategyPackageCalibrationGlobalV1 {
    pub evaluated_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_route_hp_loss: Option<f32>,
    pub mean_actual_next_combat_hp_loss: Option<f32>,
    pub mean_predicted_strategy_package_delta: Option<f32>,
    pub mean_actual_survival_delta: Option<f32>,
    pub mean_signed_error: Option<f32>,
    pub mean_absolute_error: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardStrategyPackageCalibrationBucketV1 {
    pub bucket_key: String,
    pub evaluated_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_actual_route_hp_loss: Option<f32>,
    pub mean_actual_next_combat_hp_loss: Option<f32>,
    pub mean_predicted_strategy_package_delta: Option<f32>,
    pub mean_actual_survival_delta: Option<f32>,
    pub mean_signed_error: Option<f32>,
    pub mean_absolute_error: Option<f32>,
    pub confidence: f32,
    pub uncertainty: f32,
    pub usable_for_value_estimate: bool,
    pub usable_for_autopilot_gate: bool,
}

pub fn calibrate_card_reward_strategy_package_v1(
    examples: &[CardRewardValueLoopExampleV1],
) -> CardRewardStrategyPackageCalibrationV1 {
    let mut rows = Vec::new();
    let mut missing_public_packet_examples = 0;
    let mut missing_outcome_examples = 0;
    let mut missing_selected_strategy_package_estimate_examples = 0;

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
        let Some(actual_route_hp_loss) = actual_hp_loss(example) else {
            missing_outcome_examples += 1;
            continue;
        };
        let Some(selected_candidate_id) = example.selected_candidate_id.as_ref() else {
            missing_selected_strategy_package_estimate_examples += 1;
            continue;
        };

        let replay =
            replay_card_reward_decision_v1(packet, &CardRewardPolicyConfigV1::default(), None);
        let Some(estimate) = replay.value_estimates.iter().find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate_candidate_id(estimate.index, estimate.card) == *selected_candidate_id
        }) else {
            missing_selected_strategy_package_estimate_examples += 1;
            continue;
        };
        let Some(candidate) = packet.context.candidates.iter().find(|candidate| {
            estimate_candidate_id(candidate.index, candidate.card) == *selected_candidate_id
        }) else {
            missing_selected_strategy_package_estimate_examples += 1;
            continue;
        };

        rows.push(StrategyPackageCalibrationRowV1 {
            actual_route_hp_loss,
            next_combat_hp_loss: card_reward.next_combat_hp_loss,
            predicted_strategy_package_delta: total_value_delta(estimate),
            bucket_keys: strategy_package_bucket_keys(
                candidate.plan_delta.support,
                &candidate.plan_delta.effects,
            ),
        });
    }

    let mean_route_hp_loss = mean_i32(rows.iter().map(|row| row.actual_route_hp_loss));
    let mut global = CardRewardStrategyPackageCalibrationAccumulatorV1::default();
    let mut bucket_accumulators =
        BTreeMap::<String, CardRewardStrategyPackageCalibrationAccumulatorV1>::new();

    for row in &rows {
        let actual_survival_delta = mean_route_hp_loss
            .map(|mean| mean - row.actual_route_hp_loss as f32)
            .unwrap_or(0.0);
        global.push(
            row.actual_route_hp_loss,
            row.next_combat_hp_loss,
            row.predicted_strategy_package_delta,
            actual_survival_delta,
        );
        for bucket_key in &row.bucket_keys {
            bucket_accumulators
                .entry(bucket_key.clone())
                .or_default()
                .push(
                    row.actual_route_hp_loss,
                    row.next_combat_hp_loss,
                    row.predicted_strategy_package_delta,
                    actual_survival_delta,
                );
        }
    }

    CardRewardStrategyPackageCalibrationV1 {
        schema_name: CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        estimator_kind: "strategy_package_selected_candidate_alignment_v1".to_string(),
        total_examples: examples.len(),
        evaluated_examples: rows.len(),
        missing_public_packet_examples,
        missing_outcome_examples,
        missing_selected_strategy_package_estimate_examples,
        global: global.into_global(),
        buckets: bucket_accumulators
            .into_iter()
            .map(|(bucket_key, accumulator)| accumulator.into_bucket(bucket_key))
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct StrategyPackageCalibrationRowV1 {
    actual_route_hp_loss: i32,
    next_combat_hp_loss: Option<i32>,
    predicted_strategy_package_delta: f32,
    bucket_keys: Vec<String>,
}

#[derive(Default)]
struct CardRewardStrategyPackageCalibrationAccumulatorV1 {
    route_hp_losses: Vec<i32>,
    next_combat_hp_losses: Vec<i32>,
    predicted_strategy_package_deltas: Vec<f32>,
    actual_survival_deltas: Vec<f32>,
    signed_errors: Vec<f32>,
    absolute_errors: Vec<f32>,
}

impl CardRewardStrategyPackageCalibrationAccumulatorV1 {
    fn push(
        &mut self,
        actual_route_hp_loss: i32,
        next_combat_hp_loss: Option<i32>,
        predicted_strategy_package_delta: f32,
        actual_survival_delta: f32,
    ) {
        let signed_error = predicted_strategy_package_delta - actual_survival_delta;
        self.route_hp_losses.push(actual_route_hp_loss);
        if let Some(next_combat_hp_loss) = next_combat_hp_loss {
            self.next_combat_hp_losses.push(next_combat_hp_loss);
        }
        self.predicted_strategy_package_deltas
            .push(predicted_strategy_package_delta);
        self.actual_survival_deltas.push(actual_survival_delta);
        self.signed_errors.push(signed_error);
        self.absolute_errors.push(signed_error.abs());
    }

    fn into_global(self) -> CardRewardStrategyPackageCalibrationGlobalV1 {
        CardRewardStrategyPackageCalibrationGlobalV1 {
            evaluated_count: self.route_hp_losses.len(),
            mean_actual_route_hp_loss: mean_i32(self.route_hp_losses.into_iter()),
            mean_actual_next_combat_hp_loss: mean_i32(self.next_combat_hp_losses.into_iter()),
            mean_predicted_strategy_package_delta: mean_f32(
                self.predicted_strategy_package_deltas.into_iter(),
            ),
            mean_actual_survival_delta: mean_f32(self.actual_survival_deltas.into_iter()),
            mean_signed_error: mean_f32(self.signed_errors.into_iter()),
            mean_absolute_error: mean_f32(self.absolute_errors.into_iter()),
        }
    }

    fn into_bucket(self, bucket_key: String) -> CardRewardStrategyPackageCalibrationBucketV1 {
        let evaluated_count = self.route_hp_losses.len();
        let confidence = evaluated_count as f32 / (evaluated_count as f32 + 3.0);
        let uncertainty = 1.0 - confidence;
        CardRewardStrategyPackageCalibrationBucketV1 {
            bucket_key,
            evaluated_count,
            mean_actual_route_hp_loss: mean_i32(self.route_hp_losses.into_iter()),
            mean_actual_next_combat_hp_loss: mean_i32(self.next_combat_hp_losses.into_iter()),
            mean_predicted_strategy_package_delta: mean_f32(
                self.predicted_strategy_package_deltas.into_iter(),
            ),
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

fn actual_hp_loss(example: &CardRewardValueLoopExampleV1) -> Option<i32> {
    let outcome = example.outcome.as_ref()?;
    let card_reward = outcome.card_reward.as_ref()?;
    if let Some(hp_after_next_elite) = card_reward.hp_after_next_elite {
        return Some((outcome.before.current_hp - hp_after_next_elite).max(0));
    }
    card_reward.next_combat_hp_loss
}

fn strategy_package_bucket_keys(
    support: StrategyPlanSupportV1,
    effects: &[crate::ai::noncombat_strategy_v1::StrategyPlanEffectV1],
) -> Vec<String> {
    let mut keys = vec![format!("plan_support:{support:?}")];
    keys.extend(
        effects
            .iter()
            .map(|effect| format!("plan_effect:{effect:?}")),
    );
    keys.sort();
    keys.dedup();
    keys
}

fn estimate_candidate_id(index: usize, card: CardId) -> String {
    format!("card_reward:{index}:{card:?}")
}

fn total_value_delta(estimate: &CardRewardValueEstimateV1) -> f32 {
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
