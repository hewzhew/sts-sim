use crate::ai::noncombat_strategy_v1::{
    StrategyPackageIdV2, StrategyPlanEffectV1, StrategyThreatSourceV1, StrategyThreatTagV1,
};

use super::strategy_package_value::package_would_be_completed;
use super::types::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1, CardRewardValueComponentV1,
};

const PACKAGE_THREAT_ALIGNMENT_SURVIVAL_BONUS: f32 = 0.08;

#[derive(Clone, Copy)]
enum StrategyThreatAlignmentSource {
    Boss,
    Elite,
}

struct StrategyPackageThreatAlignmentRule {
    package_id: StrategyPackageIdV2,
    package_name: &'static str,
    required_effects: &'static [StrategyPlanEffectV1],
    source: StrategyThreatAlignmentSource,
    tag: StrategyThreatTagV1,
    tag_name: &'static str,
}

const WEAK_CONTROL_ALIGNMENT_EFFECTS: &[StrategyPlanEffectV1] =
    &[StrategyPlanEffectV1::WeakCoverage];
const BLOCK_ENGINE_ALIGNMENT_EFFECTS: &[StrategyPlanEffectV1] = &[
    StrategyPlanEffectV1::BlockRetention,
    StrategyPlanEffectV1::BlockPayoff,
    StrategyPlanEffectV1::BlockMultiplier,
];
const STRENGTH_SCALING_ALIGNMENT_EFFECTS: &[StrategyPlanEffectV1] = &[
    StrategyPlanEffectV1::StrengthGenerator,
    StrategyPlanEffectV1::StrengthPayoff,
];
const EXHAUST_ENGINE_ALIGNMENT_EFFECTS: &[StrategyPlanEffectV1] = &[
    StrategyPlanEffectV1::ExhaustGenerator,
    StrategyPlanEffectV1::ExhaustPayoff,
];

const STRATEGY_PACKAGE_THREAT_ALIGNMENT_RULES: &[StrategyPackageThreatAlignmentRule] = &[
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::WeakControl,
        package_name: "weak_control",
        required_effects: WEAK_CONTROL_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::WeakControl,
        package_name: "weak_control",
        required_effects: WEAK_CONTROL_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::MultiHit,
        tag_name: "multihit",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::WeakControl,
        package_name: "weak_control",
        required_effects: WEAK_CONTROL_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::WeakControl,
        package_name: "weak_control",
        required_effects: WEAK_CONTROL_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::MultiHit,
        tag_name: "multihit",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::MultiHit,
        tag_name: "multihit",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::BlockEngine,
        package_name: "block_engine",
        required_effects: BLOCK_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::HighIncomingDamage,
        tag_name: "high_incoming",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::StrengthScaling,
        package_name: "strength_scaling",
        required_effects: STRENGTH_SCALING_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::StatusFlood,
        tag_name: "status_flood",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Boss,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::StatusFlood,
        tag_name: "status_flood",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::LongFightScaling,
        tag_name: "long_fight",
    },
    StrategyPackageThreatAlignmentRule {
        package_id: StrategyPackageIdV2::ExhaustEngine,
        package_name: "exhaust_engine",
        required_effects: EXHAUST_ENGINE_ALIGNMENT_EFFECTS,
        source: StrategyThreatAlignmentSource::Elite,
        tag: StrategyThreatTagV1::SetupWindow,
        tag_name: "setup_window",
    },
];

pub(super) fn strategy_package_threat_alignment_bonus(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> f32 {
    strategy_package_threat_alignment_components(context, candidate).len() as f32
        * PACKAGE_THREAT_ALIGNMENT_SURVIVAL_BONUS
}

pub(super) fn strategy_package_threat_alignment_components(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    STRATEGY_PACKAGE_THREAT_ALIGNMENT_RULES
        .iter()
        .filter(|rule| package_threat_alignment_rule_applies(context, candidate, rule))
        .map(|rule| threat_alignment_component(rule.package_name, rule.source, rule.tag_name))
        .collect()
}

fn candidate_completes_package_with_any_effect(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
    package_id: StrategyPackageIdV2,
    effects: &[StrategyPlanEffectV1],
) -> bool {
    package_would_be_completed(context, candidate, package_id)
        && effects
            .iter()
            .any(|effect| candidate.plan_delta.effects.contains(effect))
}

fn package_threat_alignment_rule_applies(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
    rule: &StrategyPackageThreatAlignmentRule,
) -> bool {
    if !candidate_completes_package_with_any_effect(
        context,
        candidate,
        rule.package_id,
        rule.required_effects,
    ) {
        return false;
    }
    match rule.source {
        StrategyThreatAlignmentSource::Boss => boss_threat(context, rule.tag),
        StrategyThreatAlignmentSource::Elite => elite_threat_visible_for_route(context, rule.tag),
    }
}

fn threat_alignment_component(
    package_name: &str,
    source: StrategyThreatAlignmentSource,
    tag_name: &str,
) -> CardRewardValueComponentV1 {
    let source_name = match source {
        StrategyThreatAlignmentSource::Boss => "boss",
        StrategyThreatAlignmentSource::Elite => "elite",
    };
    CardRewardValueComponentV1 {
        name: format!("strategy_threat_alignment_{package_name}_{source_name}_{tag_name}"),
        value: PACKAGE_THREAT_ALIGNMENT_SURVIVAL_BONUS,
    }
}

fn boss_threat(context: &CardRewardDecisionContextV1, tag: StrategyThreatTagV1) -> bool {
    context
        .strategy
        .threats
        .sources
        .iter()
        .any(|source| source.source == StrategyThreatSourceV1::ActBoss && source.tag == tag)
}

fn elite_threat_visible_for_route(
    context: &CardRewardDecisionContextV1,
    tag: StrategyThreatTagV1,
) -> bool {
    route_allows_elites(context)
        && context.strategy.threats.sources.iter().any(|source| {
            matches!(
                source.source,
                StrategyThreatSourceV1::ActElitePool | StrategyThreatSourceV1::ActEliteEncounter
            ) && source.tag == tag
        })
}

fn route_allows_elites(context: &CardRewardDecisionContextV1) -> bool {
    context
        .route
        .as_ref()
        .and_then(|route| route.selected_route.as_ref())
        .map(|route| route.max_elites > 0)
        .unwrap_or(true)
}
