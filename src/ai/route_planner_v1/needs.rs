use super::types::{NeedVectorV1, RouteDecisionContextV1, RoutePlannerConfigV1};

pub(super) fn estimate_needs(
    ctx: &RouteDecisionContextV1,
    config: &RoutePlannerConfigV1,
) -> NeedVectorV1 {
    let hp_ratio = if ctx.max_hp > 0 {
        ctx.hp as f32 / ctx.max_hp as f32
    } else {
        0.0
    };
    let early_act = ctx.floor <= config.easy_pool_floor_cutoff;
    let starter_load = ctx.deck.starter_strikes + ctx.deck.starter_defends;
    let weak_frontload = ctx.deck.frontload_damage_score < 45;
    let low_block = ctx.deck.block_score < 24;
    let has_empty_potion_slot = ctx.potions.filled < ctx.potions.slots;
    let high_gold = ctx.gold >= config.early_shop_good_gold;

    NeedVectorV1 {
        need_card_rewards: clamp01(
            0.35 + if early_act { 0.25 } else { 0.0 }
                + if weak_frontload { 0.25 } else { 0.0 }
                + if ctx.deck.deck_size <= 12 { 0.15 } else { 0.0 },
        ),
        need_relics: clamp01(
            0.35 + if ctx.relics.relic_count <= 2 {
                0.20
            } else {
                0.0
            } + if ctx.potions.has_elite_potion_signal {
                0.15
            } else {
                0.0
            },
        ),
        need_remove: clamp01(
            0.20 + f32::from(starter_load).min(9.0) / 18.0
                + f32::from(ctx.deck.curses).min(3.0) * 0.18,
        ),
        need_upgrade: clamp01(
            0.30 + f32::from(ctx.deck.important_cards_unupgraded).min(3.0) * 0.20
                + if low_block { 0.10 } else { 0.0 },
        ),
        need_heal: clamp01(if hp_ratio < config.very_low_hp_ratio {
            1.0
        } else if hp_ratio < config.low_hp_ratio {
            0.75
        } else {
            0.20
        }),
        need_shop: clamp01(
            if high_gold { 0.55 } else { 0.20 }
                + if ctx.deck.curses > 0 { 0.20 } else { 0.0 }
                + if ctx.relics.has_smiling_mask || ctx.relics.has_membership_card {
                    0.20
                } else {
                    0.0
                },
        ),
        need_event: clamp01(
            0.25 + if !early_act { 0.15 } else { -0.10 }
                + if ctx.deck.curses > 0 { 0.10 } else { 0.0 },
        ),
        need_potion: if has_empty_potion_slot { 0.35 } else { 0.05 },
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
                + if weak_frontload { -0.20 } else { 0.0 },
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

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}
