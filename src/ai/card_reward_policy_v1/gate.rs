use super::types::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1, CardRewardEvidenceGapV1,
    CardRewardPickCertificateV1, CardRewardPlanEffectV1, CardRewardPlanSupportV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardValueEstimateV1,
    CardRewardValueStatusV1,
};

use crate::content::cards::CardId;

pub(crate) fn pick_gate(
    context: &CardRewardDecisionContextV1,
    value_estimates: &[CardRewardValueEstimateV1],
    config: &CardRewardPolicyConfigV1,
) -> (
    CardRewardPolicyActionV1,
    Vec<CardRewardEvidenceGapV1>,
    Option<CardRewardPickCertificateV1>,
) {
    let mut gaps = Vec::new();

    if context.candidates.is_empty() {
        return (
            CardRewardPolicyActionV1::Stop {
                reason: "no visible card reward candidates".to_string(),
            },
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

    let certificate = if config.allow_automatic_pick_certificates {
        certified_pick(context)
    } else {
        None
    };

    if let Some(certificate) = certificate {
        return (
            CardRewardPolicyActionV1::Pick {
                index: certificate.index,
                card: certificate.card,
                confidence: certificate.confidence,
                reason: certificate.reasons.join("; "),
            },
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

    push_gap(&mut gaps, CardRewardEvidenceGapV1::NoAutoPickCertificate);
    (
        CardRewardPolicyActionV1::Stop {
            reason: stop_reason(&gaps),
        },
        gaps,
        None,
    )
}

fn certified_pick(context: &CardRewardDecisionContextV1) -> Option<CardRewardPickCertificateV1> {
    let certificates = context
        .candidates
        .iter()
        .filter_map(|candidate| candidate_certificate(context, candidate))
        .collect::<Vec<_>>();

    if certificates.len() == 1 {
        certificates.into_iter().next()
    } else {
        None
    }
}

fn candidate_certificate(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if has_hard_blocker(candidate) {
        return None;
    }

    match candidate.card {
        CardId::SearingBlow => upgrade_sink_certificate(candidate),
        CardId::HeavyBlade => strength_payoff_certificate(candidate),
        CardId::Clothesline => weak_frontload_certificate(context, candidate),
        _ => None,
    }
}

fn upgrade_sink_certificate(
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if candidate.plan_delta.support != CardRewardPlanSupportV1::Strong {
        return None;
    }
    Some(CardRewardPickCertificateV1 {
        index: candidate.index,
        card: candidate.card,
        confidence: 0.82,
        reasons: vec![
            "UpgradeSink plan is strongly supported by visible route fire budget".to_string(),
            "selection is a plan commitment, not an impact-prior score".to_string(),
        ],
    })
}

fn strength_payoff_certificate(
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if candidate.plan_delta.support != CardRewardPlanSupportV1::Strong {
        return None;
    }
    Some(CardRewardPickCertificateV1 {
        index: candidate.index,
        card: candidate.card,
        confidence: 0.80,
        reasons: vec![
            "StrengthPayoff plan is supported by visible strength source(s)".to_string(),
            "selection is a plan commitment, not an impact-prior score".to_string(),
        ],
    })
}

fn weak_frontload_certificate(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if context.run.floor <= 0 {
        return None;
    }
    if candidate.plan_delta.support == CardRewardPlanSupportV1::Blocked {
        return None;
    }
    if context.candidates.iter().any(|other| {
        other.index != candidate.index
            && matches!(other.card, CardId::SearingBlow | CardId::HeavyBlade)
            && other.plan_delta.support == CardRewardPlanSupportV1::Strong
    }) {
        return None;
    }
    if !candidate
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::WeakCoverage)
    {
        return None;
    }

    Some(CardRewardPickCertificateV1 {
        index: candidate.index,
        card: candidate.card,
        confidence: 0.76,
        reasons: vec![
            "WeakFrontload plan patches visible weak coverage and near-term combat pressure"
                .to_string(),
            "no competing upgrade-sink or strength-payoff plan is strongly supported".to_string(),
        ],
    })
}

fn has_hard_blocker(candidate: &CardRewardCandidateEvidenceV1) -> bool {
    candidate.impact.certification_blockers.iter().any(|gap| {
        matches!(
            gap,
            CardRewardEvidenceGapV1::UnsupportedCardMechanics
                | CardRewardEvidenceGapV1::RandomOutcomeRequiresPolicy
                | CardRewardEvidenceGapV1::ConditionalPlayabilityRequiresPolicy
        )
    })
}

fn stop_reason(gaps: &[CardRewardEvidenceGapV1]) -> String {
    if gaps.is_empty() {
        return "card reward policy stopped because no auto-pick certificate was produced"
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
