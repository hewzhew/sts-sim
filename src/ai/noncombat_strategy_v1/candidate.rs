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
