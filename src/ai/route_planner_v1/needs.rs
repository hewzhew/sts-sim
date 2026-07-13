use super::types::{NeedVectorV1, RouteDecisionContextV1, RoutePlannerConfigV1};

const FINAL_SHOP_SEARCH_WINDOW_FLOORS: i32 = 7;

pub(super) fn estimate_needs(
    ctx: &RouteDecisionContextV1,
    config: &RoutePlannerConfigV1,
) -> NeedVectorV1 {
    let hp_ratio = route_hp_ratio(ctx);
    let early_act = ctx.floor <= config.easy_pool_floor_cutoff;
    let starter_load = ctx.deck.starter_strikes + ctx.deck.starter_defends;
    let weak_frontload = ctx.deck.frontload_damage_score < 45;
    let low_block = ctx.deck.block_score < 24;
    let has_empty_potion_slot = ctx.potions.filled < ctx.potions.slots;
    let gold_conversion_pressure = route_gold_conversion_pressure(ctx, config);
    let near_boss_with_unconverted_gold = floors_to_act_boss(ctx.act, ctx.floor)
        <= FINAL_SHOP_SEARCH_WINDOW_FLOORS
        && ctx.gold >= config.early_shop_good_gold * 2;
    let elite_deck_adjustment = elite_readiness_adjustment(ctx, weak_frontload);

    NeedVectorV1 {
        need_card_rewards: need_card_rewards(ctx, early_act, weak_frontload),
        need_relics: need_relics(ctx),
        need_remove: need_remove(ctx, starter_load),
        need_upgrade: need_upgrade(ctx, low_block),
        need_heal: need_heal(hp_ratio, config),
        need_shop: need_shop(
            ctx,
            gold_conversion_pressure,
            near_boss_with_unconverted_gold,
        ),
        need_event: need_event(ctx, early_act),
        need_potion: need_potion(has_empty_potion_slot),
        can_take_elite: clamp01(
            hp_ratio
                + if ctx.potions.has_elite_potion_signal {
                    0.18
                } else {
                    0.0
                }
                + if ctx.relics.has_preserved_insect {
                    0.20
                } else {
                    0.0
                }
                + elite_deck_adjustment,
        ),
        avoid_damage: clamp01(1.0 - hp_ratio + if low_block { 0.10 } else { 0.0 }),
        value_flexibility: clamp01(
            0.30 + if hp_ratio < config.low_hp_ratio {
                0.25
            } else {
                0.0
            },
        ),
    }
}

fn route_hp_ratio(ctx: &RouteDecisionContextV1) -> f32 {
    if ctx.max_hp > 0 {
        ctx.hp as f32 / ctx.max_hp as f32
    } else {
        0.0
    }
}

fn need_card_rewards(ctx: &RouteDecisionContextV1, early_act: bool, weak_frontload: bool) -> f32 {
    clamp01(
        0.35 + if early_act { 0.25 } else { 0.0 }
            + if weak_frontload { 0.25 } else { 0.0 }
            + if ctx.deck.deck_size <= 12 { 0.15 } else { 0.0 },
    )
}

fn need_relics(ctx: &RouteDecisionContextV1) -> f32 {
    clamp01(
        0.35 + if ctx.relics.relic_count <= 2 {
            0.20
        } else {
            0.0
        } + if ctx.potions.has_elite_potion_signal {
            0.15
        } else {
            0.0
        },
    )
}

fn need_remove(ctx: &RouteDecisionContextV1, starter_load: u8) -> f32 {
    clamp01(
        0.20 + f32::from(starter_load).min(9.0) / 18.0 + f32::from(ctx.deck.curses).min(3.0) * 0.18,
    )
}

fn need_upgrade(ctx: &RouteDecisionContextV1, low_block: bool) -> f32 {
    clamp01(
        0.30 + f32::from(ctx.deck.important_cards_unupgraded).min(3.0) * 0.20
            + if low_block { 0.10 } else { 0.0 },
    )
}

fn need_heal(hp_ratio: f32, config: &RoutePlannerConfigV1) -> f32 {
    clamp01(if hp_ratio < config.very_low_hp_ratio {
        1.0
    } else if hp_ratio < config.low_hp_ratio {
        0.75
    } else {
        0.20
    })
}

fn need_shop(
    ctx: &RouteDecisionContextV1,
    gold_conversion_pressure: f32,
    near_boss_with_unconverted_gold: bool,
) -> f32 {
    clamp01(
        0.20 + gold_conversion_pressure * 0.60
            + if near_boss_with_unconverted_gold {
                0.15
            } else {
                0.0
            }
            + if ctx.deck.curses > 0 { 0.20 } else { 0.0 }
            + if ctx.relics.has_smiling_mask || ctx.relics.has_membership_card {
                0.20
            } else {
                0.0
            },
    )
}

fn need_event(ctx: &RouteDecisionContextV1, early_act: bool) -> f32 {
    clamp01(
        0.25 + if !early_act { 0.15 } else { -0.10 } + if ctx.deck.curses > 0 { 0.10 } else { 0.0 },
    )
}

fn need_potion(has_empty_potion_slot: bool) -> f32 {
    if has_empty_potion_slot {
        0.35
    } else {
        0.05
    }
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn route_gold_conversion_pressure(
    ctx: &RouteDecisionContextV1,
    config: &RoutePlannerConfigV1,
) -> f32 {
    if ctx.gold <= config.early_shop_good_gold {
        return 0.0;
    }
    let spendable_window = (ctx.gold - config.early_shop_good_gold) as f32;
    (spendable_window / 450.0).clamp(0.0, 1.0)
}

fn floors_to_act_boss(act: u8, floor: i32) -> i32 {
    let boss_floor = match act {
        1 => 16,
        2 => 32,
        3 => 48,
        _ => floor,
    };
    boss_floor.saturating_sub(floor)
}

fn elite_readiness_adjustment(ctx: &RouteDecisionContextV1, weak_frontload: bool) -> f32 {
    if ctx.act != 1 {
        return if weak_frontload { -0.20 } else { 0.0 };
    }

    let transition_attacks = ctx.deck.attacks.saturating_sub(ctx.deck.starter_strikes);
    let attack_count_score = f32::from(transition_attacks).min(4.0) / 4.0;
    let damage_score = ((ctx.deck.frontload_damage_score as f32 - 45.0) / 35.0).clamp(0.0, 1.0);
    let sentries_coverage = if ctx.deck.aoe_score > 0 { 0.10 } else { 0.0 };
    let debuff_control = (ctx.deck.debuff_score as f32).min(3.0) / 3.0;
    let scaling_setup = (ctx.deck.scaling_score as f32).min(2.0) / 2.0;

    let sentries_debt = if ctx.deck.aoe_score == 0 && damage_score < 0.50 {
        -0.12
    } else {
        0.0
    };
    let nob_skill_debt = if ctx.deck.skills > ctx.deck.attacks.saturating_add(1) {
        -0.08
    } else {
        0.0
    };
    let starter_only_debt = if transition_attacks <= 1 && damage_score < 0.25 {
        -0.35
    } else {
        0.0
    };

    -0.18
        + attack_count_score * 0.18
        + damage_score * 0.20
        + sentries_coverage
        + debuff_control * 0.08
        + scaling_setup * 0.06
        + sentries_debt
        + nob_skill_debt
        + starter_only_debt
}
