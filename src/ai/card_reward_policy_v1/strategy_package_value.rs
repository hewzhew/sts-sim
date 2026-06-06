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
const PACKAGE_GAP_FILL_PROGRESS_BONUS: f32 = 0.10;
const PACKAGE_COMPLETION_PROGRESS_BONUS: f32 = 0.20;

struct StrategyPackageGapRule {
    package_id: StrategyPackageIdV2,
    gap: StrategyPackageGapV2,
    effect: StrategyPlanEffectV1,
    component_name: &'static str,
}

const STRATEGY_PACKAGE_GAP_RULES: &[StrategyPackageGapRule] = &[
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::UpgradeSink,
        gap: StrategyPackageGapV2::UpgradeConsumer,
        effect: StrategyPlanEffectV1::UpgradeBudgetConsumer,
        component_name: "strategy_gap_upgrade_sink_consumer_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::WeakControl,
        gap: StrategyPackageGapV2::Generator,
        effect: StrategyPlanEffectV1::WeakCoverage,
        component_name: "strategy_gap_weak_control_generator_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        gap: StrategyPackageGapV2::BlockRetention,
        effect: StrategyPlanEffectV1::BlockRetention,
        component_name: "strategy_gap_block_engine_block_retention_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        gap: StrategyPackageGapV2::Generator,
        effect: StrategyPlanEffectV1::StrengthGenerator,
        component_name: "strategy_gap_strength_scaling_generator_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        gap: StrategyPackageGapV2::Payoff,
        effect: StrategyPlanEffectV1::StrengthPayoff,
        component_name: "strategy_gap_strength_scaling_payoff_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        gap: StrategyPackageGapV2::BlockPayoff,
        effect: StrategyPlanEffectV1::BlockPayoff,
        component_name: "strategy_gap_block_engine_block_payoff_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        gap: StrategyPackageGapV2::BlockMultiplier,
        effect: StrategyPlanEffectV1::BlockMultiplier,
        component_name: "strategy_gap_block_engine_block_multiplier_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        gap: StrategyPackageGapV2::Generator,
        effect: StrategyPlanEffectV1::ExhaustGenerator,
        component_name: "strategy_gap_exhaust_engine_generator_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        gap: StrategyPackageGapV2::Payoff,
        effect: StrategyPlanEffectV1::ExhaustPayoff,
        component_name: "strategy_gap_exhaust_engine_payoff_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::StatusPackage,
        gap: StrategyPackageGapV2::Generator,
        effect: StrategyPlanEffectV1::StatusGenerator,
        component_name: "strategy_gap_status_package_generator_filled",
    },
    StrategyPackageGapRule {
        package_id: StrategyPackageIdV2::StatusPackage,
        gap: StrategyPackageGapV2::Payoff,
        effect: StrategyPlanEffectV1::StatusPayoff,
        component_name: "strategy_gap_status_package_payoff_filled",
    },
];

pub(crate) fn estimate_strategy_package_values(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueEstimateV1> {
    context
        .candidates
        .iter()
        .map(|candidate| {
            let support_score = support_score(candidate.plan_delta.support);
            let gap_fill_bonus = strategy_package_gap_fill_bonus(context, candidate);
            let package_completion_bonus = strategy_package_completion_bonus(context, candidate);
            let survival_delta =
                support_score * survival_effect_weight(&candidate.plan_delta.effects);
            let progress_delta = support_score
                * progress_effect_weight(&candidate.plan_delta.effects)
                + gap_fill_bonus
                + package_completion_bonus;
            let deck_consistency_delta =
                support_score * 0.15 + gap_fill_bonus * 0.5 + package_completion_bonus * 0.75
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
    components.extend(strategy_package_completion_components(context, candidate));
    components
}

fn strategy_package_gap_fill_bonus(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> f32 {
    strategy_package_gap_fill_components(context, candidate).len() as f32
        * PACKAGE_GAP_FILL_PROGRESS_BONUS
}

fn strategy_package_completion_bonus(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> f32 {
    strategy_package_completion_components(context, candidate).len() as f32
        * PACKAGE_COMPLETION_PROGRESS_BONUS
}

fn strategy_package_gap_fill_components(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    STRATEGY_PACKAGE_GAP_RULES
        .iter()
        .filter(|rule| package_gap_is_filled(context, candidate, rule))
        .map(|rule| CardRewardValueComponentV1 {
            name: rule.component_name.to_string(),
            value: 1.0,
        })
        .collect()
}

fn strategy_package_completion_components(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    [
        (
            StrategyPackageIdV2::UpgradeSink,
            "strategy_package_completion_upgrade_sink",
        ),
        (
            StrategyPackageIdV2::WeakControl,
            "strategy_package_completion_weak_control",
        ),
        (
            StrategyPackageIdV2::StrengthScaling,
            "strategy_package_completion_strength_scaling",
        ),
        (
            StrategyPackageIdV2::BlockEngine,
            "strategy_package_completion_block_engine",
        ),
        (
            StrategyPackageIdV2::ExhaustEngine,
            "strategy_package_completion_exhaust_engine",
        ),
        (
            StrategyPackageIdV2::StatusPackage,
            "strategy_package_completion_status_package",
        ),
    ]
    .into_iter()
    .filter(|(package_id, _)| package_would_be_completed(context, candidate, *package_id))
    .map(|(_, name)| CardRewardValueComponentV1 {
        name: name.to_string(),
        value: 1.0,
    })
    .collect()
}

fn package_gap_is_filled(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
    rule: &StrategyPackageGapRule,
) -> bool {
    context
        .strategy
        .package(rule.package_id)
        .map(|package| {
            package.support != StrategyPlanSupportV1::Blocked
                && package.missing_roles.contains(&rule.gap)
        })
        .unwrap_or(false)
        && candidate.plan_delta.effects.contains(&rule.effect)
}

fn package_would_be_completed(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
    package_id: StrategyPackageIdV2,
) -> bool {
    let Some(package) = context.strategy.package(package_id) else {
        return false;
    };
    package.support != StrategyPlanSupportV1::Blocked
        && !package.missing_roles.is_empty()
        && package.missing_roles.iter().all(|gap| {
            STRATEGY_PACKAGE_GAP_RULES.iter().any(|rule| {
                rule.package_id == package_id
                    && rule.gap == *gap
                    && candidate.plan_delta.effects.contains(&rule.effect)
            })
        })
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
                    | StrategyPlanEffectV1::StrengthGenerator
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
