use std::cmp::Ordering;

use super::types::{
    CardRewardDecisionContextV1, CardRewardEstimatorArbitrationV1,
    CardRewardEstimatorCandidateArbitrationV1, CardRewardEvidenceGapV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

pub const CARD_REWARD_ESTIMATOR_ARBITRATION_SCHEMA_NAME: &str = "CardRewardEstimatorArbitrationV1";
pub const CARD_REWARD_ESTIMATOR_ARBITRATION_SCHEMA_VERSION: u32 = 1;

pub fn arbitrate_card_reward_value_estimates_v1(
    context: &CardRewardDecisionContextV1,
    estimates: &[CardRewardValueEstimateV1],
) -> CardRewardEstimatorArbitrationV1 {
    let mut gate_value_estimates = Vec::new();
    let mut candidate_reports = Vec::new();

    for candidate in &context.candidates {
        let mut candidate_estimates = estimates
            .iter()
            .filter(|estimate| estimate.index == candidate.index && estimate.card == candidate.card)
            .collect::<Vec<_>>();
        candidate_estimates.sort_by(|left, right| compare_estimates_for_arbitration(left, right));
        let selected = select_gate_estimate(&candidate_estimates);
        let mut rejected_reasons = Vec::new();
        if selected.is_none() {
            rejected_reasons.push(CardRewardEvidenceGapV1::MissingValueEstimate);
        }

        if let Some(selected) = selected {
            gate_value_estimates.push(selected.clone());
        }

        candidate_reports.push(CardRewardEstimatorCandidateArbitrationV1 {
            index: candidate.index,
            card: candidate.card,
            input_estimate_count: candidate_estimates.len(),
            selected_source: selected.map(|estimate| estimate.source),
            selected_status: selected.map(|estimate| estimate.status),
            selected_for_gate: selected.is_some(),
            autopilot_source_eligible: selected
                .map(|estimate| value_source_autopilot_eligible_v1(estimate.source))
                .unwrap_or(false),
            selected_estimate_gate_eligible: selected
                .map(estimate_source_gate_eligible_v1)
                .unwrap_or(false),
            rejected_reasons,
        });
    }

    CardRewardEstimatorArbitrationV1 {
        schema_name: CARD_REWARD_ESTIMATOR_ARBITRATION_SCHEMA_NAME,
        schema_version: CARD_REWARD_ESTIMATOR_ARBITRATION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label",
        input_estimate_count: estimates.len(),
        gate_value_estimates,
        candidate_reports,
    }
}

fn select_gate_estimate<'a>(
    candidate_estimates: &[&'a CardRewardValueEstimateV1],
) -> Option<&'a CardRewardValueEstimateV1> {
    candidate_estimates
        .iter()
        .copied()
        .filter(|estimate| estimate_source_gate_eligible_v1(estimate))
        .min_by(|left, right| compare_estimates_for_arbitration(left, right))
        .or_else(|| candidate_estimates.first().copied())
}

pub(crate) fn value_source_autopilot_eligible_v1(source: CardRewardValueSourceV1) -> bool {
    matches!(
        source,
        CardRewardValueSourceV1::CombatProbe
            | CardRewardValueSourceV1::RouteRisk
            | CardRewardValueSourceV1::LearnedValue
    )
}

pub(crate) fn estimate_source_gate_eligible_v1(estimate: &CardRewardValueEstimateV1) -> bool {
    match estimate.source {
        CardRewardValueSourceV1::UncalibratedImpactPrior => false,
        CardRewardValueSourceV1::StrategyPackage => false,
        CardRewardValueSourceV1::OutcomeCalibration => {
            estimate.eligibility.usable_for_autopilot_gate
        }
        CardRewardValueSourceV1::CombatProbe
        | CardRewardValueSourceV1::RouteRisk
        | CardRewardValueSourceV1::LearnedValue => {
            value_source_autopilot_eligible_v1(estimate.source)
                && estimate.eligibility.usable_for_autopilot_gate
        }
    }
}

pub(crate) fn value_status_autopilot_eligible_v1(status: CardRewardValueStatusV1) -> bool {
    matches!(
        status,
        CardRewardValueStatusV1::CounterfactualProbe
            | CardRewardValueStatusV1::OutcomeCalibrated
            | CardRewardValueStatusV1::RouteRiskEstimate
            | CardRewardValueStatusV1::RouteRiskCalibrated
    )
}

fn compare_estimates_for_arbitration(
    left: &CardRewardValueEstimateV1,
    right: &CardRewardValueEstimateV1,
) -> Ordering {
    source_rank(right.source)
        .cmp(&source_rank(left.source))
        .then_with(|| status_rank(right.status).cmp(&status_rank(left.status)))
        .then_with(|| {
            left.uncertainty
                .partial_cmp(&right.uncertainty)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            total_delta(right)
                .partial_cmp(&total_delta(left))
                .unwrap_or(Ordering::Equal)
        })
}

fn source_rank(source: CardRewardValueSourceV1) -> u8 {
    match source {
        CardRewardValueSourceV1::LearnedValue => 50,
        CardRewardValueSourceV1::OutcomeCalibration => 35,
        CardRewardValueSourceV1::RouteRisk => 25,
        CardRewardValueSourceV1::CombatProbe => 10,
        CardRewardValueSourceV1::StrategyPackage => 5,
        CardRewardValueSourceV1::UncalibratedImpactPrior => 0,
    }
}

fn status_rank(status: CardRewardValueStatusV1) -> u8 {
    match status {
        CardRewardValueStatusV1::CounterfactualProbe => 30,
        CardRewardValueStatusV1::OutcomeCalibrated => 20,
        CardRewardValueStatusV1::RouteRiskCalibrated => 15,
        CardRewardValueStatusV1::RouteRiskEstimate => 10,
        CardRewardValueStatusV1::StrategyPackageCalibrated => 8,
        CardRewardValueStatusV1::StrategyPackageEstimate => 5,
        CardRewardValueStatusV1::UncalibratedPrior => 0,
    }
}

fn total_delta(estimate: &CardRewardValueEstimateV1) -> f32 {
    estimate.survival_delta + estimate.progress_delta + estimate.deck_consistency_delta
}
