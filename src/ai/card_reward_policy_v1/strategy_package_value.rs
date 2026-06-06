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
    let mut components = vec![
        CardRewardValueComponentV1 {
            name: "candidate_plan_support".to_string(),
            value: support_score(candidate.plan_delta.support),
        },
        CardRewardValueComponentV1 {
            name: "strategy_support_core_plan_protection".to_string(),
            value: support_score(
                context
                    .strategy
                    .support(StrategyPackageIdV2::CorePlanProtection),
            ),
        },
        CardRewardValueComponentV1 {
            name: "strategy_support_combat_patch_window".to_string(),
            value: support_score(
                context
                    .strategy
                    .support(StrategyPackageIdV2::CombatPatchWindow),
            ),
        },
        CardRewardValueComponentV1 {
            name: "strategy_support_upgrade_commitment".to_string(),
            value: support_score(
                context
                    .strategy
                    .support(StrategyPackageIdV2::UpgradeCommitment),
            ),
        },
        CardRewardValueComponentV1 {
            name: "strategy_support_strength_scaling".to_string(),
            value: support_score(
                context
                    .strategy
                    .support(StrategyPackageIdV2::StrengthScaling),
            ),
        },
    ];
    components.extend(candidate.plan_delta.effects.iter().map(|effect| {
        CardRewardValueComponentV1 {
            name: format!("plan_effect_{effect:?}"),
            value: 1.0,
        }
    }));
    components
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
