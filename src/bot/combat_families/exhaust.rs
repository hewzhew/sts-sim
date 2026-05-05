use crate::bot::combat_families::survival::{
    hand_shaping_delay_quality_score, hand_shaping_play_now_score,
};
use crate::content::cards::{CardId, CardType};

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
