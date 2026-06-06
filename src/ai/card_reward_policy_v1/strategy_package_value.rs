use crate::ai::noncombat_strategy_v1::{
    StrategyPackageGapV2, StrategyPackageIdV2, StrategyPlanEffectV1, StrategyPlanSupportV1,
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
            let gap_fill_bonus = strategy_package_gap_fill_bonus(context, candidate);
            let survival_delta =
                support_score * survival_effect_weight(&candidate.plan_delta.effects);
            let progress_delta = support_score
                * progress_effect_weight(&candidate.plan_delta.effects)
                + gap_fill_bonus;
            let deck_consistency_delta = support_score * 0.15 + gap_fill_bonus * 0.5
                - core_plan_dilution_penalty(context, support_score);

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
    components.extend(strategy_package_gap_fill_components(context, candidate));
    components
}

fn strategy_package_gap_fill_bonus(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> f32 {
    strategy_package_gap_fill_components(context, candidate).len() as f32 * 0.10
}

fn strategy_package_gap_fill_components(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    let mut components = Vec::new();
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::BlockEngine,
        StrategyPackageGapV2::BlockRetention,
        StrategyPlanEffectV1::BlockRetention,
        "strategy_gap_block_engine_block_retention_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::BlockEngine,
        StrategyPackageGapV2::BlockPayoff,
        StrategyPlanEffectV1::BlockPayoff,
        "strategy_gap_block_engine_block_payoff_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::BlockEngine,
        StrategyPackageGapV2::BlockMultiplier,
        StrategyPlanEffectV1::BlockMultiplier,
        "strategy_gap_block_engine_block_multiplier_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::ExhaustEngine,
        StrategyPackageGapV2::Generator,
        StrategyPlanEffectV1::ExhaustGenerator,
        "strategy_gap_exhaust_engine_generator_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::ExhaustEngine,
        StrategyPackageGapV2::Payoff,
        StrategyPlanEffectV1::ExhaustPayoff,
        "strategy_gap_exhaust_engine_payoff_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::StatusPackage,
        StrategyPackageGapV2::Generator,
        StrategyPlanEffectV1::StatusGenerator,
        "strategy_gap_status_package_generator_filled",
    );
    push_gap_component(
        &mut components,
        context,
        candidate,
        StrategyPackageIdV2::StatusPackage,
        StrategyPackageGapV2::Payoff,
        StrategyPlanEffectV1::StatusPayoff,
        "strategy_gap_status_package_payoff_filled",
    );
    components
}

fn push_gap_component(
    components: &mut Vec<CardRewardValueComponentV1>,
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
    package_id: StrategyPackageIdV2,
    gap: StrategyPackageGapV2,
    effect: StrategyPlanEffectV1,
    name: &'static str,
) {
    let fills_gap = context
        .strategy
        .package(package_id)
        .map(|package| {
            package.support != StrategyPlanSupportV1::Blocked
                && package.missing_roles.contains(&gap)
        })
        .unwrap_or(false)
        && candidate.plan_delta.effects.contains(&effect);
    if fills_gap {
        components.push(CardRewardValueComponentV1 {
            name: name.to_string(),
            value: 1.0,
        });
    }
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
                    | StrategyPlanEffectV1::ExhaustPayoff
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
                    | StrategyPlanEffectV1::ExhaustGenerator
                    | StrategyPlanEffectV1::ExhaustPayoff
                    | StrategyPlanEffectV1::StatusGenerator
                    | StrategyPlanEffectV1::StatusPayoff
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
