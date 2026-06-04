use super::certificates::certified_pick;
use super::types::{
    CardRewardDecisionContextV1, CardRewardEvidenceGapV1, CardRewardPickCertificateV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardStopDispositionV1,
    CardRewardValueEstimateV1, CardRewardValueStatusV1,
};

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
                disposition: CardRewardStopDispositionV1::MayOpenRewardItem,
            },
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
            disposition: CardRewardStopDispositionV1::MayOpenRewardItem,
        },
        gaps,
        None,
    )
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
