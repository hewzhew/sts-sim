use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1, CardRewardValueComponentV1,
    CardRewardValueEligibilityReasonV1, CardRewardValueEligibilityV1, CardRewardValueEstimateV1,
    CardRewardValueHorizonV1, CardRewardValueSourceV1, CardRewardValueStatusV1,
};
use crate::ai::noncombat_decision_v1::PolicySelectionStatusV1;

use super::{
    histogram_entries, CardRewardValueLoopExampleV1, HistogramEntryV1,
    CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME, CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
};

pub const CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME: &str = "CardRewardOutcomeCalibrationV1";
pub const CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub estimator_kind: String,
    #[serde(default)]
    pub provenance: CardRewardOutcomeCalibrationProvenanceV1,
    pub total_examples: usize,
    pub usable_outcome_examples: usize,
    pub missing_outcome_examples: usize,
    pub global: CardRewardOutcomeCalibrationGlobalV1,
    pub card_id_buckets: Vec<CardRewardOutcomeCalibrationBucketV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardOutcomeCalibrationProvenanceV1 {
    pub source_example_schema_name: String,
    pub source_example_schema_version: u32,
    pub source_trace_schema_names: Vec<String>,
    pub source_trace_schema_versions: Vec<u32>,
    pub source_run_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_seed_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_version: Option<String>,
    pub data_roles: Vec<String>,
    pub hidden_simulator_state_used: bool,
    pub short_horizon_autopilot_gate_approved: bool,
}

impl Default for CardRewardOutcomeCalibrationProvenanceV1 {
    fn default() -> Self {
        Self {
            source_example_schema_name: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME.to_string(),
            source_example_schema_version: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
            source_trace_schema_names: Vec::new(),
            source_trace_schema_versions: Vec::new(),
            source_run_count: 0,
            distinct_seed_count: None,
            ruleset_version: None,
            data_roles: Vec::new(),
            hidden_simulator_state_used: false,
            short_horizon_autopilot_gate_approved: false,
        }
    }
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
        provenance: calibration_provenance(examples),
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
        eligibility: outcome_calibration_eligibility(calibration, bucket),
        components: vec![
            CardRewardValueComponentV1 {
                name: "outcome_sample_count".to_string(),
                value: bucket.outcome_attached_count as f32,
            },
            CardRewardValueComponentV1 {
                name: "mean_next_combat_hp_loss".to_string(),
                value: card_mean,
            },
            CardRewardValueComponentV1 {
                name: "global_mean_next_combat_hp_loss".to_string(),
                value: global_mean,
            },
            CardRewardValueComponentV1 {
                name: "survival_delta_from_global".to_string(),
                value: survival_delta,
            },
            CardRewardValueComponentV1 {
                name: "outcome_calibration_confidence".to_string(),
                value: bucket.confidence,
            },
            CardRewardValueComponentV1 {
                name: "outcome_calibration_uncertainty".to_string(),
                value: bucket.uncertainty,
            },
        ],
    })
}

pub(super) fn outcome_calibration_eligibility(
    calibration: &CardRewardOutcomeCalibrationV1,
    bucket: &CardRewardOutcomeCalibrationBucketV1,
) -> CardRewardValueEligibilityV1 {
    let mut reasons = Vec::new();
    if !bucket.usable_for_autopilot_gate {
        reasons.push(CardRewardValueEligibilityReasonV1::OutcomeCalibrationBucketNotGateEligible);
    }
    if calibration.provenance.distinct_seed_count.unwrap_or(0) == 0 {
        reasons.push(CardRewardValueEligibilityReasonV1::MissingDistinctSeedCount);
    }
    if calibration
        .provenance
        .ruleset_version
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        reasons.push(CardRewardValueEligibilityReasonV1::MissingRulesetVersion);
    }
    if calibration.provenance.data_roles.is_empty() {
        reasons.push(CardRewardValueEligibilityReasonV1::MissingDataRoleProvenance);
    }
    if calibration.provenance.hidden_simulator_state_used {
        reasons.push(CardRewardValueEligibilityReasonV1::HiddenSimulatorStateUsed);
    }
    if !calibration.provenance.short_horizon_autopilot_gate_approved {
        reasons.push(CardRewardValueEligibilityReasonV1::ShortHorizonMetricOnly);
    }

    CardRewardValueEligibilityV1 {
        usable_for_value_estimate: bucket.usable_for_value_estimate,
        usable_for_autopilot_gate: bucket.usable_for_autopilot_gate && reasons.is_empty(),
        reasons,
        bucket_key: Some(bucket.bucket_key.clone()),
        horizon: Some(CardRewardValueHorizonV1::NextCombatHpLoss),
        outcome_sample_count: Some(bucket.outcome_attached_count),
    }
}

fn calibration_provenance(
    examples: &[CardRewardValueLoopExampleV1],
) -> CardRewardOutcomeCalibrationProvenanceV1 {
    let mut trace_schema_names = BTreeSet::<String>::new();
    let mut trace_schema_versions = BTreeSet::<u32>::new();
    let mut seeds = BTreeSet::<u64>::new();
    let mut data_roles = BTreeSet::<String>::new();
    let mut hidden_simulator_state_used = false;
    let mut source_run_count = 0;

    for example in examples {
        if let Some(name) = &example.source_trace_schema_name {
            trace_schema_names.insert(name.clone());
        }
        if let Some(version) = example.source_trace_schema_version {
            trace_schema_versions.insert(version);
        }
        if let Some(run) = &example.source_run_config {
            source_run_count += 1;
            seeds.insert(run.seed);
        }
        data_roles.insert(format!("{:?}", example.source_record.data_role));
        hidden_simulator_state_used |= example
            .source_record
            .information_boundary
            .hidden_simulator_state_used;
    }

    CardRewardOutcomeCalibrationProvenanceV1 {
        source_example_schema_name: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME.to_string(),
        source_example_schema_version: CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_VERSION,
        source_trace_schema_names: trace_schema_names.into_iter().collect(),
        source_trace_schema_versions: trace_schema_versions.into_iter().collect(),
        source_run_count,
        distinct_seed_count: (!seeds.is_empty()).then_some(seeds.len()),
        ruleset_version: Some(default_calibration_ruleset_version()),
        data_roles: data_roles.into_iter().collect(),
        hidden_simulator_state_used,
        short_horizon_autopilot_gate_approved: false,
    }
}

fn default_calibration_ruleset_version() -> String {
    format!("sts_simulator:{}", env!("CARGO_PKG_VERSION"))
}

pub fn estimate_card_reward_values_from_calibration_v1(
    context: &CardRewardDecisionContextV1,
    calibration: &CardRewardOutcomeCalibrationV1,
) -> Vec<CardRewardValueEstimateV1> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| {
            estimate_card_reward_value_from_calibration_v1(candidate, calibration)
        })
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

fn increment(histogram: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *histogram.entry(key.into()).or_default() += 1;
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
