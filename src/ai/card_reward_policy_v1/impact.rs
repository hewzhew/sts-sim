use super::facts::scaling_signals;
use super::types::{
    CardRewardCandidateImpactV1, CardRewardDependencyAssessmentV1, CardRewardDependencyStatusV1,
    CardRewardEvidenceGapV1, CardRewardFactsV1, CardRewardPickDependencyV1,
    CardRewardRouteEvidenceV1, DeckProfileV1,
};

pub(crate) fn candidate_impact(
    facts: &CardRewardFactsV1,
    deck: &DeckProfileV1,
    route: Option<&CardRewardRouteEvidenceV1>,
) -> CardRewardCandidateImpactV1 {
    let mut dependency_assessments = Vec::new();
    let mut approval_blockers = Vec::new();
    let mut evidence_notes = Vec::new();

    for dependency in &facts.pick_dependencies {
        let assessment = assess_dependency(*dependency, deck, route);
        if let Some(gap) = blocker_for_assessment(&assessment) {
            push_gap(&mut approval_blockers, gap);
        }
        evidence_notes.push(assessment.reason.clone());
        dependency_assessments.push(assessment);
    }

    for unsupported in &facts.unsupported_mechanics {
        push_gap(
            &mut approval_blockers,
            CardRewardEvidenceGapV1::UnsupportedCardMechanics,
        );
        evidence_notes.push(format!("unsupported mechanics: {unsupported}"));
    }
    if facts.is_random_output {
        push_gap(
            &mut approval_blockers,
            CardRewardEvidenceGapV1::RandomOutcomeRequiresPolicy,
        );
        evidence_notes.push("random output requires an explicit distribution policy".to_string());
    }
    if facts.has_conditional_playability {
        push_gap(
            &mut approval_blockers,
            CardRewardEvidenceGapV1::ConditionalPlayabilityRequiresPolicy,
        );
        evidence_notes.push(
            "conditional playability must be evaluated against current deck and expected combats"
                .to_string(),
        );
    }

    CardRewardCandidateImpactV1 {
        added_deck_size: 1,
        frontload_damage_delta: facts.damage.total_damage,
        block_delta: facts.block,
        draw_delta: facts.draw_cards,
        energy_delta: facts.energy_gain,
        scaling_signals: scaling_signals(facts),
        dependency_assessments,
        approval_blockers,
        evidence_notes,
    }
}

fn assess_dependency(
    dependency: CardRewardPickDependencyV1,
    deck: &DeckProfileV1,
    route: Option<&CardRewardRouteEvidenceV1>,
) -> CardRewardDependencyAssessmentV1 {
    match dependency {
        CardRewardPickDependencyV1::RouteUpgradeDensity => match route.and_then(|r| r.selected_route.as_ref()) {
            Some(selected) if selected.max_fires >= 3 => CardRewardDependencyAssessmentV1 {
                dependency,
                status: CardRewardDependencyStatusV1::Unknown,
                reason: "route has multiple possible fires, but no committed upgrade plan proves this card should consume them".to_string(),
            },
            Some(_) => CardRewardDependencyAssessmentV1 {
                dependency,
                status: CardRewardDependencyStatusV1::Unsatisfied,
                reason: "visible route evidence does not show enough fire density for an upgrade-dependent plan".to_string(),
            },
            None => CardRewardDependencyAssessmentV1 {
                dependency,
                status: CardRewardDependencyStatusV1::Unknown,
                reason: "route fire density is unavailable".to_string(),
            },
        },
        CardRewardPickDependencyV1::StrengthScaling => {
            if deck.strength_sources > 0 {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Satisfied,
                    reason: format!(
                        "deck has {} strength source(s)",
                        deck.strength_sources
                    ),
                }
            } else {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unsatisfied,
                    reason: "deck has no observed strength source for a strength payoff".to_string(),
                }
            }
        }
        CardRewardPickDependencyV1::BlockDensity => {
            if deck.total_block > 0 {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unknown,
                    reason: "deck has block cards, but no block-engine plan is established"
                        .to_string(),
                }
            } else {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unsatisfied,
                    reason: "deck has no block support for a block payoff".to_string(),
                }
            }
        }
        CardRewardPickDependencyV1::StrikeDensity => {
            if deck.starter_strikes >= 4 {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unknown,
                    reason: "deck has strikes, but strike-density payoff is a plan-level dependency"
                        .to_string(),
                }
            } else {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unsatisfied,
                    reason: "deck does not retain enough strike-density evidence".to_string(),
                }
            }
        }
        CardRewardPickDependencyV1::ExhaustPackage => {
            if deck.exhaust_generators > 0 || deck.exhaust_payoffs > 0 {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unknown,
                    reason: "deck has exhaust-related cards, but no exhaust package evidence exists"
                        .to_string(),
                }
            } else {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unsatisfied,
                    reason: "deck has no exhaust package evidence".to_string(),
                }
            }
        }
        CardRewardPickDependencyV1::StatusPackage => {
            if deck.status_generators > 0 || deck.status_payoffs > 0 {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unknown,
                    reason: "deck has status-related cards, but no status package evidence exists"
                        .to_string(),
                }
            } else {
                CardRewardDependencyAssessmentV1 {
                    dependency,
                    status: CardRewardDependencyStatusV1::Unsatisfied,
                    reason: "deck has no status package evidence".to_string(),
                }
            }
        }
        CardRewardPickDependencyV1::SelfDamagePackage => CardRewardDependencyAssessmentV1 {
            dependency,
            status: CardRewardDependencyStatusV1::Unknown,
            reason: "self-damage payoff requires hp/relic/combat-plan evidence".to_string(),
        },
        CardRewardPickDependencyV1::RandomOutputPolicy => CardRewardDependencyAssessmentV1 {
            dependency,
            status: CardRewardDependencyStatusV1::Unknown,
            reason: "random-output card requires a distribution policy".to_string(),
        },
        CardRewardPickDependencyV1::ConditionalPlayabilityPolicy => {
            CardRewardDependencyAssessmentV1 {
                dependency,
                status: CardRewardDependencyStatusV1::Unknown,
                reason: "conditional playability requires a dedicated policy".to_string(),
            }
        }
        CardRewardPickDependencyV1::UnsupportedMechanics => CardRewardDependencyAssessmentV1 {
            dependency,
            status: CardRewardDependencyStatusV1::Unknown,
            reason: "card has mechanics not covered by the reward evidence layer".to_string(),
        },
    }
}

fn blocker_for_assessment(
    assessment: &CardRewardDependencyAssessmentV1,
) -> Option<CardRewardEvidenceGapV1> {
    if assessment.status == CardRewardDependencyStatusV1::Satisfied {
        return None;
    }
    match assessment.dependency {
        CardRewardPickDependencyV1::RouteUpgradeDensity => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedRouteUpgradeEvidence)
        }
        CardRewardPickDependencyV1::StrengthScaling => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedStrengthScalingEvidence)
        }
        CardRewardPickDependencyV1::BlockDensity => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedBlockDensityEvidence)
        }
        CardRewardPickDependencyV1::StrikeDensity => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedStrikeDensityEvidence)
        }
        CardRewardPickDependencyV1::ExhaustPackage => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedExhaustPackageEvidence)
        }
        CardRewardPickDependencyV1::StatusPackage => {
            Some(CardRewardEvidenceGapV1::UnsatisfiedStatusPackageEvidence)
        }
        CardRewardPickDependencyV1::SelfDamagePackage => {
            Some(CardRewardEvidenceGapV1::MissingStrategicPlanEvidence)
        }
        CardRewardPickDependencyV1::RandomOutputPolicy => {
            Some(CardRewardEvidenceGapV1::RandomOutcomeRequiresPolicy)
        }
        CardRewardPickDependencyV1::ConditionalPlayabilityPolicy => {
            Some(CardRewardEvidenceGapV1::ConditionalPlayabilityRequiresPolicy)
        }
        CardRewardPickDependencyV1::UnsupportedMechanics => {
            Some(CardRewardEvidenceGapV1::UnsupportedCardMechanics)
        }
    }
}

fn push_gap(gaps: &mut Vec<CardRewardEvidenceGapV1>, gap: CardRewardEvidenceGapV1) {
    if !gaps.contains(&gap) {
        gaps.push(gap);
    }
}
