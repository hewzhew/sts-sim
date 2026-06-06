use crate::content::cards::CardId;

use super::types::{
    RunStrategySnapshotV1, RunStrategySnapshotV2, StrategyCandidateFactsV1,
    StrategyCandidatePlanDeltaV1, StrategyPlanEffectV1, StrategyPlanIdV1, StrategyPlanSupportV1,
};

pub fn candidate_plan_delta_v2(
    facts: StrategyCandidateFactsV1,
    snapshot: &RunStrategySnapshotV2,
) -> StrategyCandidatePlanDeltaV1 {
    candidate_plan_delta_v1(facts, &snapshot.v1)
}

pub fn candidate_plan_delta_v1(
    facts: StrategyCandidateFactsV1,
    snapshot: &RunStrategySnapshotV1,
) -> StrategyCandidatePlanDeltaV1 {
    let mut effects = Vec::new();
    let mut notes = Vec::new();
    let mut support = StrategyPlanSupportV1::Weak;

    if facts.damage_total > 0 {
        effects.push(StrategyPlanEffectV1::FrontloadDamage);
    }

    match facts.card {
        CardId::SearingBlow => {
            effects.push(StrategyPlanEffectV1::UpgradeSink);
            effects.push(StrategyPlanEffectV1::UpgradeBudgetConsumer);
            let plan = snapshot.plan(StrategyPlanIdV1::UpgradeSink);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::HeavyBlade => {
            effects.push(StrategyPlanEffectV1::StrengthPayoff);
            let plan = snapshot.plan(StrategyPlanIdV1::StrengthScaling);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::Clothesline => {
            effects.push(StrategyPlanEffectV1::WeakCoverage);
            effects.push(StrategyPlanEffectV1::DamageMitigation);
            let plan = snapshot.plan(StrategyPlanIdV1::WeakControl);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::Barricade => {
            effects.push(StrategyPlanEffectV1::BlockRetention);
            effects.push(StrategyPlanEffectV1::DamageMitigation);
            let plan = snapshot.plan(StrategyPlanIdV1::BlockEngine);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::BodySlam => {
            effects.push(StrategyPlanEffectV1::BlockPayoff);
            let plan = snapshot.plan(StrategyPlanIdV1::BlockEngine);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::Entrench => {
            effects.push(StrategyPlanEffectV1::BlockMultiplier);
            effects.push(StrategyPlanEffectV1::DamageMitigation);
            let plan = snapshot.plan(StrategyPlanIdV1::BlockEngine);
            support = plan
                .map(|plan| plan.support)
                .unwrap_or(StrategyPlanSupportV1::Blocked);
            if let Some(plan) = plan {
                notes.extend(plan.evidence.clone());
                notes.extend(plan.blockers.clone());
                notes.extend(plan.opportunity_costs.clone());
            }
        }
        CardId::BurningPact
        | CardId::TrueGrit
        | CardId::SecondWind
        | CardId::SeverSoul
        | CardId::FiendFire
        | CardId::Exhume => {
            effects.push(StrategyPlanEffectV1::ExhaustGenerator);
            apply_plan_context(
                snapshot,
                StrategyPlanIdV1::ExhaustEngine,
                &mut support,
                &mut notes,
            );
        }
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption => {
            effects.push(StrategyPlanEffectV1::ExhaustPayoff);
            apply_plan_context(
                snapshot,
                StrategyPlanIdV1::ExhaustEngine,
                &mut support,
                &mut notes,
            );
        }
        CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough | CardId::Immolate => {
            effects.push(StrategyPlanEffectV1::StatusGenerator);
            apply_plan_context(
                snapshot,
                StrategyPlanIdV1::StatusPackage,
                &mut support,
                &mut notes,
            );
        }
        CardId::Evolve | CardId::FireBreathing => {
            effects.push(StrategyPlanEffectV1::StatusPayoff);
            apply_plan_context(
                snapshot,
                StrategyPlanIdV1::StatusPackage,
                &mut support,
                &mut notes,
            );
        }
        _ => {
            if facts.weak > 0 {
                effects.push(StrategyPlanEffectV1::WeakCoverage);
                effects.push(StrategyPlanEffectV1::DamageMitigation);
                support = snapshot
                    .plan(StrategyPlanIdV1::WeakControl)
                    .map(|plan| plan.support)
                    .unwrap_or(StrategyPlanSupportV1::Weak);
            }
            if facts.strength_gain > 0 {
                support = StrategyPlanSupportV1::Plausible;
                notes.push("card contributes a visible strength source".to_string());
            }
        }
    }

    StrategyCandidatePlanDeltaV1 {
        effects,
        support,
        notes,
    }
}

fn apply_plan_context(
    snapshot: &RunStrategySnapshotV1,
    plan_id: StrategyPlanIdV1,
    support: &mut StrategyPlanSupportV1,
    notes: &mut Vec<String>,
) {
    let plan = snapshot.plan(plan_id);
    *support = plan
        .map(|plan| plan.support)
        .unwrap_or(StrategyPlanSupportV1::Blocked);
    if let Some(plan) = plan {
        notes.extend(plan.evidence.clone());
        notes.extend(plan.blockers.clone());
        notes.extend(plan.opportunity_costs.clone());
    }
}
