use super::types::{
    CardRewardDecisionContextV1, CardRewardEvidenceGapV1, CardRewardPickCertificateV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1,
};

pub(crate) fn pick_gate(
    context: &CardRewardDecisionContextV1,
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

    for candidate in &context.candidates {
        for gap in &candidate.impact.certification_blockers {
            push_gap(&mut gaps, *gap);
        }
    }

    let certificate = if config.allow_automatic_pick_certificates {
        certified_pick(context, &gaps)
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

    push_gap(&mut gaps, CardRewardEvidenceGapV1::NoAutoPickCertificate);
    (
        CardRewardPolicyActionV1::Stop {
            reason: stop_reason(&gaps),
        },
        gaps,
        None,
    )
}

fn certified_pick(
    _context: &CardRewardDecisionContextV1,
    _gaps: &[CardRewardEvidenceGapV1],
) -> Option<CardRewardPickCertificateV1> {
    // This is intentionally empty until a separate strategy source can prove a
    // card reward is covered by deck, route, and archetype evidence. The middle
    // layer is still useful: it records why no certificate exists.
    None
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
