use serde::{Deserialize, Serialize};

use super::{
    build_card_reward_closed_loop_report_v1, calibrate_card_reward_outcomes_v1,
    calibrate_card_reward_route_risk_v1, CardRewardClosedLoopReportV1,
    CardRewardOutcomeCalibrationBucketV1, CardRewardOutcomeCalibrationV1,
    CardRewardRouteRiskCalibrationV1, CardRewardValueLoopExampleV1, HistogramEntryV1,
};

pub const CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_NAME: &str =
    "CardRewardOutcomeCalibrationPromotionReportV1";
pub const CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardOutcomeCalibrationPromotionConfigV1 {
    pub approve_short_horizon_autopilot_gate: bool,
    pub min_distinct_seeds: usize,
    pub min_bucket_outcome_attached_count: usize,
    pub min_bucket_confidence: f32,
    pub max_bucket_uncertainty: f32,
    pub reject_hidden_simulator_state: bool,
}

impl Default for CardRewardOutcomeCalibrationPromotionConfigV1 {
    fn default() -> Self {
        Self {
            approve_short_horizon_autopilot_gate: false,
            min_distinct_seeds: 3,
            min_bucket_outcome_attached_count: 3,
            min_bucket_confidence: 0.65,
            max_bucket_uncertainty: 0.35,
            reject_hidden_simulator_state: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardOutcomeCalibrationPromotionBlockerV1 {
    MissingDistinctSeedCount,
    DistinctSeedCountBelowMinimum,
    MissingRulesetVersion,
    MissingDataRoleProvenance,
    HiddenSimulatorStateUsed,
    ShortHorizonNotApproved,
    BucketNotValueUsable,
    BucketMissingMeanNextCombatHpLoss,
    BucketOutcomeCountBelowMinimum,
    BucketConfidenceTooLow,
    BucketUncertaintyTooHigh,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationPromotionBucketV1 {
    pub bucket_key: String,
    pub card_id: String,
    pub promoted_for_autopilot_gate: bool,
    pub blockers: Vec<CardRewardOutcomeCalibrationPromotionBlockerV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationPromotionReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub promoted_bucket_count: usize,
    pub blocked_bucket_count: usize,
    pub global_blockers: Vec<CardRewardOutcomeCalibrationPromotionBlockerV1>,
    pub blocker_counts: Vec<HistogramEntryV1>,
    pub buckets: Vec<CardRewardOutcomeCalibrationPromotionBucketV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardRuntimeCalibrationPipelineV1 {
    pub raw_calibration: CardRewardOutcomeCalibrationV1,
    pub promoted_calibration: CardRewardOutcomeCalibrationV1,
    pub promotion_report: CardRewardOutcomeCalibrationPromotionReportV1,
    pub route_risk_calibration: CardRewardRouteRiskCalibrationV1,
    pub closed_loop_report: CardRewardClosedLoopReportV1,
}

pub fn build_card_reward_runtime_calibration_pipeline_v1(
    examples: &[CardRewardValueLoopExampleV1],
    config: &CardRewardOutcomeCalibrationPromotionConfigV1,
) -> CardRewardRuntimeCalibrationPipelineV1 {
    let raw_calibration = calibrate_card_reward_outcomes_v1(examples);
    let route_risk_calibration = calibrate_card_reward_route_risk_v1(examples);
    let (promoted_calibration, promotion_report) =
        promote_card_reward_outcome_calibration_v1(&raw_calibration, config);
    let closed_loop_report = build_card_reward_closed_loop_report_v1(
        examples,
        &promoted_calibration,
        "runtime_pipeline_promoted_calibration",
    );

    CardRewardRuntimeCalibrationPipelineV1 {
        raw_calibration,
        promoted_calibration,
        promotion_report,
        route_risk_calibration,
        closed_loop_report,
    }
}

pub fn promote_card_reward_outcome_calibration_v1(
    calibration: &CardRewardOutcomeCalibrationV1,
    config: &CardRewardOutcomeCalibrationPromotionConfigV1,
) -> (
    CardRewardOutcomeCalibrationV1,
    CardRewardOutcomeCalibrationPromotionReportV1,
) {
    let global_blockers = global_blockers(calibration, config);
    let mut promoted = calibration.clone();
    promoted.provenance.short_horizon_autopilot_gate_approved =
        config.approve_short_horizon_autopilot_gate;

    let mut bucket_reports = Vec::new();
    for bucket in &mut promoted.card_id_buckets {
        let mut blockers = global_blockers.clone();
        blockers.extend(bucket_blockers(bucket, config));
        blockers.sort();
        blockers.dedup();
        let promoted_for_autopilot_gate = blockers.is_empty();
        bucket.usable_for_autopilot_gate = promoted_for_autopilot_gate;
        bucket_reports.push(CardRewardOutcomeCalibrationPromotionBucketV1 {
            bucket_key: bucket.bucket_key.clone(),
            card_id: bucket.card_id.clone(),
            promoted_for_autopilot_gate,
            blockers,
        });
    }

    let promoted_bucket_count = bucket_reports
        .iter()
        .filter(|bucket| bucket.promoted_for_autopilot_gate)
        .count();
    let blocked_bucket_count = bucket_reports.len().saturating_sub(promoted_bucket_count);
    let report = CardRewardOutcomeCalibrationPromotionReportV1 {
        schema_name: CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_NAME.to_string(),
        schema_version: CARD_REWARD_OUTCOME_CALIBRATION_PROMOTION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        promoted_bucket_count,
        blocked_bucket_count,
        global_blockers,
        blocker_counts: blocker_counts(&bucket_reports),
        buckets: bucket_reports,
    };

    (promoted, report)
}

fn global_blockers(
    calibration: &CardRewardOutcomeCalibrationV1,
    config: &CardRewardOutcomeCalibrationPromotionConfigV1,
) -> Vec<CardRewardOutcomeCalibrationPromotionBlockerV1> {
    let mut blockers = Vec::new();
    match calibration.provenance.distinct_seed_count {
        Some(count) if count >= config.min_distinct_seeds => {}
        Some(_) => blockers
            .push(CardRewardOutcomeCalibrationPromotionBlockerV1::DistinctSeedCountBelowMinimum),
        None => {
            blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::MissingDistinctSeedCount)
        }
    }
    if calibration
        .provenance
        .ruleset_version
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::MissingRulesetVersion);
    }
    if calibration.provenance.data_roles.is_empty() {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::MissingDataRoleProvenance);
    }
    if config.reject_hidden_simulator_state && calibration.provenance.hidden_simulator_state_used {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::HiddenSimulatorStateUsed);
    }
    if !config.approve_short_horizon_autopilot_gate {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::ShortHorizonNotApproved);
    }
    blockers
}

fn bucket_blockers(
    bucket: &CardRewardOutcomeCalibrationBucketV1,
    config: &CardRewardOutcomeCalibrationPromotionConfigV1,
) -> Vec<CardRewardOutcomeCalibrationPromotionBlockerV1> {
    let mut blockers = Vec::new();
    if !bucket.usable_for_value_estimate {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::BucketNotValueUsable);
    }
    if bucket.mean_next_combat_hp_loss.is_none() {
        blockers.push(
            CardRewardOutcomeCalibrationPromotionBlockerV1::BucketMissingMeanNextCombatHpLoss,
        );
    }
    if bucket.outcome_attached_count < config.min_bucket_outcome_attached_count {
        blockers
            .push(CardRewardOutcomeCalibrationPromotionBlockerV1::BucketOutcomeCountBelowMinimum);
    }
    if bucket.confidence < config.min_bucket_confidence {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::BucketConfidenceTooLow);
    }
    if bucket.uncertainty > config.max_bucket_uncertainty {
        blockers.push(CardRewardOutcomeCalibrationPromotionBlockerV1::BucketUncertaintyTooHigh);
    }
    blockers
}

fn blocker_counts(
    bucket_reports: &[CardRewardOutcomeCalibrationPromotionBucketV1],
) -> Vec<HistogramEntryV1> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for bucket in bucket_reports {
        for blocker in &bucket.blockers {
            *counts.entry(format!("{blocker:?}")).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(key, count)| HistogramEntryV1 { key, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::card_reward_value_loop::{
        CardRewardOutcomeCalibrationBucketV1, CardRewardOutcomeCalibrationGlobalV1,
        CardRewardOutcomeCalibrationProvenanceV1, CardRewardOutcomeCalibrationV1,
        CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME,
        CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
    };

    #[test]
    fn promotion_marks_only_buckets_that_satisfy_global_and_bucket_requirements() {
        let calibration = calibration_fixture(vec![
            bucket_fixture("card_id:TwinStrike", "TwinStrike", 5, 5.0, 0.8, 0.2),
            bucket_fixture("card_id:Cleave", "Cleave", 1, 5.0, 0.25, 0.75),
        ]);
        let config = CardRewardOutcomeCalibrationPromotionConfigV1 {
            approve_short_horizon_autopilot_gate: true,
            min_distinct_seeds: 2,
            min_bucket_outcome_attached_count: 3,
            min_bucket_confidence: 0.65,
            max_bucket_uncertainty: 0.35,
            reject_hidden_simulator_state: true,
        };

        let (promoted, report) = promote_card_reward_outcome_calibration_v1(&calibration, &config);

        assert!(promoted.provenance.short_horizon_autopilot_gate_approved);
        assert!(promoted.card_id_buckets[0].usable_for_autopilot_gate);
        assert!(!promoted.card_id_buckets[1].usable_for_autopilot_gate);
        assert_eq!(report.promoted_bucket_count, 1);
        assert_eq!(report.blocked_bucket_count, 1);
        assert!(report.buckets[1].blockers.contains(
            &CardRewardOutcomeCalibrationPromotionBlockerV1::BucketOutcomeCountBelowMinimum
        ));
        assert!(report.buckets[1]
            .blockers
            .contains(&CardRewardOutcomeCalibrationPromotionBlockerV1::BucketUncertaintyTooHigh));
    }

    #[test]
    fn promotion_does_not_open_gate_without_explicit_short_horizon_approval() {
        let calibration = calibration_fixture(vec![bucket_fixture(
            "card_id:TwinStrike",
            "TwinStrike",
            5,
            5.0,
            0.8,
            0.2,
        )]);
        let config = CardRewardOutcomeCalibrationPromotionConfigV1 {
            approve_short_horizon_autopilot_gate: false,
            ..CardRewardOutcomeCalibrationPromotionConfigV1::default()
        };

        let (promoted, report) = promote_card_reward_outcome_calibration_v1(&calibration, &config);

        assert!(!promoted.provenance.short_horizon_autopilot_gate_approved);
        assert!(!promoted.card_id_buckets[0].usable_for_autopilot_gate);
        assert!(report
            .global_blockers
            .contains(&CardRewardOutcomeCalibrationPromotionBlockerV1::ShortHorizonNotApproved));
    }

    #[test]
    fn promoted_calibration_can_drive_card_reward_policy_gate() {
        let calibration = calibration_fixture(vec![
            bucket_fixture("card_id:TwinStrike", "TwinStrike", 5, 4.0, 0.8, 0.2),
            bucket_fixture("card_id:Cleave", "Cleave", 5, 8.0, 0.8, 0.2),
        ]);
        let config = CardRewardOutcomeCalibrationPromotionConfigV1 {
            approve_short_horizon_autopilot_gate: true,
            min_distinct_seeds: 2,
            min_bucket_outcome_attached_count: 3,
            min_bucket_confidence: 0.65,
            max_bucket_uncertainty: 0.35,
            reject_hidden_simulator_state: true,
        };
        let (promoted, _) = promote_card_reward_outcome_calibration_v1(&calibration, &config);
        let run_state = crate::state::run::RunState::new(521, 0, false, "Ironclad");
        let mut context = crate::ai::card_reward_policy_v1::build_card_reward_decision_context_v1(
            &run_state,
            vec![
                crate::state::rewards::RewardCard::new(
                    crate::content::cards::CardId::TwinStrike,
                    0,
                ),
                crate::state::rewards::RewardCard::new(crate::content::cards::CardId::Cleave, 0),
            ],
            None,
        );
        context.route = Some(route_evidence_fixture());
        let estimates =
            crate::eval::card_reward_value_loop::estimate_card_reward_values_from_calibration_v1(
                &context, &promoted,
            );
        let decision =
            crate::ai::card_reward_policy_v1::plan_card_reward_decision_with_estimator_inputs_v1(
                &context,
                &crate::ai::card_reward_policy_v1::CardRewardPolicyConfigV1::default(),
                &crate::ai::card_reward_policy_v1::CardRewardEstimatorInputsV1 {
                    external_value_estimates: estimates,
                },
            );

        assert!(matches!(
            decision.action,
            crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Pick {
                card: crate::content::cards::CardId::TwinStrike,
                ..
            }
        ));
        assert!(decision.autopilot_gate.value_source_eligible);
        assert!(decision.pick_certificate.is_some());
    }

    fn calibration_fixture(
        buckets: Vec<CardRewardOutcomeCalibrationBucketV1>,
    ) -> CardRewardOutcomeCalibrationV1 {
        CardRewardOutcomeCalibrationV1 {
            schema_name: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
            provenance: CardRewardOutcomeCalibrationProvenanceV1 {
                source_example_schema_name: "CardRewardValueLoopExampleV1".to_string(),
                source_example_schema_version: 1,
                source_trace_schema_names: vec!["SessionTraceV1".to_string()],
                source_trace_schema_versions: vec![14],
                source_run_count: 2,
                distinct_seed_count: Some(2),
                ruleset_version: Some("sts_simulator:test".to_string()),
                data_roles: vec!["BehaviorPolicyNotTeacher".to_string()],
                hidden_simulator_state_used: false,
                short_horizon_autopilot_gate_approved: false,
            },
            total_examples: 5,
            usable_outcome_examples: 5,
            missing_outcome_examples: 0,
            global: CardRewardOutcomeCalibrationGlobalV1 {
                selected_count: 5,
                outcome_attached_count: 5,
                mean_next_combat_hp_loss: Some(8.0),
            },
            card_id_buckets: buckets,
        }
    }

    fn bucket_fixture(
        bucket_key: &str,
        card_id: &str,
        outcome_attached_count: usize,
        mean_next_combat_hp_loss: f32,
        confidence: f32,
        uncertainty: f32,
    ) -> CardRewardOutcomeCalibrationBucketV1 {
        CardRewardOutcomeCalibrationBucketV1 {
            bucket_key: bucket_key.to_string(),
            card_id: card_id.to_string(),
            selected_count: outcome_attached_count,
            outcome_attached_count,
            missing_outcome_count: 0,
            mean_next_combat_hp_loss: Some(mean_next_combat_hp_loss),
            hp_loss_bucket_counts: Vec::new(),
            upgraded_count: 0,
            removed_count: 0,
            confidence,
            uncertainty,
            usable_for_value_estimate: outcome_attached_count > 0,
            usable_for_autopilot_gate: false,
        }
    }

    fn route_evidence_fixture() -> crate::ai::card_reward_policy_v1::CardRewardRouteEvidenceV1 {
        crate::ai::card_reward_policy_v1::CardRewardRouteEvidenceV1 {
            route_policy: "test_route".to_string(),
            selected_route: Some(
                crate::ai::card_reward_policy_v1::CardRewardSelectedRouteV1 {
                    next_x: 0,
                    next_y: 1,
                    min_fires: 1,
                    max_fires: 2,
                    first_fire_floor: Some(6),
                    min_elites: 0,
                    max_elites: 1,
                    min_early_pressure: 1,
                    max_early_pressure: 2,
                },
            ),
            candidate_count: 1,
            need_card_rewards: 0.8,
            need_upgrade: 0.3,
            need_heal: 0.1,
            can_take_elite: 0.5,
            avoid_damage: 0.2,
            warnings: Vec::new(),
        }
    }
}
