use crate::ai::noncombat_strategy_v1::{
    StrategyPackageIdV2, StrategyPlanEffectV1, StrategyPlanSupportV1,
};

use super::types::{
    CardRewardDecisionContextV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueHorizonV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

const STRONG_CORE_PLAN_DILUTION_PENALTY: f32 = 0.18;
const PACKAGE_ALIGNMENT_UNCERTAINTY: f32 = 0.74;

pub(crate) fn estimate_strategy_package_values(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueEstimateV1> {
    context
        .candidates
        .iter()
        .map(|candidate| {
            let support_score = support_score(candidate.plan_delta.support);
            let survival_delta =
                support_score * survival_effect_weight(&candidate.plan_delta.effects);
            let progress_delta =
                support_score * progress_effect_weight(&candidate.plan_delta.effects);
            let deck_consistency_delta =
                support_score * 0.15 - core_plan_dilution_penalty(context, support_score);

            CardRewardValueEstimateV1 {
                index: candidate.index,
                card: candidate.card,
                source: CardRewardValueSourceV1::StrategyPackage,
                status: CardRewardValueStatusV1::StrategyPackageEstimate,
                survival_delta,
                progress_delta,
                deck_consistency_delta,
                uncertainty: PACKAGE_ALIGNMENT_UNCERTAINTY,
                eligibility: CardRewardValueEligibilityV1 {
                    usable_for_value_estimate: true,
                    usable_for_autopilot_gate: false,
                    reasons: vec![
                        CardRewardValueEligibilityReasonV1::StrategyPackageEstimateNotPromoted,
                    ],
                    bucket_key: None,
                    horizon: Some(CardRewardValueHorizonV1::CurrentStrategyPackage),
                    outcome_sample_count: None,
                },
                components: strategy_package_components(context, candidate),
            }
        })
        .collect()
}

fn strategy_package_components(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    let mut components = vec![CardRewardValueComponentV1 {
        name: "candidate_plan_support".to_string(),
        value: support_score(candidate.plan_delta.support),
    }];
    components.extend(package_support_components(context));
    components.extend(candidate.plan_delta.effects.iter().map(|effect| {
        CardRewardValueComponentV1 {
            name: format!("plan_effect_{effect:?}"),
            value: 1.0,
        }
    }));
    components
}

fn package_support_components(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueComponentV1> {
    [
        ("frontload_survival", StrategyPackageIdV2::FrontloadSurvival),
        ("weak_control", StrategyPackageIdV2::WeakControl),
        ("strength_scaling", StrategyPackageIdV2::StrengthScaling),
        ("upgrade_sink", StrategyPackageIdV2::UpgradeSink),
        ("exhaust_engine", StrategyPackageIdV2::ExhaustEngine),
        ("block_engine", StrategyPackageIdV2::BlockEngine),
        ("strike_density", StrategyPackageIdV2::StrikeDensity),
        ("status_package", StrategyPackageIdV2::StatusPackage),
        ("self_damage", StrategyPackageIdV2::SelfDamage),
        ("energy_draw", StrategyPackageIdV2::EnergyDraw),
        (
            "combat_patch_window",
            StrategyPackageIdV2::CombatPatchWindow,
        ),
        ("upgrade_commitment", StrategyPackageIdV2::UpgradeCommitment),
        (
            "core_plan_protection",
            StrategyPackageIdV2::CorePlanProtection,
        ),
        ("recovery_pressure", StrategyPackageIdV2::RecoveryPressure),
        ("gold_plan", StrategyPackageIdV2::GoldPlan),
        ("potion_capacity", StrategyPackageIdV2::PotionCapacity),
        ("hp_safety", StrategyPackageIdV2::HpSafety),
        ("shop_remove_window", StrategyPackageIdV2::ShopRemoveWindow),
        ("relic_constraints", StrategyPackageIdV2::RelicConstraints),
    ]
    .into_iter()
    .map(|(name, package)| CardRewardValueComponentV1 {
        name: format!("strategy_support_{name}"),
        value: support_score(context.strategy.support(package)),
    })
    .collect()
}

fn survival_effect_weight(effects: &[StrategyPlanEffectV1]) -> f32 {
    let survival_effects = effects
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                StrategyPlanEffectV1::FrontloadDamage
                    | StrategyPlanEffectV1::WeakCoverage
                    | StrategyPlanEffectV1::DamageMitigation
                    | StrategyPlanEffectV1::BlockRetention
                    | StrategyPlanEffectV1::BlockMultiplier
            )
        })
        .count() as f32;
    (survival_effects * 0.25).clamp(0.0, 0.75)
}

fn progress_effect_weight(effects: &[StrategyPlanEffectV1]) -> f32 {
    let progress_effects = effects
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                StrategyPlanEffectV1::FrontloadDamage
                    | StrategyPlanEffectV1::StrengthPayoff
                    | StrategyPlanEffectV1::UpgradeSink
                    | StrategyPlanEffectV1::BlockPayoff
            )
        })
        .count() as f32;
    (progress_effects * 0.25).clamp(0.0, 0.75)
}

fn core_plan_dilution_penalty(context: &CardRewardDecisionContextV1, support_score: f32) -> f32 {
    if context
        .strategy
        .support(StrategyPackageIdV2::CorePlanProtection)
        == StrategyPlanSupportV1::Strong
        && support_score < 0.25
    {
        STRONG_CORE_PLAN_DILUTION_PENALTY
    } else {
        0.0
    }
}

fn support_score(support: StrategyPlanSupportV1) -> f32 {
    match support {
        StrategyPlanSupportV1::Blocked => -0.25,
        StrategyPlanSupportV1::Weak => 0.05,
        StrategyPlanSupportV1::Plausible => 0.30,
        StrategyPlanSupportV1::Strong => 0.60,
    }
}
