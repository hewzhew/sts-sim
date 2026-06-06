use super::arbitration::{estimate_source_gate_eligible_v1, value_status_autopilot_eligible_v1};
use super::types::{
    CardRewardAutopilotGateReportV1, CardRewardDecisionContextV1, CardRewardEvidenceGapV1,
    CardRewardPickCertificateV1, CardRewardPolicyActionV1, CardRewardPolicyConfigV1,
    CardRewardStopDispositionV1, CardRewardValueEstimateV1, CardRewardValueStatusV1,
};

pub(crate) fn pick_gate(
    context: &CardRewardDecisionContextV1,
    value_estimates: &[CardRewardValueEstimateV1],
    config: &CardRewardPolicyConfigV1,
) -> (
    CardRewardPolicyActionV1,
    CardRewardAutopilotGateReportV1,
    Vec<CardRewardEvidenceGapV1>,
    Option<CardRewardPickCertificateV1>,
) {
    let mut gaps = Vec::new();

    if context.candidates.is_empty() {
        return (
            CardRewardPolicyActionV1::Stop {
                reason: "no visible card reward candidates".to_string(),
                disposition: CardRewardStopDispositionV1::MayOpenRewardItem,
            },
            empty_gate_report(&gaps),
            gaps,
            None,
        );
    }

    if context.has_singing_bowl {
        push_gap(
            &mut gaps,
            CardRewardEvidenceGapV1::SingingBowlAddsMaxHpChoice,
        );
        return (
            CardRewardPolicyActionV1::Stop {
                reason: "card reward policy stopped because Singing Bowl adds a max-HP alternative"
                    .to_string(),
                disposition: CardRewardStopDispositionV1::KeepRewardItemClosed,
            },
            empty_gate_report(&gaps),
            gaps,
            None,
        );
    }

    if context.route.is_none() {
        push_gap(&mut gaps, CardRewardEvidenceGapV1::MissingRouteEvidence);
    }

    if value_estimates.len() != context.candidates.len() {
        push_gap(&mut gaps, CardRewardEvidenceGapV1::MissingValueEstimate);
    }
    for candidate in &context.candidates {
        for gap in &candidate.impact.certification_blockers {
            push_gap(&mut gaps, *gap);
        }
    }

    let gate_report = evaluate_autopilot_gate(context, value_estimates, &gaps, config);

    if let Some(certificate) = generic_certificate(context, value_estimates, &gate_report, config) {
        return (
            CardRewardPolicyActionV1::Pick {
                index: certificate.index,
                card: certificate.card,
                confidence: certificate.confidence,
                reason: certificate.reasons.join("; "),
            },
            gate_report,
            gaps,
            Some(certificate),
        );
    }

    for estimate in value_estimates {
        if estimate.status == CardRewardValueStatusV1::UncalibratedPrior {
            push_gap(
                &mut gaps,
                CardRewardEvidenceGapV1::UncalibratedValueEstimate,
            );
        }
    }

    for gap in &gate_report.blocked_reasons {
        push_gap(&mut gaps, *gap);
    }
    push_gap(&mut gaps, CardRewardEvidenceGapV1::NoAutoPickCertificate);
    (
        CardRewardPolicyActionV1::Stop {
            reason: stop_reason(&gaps),
            disposition: CardRewardStopDispositionV1::MayOpenRewardItem,
        },
        gate_report,
        gaps,
        None,
    )
}

fn stop_reason(gaps: &[CardRewardEvidenceGapV1]) -> String {
    if gaps.is_empty() {
        return "card reward policy stopped because the autopilot value gate did not select"
            .to_string();
    }
    let rendered = gaps
        .iter()
        .map(|gap| format!("{gap:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("card reward policy stopped; missing or unresolved evidence: {rendered}")
}

fn push_gap(gaps: &mut Vec<CardRewardEvidenceGapV1>, gap: CardRewardEvidenceGapV1) {
    if !gaps.contains(&gap) {
        gaps.push(gap);
    }
}

fn evaluate_autopilot_gate(
    context: &CardRewardDecisionContextV1,
    value_estimates: &[CardRewardValueEstimateV1],
    inherited_gaps: &[CardRewardEvidenceGapV1],
    _config: &CardRewardPolicyConfigV1,
) -> CardRewardAutopilotGateReportV1 {
    let candidate_coverage_complete = value_estimates.len() == context.candidates.len()
        && context.candidates.iter().all(|candidate| {
            value_estimates.iter().any(|estimate| {
                estimate.index == candidate.index && estimate_eligible_for_autopilot_gate(estimate)
            })
        });

    let eligible_values = value_estimates
        .iter()
        .filter(|estimate| estimate_eligible_for_autopilot_gate(estimate))
        .filter(|estimate| calibration_status_allowed(estimate.status))
        .filter(|estimate| estimate.uncertainty <= 0.35)
        .filter(|estimate| total_value_delta(estimate) > 0.0)
        .filter(|estimate| candidate_dependencies_clear(context, estimate.index))
        .collect::<Vec<_>>();

    let value_source_eligible = value_estimates
        .iter()
        .any(estimate_eligible_for_autopilot_gate);
    let calibration_status_allowed = value_estimates
        .iter()
        .any(|estimate| calibration_status_allowed(estimate.status));
    let value_vs_skip_positive = eligible_values
        .iter()
        .any(|estimate| total_value_delta(estimate) > 0.0);
    let uncertainty_below_limit = value_estimates
        .iter()
        .any(|estimate| estimate.uncertainty <= 0.35);
    let unresolved_dependencies_empty = eligible_values
        .iter()
        .any(|estimate| candidate_dependencies_clear(context, estimate.index));

    let selected_candidate_index =
        select_by_margin(&eligible_values).map(|estimate| estimate.index);
    let margin_sufficient = selected_candidate_index.is_some();

    let mut blocked_reasons = Vec::new();
    if !candidate_coverage_complete {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::MissingValueEstimate,
        );
    }
    if !value_source_eligible {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::IneligibleValueSource,
        );
    }
    if !calibration_status_allowed {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::UncalibratedValueEstimate,
        );
    }
    if !value_vs_skip_positive {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::ValueNotPositive,
        );
    }
    if !uncertainty_below_limit {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::ValueUncertaintyTooHigh,
        );
    }
    if !unresolved_dependencies_empty {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::UnresolvedCandidateDependencies,
        );
    }
    if selected_candidate_index.is_none() {
        push_gap(
            &mut blocked_reasons,
            CardRewardEvidenceGapV1::ValueMarginTooSmall,
        );
    }
    for gap in inherited_gaps {
        push_gap(&mut blocked_reasons, *gap);
    }

    CardRewardAutopilotGateReportV1 {
        hidden_free: true,
        candidate_coverage_complete,
        value_source_eligible,
        calibration_status_allowed,
        value_vs_skip_positive,
        margin_sufficient,
        uncertainty_below_limit,
        unresolved_dependencies_empty,
        selected_candidate_index,
        blocked_reasons,
    }
}

fn generic_certificate(
    context: &CardRewardDecisionContextV1,
    value_estimates: &[CardRewardValueEstimateV1],
    gate_report: &CardRewardAutopilotGateReportV1,
    config: &CardRewardPolicyConfigV1,
) -> Option<CardRewardPickCertificateV1> {
    if !config.allow_autopilot_value_gate {
        return None;
    }
    if !gate_report.hidden_free
        || !gate_report.candidate_coverage_complete
        || !gate_report.value_source_eligible
        || !gate_report.calibration_status_allowed
        || !gate_report.value_vs_skip_positive
        || !gate_report.margin_sufficient
        || !gate_report.uncertainty_below_limit
        || !gate_report.unresolved_dependencies_empty
    {
        return None;
    }
    let index = gate_report.selected_candidate_index?;
    let estimate = value_estimates
        .iter()
        .find(|estimate| estimate.index == index)?;
    let candidate = context
        .candidates
        .iter()
        .find(|candidate| candidate.index == index)?;
    Some(CardRewardPickCertificateV1 {
        index,
        card: candidate.card,
        confidence: (1.0 - estimate.uncertainty).clamp(0.0, 1.0),
        reasons: vec![
            "generic autopilot gate accepted calibrated value estimate".to_string(),
            format!(
                "source={:?} status={:?} total_delta={:.3} uncertainty={:.3}",
                estimate.source,
                estimate.status,
                total_value_delta(estimate),
                estimate.uncertainty
            ),
        ],
    })
}

fn estimate_eligible_for_autopilot_gate(estimate: &CardRewardValueEstimateV1) -> bool {
    estimate_source_gate_eligible_v1(estimate)
}

fn calibration_status_allowed(status: CardRewardValueStatusV1) -> bool {
    value_status_autopilot_eligible_v1(status)
}

fn total_value_delta(estimate: &CardRewardValueEstimateV1) -> f32 {
    estimate.survival_delta + estimate.progress_delta + estimate.deck_consistency_delta
}

fn candidate_dependencies_clear(context: &CardRewardDecisionContextV1, index: usize) -> bool {
    context
        .candidates
        .iter()
        .find(|candidate| candidate.index == index)
        .map(|candidate| candidate.impact.certification_blockers.is_empty())
        .unwrap_or(false)
}

fn select_by_margin<'a>(
    estimates: &[&'a CardRewardValueEstimateV1],
) -> Option<&'a CardRewardValueEstimateV1> {
    let mut ordered = estimates.to_vec();
    ordered.sort_by(|left, right| {
        total_value_delta(right)
            .partial_cmp(&total_value_delta(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best = ordered.first().copied()?;
    let second_value = ordered
        .get(1)
        .map(|estimate| total_value_delta(estimate))
        .unwrap_or(0.0);
    if total_value_delta(best) - second_value >= 0.25 {
        Some(best)
    } else {
        None
    }
}

fn empty_gate_report(gaps: &[CardRewardEvidenceGapV1]) -> CardRewardAutopilotGateReportV1 {
    CardRewardAutopilotGateReportV1 {
        hidden_free: true,
        candidate_coverage_complete: false,
        value_source_eligible: false,
        calibration_status_allowed: false,
        value_vs_skip_positive: false,
        margin_sufficient: false,
        uncertainty_below_limit: false,
        unresolved_dependencies_empty: false,
        selected_candidate_index: None,
        blocked_reasons: gaps.to_vec(),
    }
}
