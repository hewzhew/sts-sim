use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::relics::RelicId;
use crate::state::run::RunState;

use super::types::{
    BossRelicCandidateEvidenceV1, BossRelicDecisionContextV1, BossRelicDecisionV1,
    BossRelicPolicyActionV1, BossRelicPolicyClassV1, BossRelicPolicyConfigV1,
};

pub fn build_boss_relic_decision_context_v1(
    run_state: &RunState,
    relics: Vec<RelicId>,
) -> BossRelicDecisionContextV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let candidates = relics
        .into_iter()
        .enumerate()
        .map(|(index, relic)| candidate_evidence(index, relic, &strategy))
        .collect();
    BossRelicDecisionContextV1 {
        strategy,
        candidates,
    }
}

pub fn plan_boss_relic_decision_v1(
    context: &BossRelicDecisionContextV1,
    config: &BossRelicPolicyConfigV1,
) -> BossRelicDecisionV1 {
    let certificates = context
        .candidates
        .iter()
        .filter_map(|candidate| pick_certificate(candidate, context, config))
        .collect::<Vec<_>>();

    let action = match certificates.as_slice() {
        [certificate] => BossRelicPolicyActionV1::Pick {
            index: certificate.index,
            relic: certificate.relic,
            confidence: certificate.confidence,
            reason: certificate.reason.clone(),
        },
        [] => BossRelicPolicyActionV1::Stop {
            reason: stop_reason(context),
        },
        _ => BossRelicPolicyActionV1::Stop {
            reason: "boss relic policy stopped because multiple conservative certificates matched"
                .to_string(),
        },
    };

    BossRelicDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PickCertificate {
    index: usize,
    relic: RelicId,
    confidence: f32,
    reason: String,
}

fn pick_certificate(
    candidate: &BossRelicCandidateEvidenceV1,
    context: &BossRelicDecisionContextV1,
    config: &BossRelicPolicyConfigV1,
) -> Option<PickCertificate> {
    match candidate.class {
        BossRelicPolicyClassV1::StarterRelicUpgrade if config.allow_starter_upgrade => {
            Some(PickCertificate {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.95,
                reason: format!(
                    "{:?} upgrades the starter relic with no visible downside",
                    candidate.relic
                ),
            })
        }
        BossRelicPolicyClassV1::DeckCleanup
            if config.allow_empty_cage_when_cleanup_supported
                && candidate.support_gate_at_least(StrategyPlanSupportV1::Plausible)
                && no_higher_agency_competitor(context, candidate.index) =>
        {
            Some(PickCertificate {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.82,
                reason: format!(
                    "{:?} matches cleanup pressure and avoids higher-agency boss relic uncertainty",
                    candidate.relic
                ),
            })
        }
        BossRelicPolicyClassV1::BroadSafeValue
            if config.allow_tiny_house_as_safe_fallback
                && candidate.relic == RelicId::TinyHouse
                && all_other_candidates_are_constrained(context, candidate.index) =>
        {
            Some(PickCertificate {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.78,
                reason:
                    "TinyHouse is the only broad low-downside option against constrained alternatives"
                        .to_string(),
            })
        }
        _ => None,
    }
}

fn candidate_evidence(
    index: usize,
    relic: RelicId,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> BossRelicCandidateEvidenceV1 {
    let class = classify_boss_relic(relic);
    let support_gate = support_gate_for_candidate(relic, class, strategy);
    let mut evidence = vec![format!("boss relic class is {class:?}")];
    let mut risks = Vec::new();

    match class {
        BossRelicPolicyClassV1::StarterRelicUpgrade => {
            evidence.push("replaces the starter relic with a class upgrade".to_string());
        }
        BossRelicPolicyClassV1::DeckCleanup => {
            evidence.push(format!(
                "ShopRemoveWindow support is {:?}",
                strategy.support(StrategyPackageIdV2::ShopRemoveWindow)
            ));
            evidence.push(format!(
                "starter_cards={}",
                strategy.resources.starter_cards
            ));
            evidence.push(format!("curses={}", strategy.resources.curses));
        }
        BossRelicPolicyClassV1::BroadSafeValue => {
            evidence.push(
                "broad value with no explicit downside in the candidate classifier".to_string(),
            );
        }
        BossRelicPolicyClassV1::RouteDependentValue => {
            risks.push("value depends on future route, elite density, or potion usage".to_string());
        }
        BossRelicPolicyClassV1::EnergyWithConstraint => {
            risks.push("extra energy has a persistent strategic constraint".to_string());
        }
        BossRelicPolicyClassV1::TransformAgency => {
            risks.push("changes deck identity or opens irreversible card selection".to_string());
        }
        BossRelicPolicyClassV1::CurseDebt => {
            risks.push("adds curse/debt or future reward constraints".to_string());
        }
        BossRelicPolicyClassV1::StrategicPower => {
            risks.push("high-impact boss relic requires deck-specific human judgment".to_string());
        }
        BossRelicPolicyClassV1::Unknown => {
            risks.push("boss relic policy has no safe certificate for this relic".to_string());
        }
    }

    BossRelicCandidateEvidenceV1 {
        index,
        relic,
        class,
        support_gate,
        evidence,
        risks,
    }
}

fn classify_boss_relic(relic: RelicId) -> BossRelicPolicyClassV1 {
    match relic {
        RelicId::BlackBlood
        | RelicId::RingOfTheSerpent
        | RelicId::FrozenCore
        | RelicId::HolyWater => BossRelicPolicyClassV1::StarterRelicUpgrade,
        RelicId::EmptyCage => BossRelicPolicyClassV1::DeckCleanup,
        RelicId::TinyHouse => BossRelicPolicyClassV1::BroadSafeValue,
        RelicId::BlackStar | RelicId::SacredBark | RelicId::SlaversCollar => {
            BossRelicPolicyClassV1::RouteDependentValue
        }
        RelicId::BustedCrown
        | RelicId::CoffeeDripper
        | RelicId::Ectoplasm
        | RelicId::FusionHammer
        | RelicId::PhilosopherStone
        | RelicId::RunicDome
        | RelicId::Sozu
        | RelicId::VelvetChoker
        | RelicId::MarkOfPain => BossRelicPolicyClassV1::EnergyWithConstraint,
        RelicId::Astrolabe | RelicId::PandorasBox => BossRelicPolicyClassV1::TransformAgency,
        RelicId::CallingBell | RelicId::CursedKey => BossRelicPolicyClassV1::CurseDebt,
        RelicId::RunicPyramid | RelicId::SneckoEye => BossRelicPolicyClassV1::StrategicPower,
        _ => BossRelicPolicyClassV1::Unknown,
    }
}

fn support_gate_for_candidate(
    relic: RelicId,
    class: BossRelicPolicyClassV1,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> StrategyPlanSupportV1 {
    match (relic, class) {
        (_, BossRelicPolicyClassV1::StarterRelicUpgrade) => StrategyPlanSupportV1::Strong,
        (RelicId::EmptyCage, BossRelicPolicyClassV1::DeckCleanup) => {
            let cleanup = strategy.support(StrategyPackageIdV2::ShopRemoveWindow);
            if cleanup != StrategyPlanSupportV1::Blocked {
                cleanup
            } else if strategy.resources.starter_cards >= 7 {
                StrategyPlanSupportV1::Weak
            } else {
                StrategyPlanSupportV1::Blocked
            }
        }
        (RelicId::TinyHouse, BossRelicPolicyClassV1::BroadSafeValue) => {
            if strategy.support(StrategyPackageIdV2::HpSafety) != StrategyPlanSupportV1::Blocked {
                StrategyPlanSupportV1::Plausible
            } else {
                StrategyPlanSupportV1::Weak
            }
        }
        _ => StrategyPlanSupportV1::Blocked,
    }
}

fn no_higher_agency_competitor(
    context: &BossRelicDecisionContextV1,
    selected_index: usize,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.index != selected_index)
        .all(|candidate| {
            matches!(
                candidate.class,
                BossRelicPolicyClassV1::EnergyWithConstraint
                    | BossRelicPolicyClassV1::CurseDebt
                    | BossRelicPolicyClassV1::TransformAgency
                    | BossRelicPolicyClassV1::Unknown
            )
        })
}

fn all_other_candidates_are_constrained(
    context: &BossRelicDecisionContextV1,
    selected_index: usize,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.index != selected_index)
        .all(|candidate| {
            matches!(
                candidate.class,
                BossRelicPolicyClassV1::EnergyWithConstraint
                    | BossRelicPolicyClassV1::CurseDebt
                    | BossRelicPolicyClassV1::TransformAgency
                    | BossRelicPolicyClassV1::Unknown
            )
        })
}

fn stop_reason(context: &BossRelicDecisionContextV1) -> String {
    if context.candidates.is_empty() {
        return "boss relic policy stopped because there are no candidates".to_string();
    }

    let classes = context
        .candidates
        .iter()
        .map(|candidate| format!("{:?}:{:?}", candidate.relic, candidate.class))
        .collect::<Vec<_>>()
        .join(", ");
    format!("boss relic policy stopped because no conservative V2 certificate matched ({classes})")
}

impl BossRelicCandidateEvidenceV1 {
    fn support_gate_at_least(&self, minimum: StrategyPlanSupportV1) -> bool {
        support_rank(self.support_gate) >= support_rank(minimum)
    }
}

fn support_rank(support: StrategyPlanSupportV1) -> u8 {
    match support {
        StrategyPlanSupportV1::Blocked => 0,
        StrategyPlanSupportV1::Weak => 1,
        StrategyPlanSupportV1::Plausible => 2,
        StrategyPlanSupportV1::Strong => 3,
    }
}
