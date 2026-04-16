use crate::bot::combat_families::draw::deck_cycle_thinning_score;
use crate::bot::combat_families::survival::{
    hand_shaping_delay_quality_score, hand_shaping_play_now_score,
};
use crate::content::cards::{CardId, CardType};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MassExhaustProfile {
    pub exhausted_count: i32,
    pub total_fuel: i32,
    pub remaining_cards_after: i32,
    pub remaining_low_value_fuel_after: i32,
    pub closeout_bonus: i32,
    pub junk_fuel_count: i32,
    pub protected_piece_count: i32,
    pub core_piece_count: i32,
    pub engine_support_level: i32,
    pub block_per_exhaust: i32,
    pub imminent_unblocked_damage: i32,
    pub playable_block_lost: i32,
    pub exact_stabilize: bool,
    pub low_pressure_high_hp: bool,
    pub dark_embrace_draw_count: i32,
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

pub(crate) fn mass_exhaust_base_score(profile: &MassExhaustProfile, total_cycle_cards: i32) -> i32 {
    exhaust_mass_play_score(
        profile.total_fuel,
        profile.exhausted_count,
        profile.remaining_cards_after,
        profile.remaining_low_value_fuel_after,
        profile.closeout_bonus,
    ) + exhaust_engine_immediate_payoff_score(
        profile.exhausted_count,
        profile.block_per_exhaust,
        profile.dark_embrace_draw_count,
    ) + deck_cycle_thinning_score(
        total_cycle_cards,
        profile.remaining_cards_after,
        profile.dark_embrace_draw_count,
        0,
        0,
        0,
    )
}

pub(crate) fn mass_exhaust_second_wind_selectivity_score(profile: &MassExhaustProfile) -> i32 {
    let mut score = forced_mass_exhaust_selectivity_score(
        profile.junk_fuel_count,
        profile.protected_piece_count,
        profile.core_piece_count,
        profile.exact_stabilize,
        profile.imminent_unblocked_damage,
        profile.engine_support_level,
    );

    if profile.playable_block_lost > 0 {
        let emergency_relief = if profile.exact_stabilize {
            0
        } else {
            profile.imminent_unblocked_damage.min(18) * 180
        };
        let preserve_penalty =
            (profile.playable_block_lost * 6_200) - profile.junk_fuel_count * 650 - emergency_relief;
        score -= preserve_penalty.max(0);
        if profile.junk_fuel_count >= 1
            && profile.playable_block_lost >= 2
            && !profile.exact_stabilize
        {
            score -= 12_000 + profile.playable_block_lost * 2_500;
        }
    }

    if profile.engine_support_level == 0
        && profile.junk_fuel_count == 0
        && profile.protected_piece_count >= 2
        && profile.low_pressure_high_hp
    {
        let base_penalty = if profile.exact_stabilize {
            9_000
        } else {
            14_000
        };
        score -=
            base_penalty + profile.protected_piece_count * 1_800 + profile.core_piece_count * 2_500;
    }

    score
}

pub(crate) fn mass_exhaust_keeper_penalty(
    profile: &MassExhaustProfile,
    protected_piece_weight: i32,
    core_piece_weight: i32,
) -> i32 {
    profile.protected_piece_count * protected_piece_weight
        + profile.core_piece_count * core_piece_weight
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
