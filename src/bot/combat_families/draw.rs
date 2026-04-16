#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DrawTimingContext {
    pub current_energy: i32,
    pub player_no_draw: bool,
    pub current_hand_size: i32,
    pub future_zero_cost_cards: i32,
    pub future_one_cost_cards: i32,
    pub future_two_plus_cost_cards: i32,
    pub future_key_delay_weight: i32,
    pub future_high_cost_key_delay_weight: i32,
    pub future_status_cards: i32,
    pub other_draw_sources_in_hand: i32,
}

const DRAW_BLOCKED_BY_NO_DRAW_PENALTY: i32 = 14_000;
const DRAW_BASE_SCORE: i32 = 2_800;
const DRAW_CARD_SCORE: i32 = 1_200;
const DRAW_HAND_OVERFLOW_PENALTY: i32 = 1_100;
const DRAW_NO_DRAW_SELF_PENALTY: i32 = 1_200;

pub(crate) fn draw_continuity_score(
    remaining_cards_after: i32,
    immediate_draws: i32,
    future_draws: i32,
    shuffle_recovery_cards: i32,
) -> i32 {
    let remaining_cards_after = remaining_cards_after.max(0);
    let accessible_cycle_cards =
        remaining_cards_after + immediate_draws.max(0) + shuffle_recovery_cards.max(0);
    let mut score = immediate_draws.max(0) * 900 + future_draws.max(0) * 240;

    score += match accessible_cycle_cards {
        i32::MIN..=3 => -10_000,
        4 => -6_500,
        5 => -3_500,
        6 => -1_400,
        7 => -400,
        8..=10 => 300,
        _ => 0,
    };

    if remaining_cards_after >= 12 {
        score += 500;
    }

    score
}

pub(crate) fn battle_trance_timing_score(ctx: &DrawTimingContext, draw_count: i32) -> i32 {
    draw_action_timing_score(ctx, true, draw_count)
}

pub(crate) fn draw_action_timing_score(
    ctx: &DrawTimingContext,
    applies_no_draw: bool,
    draw_count: i32,
) -> i32 {
    if ctx.player_no_draw {
        return -DRAW_BLOCKED_BY_NO_DRAW_PENALTY;
    }

    let mut score = DRAW_BASE_SCORE + draw_count.max(0) * DRAW_CARD_SCORE;
    let hand_after_draw = ctx.current_hand_size + draw_count;
    score -= (hand_after_draw - 9).max(0) * DRAW_HAND_OVERFLOW_PENALTY;
    if applies_no_draw {
        score -= DRAW_NO_DRAW_SELF_PENALTY;
    }

    match ctx.current_energy {
        i32::MIN..=0 => {
            score -= 4_800;
            score += ctx.future_zero_cost_cards * 700;
            score += ctx.future_one_cost_cards * 120;
            score -= ctx.future_two_plus_cost_cards * 900;
            score -= ctx.future_key_delay_weight * 260;
            score -= ctx.future_high_cost_key_delay_weight * 420;
            score -= ctx.future_status_cards * 850;
            score -= ctx.other_draw_sources_in_hand * 1_200;
            if applies_no_draw {
                score -= 1_400 + ctx.other_draw_sources_in_hand * 600;
            }
            if ctx.future_zero_cost_cards == 0 {
                score -= 2_000;
            }
        }
        1 => {
            score += ctx.future_zero_cost_cards * 600;
            score += ctx.future_one_cost_cards * 450;
            score -= ctx.future_two_plus_cost_cards * 450;
            score -= ctx.future_key_delay_weight * 140;
            score -= ctx.future_high_cost_key_delay_weight * 220;
            score -= ctx.future_status_cards * 600;
            score -= ctx.other_draw_sources_in_hand * 900;
            if applies_no_draw {
                score -= 1_000 + ctx.other_draw_sources_in_hand * 450;
            }
        }
        _ => {
            score += ctx.future_zero_cost_cards * 280;
            score += ctx.future_one_cost_cards * 420;
            score += ctx.future_two_plus_cost_cards * 260;
            score -= ctx.future_key_delay_weight * 40;
            score -= ctx.future_high_cost_key_delay_weight * 60;
            score -= ctx.future_status_cards * 350;
            score -= ctx.other_draw_sources_in_hand * 650;
            if applies_no_draw {
                score -= 500 + ctx.other_draw_sources_in_hand * 250;
            }
        }
    }

    score
}

pub(crate) fn deck_cycle_thinning_score(
    card_pool_size_before: i32,
    remaining_cards_after: i32,
    immediate_draws: i32,
    future_draws: i32,
    shuffle_recovery_cards: i32,
    extra_loop_value: i32,
) -> i32 {
    let removed_cards = (card_pool_size_before - remaining_cards_after).max(0);
    let mut score = removed_cards * 260;

    if card_pool_size_before <= 8 {
        score -= removed_cards * 700;
    } else if card_pool_size_before <= 10 {
        score -= removed_cards * 300;
    }

    score
        + draw_continuity_score(
            remaining_cards_after,
            immediate_draws,
            future_draws,
            shuffle_recovery_cards,
        )
        + extra_loop_value
}

pub(crate) fn status_loop_cycle_score(
    draw_per_status: i32,
    status_in_draw: i32,
    status_in_discard: i32,
    shuffle_discard_into_draw: bool,
    extra_cycle_draws: i32,
    sentry_count: i32,
) -> i32 {
    let draw_per_status = draw_per_status.max(0);
    let draw_status_value = status_in_draw.max(0) * draw_per_status * 850;
    let discard_status_value = if shuffle_discard_into_draw {
        status_in_discard.max(0) * draw_per_status * 1_050
    } else {
        status_in_discard.max(0) * draw_per_status * 240
    };

    draw_status_value
        + discard_status_value
        + extra_cycle_draws.max(0) * 240
        + sentry_count.max(0) * draw_per_status * 1_800
}
