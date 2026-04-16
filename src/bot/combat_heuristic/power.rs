use crate::bot::card_taxonomy::taxonomy;
use crate::bot::combat_families::draw::status_loop_cycle_score;
use crate::bot::combat_families::exhaust::exhaust_engine_setup_score;
use crate::content::cards::{CardId, CardType};

use super::posture::HeuristicPostureFeatures;
use super::sim::{active_hand_cards, SimState};

pub(super) fn power_timing_value(state: &SimState, card_idx: usize) -> Option<i32> {
    let card = &state.hand[card_idx];
    let value = match card.card_id {
        CardId::Corruption => exhaust_engine_setup_score(
            state.has_corruption,
            immediate_exhaust_count(state),
            future_exhaust_source_count(state, card_idx),
            if state.has_feel_no_pain { 4 } else { 0 },
            if state.has_dark_embrace { 1 } else { 0 },
            status_cards_in_hand(state),
            future_status_card_count(state),
            state.sentry_count,
            active_hand_cards(state)
                .filter(|(idx, c)| *idx != card_idx && c.card_type == CardType::Skill && c.cost > 0)
                .count() as i32
                * 1_000,
        ),
        CardId::FeelNoPain => {
            exhaust_engine_setup_score(
                state.has_feel_no_pain,
                immediate_exhaust_count(state),
                future_exhaust_source_count(state, card_idx),
                4,
                0,
                status_cards_in_hand(state),
                future_status_card_count(state),
                state.sentry_count,
                same_turn_exhaust_setup_bonus(state, card_idx, false),
            ) - exhaust_setup_brake(state, card_idx, false)
        }
        CardId::DarkEmbrace => {
            exhaust_engine_setup_score(
                state.has_dark_embrace,
                immediate_exhaust_count(state),
                future_exhaust_source_count(state, card_idx),
                0,
                1,
                status_cards_in_hand(state),
                future_status_card_count(state),
                state.sentry_count,
                same_turn_exhaust_setup_bonus(state, card_idx, true),
            ) - exhaust_setup_brake(state, card_idx, true)
        }
        CardId::Rupture => {
            (if state.has_rupture { -7_500 } else { 8_500 })
                + self_damage_cards_in_hand(state) * 1_600
        }
        CardId::Combust => {
            (if state.has_combust { -7_500 } else { 8_500 }) + alive_monster_count(state) * 500
        }
        CardId::Metallicize => {
            (if state.has_metallicize { -8_000 } else { 8_500 })
                + alive_monster_count(state) * 500
                + total_incoming_damage(state).min(18) * 120
        }
        CardId::Evolve => {
            let mut value = if state.has_evolve { -8_000 } else { 8_000 };
            value += status_loop_cycle_score(
                1,
                state.status_in_draw,
                state.status_in_discard,
                true,
                0,
                state.sentry_count,
            );
            if active_hand_cards(state).any(|(_, c)| c.card_id == CardId::DeepBreath) {
                value += 1_500;
            }
            value
        }
        CardId::FireBreathing => fire_breathing_play_value(state),
        CardId::Brutality => brutality_timing_value(state),
        CardId::Berserk => {
            let mut value = if state.has_berserk { -8_000 } else { 6_000 };
            if state.player_artifact > 0 {
                value += 3_000;
            }
            if state.energy <= 1 {
                value += 2_000;
            }
            value
        }
        CardId::Inflame => 9_500,
        CardId::Panache => panache_timing_value(state),
        CardId::Mayhem => {
            if state.has_mayhem {
                -8_000
            } else {
                8_000
            }
        }
        CardId::Magnetism => {
            if state.has_magnetism {
                -8_000
            } else {
                7_000
            }
        }
        _ => return None,
    };

    Some(value)
}

pub(super) fn power_posture_adjustment(card_id: CardId, posture: &HeuristicPostureFeatures) -> i32 {
    match card_id {
        CardId::DarkEmbrace => dark_embrace_posture_adjustment(posture),
        CardId::FireBreathing => fire_breathing_posture_adjustment(posture),
        _ => 0,
    }
}

fn immediate_exhaust_count(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| {
            taxonomy(c.card_id).is_exhaust_outlet()
                || matches!(
                    c.card_id,
                    CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                )
        })
        .count() as i32
}

fn future_exhaust_source_count(state: &SimState, current_card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_card_idx
                && (taxonomy(c.card_id).is_exhaust_outlet()
                    || matches!(
                        c.card_id,
                        CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                    ))
        })
        .count() as i32
}

fn same_turn_exhaust_setup_bonus(
    state: &SimState,
    current_card_idx: usize,
    draw_engine: bool,
) -> i32 {
    let immediate_triggers = active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_card_idx
                && (taxonomy(c.card_id).is_exhaust_outlet()
                    || matches!(
                        c.card_id,
                        CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                    ))
        })
        .count() as i32;
    if immediate_triggers <= 0 {
        return 0;
    }

    let junk_fuel = active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_card_idx
                && matches!(
                    c.card_id,
                    CardId::Burn | CardId::Dazed | CardId::Slimed | CardId::Wound | CardId::Injury
                )
        })
        .count() as i32;

    let mut bonus = 3_000 + immediate_triggers * if draw_engine { 1_900 } else { 1_300 };
    bonus += junk_fuel * if draw_engine { 2_200 } else { 1_000 };
    if draw_engine && junk_fuel > 0 {
        bonus += 1_600;
    }
    bonus
}

fn exhaust_setup_brake(state: &SimState, current_card_idx: usize, draw_engine: bool) -> i32 {
    let immediate_triggers = active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_card_idx
                && (taxonomy(c.card_id).is_exhaust_outlet()
                    || matches!(
                        c.card_id,
                        CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                    ))
        })
        .count() as i32;
    let incoming = total_incoming_damage(state);
    let imminent = (incoming - state.player_block).max(0);
    let alive = alive_monster_count(state);

    let mut penalty = 0;
    if immediate_triggers <= 0 {
        penalty += 3_200;
        if !state.is_boss_fight && !state.is_elite_fight && alive <= 1 {
            penalty += 2_600;
        }
    }
    if imminent > 0 {
        penalty += imminent.min(18) * 180;
    }
    if imminent >= state.player_hp.saturating_sub(6) {
        penalty += 4_200;
    }
    if !draw_engine && state.energy <= 1 {
        penalty += 1_200;
    }
    penalty
}

fn status_cards_in_hand(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| matches!(c.card_type, CardType::Status | CardType::Curse))
        .count() as i32
}

fn future_status_card_count(state: &SimState) -> i32 {
    state.status_in_draw + state.status_in_discard + status_cards_in_hand(state)
}

fn alive_monster_count(state: &SimState) -> i32 {
    state.monsters.iter().filter(|m| !m.is_gone).count() as i32
}

fn total_incoming_damage(state: &SimState) -> i32 {
    state
        .monsters
        .iter()
        .filter(|m| !m.is_gone && m.is_attacking)
        .map(|m| m.intent_dmg * m.intent_hits.max(1))
        .sum()
}

fn self_damage_cards_in_hand(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| taxonomy(c.card_id).is_self_damage_source())
        .count() as i32
}

fn fire_breathing_play_value(state: &SimState) -> i32 {
    let status_now = status_cards_in_hand(state);
    let status_backlog = state.future_status_cards.max(0);
    let monster_count = alive_monster_count(state).max(1);
    let sentry_pressure = state.sentry_count.max(0);

    if state.has_fire_breathing {
        return -8_000;
    }

    let immediate_hits = status_now * monster_count * 900;
    let backlog_hits = status_backlog * monster_count * 420;
    let sentry_bonus = sentry_pressure * 4_500;
    let urgent_hand_bonus = if status_now > 0 { 2_500 } else { 0 };

    2_500 + immediate_hits + backlog_hits + sentry_bonus + urgent_hand_bonus
}

fn fire_breathing_posture_adjustment(posture: &HeuristicPostureFeatures) -> i32 {
    posture.future_pollution_risk * 650
        + posture.expected_fight_length_bucket * 1_600
        + posture.setup_payoff_density * 520
        - posture.immediate_survival_pressure.min(24) * 120
}

fn dark_embrace_posture_adjustment(posture: &HeuristicPostureFeatures) -> i32 {
    posture.future_pollution_risk * 320
        + posture.expected_fight_length_bucket * 950
        + posture.setup_payoff_density * 420
        - posture.immediate_survival_pressure.min(20) * 130
}

fn brutality_timing_value(state: &SimState) -> i32 {
    if state.has_brutality {
        return -7_500;
    }

    let mut value = 8_000;
    if state.player_hp >= 24 {
        value += 1_800;
    } else if state.player_hp >= 16 {
        value += 1_000;
    } else if state.player_hp <= 8 {
        value -= 3_000;
    }

    if total_incoming_damage(state) == 0 && state.player_hp >= 14 {
        value += 800;
    }
    if state.player_hp <= 5 {
        value -= 2_000;
    }

    value
}

fn panache_timing_value(state: &SimState) -> i32 {
    if state.has_panache {
        return -8_000;
    }

    9_000
        + active_hand_cards(state)
            .filter(|(_, c)| c.cost == 0 || c.cost == 1)
            .count() as i32
            * 500
}
