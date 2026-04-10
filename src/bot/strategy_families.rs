use crate::content::cards::{CardId, CardType};

pub(crate) fn survival_swing_score(
    current_hp: i32,
    imminent_unblocked_damage: i32,
    missing_hp: i32,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    let effective_heal = hp_gain.min(missing_hp.max(0));
    let lethalish = imminent_unblocked_damage >= current_hp.saturating_sub(4);
    let covered = effective_heal + prevented_damage;
    let exact_stabilize = imminent_unblocked_damage > 0 && covered >= imminent_unblocked_damage;
    let lethal_window = imminent_unblocked_damage >= current_hp;
    let massive_window = imminent_unblocked_damage >= current_hp + 10
        || imminent_unblocked_damage >= current_hp.saturating_mul(2);
    let remaining_gap = (imminent_unblocked_damage - covered).max(0);

    let mut value = effective_heal * 1_200;
    value += prevented_damage * 220;
    value += kills * 1_600;

    if current_hp <= 30 && effective_heal > 0 {
        value += 4_000;
    }
    if lethalish && covered > 0 {
        value += 5_000 + covered * 180;
    } else if covered >= imminent_unblocked_damage.max(0) && covered > 0 {
        value += 2_500;
    }
    if exact_stabilize {
        value += 6_500 + imminent_unblocked_damage.max(0) * 220;
        if lethal_window {
            value += 10_000;
        }
        if massive_window {
            value += 6_000;
        }
    } else if lethal_window && covered > 0 {
        value += 7_500 + covered * 220 - remaining_gap * 260;
    }
    if lethal_window && kills > 0 {
        value += 2_500;
    }

    value
}

pub(crate) fn reaper_timing_score(
    current_hp: i32,
    imminent_unblocked_damage: i32,
    missing_hp: i32,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    survival_swing_score(
        current_hp,
        imminent_unblocked_damage,
        missing_hp,
        hp_gain,
        prevented_damage,
        kills,
    )
}

pub(crate) fn hand_shaping_play_now_score(can_play_now: bool) -> i32 {
    if can_play_now {
        -1_500
    } else {
        1_500
    }
}

pub(crate) fn apotheosis_timing_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    let mut value = 8_500 + upgradable_targets.max(0) * 1_400;
    if upgradable_targets <= 0 {
        value -= 6_000;
    } else if upgradable_targets >= 3 {
        value += 4_000;
    }
    if imminent_unblocked_damage > 8 {
        value -= 1_500;
    }
    value
}

pub(crate) fn apotheosis_hand_shaping_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    -(apotheosis_timing_score(upgradable_targets, imminent_unblocked_damage) / 2)
}

pub(crate) fn apparition_timing_score(
    current_hp: i32,
    current_intangible: i32,
    imminent_unblocked_damage: i32,
    total_incoming_damage: i32,
    apparitions_in_hand: i32,
    remaining_apparitions_total: i32,
    upgraded: bool,
    has_runic_pyramid: bool,
    encounter_pressure: i32,
) -> i32 {
    let prevented_damage = if imminent_unblocked_damage > 0 {
        imminent_unblocked_damage
    } else {
        total_incoming_damage
    };
    let swing = survival_swing_score(
        current_hp,
        imminent_unblocked_damage,
        0,
        0,
        prevented_damage,
        0,
    );
    let lethal_window = imminent_unblocked_damage >= current_hp;
    let massive_window = imminent_unblocked_damage >= current_hp + 10
        || imminent_unblocked_damage >= current_hp.saturating_mul(2);
    let hand_pressure = apparitions_in_hand.saturating_sub(1);
    let reserve_pressure = remaining_apparitions_total.saturating_sub(1);

    if current_intangible > 0 {
        let mut value = -6_000 - current_intangible * 2_000;
        if !upgraded {
            value += 2_000;
        } else if has_runic_pyramid {
            value -= 1_000;
        }
        if current_hp <= 25 {
            value += 1_500;
        }
        if hand_pressure >= 2 {
            value += hand_pressure * 1_500;
        }
        if reserve_pressure >= 2 {
            value += reserve_pressure.min(4) * 800;
        }
        value += encounter_pressure.max(0) * 180;
        if total_incoming_damage >= 12 {
            value += 1_500;
        }
        if lethal_window {
            value += 4_000;
        }
        if massive_window {
            value += 3_000;
        }
        return value;
    }

    if !upgraded {
        let mut value = 5_500 + swing;
        if imminent_unblocked_damage > 0 || current_hp <= 35 {
            value += 5_000;
        }
        if lethal_window {
            value += 8_000;
        }
        if apparitions_in_hand >= 2 {
            value += 1_500;
        }
        if imminent_unblocked_damage == 0
            && total_incoming_damage == 0
            && current_hp <= 35
            && reserve_pressure >= 2
            && encounter_pressure >= 10
        {
            value += 2_500 + encounter_pressure * 120;
        }
        value
    } else {
        let mut value = if imminent_unblocked_damage > 0 || current_hp <= 22 {
            8_500 + swing
        } else if has_runic_pyramid {
            -3_000
        } else {
            -1_500
        };
        if lethal_window {
            value += 10_000;
        }
        if massive_window {
            value += 8_000;
        }
        if has_runic_pyramid && (lethal_window || massive_window) {
            value += 2_500;
        }
        if imminent_unblocked_damage == 0
            && total_incoming_damage == 0
            && current_hp <= 28
            && reserve_pressure >= 2
            && encounter_pressure >= 12
        {
            value += 2_000 + encounter_pressure * 110;
        }
        value
    }
}

pub(crate) fn apparition_hand_shaping_score(
    current_hp: i32,
    current_intangible: i32,
    imminent_unblocked_damage: i32,
    total_incoming_damage: i32,
    apparitions_in_hand: i32,
    remaining_apparitions_total: i32,
    upgraded: bool,
    has_runic_pyramid: bool,
    encounter_pressure: i32,
) -> i32 {
    let timing = apparition_timing_score(
        current_hp,
        current_intangible,
        imminent_unblocked_damage,
        total_incoming_damage,
        apparitions_in_hand,
        remaining_apparitions_total,
        upgraded,
        has_runic_pyramid,
        encounter_pressure,
    );

    if !upgraded {
        if timing >= 8_000 {
            -(timing / 4).max(2_000)
        } else {
            2_500 + apparitions_in_hand.saturating_sub(1) * 500
        }
    } else if timing > 0 {
        -(timing / 5).max(1_500)
    } else if has_runic_pyramid {
        -2_400
    } else {
        -800
    }
}

pub(crate) fn reaper_hand_shaping_score(
    current_hp: i32,
    imminent_unblocked_damage: i32,
    missing_hp: i32,
) -> i32 {
    let assumed_heal = missing_hp.min(12).max(0);
    let assumed_prevented = imminent_unblocked_damage.min(assumed_heal).max(0);
    -(reaper_timing_score(
        current_hp,
        imminent_unblocked_damage,
        missing_hp,
        assumed_heal,
        assumed_prevented,
        0,
    ) / 3)
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

pub(crate) fn forced_mass_exhaust_selectivity_score(
    junk_fuel_count: i32,
    protected_piece_count: i32,
    core_piece_count: i32,
    exact_stabilize: bool,
    imminent_unblocked_damage: i32,
    engine_support_level: i32,
) -> i32 {
    let mut score = 0;

    if junk_fuel_count > 0 {
        score += 2_500 + junk_fuel_count * 2_000;
        if imminent_unblocked_damage > 0 {
            score += junk_fuel_count * 900;
        }
    }

    score -= protected_piece_count * 7_500;
    score -= core_piece_count * 14_000;

    let junk_shortage = (protected_piece_count - junk_fuel_count).max(0);
    if junk_shortage > 0 {
        score -= junk_shortage * 8_500;
    }
    if junk_fuel_count == 0 && protected_piece_count > 0 {
        score -= 8_000;
    }
    if engine_support_level > 0 && junk_fuel_count > 0 {
        score += junk_fuel_count * engine_support_level * 2_000;
    }
    if exact_stabilize {
        score += 5_000 + imminent_unblocked_damage.min(25).max(0) * 220;
    }

    score
}

pub(crate) fn exhaust_fuel_value_score(
    card_id: CardId,
    card_type: CardType,
    cost: i32,
    current_energy: i32,
    safe_block_turn: bool,
    can_play_now: bool,
    timing_hold_score: i32,
    feel_no_pain_amount: i32,
    has_dark_embrace: bool,
) -> i32 {
    let mut score =
        hand_shaping_delay_quality_score(card_id, card_type, cost, current_energy, safe_block_turn);
    score += hand_shaping_play_now_score(can_play_now);
    score += timing_hold_score;

    if matches!(card_type, CardType::Curse | CardType::Status) {
        score +=
            status_exhaust_value_score(card_id, card_type, feel_no_pain_amount, has_dark_embrace);
    }

    if card_type == CardType::Power {
        score -= 4_500;
    }

    score -= match card_id {
        CardId::FiendFire
        | CardId::SecondWind
        | CardId::TrueGrit
        | CardId::BurningPact
        | CardId::SeverSoul
        | CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace
        | CardId::LimitBreak
        | CardId::DemonForm => 2_800,
        CardId::Inflame | CardId::Shockwave => 4_200,
        CardId::Offering => 1_400,
        _ => 0,
    };

    score
}

fn status_exhaust_value_score(
    card_id: CardId,
    card_type: CardType,
    feel_no_pain_amount: i32,
    has_dark_embrace: bool,
) -> i32 {
    match card_type {
        CardType::Curse => match card_id {
            CardId::Parasite => 49_000,
            CardId::Pain | CardId::Normality => 47_000,
            CardId::Regret | CardId::Writhe | CardId::Decay => 44_000,
            _ => 40_000,
        },
        CardType::Status => match card_id {
            CardId::Dazed => {
                let mut score = 10_000;
                if feel_no_pain_amount > 0 {
                    score += 7_000 + feel_no_pain_amount * 450;
                }
                if has_dark_embrace {
                    score += 9_000;
                }
                score
            }
            CardId::Slimed => 44_000,
            CardId::Burn => 47_000,
            _ => 40_000,
        },
        _ => 0,
    }
}

pub(crate) fn exhaust_deck_floor_penalty(remaining_cards_after: i32) -> i32 {
    match remaining_cards_after {
        i32::MIN..=4 => 12_000,
        5 => 7_500,
        6 => 4_500,
        7 => 2_000,
        8 => 800,
        _ => 0,
    }
}

pub(crate) fn exhaust_fuel_reserve_penalty(
    remaining_low_value_fuel_after: i32,
    exhaust_count: i32,
) -> i32 {
    if exhaust_count <= 1 {
        return 0;
    }

    match remaining_low_value_fuel_after {
        i32::MIN..=0 => 3_500,
        1 => 1_800,
        _ => 0,
    }
}

pub(crate) fn exhaust_future_fuel_reserve_score(
    remaining_low_value_fuel_after: i32,
    future_exhaust_demand: i32,
) -> i32 {
    if future_exhaust_demand <= 0 {
        return 0;
    }

    let shortage = (future_exhaust_demand - remaining_low_value_fuel_after).max(0);
    -(shortage * 25_000)
}

pub(crate) fn exhaust_mass_play_score(
    total_fuel_value: i32,
    exhaust_count: i32,
    remaining_cards_after: i32,
    remaining_low_value_fuel_after: i32,
    closeout_bonus: i32,
) -> i32 {
    total_fuel_value + exhaust_count.max(0) * 220 + closeout_bonus
        - exhaust_deck_floor_penalty(remaining_cards_after)
        - exhaust_fuel_reserve_penalty(remaining_low_value_fuel_after, exhaust_count)
}

pub(crate) fn exhaust_random_play_score(
    low_value_fuel_count: i32,
    protected_piece_count: i32,
    remaining_cards_after: i32,
) -> i32 {
    let mut score = low_value_fuel_count * 1_600 - protected_piece_count * 1_400;
    score -= exhaust_deck_floor_penalty(remaining_cards_after);

    if low_value_fuel_count <= 0 {
        score -= 4_500;
    } else if low_value_fuel_count == 1 {
        score -= 1_200;
    }

    score
}

pub(crate) fn exhaust_random_core_risk_score(
    low_value_fuel_count: i32,
    core_piece_count: i32,
    near_core_piece_count: i32,
) -> i32 {
    let mut score = 0;

    if low_value_fuel_count <= 0 {
        score -= core_piece_count * 3_500 + near_core_piece_count * 1_800;
    } else if low_value_fuel_count == 1 {
        score -= core_piece_count * 1_800 + near_core_piece_count * 900;
    }

    score
}

pub(crate) fn exhaust_engine_payoff_score(
    exhaust_count: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
    status_fuel_count: i32,
    future_status_cards: i32,
) -> i32 {
    exhaust_engine_immediate_payoff_score(exhaust_count, block_per_exhaust, draw_per_exhaust)
        + exhaust_engine_future_payoff_score(status_fuel_count, future_status_cards, 0)
}

pub(crate) fn exhaust_engine_immediate_payoff_score(
    exhaust_count: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
) -> i32 {
    let exhaust_count = exhaust_count.max(0);
    let mut score = 0;
    score += exhaust_count * block_per_exhaust.max(0) * 220;
    score += exhaust_count * draw_per_exhaust.max(0) * 1_000;
    score
}

pub(crate) fn exhaust_engine_future_payoff_score(
    status_fuel_count: i32,
    future_status_cards: i32,
    sentry_count: i32,
) -> i32 {
    let mut score = 0;
    score += status_fuel_count.max(0) * 450;
    score += future_status_cards.max(0) * 160;
    score += sentry_count.max(0) * 2_500;
    score
}

pub(crate) fn exhaust_engine_setup_score(
    already_active: bool,
    immediate_exhaust_count: i32,
    future_exhaust_sources: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
    status_fuel_count: i32,
    future_status_cards: i32,
    sentry_count: i32,
    extra_synergy: i32,
) -> i32 {
    if already_active {
        return -8_000;
    }

    2_500
        + exhaust_engine_immediate_payoff_score(
            immediate_exhaust_count,
            block_per_exhaust,
            draw_per_exhaust,
        )
        + future_exhaust_sources.max(0) * 900
        + exhaust_engine_future_payoff_score(status_fuel_count, future_status_cards, sentry_count)
        + extra_synergy
}

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

pub(crate) fn battle_trance_timing_score(
    current_energy: i32,
    player_no_draw: bool,
    draw_count: i32,
    current_hand_size: i32,
    future_zero_cost_cards: i32,
    future_one_cost_cards: i32,
    future_two_plus_cost_cards: i32,
    future_key_delay_weight: i32,
    future_high_cost_key_delay_weight: i32,
    future_status_cards: i32,
    other_draw_sources_in_hand: i32,
) -> i32 {
    draw_action_timing_score(
        current_energy,
        player_no_draw,
        true,
        draw_count,
        current_hand_size,
        future_zero_cost_cards,
        future_one_cost_cards,
        future_two_plus_cost_cards,
        future_key_delay_weight,
        future_high_cost_key_delay_weight,
        future_status_cards,
        other_draw_sources_in_hand,
    )
}

pub(crate) fn draw_action_timing_score(
    current_energy: i32,
    player_no_draw: bool,
    applies_no_draw: bool,
    draw_count: i32,
    current_hand_size: i32,
    future_zero_cost_cards: i32,
    future_one_cost_cards: i32,
    future_two_plus_cost_cards: i32,
    future_key_delay_weight: i32,
    future_high_cost_key_delay_weight: i32,
    future_status_cards: i32,
    other_draw_sources_in_hand: i32,
) -> i32 {
    if player_no_draw {
        return -14_000;
    }

    let mut score = 2_800 + draw_count.max(0) * 1_200;
    let hand_after_draw = current_hand_size + draw_count;
    score -= (hand_after_draw - 9).max(0) * 1_100;
    if applies_no_draw {
        score -= 1_200;
    }

    match current_energy {
        i32::MIN..=0 => {
            score -= 4_800;
            score += future_zero_cost_cards * 700;
            score += future_one_cost_cards * 120;
            score -= future_two_plus_cost_cards * 900;
            score -= future_key_delay_weight * 260;
            score -= future_high_cost_key_delay_weight * 420;
            score -= future_status_cards * 850;
            score -= other_draw_sources_in_hand * 1_200;
            if applies_no_draw {
                score -= 1_400 + other_draw_sources_in_hand * 600;
            }
            if future_zero_cost_cards == 0 {
                score -= 2_000;
            }
        }
        1 => {
            score += future_zero_cost_cards * 600;
            score += future_one_cost_cards * 450;
            score -= future_two_plus_cost_cards * 450;
            score -= future_key_delay_weight * 140;
            score -= future_high_cost_key_delay_weight * 220;
            score -= future_status_cards * 600;
            score -= other_draw_sources_in_hand * 900;
            if applies_no_draw {
                score -= 1_000 + other_draw_sources_in_hand * 450;
            }
        }
        _ => {
            score += future_zero_cost_cards * 280;
            score += future_one_cost_cards * 420;
            score += future_two_plus_cost_cards * 260;
            score -= future_key_delay_weight * 40;
            score -= future_high_cost_key_delay_weight * 60;
            score -= future_status_cards * 350;
            score -= other_draw_sources_in_hand * 650;
            if applies_no_draw {
                score -= 500 + other_draw_sources_in_hand * 250;
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

#[cfg(test)]
mod tests {
    use crate::content::cards::{CardId, CardType};

    use super::{
        apotheosis_hand_shaping_score, apotheosis_timing_score, apparition_hand_shaping_score,
        apparition_timing_score, battle_trance_timing_score, body_slam_delay_score,
        deck_cycle_thinning_score, draw_action_timing_score, draw_continuity_score,
        exhaust_deck_floor_penalty, exhaust_engine_future_payoff_score,
        exhaust_engine_immediate_payoff_score, exhaust_engine_payoff_score,
        exhaust_engine_setup_score, exhaust_finish_window_score, exhaust_fuel_value_score,
        exhaust_future_fuel_reserve_score, exhaust_mass_play_score, exhaust_random_core_risk_score,
        exhaust_random_play_score, flight_break_progress_score,
        forced_mass_exhaust_selectivity_score, hand_shaping_delay_quality_score,
        hand_shaping_next_draw_window_score, hand_shaping_play_now_score,
        persistent_block_progress_score, reaper_hand_shaping_score, reaper_timing_score,
        status_loop_cycle_score, survival_swing_score,
    };

    #[test]
    fn survival_swing_rewards_covering_lethal() {
        let low = survival_swing_score(20, 18, 40, 0, 4, 0);
        let high = survival_swing_score(20, 18, 40, 0, 18, 0);
        assert!(high > low);
    }

    #[test]
    fn flight_break_is_better_than_partial_chip() {
        let chip = flight_break_progress_score(1, 2, 9);
        let break_it = flight_break_progress_score(2, 2, 9);
        assert!(break_it > chip);
    }

    #[test]
    fn persistent_block_progress_scales_with_damage_removed() {
        assert!(persistent_block_progress_score(12) > persistent_block_progress_score(4));
    }

    #[test]
    fn apotheosis_prefers_more_upgrade_targets() {
        let low = apotheosis_timing_score(1, 0);
        let high = apotheosis_timing_score(4, 0);
        assert!(high > low);
    }

    #[test]
    fn unupgraded_apparition_is_more_urgent_than_upgraded_when_safe() {
        let unupgraded = apparition_timing_score(40, 0, 0, 0, 1, 1, false, false, 0);
        let upgraded = apparition_timing_score(40, 0, 0, 0, 1, 1, true, false, 0);
        assert!(unupgraded > upgraded);
    }

    #[test]
    fn reaper_timing_reuses_survival_swing_language() {
        let low = reaper_timing_score(18, 14, 30, 4, 0, 0);
        let high = reaper_timing_score(18, 14, 30, 10, 6, 1);
        assert!(high > low);
    }

    #[test]
    fn hand_shaping_prefers_playable_cards_in_hand() {
        assert!(hand_shaping_play_now_score(true) < hand_shaping_play_now_score(false));
    }

    #[test]
    fn apotheosis_hand_shaping_reuses_timing_language() {
        let low = apotheosis_hand_shaping_score(1, 0);
        let high = apotheosis_hand_shaping_score(4, 0);
        assert!(high < low);
    }

    #[test]
    fn unupgraded_apparition_can_be_topdecked_when_safe() {
        let safe = apparition_hand_shaping_score(40, 0, 0, 0, 1, 1, false, false, 0);
        let pressured = apparition_hand_shaping_score(18, 0, 14, 14, 1, 1, false, false, 10);
        assert!(safe > 0);
        assert!(pressured < safe);
    }

    #[test]
    fn survival_swing_spikes_when_exact_lethal_is_covered() {
        let chip = survival_swing_score(28, 26, 40, 0, 6, 0);
        let stabilize = survival_swing_score(28, 26, 40, 0, 26, 0);
        assert!(stabilize > chip);
    }

    #[test]
    fn upgraded_apparition_with_runic_pyramid_is_urgent_in_massive_window() {
        let safe = apparition_timing_score(60, 0, 0, 0, 1, 1, true, true, 0);
        let lethal = apparition_timing_score(38, 0, 51, 51, 1, 1, true, true, 15);
        assert!(lethal > safe);
        assert!(lethal > 20_000);
    }

    #[test]
    fn apparition_stacking_is_not_hard_disabled_when_hand_is_flooded() {
        let one_copy = apparition_timing_score(18, 1, 0, 12, 1, 1, false, false, 8);
        let many_copies = apparition_timing_score(18, 1, 0, 12, 4, 4, false, false, 12);
        assert!(many_copies > one_copy);
        assert!(many_copies > -10_000);
    }

    #[test]
    fn apparition_can_be_frontloaded_for_next_turn_safety_in_scaling_fight() {
        let calm = apparition_timing_score(24, 0, 0, 0, 2, 3, false, false, 2);
        let scaling = apparition_timing_score(24, 0, 0, 0, 2, 3, false, false, 16);
        assert!(scaling > calm);
    }

    #[test]
    fn reaper_timing_spikes_when_heal_and_kills_cover_lethal_window() {
        let chip = reaper_timing_score(24, 22, 40, 6, 0, 0);
        let swing = reaper_timing_score(24, 22, 40, 12, 22, 2);
        assert!(swing > chip);
        assert!(swing > 20_000);
    }

    #[test]
    fn runic_pyramid_discourages_topdecking_upgraded_apparition_when_safe() {
        let without_pyramid = apparition_hand_shaping_score(55, 0, 0, 0, 1, 1, true, false, 0);
        let with_pyramid = apparition_hand_shaping_score(55, 0, 0, 0, 1, 1, true, true, 0);
        assert!(with_pyramid < without_pyramid);
    }

    #[test]
    fn reaper_hand_shaping_keeps_card_when_low_hp() {
        let safe = reaper_hand_shaping_score(60, 0, 0);
        let pressured = reaper_hand_shaping_score(18, 14, 20);
        assert!(pressured < safe);
    }

    #[test]
    fn next_draw_window_tax_penalizes_guaranteed_topdeck() {
        assert!(
            hand_shaping_next_draw_window_score(5, true)
                < hand_shaping_next_draw_window_score(5, false)
        );
    }

    #[test]
    fn low_value_delay_quality_prefers_safe_defend_over_zero_cost_attack() {
        let safe_defend =
            hand_shaping_delay_quality_score(CardId::Defend, CardType::Skill, 1, 3, true);
        let zero_cost_attack =
            hand_shaping_delay_quality_score(CardId::Anger, CardType::Attack, 0, 3, true);
        assert!(safe_defend > zero_cost_attack);
    }

    #[test]
    fn exhaust_fuel_prefers_curse_over_apotheosis() {
        let curse = exhaust_fuel_value_score(
            CardId::Injury,
            CardType::Curse,
            -2,
            3,
            true,
            false,
            0,
            0,
            false,
        );
        let apotheosis = exhaust_fuel_value_score(
            CardId::Apotheosis,
            CardType::Skill,
            2,
            3,
            true,
            true,
            apotheosis_hand_shaping_score(3, 0),
            0,
            false,
        );
        assert!(curse > apotheosis);
    }

    #[test]
    fn dazed_exhaust_value_rises_with_exhaust_engine_support() {
        let low = exhaust_fuel_value_score(
            CardId::Dazed,
            CardType::Status,
            -2,
            3,
            true,
            false,
            0,
            0,
            false,
        );
        let high = exhaust_fuel_value_score(
            CardId::Dazed,
            CardType::Status,
            -2,
            3,
            true,
            false,
            0,
            4,
            true,
        );
        assert!(high > low);
    }

    #[test]
    fn forced_mass_exhaust_selectivity_penalizes_burning_more_core_than_junk() {
        let risky = forced_mass_exhaust_selectivity_score(1, 2, 1, false, 10, 0);
        let clean = forced_mass_exhaust_selectivity_score(2, 0, 0, false, 10, 0);
        assert!(clean > risky);
    }

    #[test]
    fn body_slam_delay_penalizes_early_fire_without_lethal() {
        let delayed = body_slam_delay_score(0, 12, false, 8);
        let lethal = body_slam_delay_score(18, 12, true, 8);
        assert!(lethal > delayed);
    }

    #[test]
    fn exhaust_deck_floor_penalty_spikes_below_five() {
        assert!(exhaust_deck_floor_penalty(4) > exhaust_deck_floor_penalty(6));
    }

    #[test]
    fn random_exhaust_prefers_having_real_fuel() {
        let no_fuel = exhaust_random_play_score(0, 2, 7);
        let good_fuel = exhaust_random_play_score(2, 1, 7);
        assert!(good_fuel > no_fuel);
    }

    #[test]
    fn mass_exhaust_is_penalized_when_it_overthins_small_deck() {
        let thin = exhaust_mass_play_score(2_000, 3, 4, 0, 0);
        let healthy = exhaust_mass_play_score(2_000, 3, 9, 2, 0);
        assert!(healthy > thin);
    }

    #[test]
    fn future_fuel_reserve_penalizes_spending_last_bad_card_too_early() {
        assert!(exhaust_future_fuel_reserve_score(0, 2) < exhaust_future_fuel_reserve_score(2, 2));
    }

    #[test]
    fn finish_window_rewards_closeout_more_than_chip() {
        let chip = exhaust_finish_window_score(false, 0, 0, 2);
        let closeout = exhaust_finish_window_score(true, 1, 8, 0);
        assert!(closeout > chip);
    }

    #[test]
    fn random_exhaust_penalizes_core_risk_when_no_real_fuel() {
        let risky = exhaust_random_core_risk_score(0, 2, 1);
        let safer = exhaust_random_core_risk_score(2, 2, 1);
        assert!(safer > risky);
    }

    #[test]
    fn exhaust_engine_payoff_scales_with_real_engine_amounts() {
        let low = exhaust_engine_payoff_score(2, 0, 0, 0, 0);
        let high = exhaust_engine_payoff_score(2, 4, 1, 2, 4);
        assert!(high > low);
    }

    #[test]
    fn exhaust_engine_setup_prefers_rich_exhaust_windows() {
        let poor = exhaust_engine_setup_score(false, 0, 0, 4, 1, 0, 0, 0, 0);
        let rich = exhaust_engine_setup_score(false, 2, 3, 4, 1, 2, 4, 1, 0);
        assert!(rich > poor);
    }

    #[test]
    fn immediate_and_future_engine_payoff_are_distinct() {
        let immediate = exhaust_engine_immediate_payoff_score(2, 4, 1);
        let future = exhaust_engine_future_payoff_score(2, 4, 1);
        assert!(immediate > 0);
        assert!(future > 0);
    }

    #[test]
    fn draw_continuity_penalizes_overthinning_without_refill() {
        let thin = draw_continuity_score(4, 0, 0, 0);
        let stable = draw_continuity_score(7, 2, 0, 0);
        assert!(stable > thin);
    }

    #[test]
    fn battle_trance_is_bad_when_no_draw_is_already_active() {
        let blocked = battle_trance_timing_score(1, true, 3, 3, 1, 2, 2, 0, 0, 0, 0);
        let open = battle_trance_timing_score(1, false, 3, 3, 1, 2, 2, 0, 0, 0, 0);
        assert!(open > blocked);
        assert!(blocked < 0);
    }

    #[test]
    fn battle_trance_prefers_cheap_draws_over_expensive_zero_energy_whiffs() {
        let cheap = battle_trance_timing_score(0, false, 3, 3, 4, 2, 0, 0, 0, 0, 0);
        let expensive = battle_trance_timing_score(0, false, 3, 3, 0, 0, 4, 0, 0, 1, 0);
        assert!(cheap > expensive);
    }

    #[test]
    fn generic_draw_action_prefers_cheap_draws_over_expensive_zero_energy_whiffs() {
        let cheap = draw_action_timing_score(0, false, false, 1, 3, 3, 2, 0, 0, 0, 0, 0);
        let expensive = draw_action_timing_score(0, false, false, 1, 3, 0, 0, 3, 0, 2, 1, 0);
        assert!(cheap > expensive);
    }

    #[test]
    fn generic_draw_action_penalizes_key_card_delay_risk_when_energy_is_low() {
        let safe = draw_action_timing_score(0, false, false, 1, 3, 3, 1, 0, 0, 0, 0, 0);
        let risky = draw_action_timing_score(0, false, false, 1, 3, 1, 0, 2, 6, 6, 0, 0);
        assert!(safe > risky);
    }

    #[test]
    fn deck_cycle_thinning_needs_continuity_to_be_good() {
        let brittle = deck_cycle_thinning_score(8, 4, 0, 0, 0, 0);
        let healthy = deck_cycle_thinning_score(14, 7, 2, 0, 0, 0);
        assert!(healthy > brittle);
    }

    #[test]
    fn status_loop_values_shuffling_discard_back_into_draw() {
        let no_loop = status_loop_cycle_score(1, 1, 3, false, 1, 0);
        let with_shuffle = status_loop_cycle_score(1, 1, 3, true, 1, 0);
        assert!(with_shuffle > no_loop);
    }
}
