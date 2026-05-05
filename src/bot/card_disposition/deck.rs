use crate::bot::card_taxonomy::{is_multi_attack_payoff, is_strength_enabler, is_strength_payoff};
use crate::bot::DeckProfile;
use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

use super::helpers::{
    is_block_core_card, is_draw_core_card, is_exhaust_engine_card, is_exhaust_outlet_card,
    is_keeper_priority_card, is_setup_power_card,
};

#[derive(Clone, Copy)]
pub(crate) enum DeckDispositionMode {
    Purge,
    Transform,
    TransformUpgraded,
}

pub(crate) fn deck_cut_score(
    rs: &RunState,
    profile: &DeckProfile,
    card: &CombatCard,
    mode: DeckDispositionMode,
    bash_preservation_bonus: i32,
) -> i32 {
    let mut score = retention_value(rs, profile, card, bash_preservation_bonus);
    let def = cards::get_card_definition(card.id);

    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        score -= 2_000;
    }
    score -= crate::bot::curse_remove_severity(card.id) * 450;

    if cards::is_starter_basic(card.id) {
        score -= match mode {
            DeckDispositionMode::Purge => 260,
            DeckDispositionMode::Transform => 240,
            DeckDispositionMode::TransformUpgraded => 360,
        };
    }

    if card.upgrades > 0 {
        score += match mode {
            DeckDispositionMode::Purge => 80 + card.upgrades as i32 * 35,
            DeckDispositionMode::Transform => 100 + card.upgrades as i32 * 45,
            DeckDispositionMode::TransformUpgraded => 160 + card.upgrades as i32 * 60,
        };
    }

    score
}

pub(crate) fn duplicate_score(rs: &RunState, profile: &DeckProfile, card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    let mut score = retention_value(rs, profile, card, 0);

    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return i32::MIN / 4;
    }

    if cards::is_starter_basic(card.id) {
        score -= 140;
    }

    if card.upgrades > 0 {
        score += 35 + card.upgrades as i32 * 20;
    }

    if is_keeper_priority_card(card.id) {
        score += 90;
    }
    if is_setup_power_card(card.id) {
        score += 120;
    }
    if is_draw_core_card(card.id) {
        score += 60;
    }
    if is_strength_enabler(card.id) && profile.strength_payoffs >= 1 {
        score += 120;
    }
    if is_strength_payoff(card.id) && profile.strength_enablers >= 1 {
        score += 110;
    }
    if is_exhaust_engine_card(card.id) && profile.exhaust_outlets >= 1 {
        score += 120;
    }
    if is_exhaust_outlet_card(card.id) && profile.exhaust_engines >= 1 {
        score += 80;
    }
    if is_block_core_card(card.id) && profile.block_core >= 2 {
        score += 50;
    }

    score + duplicate_shell_bonus(card.id, profile)
}

pub(crate) fn retention_value(
    rs: &RunState,
    profile: &DeckProfile,
    card: &CombatCard,
    bash_preservation_bonus: i32,
) -> i32 {
    let mut score = crate::bot::score_owned_card(card.id, rs);
    let def = cards::get_card_definition(card.id);

    if card.id == CardId::Bash {
        score += bash_preservation_bonus;
    }

    score += shell_core_preservation_penalty(card.id, profile);

    if is_draw_core_card(card.id) {
        score += 18 + profile.draw_sources * 2;
    }
    if is_setup_power_card(card.id) {
        score += 24 + profile.power_scalers * 2;
    }
    if is_keeper_priority_card(card.id) {
        score += 18;
    }
    if !cards::is_starter_basic(card.id) && card.upgrades > 0 {
        score += 24 + card.upgrades as i32 * 18;
    }
    if matches!(def.card_type, CardType::Power) {
        score += 22;
    }

    score
}

pub(crate) fn shell_core_preservation_penalty(card_id: CardId, profile: &DeckProfile) -> i32 {
    let mut penalty = 0;

    if is_strength_enabler(card_id) && profile.strength_payoffs >= 1 {
        penalty += 110;
    }
    if is_strength_payoff(card_id) && profile.strength_enablers >= 1 {
        penalty += if is_multi_attack_payoff(card_id) {
            70
        } else {
            58
        };
    }
    if is_exhaust_engine_card(card_id)
        && (profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1)
    {
        penalty += 120;
    }
    if is_exhaust_outlet_card(card_id) && profile.exhaust_engines >= 1 {
        penalty += 70;
    }
    if is_block_core_card(card_id) && profile.block_core >= 2 {
        penalty += match card_id {
            CardId::Barricade | CardId::Entrench => 90,
            CardId::BodySlam
            | CardId::Juggernaut
            | CardId::Impervious
            | CardId::FlameBarrier
            | CardId::PowerThrough => 60,
            _ => 26,
        };
    }
    if is_draw_core_card(card_id) && profile.draw_sources >= 2 {
        penalty += 22;
    }
    if is_keeper_priority_card(card_id) {
        penalty += 24;
    }

    penalty
}

pub(crate) fn duplicate_shell_bonus(card_id: CardId, profile: &DeckProfile) -> i32 {
    match card_id {
        CardId::LimitBreak if profile.strength_enablers >= 1 => 24,
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Whirlwind
            if profile.strength_enablers >= 1 =>
        {
            12
        }
        CardId::FeelNoPain | CardId::DarkEmbrace if profile.exhaust_outlets >= 1 => 18,
        CardId::SecondWind | CardId::BurningPact | CardId::FiendFire
            if profile.exhaust_engines >= 1 =>
        {
            16
        }
        CardId::BodySlam | CardId::Impervious | CardId::FlameBarrier
            if profile.block_payoffs >= 1 =>
        {
            12
        }
        CardId::Offering | CardId::Shockwave | CardId::Apotheosis | CardId::DemonForm => 24,
        _ => 0,
    }
}
