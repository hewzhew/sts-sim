use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    candidate_response_threat_tags_v1, replay_card_reward_decision_v1, CardRewardDecisionContextV1,
    CardRewardPolicyConfigV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueHorizonV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};
use crate::ai::noncombat_strategy_v1::StrategyThreatTagV1;
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
                candidate,
                &packet.context.strategy.threats.tags,
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

pub fn estimate_card_reward_values_from_strategy_package_calibration_v1(
    context: &CardRewardDecisionContextV1,
    calibration: &CardRewardStrategyPackageCalibrationV1,
) -> Vec<CardRewardValueEstimateV1> {
    let packet =
        crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1::from_context(context);
    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);
    replay
        .value_estimates
        .iter()
        .filter(|estimate| estimate.source == CardRewardValueSourceV1::StrategyPackage)
        .filter_map(|estimate| {
            let candidate = context.candidates.iter().find(|candidate| {
                candidate.index == estimate.index && candidate.card == estimate.card
            })?;
            corrected_strategy_package_estimate(context, estimate, candidate, calibration)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct StrategyPackageCalibrationRowV1 {
    actual_route_hp_loss: i32,
    next_combat_hp_loss: Option<i32>,
    predicted_strategy_package_delta: f32,
    bucket_keys: Vec<String>,
}

fn corrected_strategy_package_estimate(
    context: &CardRewardDecisionContextV1,
    estimate: &CardRewardValueEstimateV1,
    candidate: &crate::ai::card_reward_policy_v1::CardRewardCandidateEvidenceV1,
    calibration: &CardRewardStrategyPackageCalibrationV1,
) -> Option<CardRewardValueEstimateV1> {
    let bucket = best_matching_bucket(context, candidate, calibration)?;
    let raw_total = total_value_delta(estimate);
    let signed_error = bucket.mean_signed_error.unwrap_or(0.0);
    let corrected_total = raw_total - signed_error;
    Some(CardRewardValueEstimateV1 {
        index: estimate.index,
        card: estimate.card,
        source: CardRewardValueSourceV1::StrategyPackage,
        status: CardRewardValueStatusV1::StrategyPackageCalibrated,
        survival_delta: corrected_total,
        progress_delta: 0.0,
        deck_consistency_delta: 0.0,
        uncertainty: bucket.uncertainty.max(0.45),
        eligibility: CardRewardValueEligibilityV1 {
            usable_for_value_estimate: true,
            usable_for_autopilot_gate: false,
            reasons: vec![
                CardRewardValueEligibilityReasonV1::StrategyPackageCalibrationNotGateEligible,
            ],
            bucket_key: Some(bucket.bucket_key.clone()),
            horizon: Some(CardRewardValueHorizonV1::CurrentStrategyPackage),
            outcome_sample_count: Some(bucket.evaluated_count),
        },
        components: vec![
            CardRewardValueComponentV1 {
                name: "strategy_package_raw_total_delta".to_string(),
                value: raw_total,
            },
            CardRewardValueComponentV1 {
                name: "strategy_package_bucket_mean_signed_error".to_string(),
                value: signed_error,
            },
            CardRewardValueComponentV1 {
                name: "strategy_package_corrected_total_delta".to_string(),
                value: corrected_total,
            },
            CardRewardValueComponentV1 {
                name: "strategy_package_calibration_bucket_count".to_string(),
                value: bucket.evaluated_count as f32,
            },
        ],
    })
}

fn best_matching_bucket<'a>(
    context: &CardRewardDecisionContextV1,
    candidate: &crate::ai::card_reward_policy_v1::CardRewardCandidateEvidenceV1,
    calibration: &'a CardRewardStrategyPackageCalibrationV1,
) -> Option<&'a CardRewardStrategyPackageCalibrationBucketV1> {
    let keys = strategy_package_bucket_keys(candidate, &context.strategy.threats.tags);
    calibration
        .buckets
        .iter()
        .filter(|bucket| bucket.usable_for_value_estimate && keys.contains(&bucket.bucket_key))
        .max_by(|left, right| {
            left.evaluated_count
                .cmp(&right.evaluated_count)
                .then_with(|| bucket_specificity(left).cmp(&bucket_specificity(right)))
                .then_with(|| {
                    right
                        .uncertainty
                        .partial_cmp(&left.uncertainty)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| right.bucket_key.cmp(&left.bucket_key))
        })
}

fn bucket_specificity(bucket: &CardRewardStrategyPackageCalibrationBucketV1) -> u8 {
    match bucket.bucket_key.as_str() {
        "threat:StrengthDebuffValuable"
        | "threat:WeakValuable"
        | "threat:ArtifactBlocksDebuff"
        | "threat:StatusFlood"
        | "threat:SplitThreshold"
        | "threat:ModeShiftThreshold"
        | "threat:PowerPunish"
        | "threat:CardPlayLimit"
        | "threat:LongFightScaling"
        | "threat:SetupWindow" => 5,
        "threat:MultiHit" | "threat:AoEValuable" => 4,
        "threat:HighIncomingDamage" => 3,
        _ if bucket.bucket_key.starts_with("plan_effect:") => 2,
        _ if bucket.bucket_key.starts_with("plan_support:") => 1,
        _ => 0,
    }
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
    candidate: &crate::ai::card_reward_policy_v1::CardRewardCandidateEvidenceV1,
    threat_tags: &[StrategyThreatTagV1],
) -> Vec<String> {
    let mut keys = vec![format!("plan_support:{:?}", candidate.plan_delta.support)];
    keys.extend(
        candidate
            .plan_delta
            .effects
            .iter()
            .map(|effect| format!("plan_effect:{effect:?}")),
    );
    let candidate_response_tags = candidate_response_threat_tags_v1(candidate);
    keys.extend(
        threat_tags
            .iter()
            .filter(|tag| candidate_response_tags.contains(tag))
            .map(|tag| format!("threat:{tag:?}")),
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

#[cfg(test)]
mod tests {
    use crate::ai::card_reward_policy_v1::{
        build_card_reward_decision_context_v1, CardRewardRouteEvidenceV1,
        CardRewardSelectedRouteV1, PublicRewardDecisionPacketV1,
    };
    use crate::ai::noncombat_decision_v1::{
        CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1,
        InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
        NonCombatOutcomeAttachmentV1, NonCombatOutcomeMetricsV1, NonCombatOutcomeSnapshotV1,
        NonCombatOutcomeWindowV1, PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1,
        PublicActionPlanV1, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
        NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION, NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_NAME,
        NONCOMBAT_OUTCOME_ATTACHMENT_SCHEMA_VERSION,
    };
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::state::rewards::RewardCard;
    use crate::state::run::RunState;

    use super::*;

    #[test]
    fn strategy_package_calibration_buckets_include_public_threat_tags() {
        let example = strategy_package_example(CardId::Disarm, EncounterId::TheChamp, 6);

        let calibration = calibrate_card_reward_strategy_package_v1(&[example]);

        assert!(calibration.buckets.iter().any(|bucket| {
            bucket.bucket_key == "threat:StrengthDebuffValuable" && bucket.evaluated_count == 1
        }));
        assert!(calibration.buckets.iter().any(|bucket| {
            bucket.bucket_key == "threat:HighIncomingDamage" && bucket.evaluated_count == 1
        }));
    }

    #[test]
    fn strategy_package_calibrated_estimates_can_match_threat_buckets() {
        let example = strategy_package_example(CardId::Disarm, EncounterId::TheChamp, 6);
        let context = example
            .public_packet
            .as_ref()
            .expect("fixture has packet")
            .context
            .clone();
        let calibration = calibrate_card_reward_strategy_package_v1(&[example]);

        let estimates = estimate_card_reward_values_from_strategy_package_calibration_v1(
            &context,
            &calibration,
        );

        let disarm = estimates
            .iter()
            .find(|estimate| estimate.card == CardId::Disarm)
            .expect("Disarm calibrated estimate");
        assert_eq!(
            disarm.eligibility.bucket_key.as_deref(),
            Some("threat:StrengthDebuffValuable")
        );
        assert!(!disarm.eligibility.usable_for_autopilot_gate);
    }

    fn strategy_package_example(
        selected_card: CardId,
        boss: EncounterId,
        next_combat_hp_loss: i32,
    ) -> CardRewardValueLoopExampleV1 {
        let mut run_state = RunState::new(521, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.floor_num = 20;
        run_state.boss_key = Some(boss);
        let mut context = build_card_reward_decision_context_v1(
            &run_state,
            vec![
                RewardCard::new(selected_card, 0),
                RewardCard::new(CardId::TwinStrike, 0),
            ],
            None,
        );
        context.route = Some(route_with_combat_pressure());

        let selected_candidate_id = format!("card_reward:0:{selected_card:?}");
        let decision_record_hash = format!("strategy-package-threat-hash-{selected_card:?}");
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
                seed: 521,
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
            card_reward: Some(
                crate::ai::noncombat_decision_v1::CardRewardOutcomeAttachmentV1 {
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
                },
            ),
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
