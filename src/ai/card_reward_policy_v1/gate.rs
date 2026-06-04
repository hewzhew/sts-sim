use super::types::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1, CardRewardEvidenceGapV1,
    CardRewardPickCertificateV1, CardRewardPlanEffectV1, CardRewardPlanSupportV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardStopDispositionV1,
    CardRewardValueEstimateV1, CardRewardValueStatusV1,
};

use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::content::cards::{CardId, CardType};

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
        CardId::SearingBlow => upgrade_sink_certificate(context, candidate),
        CardId::HeavyBlade => strength_payoff_certificate(candidate),
        CardId::Clothesline => weak_frontload_certificate(context, candidate),
        _ => transition_frontload_certificate(context, candidate),
    }
}

fn upgrade_sink_certificate(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if candidate.plan_delta.support != CardRewardPlanSupportV1::Strong {
        return None;
    }
    let route_package = context
        .strategy
        .package(StrategyPackageIdV2::UpgradeCommitment)?;
    if route_package.support != StrategyPlanSupportV1::Strong {
        return None;
    }
    Some(CardRewardPickCertificateV1 {
        index: candidate.index,
        card: candidate.card,
        confidence: 0.82,
        reasons: vec![
            format!(
                "UpgradeCommitment route package is {:?}: {}",
                route_package.support,
                route_package.evidence.join(", ")
            ),
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
    let combat_patch = context
        .strategy
        .package(StrategyPackageIdV2::CombatPatchWindow)?;
    if combat_patch.support != StrategyPlanSupportV1::Strong {
        return None;
    }
    if context
        .strategy
        .support(StrategyPackageIdV2::CorePlanProtection)
        == StrategyPlanSupportV1::Strong
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
            format!(
                "CombatPatchWindow route package is {:?}: {}",
                combat_patch.support,
                combat_patch.evidence.join(", ")
            ),
            "no competing upgrade-sink or strength-payoff plan is strongly supported".to_string(),
        ],
    })
}

fn transition_frontload_certificate(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> Option<CardRewardPickCertificateV1> {
    if context.run.floor <= 0 {
        return None;
    }
    if candidate.card_type != CardType::Attack {
        return None;
    }
    if candidate.facts.cost < 0 || candidate.facts.cost > 1 {
        return None;
    }
    if !candidate
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::FrontloadDamage)
    {
        return None;
    }
    if candidate.impact.frontload_damage_delta < transition_frontload_floor(context) {
        return None;
    }
    if has_competing_strong_plan_candidate(context, candidate.index) {
        return None;
    }

    let frontload = context
        .strategy
        .package(StrategyPackageIdV2::FrontloadSurvival)?;
    if !matches!(
        frontload.support,
        StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
    ) {
        return None;
    }
    let combat_patch = context
        .strategy
        .package(StrategyPackageIdV2::CombatPatchWindow)?;
    if !matches!(
        combat_patch.support,
        StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
    ) {
        return None;
    }

    Some(CardRewardPickCertificateV1 {
        index: candidate.index,
        card: candidate.card,
        confidence: 0.72,
        reasons: vec![
            format!(
                "FrontloadSurvival is {:?}: {}",
                frontload.support,
                frontload.evidence.join(", ")
            ),
            format!(
                "CombatPatchWindow is {:?}: {}",
                combat_patch.support,
                combat_patch.evidence.join(", ")
            ),
            format!(
                "deterministic low-cost attack adds {} frontload damage against a deck average threshold of {}",
                candidate.impact.frontload_damage_delta,
                transition_frontload_floor(context)
            ),
        ],
    })
}

fn transition_frontload_floor(context: &CardRewardDecisionContextV1) -> i32 {
    let average_attack = if context.deck.attacks > 0 {
        (context.deck.total_attack_damage + i32::from(context.deck.attacks) - 1)
            / i32::from(context.deck.attacks)
    } else {
        0
    };
    average_attack.saturating_add(2).max(8)
}

fn has_competing_strong_plan_candidate(
    context: &CardRewardDecisionContextV1,
    candidate_index: usize,
) -> bool {
    context.candidates.iter().any(|other| {
        other.index != candidate_index
            && other.plan_delta.support == CardRewardPlanSupportV1::Strong
            && other.plan_delta.effects.iter().any(|effect| {
                matches!(
                    effect,
                    CardRewardPlanEffectV1::UpgradeSink
                        | CardRewardPlanEffectV1::StrengthPayoff
                        | CardRewardPlanEffectV1::WeakCoverage
                )
            })
            && !has_hard_blocker(other)
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
