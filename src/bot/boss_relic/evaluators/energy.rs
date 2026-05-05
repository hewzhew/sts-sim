use super::super::context::BossRelicContext;
use super::super::types::{RelicCompatibility, RelicJudgement};
use super::support::{rest_soon, smith_soon};

pub(super) fn eval_coffee_dripper(context: &BossRelicContext) -> RelicJudgement {
    let upside = 24 + context.energy_sink_value / 2 + if !rest_soon(context) { 4 } else { 0 };
    let downside = 14 + context.campfire_heal_dependence;
    let compatibility = if context.campfire_heal_dependence >= 55 {
        RelicCompatibility::HardReject
    } else if context.campfire_heal_dependence <= 24
        && context.sustain_strength >= 40
        && context.need.hp_ratio >= 0.72
    {
        RelicCompatibility::StrongFit
    } else if downside > upside + 8 {
        RelicCompatibility::HighRisk
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        22,
        90,
        "boss_relic_energy_no_rest",
        vec!["energy_now", "rest_independent_shell"],
        vec!["campfire_heal_lock"],
    )
}

pub(super) fn eval_fusion_hammer(context: &BossRelicContext) -> RelicJudgement {
    let upside = 24 + context.energy_sink_value / 2 + context.block_core * 2;
    let downside = 12 + context.upgrade_backlog / 2 + if smith_soon(context) { 6 } else { 0 };
    let compatibility = if context.upgrade_backlog >= 100 {
        RelicCompatibility::HardReject
    } else if context.upgrade_backlog >= 72 {
        RelicCompatibility::HighRisk
    } else if context.deck_maturity >= 65 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        14,
        90,
        "boss_relic_energy_no_smith",
        vec!["energy_now", "upgrade_light_shell"],
        vec!["upgrade_lock"],
    )
}

pub(super) fn eval_busted_crown(context: &BossRelicContext) -> RelicJudgement {
    let upside = 18 + context.energy_sink_value / 3 + context.crown_maturity_buffer / 6;
    let downside = 16 + context.crown_reward_dependency / 2;
    let compatibility =
        if context.crown_reward_dependency >= 75 || context.crown_maturity_buffer < 35 {
            RelicCompatibility::HardReject
        } else if context.crown_reward_dependency >= 52 {
            RelicCompatibility::HighRisk
        } else if context.crown_maturity_buffer >= 62 && context.crown_reward_dependency <= 44 {
            RelicCompatibility::StrongFit
        } else {
            RelicCompatibility::Neutral
        };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        10,
        94,
        "boss_relic_reward_lock",
        vec!["energy_now", "closed_shell"],
        vec!["future_reward_lock"],
    )
}

pub(super) fn eval_ectoplasm(context: &BossRelicContext) -> RelicJudgement {
    let upside = 20 + context.energy_sink_value / 3 + context.deck_maturity / 5;
    let downside = 12 + context.shop_value_pressure;
    let compatibility = if context.next_act <= 2 && context.shop_value_pressure >= 26 {
        RelicCompatibility::HardReject
    } else if downside > upside + 4 {
        RelicCompatibility::HighRisk
    } else if context.deck_maturity >= 75 && context.next_act >= 3 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        12,
        86,
        "boss_relic_gold_lock",
        vec!["energy_now", "shop_light_shell"],
        vec!["future_gold_lock"],
    )
}

pub(super) fn eval_cursed_key(context: &BossRelicContext) -> RelicJudgement {
    let upside = 24 + context.expensive_cards * 3 + context.profile.x_cost_payoffs * 5;
    let downside =
        14 + (100 - context.curse_tolerance) / 3 + if context.has_omamori { 0 } else { 6 };
    let compatibility = if context.curse_tolerance < 20 && context.need.purge_pressure >= 100 {
        RelicCompatibility::HardReject
    } else if context.curse_tolerance < 35 {
        RelicCompatibility::HighRisk
    } else if context.curse_tolerance >= 60 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        18,
        84,
        "boss_relic_energy_chest_curse",
        vec!["energy_now", "curse_tolerant_shell"],
        vec!["future_chest_curses"],
    )
}

pub(super) fn eval_snecko_eye(context: &BossRelicContext) -> RelicJudgement {
    let draw_bonus = context.snecko_draw_bonus;
    let randomization_benefit = context.snecko_cost_randomization_benefit;
    let randomization_risk = context.snecko_cost_randomization_risk;
    let upside = draw_bonus + randomization_benefit / 2;
    let downside = 8 + randomization_risk / 2;
    let volatility = (30 + (100 - context.volatility_tolerance) / 4).clamp(0, 100);
    let compatibility = if randomization_risk >= 72 && context.expensive_cards <= 3 {
        RelicCompatibility::HardReject
    } else if context.expensive_cards >= 6
        && randomization_benefit >= randomization_risk + 18
        && context.draw_sources <= 3
    {
        RelicCompatibility::StrongFit
    } else if randomization_risk > randomization_benefit + 10 {
        RelicCompatibility::HighRisk
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        volatility,
        92,
        "boss_relic_cost_variance",
        vec!["draw_bonus", "cost_randomization_benefit"],
        vec!["cost_randomization_risk"],
    )
}

pub(super) fn eval_sozu(context: &BossRelicContext) -> RelicJudgement {
    let upside = 22
        + context.energy_sink_value / 3
        + (30 - context.potion_dependence).max(0) / 2
        + if context.empty_potion_slots == 0 {
            6
        } else {
            0
        };
    let downside = 10 + context.potion_lock_pressure / 3;
    let compatibility =
        if context.potion_lock_pressure >= 58 && context.current_potion_quality >= 80 {
            RelicCompatibility::HardReject
        } else if context.potion_lock_pressure >= 54 && context.potion_dependence >= 50 {
            RelicCompatibility::HighRisk
        } else if context.potion_dependence <= 24 && context.empty_potion_slots == 0 {
            RelicCompatibility::StrongFit
        } else {
            RelicCompatibility::Neutral
        };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        10,
        90,
        "boss_relic_energy_no_potions",
        vec!["energy_now", "low_potion_dependence"],
        vec!["potion_lock"],
    )
}

pub(super) fn eval_velvet_choker(context: &BossRelicContext) -> RelicJudgement {
    let upside = 22 + context.two_plus_cost_count * 3 + context.expensive_cards * 2;
    let downside = 16 + context.high_action_dependence / 2;
    let compatibility = if context.high_action_dependence >= 70 {
        RelicCompatibility::HardReject
    } else if context.high_action_dependence >= 50 {
        RelicCompatibility::HighRisk
    } else if context.high_action_dependence <= 25 && context.avg_cost_times_10 >= 14 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        14,
        90,
        "boss_relic_energy_action_cap",
        vec!["frontloaded_energy", "low_action_shell"],
        vec!["action_cap"],
    )
}

pub(super) fn eval_philosophers_stone(context: &BossRelicContext) -> RelicJudgement {
    let fast_kill_signal = context.attack_count * 4 + context.profile.strength_payoffs * 5;
    let upside = 22 + fast_kill_signal / 4 + context.energy_sink_value / 4;
    let downside = 12 + context.enemy_buff_risk / 2;
    let compatibility = if context.need.hp_ratio < 0.55 && context.block_core < 4 {
        RelicCompatibility::HighRisk
    } else if fast_kill_signal >= 40 && context.need.hp_ratio >= 0.65 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        18,
        86,
        "boss_relic_energy_enemy_buff",
        vec!["fast_kill_window", "damage_race"],
        vec!["enemy_strength_buff"],
    )
}

pub(super) fn eval_mark_of_pain(context: &BossRelicContext) -> RelicJudgement {
    let upside = 22 + context.energy_sink_value / 3 + context.wound_absorption / 3;
    let downside = 10 + (100 - context.wound_absorption) / 4 + context.hand_clog_risk / 5;
    let compatibility = if context.exhaust_outlets == 0 && context.draw_sources < 3 {
        RelicCompatibility::HighRisk
    } else if context.exhaust_outlets >= 2 {
        RelicCompatibility::StrongFit
    } else {
        RelicCompatibility::Neutral
    };
    RelicJudgement::new(
        compatibility,
        upside,
        downside,
        16,
        88,
        "boss_relic_energy_wounds",
        vec!["energy_now", "exhaust_outlets"],
        vec!["wound_clog"],
    )
}
