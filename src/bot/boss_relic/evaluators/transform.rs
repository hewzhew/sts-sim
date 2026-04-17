use super::super::context::BossRelicContext;
use super::super::types::{RelicCompatibility, RelicJudgement};
use super::support::act4_setup_bonus;

pub(super) fn eval_astrolabe(context: &BossRelicContext) -> RelicJudgement {
    let upside = 24 + context.transform_targets_value / 4 + act4_setup_bonus(context);
    let downside = if context.deck_maturity >= 70 { 12 } else { 4 };
    let compatibility = if context.starter_cards >= 3 || context.reward_dependence >= 55 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        22,
        78,
        "boss_relic_transform_cards",
        vec!["transform_targets", "upgrade_bundle"],
        vec!["variance"],
    )
}

pub(super) fn eval_empty_cage(context: &BossRelicContext) -> RelicJudgement {
    let upside = 20 + context.remove_targets_value / 3 + act4_setup_bonus(context);
    let downside = if context.need.deck_size <= 12 { 8 } else { 2 };
    let compatibility = if context.remove_targets_value >= 80 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        8,
        86,
        "boss_relic_remove_cards",
        vec!["clean_draws", "remove_starters"],
        vec!["low_impact_if_trimmed"],
    )
}

pub(super) fn eval_pandoras_box(context: &BossRelicContext) -> RelicJudgement {
    let upside = 18 + context.pandora_rebuild_value / 2;
    let downside = 8 + context.pandora_plan_disruption_risk / 2;
    let volatility =
        (24 + (100 - context.volatility_tolerance) / 3 + context.pandora_plan_disruption_risk / 4)
            .clamp(0, 100);
    let compatibility = if context.pandora_rebuild_value >= 72
        && context.starter_cards >= 4
        && context.volatility_tolerance >= 35
    {
        RelicCompatibility::StrongFit
    } else if context.pandora_plan_disruption_risk >= 65 && context.pandora_rebuild_value <= 45 {
        RelicCompatibility::HardReject
    } else if context.pandora_plan_disruption_risk > context.pandora_rebuild_value {
        RelicCompatibility::HighRisk
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        volatility,
        84,
        "boss_relic_rebuild_starters",
        vec!["starter_rebuild", "ceiling_spike"],
        vec!["high_variance", "plan_disruption"],
    )
}
