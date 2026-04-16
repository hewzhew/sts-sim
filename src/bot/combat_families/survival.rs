use crate::content::cards::{CardId, CardType};

const SURVIVAL_HEAL_POINT_SCORE: i32 = 1_200;
const SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE: i32 = 220;
const SURVIVAL_KILL_SCORE: i32 = 1_600;
const SURVIVAL_LOW_HP_HEAL_BONUS: i32 = 4_000;
const SURVIVAL_LETHALISH_COVER_BONUS: i32 = 5_000;
const SURVIVAL_STABILIZE_BONUS: i32 = 2_500;
const SURVIVAL_EXACT_STABILIZE_BONUS: i32 = 6_500;
const SURVIVAL_LETHAL_WINDOW_BONUS: i32 = 10_000;
const SURVIVAL_MASSIVE_WINDOW_BONUS: i32 = 6_000;
const SURVIVAL_PARTIAL_LETHAL_SAVE_BONUS: i32 = 7_500;
const SURVIVAL_KILL_IN_LETHAL_WINDOW_BONUS: i32 = 2_500;

const PLAY_NOW_KEEP_PENALTY: i32 = -1_500;
const DELAY_TO_TOPDECK_BONUS: i32 = 1_500;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SurvivalTimingContext {
    pub current_hp: i32,
    pub imminent_unblocked_damage: i32,
    pub missing_hp: i32,
}

pub(crate) fn survival_swing_score(
    ctx: &SurvivalTimingContext,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    let effective_heal = hp_gain.min(ctx.missing_hp.max(0));
    let lethalish = ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_sub(4);
    let covered = effective_heal + prevented_damage;
    let exact_stabilize =
        ctx.imminent_unblocked_damage > 0 && covered >= ctx.imminent_unblocked_damage;
    let lethal_window = ctx.imminent_unblocked_damage >= ctx.current_hp;
    let massive_window = ctx.imminent_unblocked_damage >= ctx.current_hp + 10
        || ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_mul(2);
    let remaining_gap = (ctx.imminent_unblocked_damage - covered).max(0);

    let mut value = effective_heal * SURVIVAL_HEAL_POINT_SCORE;
    value += prevented_damage * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE;
    value += kills * SURVIVAL_KILL_SCORE;

    if ctx.current_hp <= 30 && effective_heal > 0 {
        value += SURVIVAL_LOW_HP_HEAL_BONUS;
    }
    if lethalish && covered > 0 {
        value += SURVIVAL_LETHALISH_COVER_BONUS + covered * 180;
    } else if covered >= ctx.imminent_unblocked_damage.max(0) && covered > 0 {
        value += SURVIVAL_STABILIZE_BONUS;
    }
    if exact_stabilize {
        value += SURVIVAL_EXACT_STABILIZE_BONUS
            + ctx.imminent_unblocked_damage.max(0) * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE;
        if lethal_window {
            value += SURVIVAL_LETHAL_WINDOW_BONUS;
        }
        if massive_window {
            value += SURVIVAL_MASSIVE_WINDOW_BONUS;
        }
    } else if lethal_window && covered > 0 {
        value += SURVIVAL_PARTIAL_LETHAL_SAVE_BONUS
            + covered * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE
            - remaining_gap * 260;
    }
    if lethal_window && kills > 0 {
        value += SURVIVAL_KILL_IN_LETHAL_WINDOW_BONUS;
    }

    value
}

pub(crate) fn reaper_timing_score(
    ctx: &SurvivalTimingContext,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    survival_swing_score(ctx, hp_gain, prevented_damage, kills)
}

pub(crate) fn hand_shaping_play_now_score(can_play_now: bool) -> i32 {
    if can_play_now {
        PLAY_NOW_KEEP_PENALTY
    } else {
        DELAY_TO_TOPDECK_BONUS
    }
}

pub(crate) fn reaper_hand_shaping_score(ctx: &SurvivalTimingContext) -> i32 {
    let assumed_heal = ctx.missing_hp.min(12).max(0);
    let assumed_prevented = ctx.imminent_unblocked_damage.min(assumed_heal).max(0);
    -(reaper_timing_score(ctx, assumed_heal, assumed_prevented, 0) / 3)
}

pub(crate) fn hand_shaping_next_draw_window_score(
    draws_next_turn: i32,
    guaranteed_topdeck: bool,
) -> i32 {
    if !guaranteed_topdeck || draws_next_turn <= 0 {
        0
    } else {
        -600 - draws_next_turn.min(5) * 120
    }
}

pub(crate) fn hand_shaping_delay_quality_score(
    card_id: CardId,
    card_type: CardType,
    cost: i32,
    current_energy: i32,
    safe_block_turn: bool,
) -> i32 {
    let mut score = match card_type {
        CardType::Curse | CardType::Status => -20_000,
        _ => 0,
    };

    if card_type == CardType::Skill && safe_block_turn {
        score += 900;
    }
    if matches!(card_id, CardId::Defend | CardId::DefendG) && safe_block_turn {
        score += 1_200;
    }
    if matches!(card_id, CardId::Warcry | CardId::ThinkingAhead) {
        score += 800;
    }
    if card_type == CardType::Attack && cost > current_energy {
        score += 1_000;
    }
    if cost == 0 {
        score -= 700;
    }

    score
}

pub(crate) fn body_slam_delay_score(
    current_damage: i32,
    additional_block_before_slam: i32,
    can_kill_now: bool,
    imminent_unblocked_damage: i32,
) -> i32 {
    if can_kill_now {
        return 4_500 + current_damage.max(0) * 80;
    }
    if additional_block_before_slam <= 0 {
        return 0;
    }

    let mut score = -(2_500 + additional_block_before_slam * 280);
    if current_damage <= 0 {
        score -= 2_200;
    }
    if imminent_unblocked_damage > 0 {
        score -= additional_block_before_slam.min(imminent_unblocked_damage.max(0)) * 120;
    }
    score
}

pub(crate) fn exhaust_finish_window_score(
    exact_lethal: bool,
    kills: i32,
    prevented_damage: i32,
    remaining_alive_after: i32,
) -> i32 {
    let mut score = prevented_damage.max(0) * 180 + kills.max(0) * 1_600;

    if exact_lethal {
        score += 8_000;
    } else if remaining_alive_after <= 1 && kills > 0 {
        score += 3_500;
    }

    score
}

pub(crate) fn flight_break_progress_score(hits: i32, flight: i32, prevented_damage: i32) -> f32 {
    if hits <= 0 || flight <= 0 {
        return 0.0;
    }

    if hits >= flight {
        4_500.0 + prevented_damage as f32 * 220.0
    } else {
        900.0 * hits as f32
    }
}

pub(crate) fn persistent_block_progress_score(block_damage: i32) -> f32 {
    block_damage.max(0) as f32 * 90.0
}
