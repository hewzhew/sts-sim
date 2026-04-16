use crate::bot::card_taxonomy::taxonomy;
use crate::bot::combat_families::draw::{
    battle_trance_timing_score, draw_action_timing_score, draw_continuity_score,
    status_loop_cycle_score, DrawTimingContext,
};
use crate::content::cards::CardId;

use super::sim::{active_hand_cards, SimState};

pub(super) fn battle_trance_timing_value(state: &SimState, current_idx: usize) -> i32 {
    battle_trance_timing_score(
        &build_draw_timing_context(state, current_idx),
        state.hand[current_idx].base_magic.max(0),
    )
}

pub(super) fn generic_draw_timing_value(
    state: &SimState,
    current_idx: usize,
    draw_count: i32,
    applies_no_draw: bool,
) -> i32 {
    draw_action_timing_score(
        &build_draw_timing_context(state, current_idx),
        applies_no_draw,
        draw_count,
    )
}

pub(super) fn deep_breath_timing_value(state: &SimState) -> i32 {
    draw_continuity_score(state.card_pool_size, 1, 0, state.discard_pile_size)
        + status_loop_cycle_score(
            i32::from(state.has_evolve),
            state.status_in_draw,
            state.status_in_discard,
            true,
            1,
            state.sentry_count,
        )
}

fn build_draw_timing_context(state: &SimState, current_idx: usize) -> DrawTimingContext {
    let other_draw_sources_in_hand = active_hand_cards(state)
        .filter(|(idx, c)| *idx != current_idx && is_draw_source(c.card_id))
        .count() as i32;

    DrawTimingContext {
        current_energy: state.energy,
        player_no_draw: state.player_no_draw,
        current_hand_size: active_hand_cards(state).count() as i32,
        future_zero_cost_cards: state.future_zero_cost_cards,
        future_one_cost_cards: state.future_one_cost_cards,
        future_two_plus_cost_cards: state.future_two_plus_cost_cards,
        future_key_delay_weight: state.future_key_delay_weight,
        future_high_cost_key_delay_weight: state.future_high_cost_key_delay_weight,
        future_status_cards: state.status_in_draw + state.status_in_discard,
        other_draw_sources_in_hand,
    }
}

fn is_draw_source(card_id: CardId) -> bool {
    let tags = taxonomy(card_id);
    tags.is_draw_core() && !matches!(card_id, CardId::DarkEmbrace | CardId::Evolve)
}
