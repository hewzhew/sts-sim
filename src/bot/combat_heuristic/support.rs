use crate::bot::card_taxonomy::taxonomy;
use crate::content::cards::CardId;

use super::posture::HeuristicPostureFeatures;
use super::sim::{active_hand_cards, SimCard, SimState};

pub(super) fn support_timing_value(state: &SimState, card_idx: usize) -> Option<i32> {
    let card = &state.hand[card_idx];
    let value = match card.card_id {
        CardId::Panacea => panacea_timing_value(state),
        CardId::PowerThrough => power_through_timing_value(state, card),
        _ => return None,
    };
    Some(value)
}

pub(super) fn support_posture_adjustment(
    card_id: CardId,
    posture: &HeuristicPostureFeatures,
) -> i32 {
    match card_id {
        CardId::PowerThrough => power_through_posture_adjustment(posture),
        _ => 0,
    }
}

fn total_incoming_damage(state: &SimState) -> i32 {
    state
        .monsters
        .iter()
        .filter(|m| !m.is_gone && m.is_attacking)
        .map(|m| m.intent_dmg * m.intent_hits.max(1))
        .sum()
}

fn power_through_needed_block(state: &SimState, card: &SimCard) -> i32 {
    let mut block_gain = card.base_block + state.player_dexterity;
    if state.player_frail {
        block_gain = (block_gain as f32 * 0.75).floor() as i32;
    }
    let incoming = total_incoming_damage(state);
    let covered_now = state.player_block.min(incoming);
    let covered_after = (state.player_block + block_gain.max(0)).min(incoming);
    covered_after - covered_now
}

fn power_through_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let needed_block = power_through_needed_block(state, card);
    let wound_penalty = power_through_wound_penalty(state);
    let mut value = needed_block * 180 - wound_penalty;
    if total_incoming_damage(state) == 0 {
        value -= 12_000;
    }
    if needed_block <= 0 {
        value -= 5_500;
    }
    value
}

fn power_through_wound_penalty(state: &SimState) -> i32 {
    let has_exhaust_out =
        active_hand_cards(state).any(|(_, c)| taxonomy(c.card_id).is_exhaust_outlet());
    let has_status_payoff = state.has_dark_embrace || state.has_feel_no_pain || state.has_evolve;
    let mut penalty = 6_000 + state.future_status_cards.max(0) * 350;
    if has_exhaust_out {
        penalty -= 2_200;
    }
    if has_status_payoff {
        penalty -= 1_600;
    }
    penalty.max(0)
}

fn power_through_posture_adjustment(posture: &HeuristicPostureFeatures) -> i32 {
    posture.immediate_survival_pressure.min(30) * 230 - posture.resource_preservation_pressure * 700
}

fn panacea_timing_value(state: &SimState) -> i32 {
    let mut value = 5_500;
    let self_combo_cards = active_hand_cards(state)
        .filter(|(_, c)| taxonomy(c.card_id).is_panacea_self_combo())
        .count() as i32;
    let enemy_might_debuff = state
        .monsters
        .iter()
        .any(|m| !m.is_gone && !m.is_attacking && m.intent_dmg <= 0);

    value += self_combo_cards * 2_000;

    if state.player_artifact == 0 {
        value += 1_200;
        if enemy_might_debuff {
            value += 900;
        }
    } else {
        value -= 3_500;
    }

    value
}
