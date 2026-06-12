use std::cmp::Ordering;

use super::types::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionApprovalV1, CardRewardDecisionContextV1,
    CardRewardPolicyConfigV1, CardRewardValueEstimateV1, CardRewardValueSourceV1,
    CardRewardValueStatusV1,
};

#[derive(Clone, Debug)]
struct BehaviorCandidate<'a> {
    candidate: &'a CardRewardCandidateEvidenceV1,
    score: f32,
    max_uncertainty: f32,
    source_labels: Vec<&'static str>,
}

pub(crate) fn behavior_decision_approval(
    context: &CardRewardDecisionContextV1,
    value_estimates: &[CardRewardValueEstimateV1],
    config: &CardRewardPolicyConfigV1,
) -> Option<CardRewardDecisionApprovalV1> {
    if !config.allow_behavior_autopick_gate || context.has_singing_bowl {
        return None;
    }
    if offer_has_plan_dependency_blocker(context) {
        return None;
    }

    let mut candidates = context
        .candidates
        .iter()
        .filter(|candidate| candidate.impact.approval_blockers.is_empty())
        .filter_map(|candidate| behavior_candidate(candidate, value_estimates, config))
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                left.max_uncertainty
                    .partial_cmp(&right.max_uncertainty)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.candidate.index.cmp(&right.candidate.index))
    });

    let best = candidates.first()?;
    if best.score < config.behavior_min_total_delta {
        return None;
    }
    let second_score = candidates
        .get(1)
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    if best.score - second_score < config.behavior_min_margin {
        return None;
    }

    Some(CardRewardDecisionApprovalV1 {
        index: best.candidate.index,
        card: best.candidate.card,
        confidence: (1.0 - best.max_uncertainty).clamp(0.0, 1.0),
        selection_mode: "behavior_autopick_gate",
        reasons: vec![
            "behavior autopick accepted structured public estimates".to_string(),
            format!(
                "score={:.3} margin={:.3} max_uncertainty={:.3} sources={}",
                best.score,
                best.score - second_score,
                best.max_uncertainty,
                best.source_labels.join("+")
            ),
        ],
    })
}

fn offer_has_plan_dependency_blocker(context: &CardRewardDecisionContextV1) -> bool {
    context.candidates.iter().any(|candidate| {
        candidate.impact.approval_blockers.iter().any(|gap| {
            matches!(
                gap,
                super::types::CardRewardEvidenceGapV1::UnsatisfiedRouteUpgradeEvidence
                    | super::types::CardRewardEvidenceGapV1::UnsatisfiedStrengthScalingEvidence
                    | super::types::CardRewardEvidenceGapV1::UnsatisfiedBlockDensityEvidence
                    | super::types::CardRewardEvidenceGapV1::UnsatisfiedStrikeDensityEvidence
                    | super::types::CardRewardEvidenceGapV1::UnsatisfiedExhaustPackageEvidence
                    | super::types::CardRewardEvidenceGapV1::UnsatisfiedStatusPackageEvidence
                    | super::types::CardRewardEvidenceGapV1::MissingStrategicPlanEvidence
            )
        })
    })
}

fn behavior_candidate<'a>(
    candidate: &'a CardRewardCandidateEvidenceV1,
    value_estimates: &[CardRewardValueEstimateV1],
    config: &CardRewardPolicyConfigV1,
) -> Option<BehaviorCandidate<'a>> {
    let estimates = value_estimates
        .iter()
        .filter(|estimate| estimate.index == candidate.index && estimate.card == candidate.card)
        .filter(|estimate| behavior_estimate_allowed(estimate))
        .collect::<Vec<_>>();
    if estimates.is_empty() {
        return None;
    }
    if estimates
        .iter()
        .any(|estimate| estimate.uncertainty > config.behavior_max_uncertainty)
    {
        return None;
    }
    if estimates
        .iter()
        .any(|estimate| total_delta(estimate) < -0.05)
    {
        return None;
    }

    let score = estimates
        .iter()
        .map(|estimate| source_weight(estimate.source) * total_delta(estimate))
        .sum::<f32>();
    let max_uncertainty = estimates
        .iter()
        .map(|estimate| estimate.uncertainty)
        .fold(0.0_f32, f32::max);
    let mut source_labels = estimates
        .iter()
        .map(|estimate| source_label(estimate.source))
        .collect::<Vec<_>>();
    source_labels.sort();
    source_labels.dedup();

    Some(BehaviorCandidate {
        candidate,
        score,
        max_uncertainty,
        source_labels,
    })
}

fn behavior_estimate_allowed(estimate: &CardRewardValueEstimateV1) -> bool {
    matches!(
        estimate.source,
        CardRewardValueSourceV1::PublicCombatHeuristic
            | CardRewardValueSourceV1::RouteRisk
            | CardRewardValueSourceV1::StrategyPackage
    ) && matches!(
        estimate.status,
        CardRewardValueStatusV1::PublicCombatHeuristic
            | CardRewardValueStatusV1::RouteRiskEstimate
            | CardRewardValueStatusV1::RouteRiskCalibrated
            | CardRewardValueStatusV1::StrategyPackageEstimate
            | CardRewardValueStatusV1::StrategyPackageCalibrated
    )
}

fn source_weight(source: CardRewardValueSourceV1) -> f32 {
    match source {
        CardRewardValueSourceV1::RouteRisk => 1.0,
        CardRewardValueSourceV1::PublicCombatHeuristic => 0.85,
        CardRewardValueSourceV1::StrategyPackage => 0.75,
        _ => 0.0,
    }
}

fn total_delta(estimate: &CardRewardValueEstimateV1) -> f32 {
    estimate.survival_delta + estimate.progress_delta + estimate.deck_consistency_delta
}

fn source_label(source: CardRewardValueSourceV1) -> &'static str {
    match source {
        CardRewardValueSourceV1::PublicCombatHeuristic => "public_combat",
        CardRewardValueSourceV1::RouteRisk => "route_risk",
        CardRewardValueSourceV1::StrategyPackage => "strategy_package",
        CardRewardValueSourceV1::UncalibratedImpactPrior
        | CardRewardValueSourceV1::OutcomeCalibration
        | CardRewardValueSourceV1::CombatProbe
        | CardRewardValueSourceV1::LearnedValue => "other",
    }
}
