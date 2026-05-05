use super::super::context::BossRelicContext;
use super::super::types::{RelicCompatibility, RelicJudgement};

pub(super) fn eval_black_blood(context: &BossRelicContext) -> RelicJudgement {
    let low_hp_bonus = ((0.70 - context.need.hp_ratio).max(0.0) * 70.0).round() as i32;
    let upside = 18
        + low_hp_bonus
        + context.need.survival_pressure / 5
        + (50 - context.sustain_strength).max(0) / 2;
    let downside = if context.reward_dependence >= 60 {
        10
    } else {
        2
    };
    let compatibility = if context.need.hp_ratio < 0.55 && context.sustain_strength < 45 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        6,
        82,
        "boss_relic_sustain",
        vec!["act_healing", "survival_buffer"],
        vec!["lower_ceiling"],
    )
}

pub(super) fn eval_calling_bell(context: &BossRelicContext) -> RelicJudgement {
    let upside = 24 + context.volatility_tolerance / 4;
    let downside = 16 + (100 - context.curse_tolerance) / 3 + context.need.purge_pressure / 10;
    let compatibility = if context.curse_tolerance < 25 && context.need.purge_pressure >= 90 {
        RelicCompatibility::HighRisk
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        34,
        72,
        "boss_relic_random_relics",
        vec!["triple_relic_roll"],
        vec!["curse_payload", "high_variance"],
    )
}

pub(super) fn eval_runic_pyramid(context: &BossRelicContext) -> RelicJudgement {
    let retention_value = context.pyramid_retention_quality / 2;
    let conversion_value = context.pyramid_cleanup_capacity / 2;
    let clog_risk = context.pyramid_clog_liability / 2;
    let upside = 14 + retention_value + conversion_value;
    let downside = 10 + clog_risk;
    let compatibility =
        if context.pyramid_clog_liability >= 72 && context.pyramid_cleanup_capacity < 45 {
            RelicCompatibility::HardReject
        } else if context.pyramid_retention_quality >= 60
            && context.pyramid_cleanup_capacity >= 50
            && context.pyramid_clog_liability <= 45
        {
            RelicCompatibility::StrongFit
        } else if downside > upside {
            RelicCompatibility::HighRisk
        } else {
            RelicCompatibility::Neutral
        };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        14,
        92,
        "boss_relic_retain_engine",
        vec!["retention_quality", "hand_conversion_ability"],
        vec!["hand_clog_risk"],
    )
}

pub(super) fn eval_slavers_collar(context: &BossRelicContext) -> RelicJudgement {
    let elite_bonus = match context.elite_distance {
        Some(distance) if distance <= 2 => 18,
        Some(distance) if distance <= 4 => 10,
        Some(_) => 4,
        None => 2,
    };
    let upside = 12 + elite_bonus + context.attack_count.min(8);
    let downside = if context.elite_distance.is_none() {
        8
    } else {
        2
    };
    let compatibility = if context.elite_distance.is_some_and(|distance| distance <= 2) {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        6,
        74,
        "boss_relic_elite_energy",
        vec!["elite_spike"],
        vec!["hallway_blank"],
    )
}

pub(super) fn eval_tiny_house(context: &BossRelicContext) -> RelicJudgement {
    let upside = 16 + context.need.upgrade_pressure / 10 + context.need.purge_pressure / 12;
    RelicJudgement::new(
        RelicCompatibility::Neutral,
        upside,
        2,
        4,
        90,
        "boss_relic_safe_bundle",
        vec!["low_volatility", "floor_raising_bundle"],
        vec![],
    )
}

pub(super) fn eval_unmodeled() -> RelicJudgement {
    RelicJudgement::new(
        RelicCompatibility::HighRisk,
        4,
        12,
        30,
        20,
        "unmodeled_boss_relic",
        vec![],
        vec!["low_confidence"],
    )
}
