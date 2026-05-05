use crate::bot::combat_families::survival::{survival_swing_score, SurvivalTimingContext};

const APPARITION_EXISTING_INTANGIBLE_PENALTY: i32 = 6_000;
const APPARITION_INTANGIBLE_STACK_PENALTY: i32 = 2_000;
const APPARITION_UNUPGRADED_OVERLAP_RELIEF: i32 = 2_000;
const APPARITION_PYRAMID_OVERLAP_PENALTY: i32 = 1_000;
const APPARITION_LOW_HP_OVERLAP_RELIEF: i32 = 1_500;
const APPARITION_HAND_FLOOD_RELIEF: i32 = 1_500;
const APPARITION_RESERVE_FLOOD_RELIEF: i32 = 800;
const APPARITION_PRESSURE_POINT_SCORE: i32 = 180;
const APPARITION_INCOMING_OVERLAP_RELIEF: i32 = 1_500;
const APPARITION_LETHAL_OVERLAP_RELIEF: i32 = 4_000;
const APPARITION_MASSIVE_OVERLAP_RELIEF: i32 = 3_000;
const APPARITION_UNUPGRADED_BASE_SCORE: i32 = 5_500;
const APPARITION_UNUPGRADED_THREAT_BONUS: i32 = 5_000;
const APPARITION_UNUPGRADED_LETHAL_BONUS: i32 = 8_000;
const APPARITION_UNUPGRADED_EXTRA_COPY_BONUS: i32 = 1_500;
const APPARITION_UNUPGRADED_FRONTLOAD_BONUS: i32 = 2_500;
const APPARITION_UNUPGRADED_FRONTLOAD_PRESSURE_SCORE: i32 = 120;
const APPARITION_UPGRADED_ACTIVE_BASE_SCORE: i32 = 8_500;
const APPARITION_UPGRADED_SAFE_PYRAMID_PENALTY: i32 = 3_000;
const APPARITION_UPGRADED_SAFE_DELAY_PENALTY: i32 = 1_500;
const APPARITION_UPGRADED_LETHAL_BONUS: i32 = 10_000;
const APPARITION_UPGRADED_MASSIVE_BONUS: i32 = 8_000;
const APPARITION_UPGRADED_PYRAMID_THREAT_BONUS: i32 = 2_500;
const APPARITION_UPGRADED_FRONTLOAD_BONUS: i32 = 2_000;
const APPARITION_UPGRADED_FRONTLOAD_PRESSURE_SCORE: i32 = 110;
const APPARITION_HAND_SHAPING_TOPDECK_BONUS: i32 = 2_500;
const APPARITION_HAND_SHAPING_EXTRA_COPY_SCORE: i32 = 500;
const APPARITION_HAND_SHAPING_TIMING_DIVISOR: i32 = 4;
const APPARITION_UPGRADED_HAND_SHAPING_TIMING_DIVISOR: i32 = 5;
const APPARITION_HAND_SHAPING_MIN_KEEP_PENALTY: i32 = 2_000;
const APPARITION_UPGRADED_HAND_SHAPING_MIN_KEEP_PENALTY: i32 = 1_500;
const APPARITION_UPGRADED_PYRAMID_HOLD_PENALTY: i32 = 2_400;
const APPARITION_UPGRADED_GENERIC_HOLD_PENALTY: i32 = 800;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ApparitionTimingContext {
    pub current_hp: i32,
    pub current_intangible: i32,
    pub imminent_unblocked_damage: i32,
    pub total_incoming_damage: i32,
    pub apparitions_in_hand: i32,
    pub remaining_apparitions_total: i32,
    pub upgraded: bool,
    pub has_runic_pyramid: bool,
    pub encounter_pressure: i32,
}

pub(crate) fn apparition_timing_score(ctx: &ApparitionTimingContext) -> i32 {
    let prevented_damage = if ctx.imminent_unblocked_damage > 0 {
        ctx.imminent_unblocked_damage
    } else {
        ctx.total_incoming_damage
    };
    let swing = survival_swing_score(
        &SurvivalTimingContext {
            current_hp: ctx.current_hp,
            imminent_unblocked_damage: ctx.imminent_unblocked_damage,
            missing_hp: 0,
        },
        0,
        prevented_damage,
        0,
    );
    let lethal_window = ctx.imminent_unblocked_damage >= ctx.current_hp;
    let massive_window = ctx.imminent_unblocked_damage >= ctx.current_hp + 10
        || ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_mul(2);
    let hand_pressure = ctx.apparitions_in_hand.saturating_sub(1);
    let reserve_pressure = ctx.remaining_apparitions_total.saturating_sub(1);

    if ctx.current_intangible > 0 {
        let mut value = -APPARITION_EXISTING_INTANGIBLE_PENALTY
            - ctx.current_intangible * APPARITION_INTANGIBLE_STACK_PENALTY;
        if !ctx.upgraded {
            value += APPARITION_UNUPGRADED_OVERLAP_RELIEF;
        } else if ctx.has_runic_pyramid {
            value -= APPARITION_PYRAMID_OVERLAP_PENALTY;
        }
        if ctx.current_hp <= 25 {
            value += APPARITION_LOW_HP_OVERLAP_RELIEF;
        }
        if hand_pressure >= 2 {
            value += hand_pressure * APPARITION_HAND_FLOOD_RELIEF;
        }
        if reserve_pressure >= 2 {
            value += reserve_pressure.min(4) * APPARITION_RESERVE_FLOOD_RELIEF;
        }
        value += ctx.encounter_pressure.max(0) * APPARITION_PRESSURE_POINT_SCORE;
        if ctx.total_incoming_damage >= 12 {
            value += APPARITION_INCOMING_OVERLAP_RELIEF;
        }
        if lethal_window {
            value += APPARITION_LETHAL_OVERLAP_RELIEF;
        }
        if massive_window {
            value += APPARITION_MASSIVE_OVERLAP_RELIEF;
        }
        return value;
    }

    if !ctx.upgraded {
        let mut value = APPARITION_UNUPGRADED_BASE_SCORE + swing;
        if ctx.imminent_unblocked_damage > 0 || ctx.current_hp <= 35 {
            value += APPARITION_UNUPGRADED_THREAT_BONUS;
        }
        if lethal_window {
            value += APPARITION_UNUPGRADED_LETHAL_BONUS;
        }
        if ctx.apparitions_in_hand >= 2 {
            value += APPARITION_UNUPGRADED_EXTRA_COPY_BONUS;
        }
        if ctx.imminent_unblocked_damage == 0
            && ctx.total_incoming_damage == 0
            && ctx.current_hp <= 35
            && reserve_pressure >= 2
            && ctx.encounter_pressure >= 10
        {
            value += APPARITION_UNUPGRADED_FRONTLOAD_BONUS
                + ctx.encounter_pressure * APPARITION_UNUPGRADED_FRONTLOAD_PRESSURE_SCORE;
        }
        value
    } else {
        let mut value = if ctx.imminent_unblocked_damage > 0 || ctx.current_hp <= 22 {
            APPARITION_UPGRADED_ACTIVE_BASE_SCORE + swing
        } else if ctx.has_runic_pyramid {
            -APPARITION_UPGRADED_SAFE_PYRAMID_PENALTY
        } else {
            -APPARITION_UPGRADED_SAFE_DELAY_PENALTY
        };
        if lethal_window {
            value += APPARITION_UPGRADED_LETHAL_BONUS;
        }
        if massive_window {
            value += APPARITION_UPGRADED_MASSIVE_BONUS;
        }
        if ctx.has_runic_pyramid && (lethal_window || massive_window) {
            value += APPARITION_UPGRADED_PYRAMID_THREAT_BONUS;
        }
        if ctx.imminent_unblocked_damage == 0
            && ctx.total_incoming_damage == 0
            && ctx.current_hp <= 28
            && reserve_pressure >= 2
            && ctx.encounter_pressure >= 12
        {
            value += APPARITION_UPGRADED_FRONTLOAD_BONUS
                + ctx.encounter_pressure * APPARITION_UPGRADED_FRONTLOAD_PRESSURE_SCORE;
        }
        value
    }
}

pub(crate) fn apparition_hand_shaping_score(ctx: &ApparitionTimingContext) -> i32 {
    let timing = apparition_timing_score(ctx);

    if !ctx.upgraded {
        if timing >= 8_000 {
            -(timing / APPARITION_HAND_SHAPING_TIMING_DIVISOR)
                .max(APPARITION_HAND_SHAPING_MIN_KEEP_PENALTY)
        } else {
            APPARITION_HAND_SHAPING_TOPDECK_BONUS
                + ctx.apparitions_in_hand.saturating_sub(1)
                    * APPARITION_HAND_SHAPING_EXTRA_COPY_SCORE
        }
    } else if timing > 0 {
        -(timing / APPARITION_UPGRADED_HAND_SHAPING_TIMING_DIVISOR)
            .max(APPARITION_UPGRADED_HAND_SHAPING_MIN_KEEP_PENALTY)
    } else if ctx.has_runic_pyramid {
        -APPARITION_UPGRADED_PYRAMID_HOLD_PENALTY
    } else {
        -APPARITION_UPGRADED_GENERIC_HOLD_PENALTY
    }
}
