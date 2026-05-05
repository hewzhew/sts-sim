use crate::bot::card_facts::{facts as card_facts, CardFacts};
use crate::bot::card_structure::{structure as card_structure, CardStructure};
use crate::bot::deck_profile::{deck_profile, DeckProfile};
use crate::bot::noncombat_card_signals::signals as noncombat_card_signals;
use crate::bot::upgrade_facts::upgrade_facts;
use crate::content::cards::{self, get_card_definition, CardId, CardType};
use crate::state::run::RunState;

pub(crate) fn curse_remove_severity(card_id: CardId) -> i32 {
    match card_id {
        CardId::Parasite | CardId::Pain | CardId::Normality => 10,
        CardId::Regret | CardId::Writhe | CardId::Decay => 8,
        CardId::CurseOfTheBell => 7,
        CardId::Doubt | CardId::Shame | CardId::Injury | CardId::Clumsy => 5,
        _ => 0,
    }
}

pub fn score_card_offer(card_id: CardId, run_state: &RunState) -> i32 {
    let profile = deck_profile(run_state);
    let copies = count_copies(run_state, card_id);
    score_card(card_id, &profile, copies, true)
}

pub fn score_owned_card(card_id: CardId, run_state: &RunState) -> i32 {
    let profile = deck_profile(run_state);
    let copies = count_copies(run_state, card_id).saturating_sub(1);
    score_card(card_id, &profile, copies, false)
}

fn count_copies(run_state: &RunState, card_id: CardId) -> i32 {
    run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32
}

fn score_card(card_id: CardId, profile: &DeckProfile, duplicate_count: i32, as_offer: bool) -> i32 {
    let def = get_card_definition(card_id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return -90 - curse_remove_severity(card_id) * 4;
    }

    let facts = card_facts(card_id);
    let structure = card_structure(card_id);
    let signals = noncombat_card_signals(card_id);
    let upgrades = upgrade_facts(card_id);

    let mut score = 18;
    score += type_bias(def.card_type);
    score += signal_value(&signals);
    score += structure_value(structure);
    score += future_growth_value(as_offer, &upgrades);
    score += shell_fit_value(card_id, profile, structure, &facts);
    score -= mismatch_penalty(profile, structure, &facts);
    score -= duplicate_penalty(card_id, duplicate_count, profile, structure, &facts);
    score -= signals.filler_attack_risk * 9;

    if cards::is_starter_basic(card_id) {
        score -= starter_penalty(card_id);
    }
    if facts.ethereal {
        score -= 2;
    }
    if facts.self_damage && profile.self_damage_sources == 0 && !structure.is_strength_enabler() {
        score -= 4;
    }
    if facts.random_generation {
        score += 5;
    }
    if facts.conditional_free {
        score += 4;
    }
    if as_offer && duplicate_count == 0 {
        score += first_copy_presence_bonus(structure, &facts);
    }

    score.clamp(-120, 120)
}

fn type_bias(card_type: CardType) -> i32 {
    match card_type {
        CardType::Power => 6,
        CardType::Skill => 3,
        CardType::Attack => 0,
        _ => 0,
    }
}

fn signal_value(signals: &crate::bot::noncombat_card_signals::NoncombatCardSignals) -> i32 {
    signals.damage_patch_strength * 3
        + signals.block_patch_strength * 3
        + signals.control_patch_strength * 3
        + signals.frontload_patch_strength * 2
        + signals.scaling_signal * 2
}

fn structure_value(structure: CardStructure) -> i32 {
    let mut value = 0;
    if structure.is_engine_piece() {
        value += 8;
    }
    if structure.is_setup_piece() {
        value += 4;
    }
    if structure.is_scaling_piece() {
        value += 4;
    }
    if structure.is_draw_core() {
        value += 5;
    }
    if structure.is_block_core() {
        value += 4;
    }
    if structure.is_exhaust_outlet() {
        value += 3;
    }
    if structure.is_exhaust_engine() {
        value += 5;
    }
    if structure.is_block_payoff() {
        value += 3;
    }
    if structure.is_vuln_payoff() {
        value += 2;
    }
    value
}

fn future_growth_value(as_offer: bool, upgrades: &crate::bot::upgrade_facts::UpgradeFacts) -> i32 {
    if !as_offer {
        return 0;
    }

    let mut value = 0;
    if upgrades.changes_cost {
        value += 4;
    }
    if upgrades.improves_draw_consistency {
        value += 3;
    }
    if upgrades.improves_scaling {
        value += 4;
    }
    if upgrades.improves_target_control || upgrades.extends_debuff_duration {
        value += 3;
    }
    if upgrades.improves_exhaust_control {
        value += 3;
    }
    if upgrades.repeatable_upgrade {
        value += 4;
    }
    value
}

fn shell_fit_value(
    card_id: CardId,
    profile: &DeckProfile,
    structure: CardStructure,
    facts: &CardFacts,
) -> i32 {
    let mut value = 0;

    if structure.is_strength_enabler() {
        value += 4 + profile.strength_payoffs * 5;
    }
    if structure.is_strength_payoff() {
        value += if profile.strength_enablers > 0 {
            10 + profile.strength_enablers * 6
        } else {
            -10
        };
    }
    if structure.is_exhaust_engine() {
        value += if profile.exhaust_outlets > 0 || profile.exhaust_fodder > 0 {
            10 + profile.exhaust_outlets * 4 + profile.exhaust_fodder * 2
        } else {
            -4
        };
    }
    if structure.is_exhaust_outlet() {
        value += if profile.exhaust_engines > 0 {
            9 + profile.exhaust_engines * 4 + profile.exhaust_fodder * 2
        } else {
            1
        };
    }
    if structure.is_block_payoff() {
        value += if profile.block_core >= 2 {
            10 + profile.block_core * 4
        } else {
            -8
        };
    }
    if structure.is_block_core() && profile.block_payoffs > 0 {
        value += 6 + profile.block_payoffs * 4;
    }
    if structure.is_draw_core() {
        value += if profile.draw_sources < 3 { 8 } else { 3 };
    }
    if structure.is_resource_conversion() {
        value += if profile.x_cost_payoffs > 0 || profile.power_scalers > 0 {
            8
        } else {
            4
        };
    }
    if structure.is_status_engine() {
        value += if profile.status_payoffs > 0 { 8 } else { 2 };
    }
    if facts.combat_heal && profile.self_damage_sources > 0 {
        value += 6;
    }
    if card_id == CardId::SearingBlow {
        value += 6 + profile.searing_blow_upgrades * 5;
    }

    value
}

fn mismatch_penalty(profile: &DeckProfile, structure: CardStructure, facts: &CardFacts) -> i32 {
    let mut penalty = 0;

    if structure.is_strength_payoff() && profile.strength_enablers == 0 {
        penalty += 10;
    }
    if structure.is_block_payoff() && profile.block_core < 2 {
        penalty += 8;
    }
    if structure.is_exhaust_outlet() && profile.exhaust_engines == 0 && profile.exhaust_fodder == 0
    {
        penalty += 4;
    }
    if structure.is_status_engine() && profile.status_payoffs == 0 && facts.produces_status {
        penalty += 4;
    }

    penalty
}

fn duplicate_penalty(
    card_id: CardId,
    duplicate_count: i32,
    profile: &DeckProfile,
    structure: CardStructure,
    facts: &CardFacts,
) -> i32 {
    if duplicate_count <= 0 {
        return 0;
    }

    let def = get_card_definition(card_id);
    let mut penalty = duplicate_count * 6;

    if def.card_type == CardType::Power {
        penalty += duplicate_count * 10;
    }
    if structure.is_engine_piece() {
        penalty += duplicate_count * 8;
    }
    if structure.is_setup_piece() {
        penalty += duplicate_count * 4;
    }
    if structure.is_draw_core() {
        penalty -= duplicate_count * 2;
    }
    if structure.is_strength_enabler() && profile.strength_payoffs >= 2 {
        penalty -= duplicate_count * 2;
    }
    if structure.is_strength_payoff() && profile.strength_enablers >= 2 {
        penalty -= duplicate_count * 2;
    }
    if facts.self_replicating {
        penalty += duplicate_count * 12;
    }
    if cards::is_starter_basic(card_id) {
        penalty += duplicate_count * 10;
    }

    penalty.max(0)
}

fn starter_penalty(card_id: CardId) -> i32 {
    match card_id {
        CardId::Strike | CardId::StrikeG => 32,
        CardId::Defend | CardId::DefendG => 22,
        _ => 18,
    }
}

fn first_copy_presence_bonus(structure: CardStructure, facts: &CardFacts) -> i32 {
    let mut bonus = 0;
    if structure.is_engine_piece() {
        bonus += 10;
    }
    if structure.is_setup_piece() {
        bonus += 6;
    }
    if structure.is_draw_core() || facts.draws_cards {
        bonus += 6;
    }
    if facts.gains_energy {
        bonus += 6;
    }
    bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::state::run::RunState;

    #[test]
    fn starter_basics_score_below_real_payoff_cards() {
        let rs = RunState::new(1, 0, false, "Ironclad");
        assert!(score_card_offer(CardId::Offering, &rs) > score_card_offer(CardId::Strike, &rs));
        assert!(score_card_offer(CardId::ShrugItOff, &rs) > score_card_offer(CardId::Defend, &rs));
    }
}
