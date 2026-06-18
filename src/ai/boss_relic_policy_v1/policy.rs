use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::ai::strategic::run_debt_projection_for_relic_v1;
use crate::content::relics::RelicId;
use crate::state::run::RunState;

use super::evaluator::autopilot_picks;
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
        .map(|(index, relic)| candidate_evidence(run_state, index, relic, &strategy))
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
    let autopilot_picks = autopilot_picks(context, config);

    let action = match autopilot_picks.as_slice() {
        [pick] => BossRelicPolicyActionV1::Pick {
            index: pick.index,
            relic: pick.relic,
            confidence: pick.confidence,
            reason: pick.reason.clone(),
        },
        [] => BossRelicPolicyActionV1::Stop {
            reason: stop_reason(context),
        },
        _ => BossRelicPolicyActionV1::Stop {
            reason: "boss relic policy stopped because multiple autopilot picks matched"
                .to_string(),
        },
    };

    BossRelicDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn candidate_evidence(
    run_state: &RunState,
    index: usize,
    relic: RelicId,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> BossRelicCandidateEvidenceV1 {
    let class = classify_boss_relic(relic);
    let support_gate = support_gate_for_candidate(relic, class, strategy);
    let debt_projection = run_debt_projection_for_relic_v1(run_state, relic);
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
            risks.push("boss relic policy did not select this relic for autopilot".to_string());
        }
    }

    for contract in &debt_projection.added_contracts {
        evidence.push(format!(
            "adds run debt contract {}={}",
            contract.source,
            contract.kind.label()
        ));
    }
    for label in &debt_projection.compounding_labels {
        risks.push(format!("debt compounding: {label}"));
    }

    BossRelicCandidateEvidenceV1 {
        index,
        relic,
        class,
        support_gate,
        debt_projection,
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
    format!("boss relic policy stopped because no autopilot pick matched ({classes})")
}
