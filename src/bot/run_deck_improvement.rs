use crate::bot::evaluator::{CardEvaluator, DeckProfile};
use crate::combat::CombatCard;
use crate::content::cards::{self, CardId, CardType};
use crate::state::run::RunState;

pub(crate) fn run_deck_value(rs: &RunState) -> i32 {
    let profile = CardEvaluator::deck_profile(rs);
    let mut score = 0;
    let mut starter_basic_count = 0;
    let mut starter_strike_count = 0;
    let mut curse_severity_total = 0;

    for card in &rs.master_deck {
        score += per_card_run_value(rs, card);
        if cards::is_starter_basic(card.id) {
            starter_basic_count += 1;
        }
        if cards::is_starter_strike(card.id) {
            starter_strike_count += 1;
        }
        curse_severity_total += curse_severity(card.id);
    }

    score += shell_completion_bonus(&profile);
    score -= shell_incompleteness_penalty(&profile);
    score -= deck_clutter_penalty(
        starter_basic_count,
        starter_strike_count,
        curse_severity_total,
    );
    score += profile.draw_sources * 10;
    score += profile.power_scalers * 6;
    score += profile.block_core.min(4) * 6;
    score += profile.attack_count.min(8) * 2;

    score
}

pub(crate) fn card_add_improvement_delta(rs: &RunState, card_id: CardId) -> i32 {
    let before = run_deck_value(rs);
    crate::state::run::with_suppressed_obtain_logs(|| {
        let mut after = rs.clone();
        after.add_card_to_deck(card_id);
        run_deck_value(&after) - before
    })
}

pub(crate) fn best_remove_improvement(rs: &RunState) -> i32 {
    best_remove_candidate(rs)
        .map(|(_, delta)| delta)
        .unwrap_or(0)
}

pub(crate) fn best_remove_uuid(rs: &RunState) -> Option<u32> {
    best_remove_candidate(rs).map(|(idx, _)| rs.master_deck[idx].uuid)
}

pub(crate) fn best_upgrade_uuid(rs: &RunState) -> Option<u32> {
    best_upgrade_candidate(rs).map(|(idx, _)| rs.master_deck[idx].uuid)
}

pub(crate) fn best_upgrade_improvement(rs: &RunState) -> i32 {
    best_upgrade_candidate(rs)
        .map(|(_, delta)| delta)
        .unwrap_or(0)
}

pub(crate) fn best_duplicate_improvement(rs: &RunState) -> i32 {
    best_duplicate_candidate(rs)
        .map(|(_, delta)| delta)
        .unwrap_or(0)
}

pub(crate) fn best_duplicate_uuid(rs: &RunState) -> Option<u32> {
    best_duplicate_candidate(rs).map(|(idx, _)| rs.master_deck[idx].uuid)
}

pub(crate) fn best_transform_improvement(
    rs: &RunState,
    count: usize,
    upgraded_context: bool,
) -> i32 {
    transform_candidates(rs, upgraded_context)
        .into_iter()
        .take(count)
        .map(|(_, delta)| delta)
        .sum()
}

pub(crate) fn best_transform_uuids(
    rs: &RunState,
    count: usize,
    upgraded_context: bool,
) -> Vec<u32> {
    transform_candidates(rs, upgraded_context)
        .into_iter()
        .take(count)
        .map(|(idx, _)| rs.master_deck[idx].uuid)
        .collect()
}

pub(crate) fn vampires_bite_exchange_value(rs: &RunState) -> i32 {
    let before = run_deck_value(rs);
    let mut after = rs.clone();
    let strike_count = after
        .master_deck
        .iter()
        .filter(|card| cards::is_starter_strike(card.id))
        .count();

    if strike_count == 0 {
        return 0;
    }

    after
        .master_deck
        .retain(|card| !cards::is_starter_strike(card.id));
    for _ in 0..strike_count {
        after
            .master_deck
            .push(CombatCard::new(CardId::Bite, next_synthetic_uuid(&after)));
    }

    run_deck_value(&after) - before
}

fn best_remove_candidate(rs: &RunState) -> Option<(usize, i32)> {
    let before = run_deck_value(rs);
    let mut working = rs.clone();
    let mut best: Option<(usize, i32)> = None;

    for idx in 0..working.master_deck.len() {
        let removed = working.master_deck.remove(idx);
        let delta = run_deck_value(&working) - before;
        working.master_deck.insert(idx, removed);
        match best {
            Some((best_idx, best_delta))
                if best_delta > delta || (best_delta == delta && best_idx <= idx) => {}
            _ => best = Some((idx, delta)),
        }
    }

    best
}

fn best_upgrade_candidate(rs: &RunState) -> Option<(usize, i32)> {
    let before = run_deck_value(rs);
    let mut working = rs.clone();
    let mut best: Option<(usize, i32)> = None;

    for idx in 0..working.master_deck.len() {
        if !is_upgradable(&working.master_deck[idx]) {
            continue;
        }
        working.master_deck[idx].upgrades += 1;
        let delta = run_deck_value(&working) - before;
        working.master_deck[idx].upgrades -= 1;
        match best {
            Some((best_idx, best_delta))
                if best_delta > delta || (best_delta == delta && best_idx <= idx) => {}
            _ => best = Some((idx, delta)),
        }
    }

    best
}

fn best_duplicate_candidate(rs: &RunState) -> Option<(usize, i32)> {
    let before = run_deck_value(rs);
    let mut working = rs.clone();
    let mut best: Option<(usize, i32)> = None;

    for idx in 0..rs.master_deck.len() {
        let mut duplicated = rs.master_deck[idx].clone();
        duplicated.uuid = next_synthetic_uuid(&working);
        working.master_deck.push(duplicated);
        let delta = run_deck_value(&working) - before;
        working.master_deck.pop();
        match best {
            Some((best_idx, best_delta))
                if best_delta > delta || (best_delta == delta && best_idx <= idx) => {}
            _ => best = Some((idx, delta)),
        }
    }

    best
}

fn transform_candidates(rs: &RunState, upgraded_context: bool) -> Vec<(usize, i32)> {
    let before = run_deck_value(rs);
    let mut working = rs.clone();
    let mut candidates = Vec::new();

    for idx in 0..working.master_deck.len() {
        let removed = working.master_deck.remove(idx);
        let remove_delta = run_deck_value(&working) - before;
        let transform_delta =
            remove_delta + transform_replacement_expectation(rs, &removed, upgraded_context);
        working.master_deck.insert(idx, removed);
        candidates.push((idx, transform_delta));
    }

    candidates.sort_by_key(|(idx, delta)| (-*delta, *idx as i32));
    candidates
}

fn per_card_run_value(rs: &RunState, card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    let mut score = CardEvaluator::evaluate_owned_card(card.id, rs);

    if cards::is_starter_strike(card.id) {
        score -= 14;
    } else if cards::is_starter_defend(card.id) {
        score -= 10;
    } else if cards::is_starter_basic(card.id) {
        score -= 8;
    }

    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        score -= 140;
    }
    score -= curse_severity(card.id) * 24;

    if card.upgrades > 0 && !matches!(def.card_type, CardType::Curse | CardType::Status) {
        score += upgrade_delta_value(card.id, card.upgrades);
    }

    score
}

fn upgrade_delta_value(card_id: CardId, upgrades: u8) -> i32 {
    let def = cards::get_card_definition(card_id);
    let upgrade_count = upgrades.max(1) as i32;
    let per_upgrade = (def.upgrade_damage as i32) * 10
        + (def.upgrade_block as i32) * 8
        + (def.upgrade_magic as i32) * 12;
    let premium = match card_id {
        CardId::Bash
        | CardId::Armaments
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::FlameBarrier
        | CardId::ShrugItOff
        | CardId::PommelStrike
        | CardId::BattleTrance
        | CardId::Offering
        | CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace
        | CardId::LimitBreak
        | CardId::BodySlam
        | CardId::TrueGrit
        | CardId::BurningPact
        | CardId::SecondWind
        | CardId::Headbutt
        | CardId::SeeingRed => 14,
        CardId::SearingBlow => 22 + upgrades as i32 * 8,
        _ => 0,
    };

    (10 + per_upgrade.max(6)) * upgrade_count + premium
}

fn shell_completion_bonus(profile: &DeckProfile) -> i32 {
    let mut bonus = 0;
    bonus += profile.strength_enablers.min(profile.strength_payoffs) * 26;
    bonus += profile.exhaust_engines.min(profile.exhaust_outlets) * 28;
    if profile.block_core >= 2 && profile.block_payoffs >= 1 {
        bonus += 36 + profile.block_payoffs.min(profile.block_core) * 12;
    }
    if profile.status_generators >= 1 && profile.status_payoffs >= 1 {
        bonus += 24;
    }
    if profile.searing_blow_count > 0 {
        bonus += 18 + profile.searing_blow_upgrades * 8;
    }
    bonus
}

fn shell_incompleteness_penalty(profile: &DeckProfile) -> i32 {
    let mut penalty = 0;
    if profile.strength_enablers > 0 && profile.strength_payoffs == 0 {
        penalty += 42 + profile.strength_enablers * 10;
    }
    if profile.strength_payoffs >= 2 && profile.strength_enablers == 0 {
        penalty += 34 + profile.strength_payoffs * 8;
    }
    if profile.exhaust_engines > 0 && profile.exhaust_outlets == 0 {
        penalty += 48 + profile.exhaust_engines * 10;
    }
    if profile.exhaust_outlets >= 2 && profile.exhaust_engines == 0 {
        penalty += 40 + profile.exhaust_outlets * 8;
    }
    if profile.block_core >= 2 && profile.block_payoffs == 0 {
        penalty += 34 + profile.block_core * 6;
    }
    if profile.status_generators > 0 && profile.status_payoffs == 0 {
        penalty += 20 + profile.status_generators * 8;
    }
    penalty
}

fn deck_clutter_penalty(starter_basics: i32, starter_strikes: i32, curse_severity: i32) -> i32 {
    starter_basics * 6 + starter_strikes.saturating_sub(2) * 4 + curse_severity * 14
}

fn curse_severity(card_id: CardId) -> i32 {
    let severity = crate::bot::evaluator::curse_remove_severity(card_id);
    if severity > 0 {
        severity
    } else if matches!(
        cards::get_card_definition(card_id).card_type,
        CardType::Curse
    ) {
        3
    } else {
        0
    }
}

fn transform_replacement_expectation(
    rs: &RunState,
    card: &CombatCard,
    upgraded_context: bool,
) -> i32 {
    let def = cards::get_card_definition(card.id);
    let owned = CardEvaluator::evaluate_owned_card(card.id, rs);
    let mut bonus = if matches!(def.card_type, CardType::Curse | CardType::Status) {
        120 + curse_severity(card.id) * 12
    } else if cards::is_starter_basic(card.id) {
        82
    } else if def.rarity == cards::CardRarity::Common && !matches!(def.card_type, CardType::Power) {
        58
    } else if owned <= 15 {
        42
    } else if owned <= 30 {
        24
    } else {
        8
    };

    if upgraded_context {
        bonus += 20;
    }

    bonus
}

fn next_synthetic_uuid(rs: &RunState) -> u32 {
    rs.master_deck
        .iter()
        .map(|card| card.uuid)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

pub(crate) fn is_upgradable(card: &CombatCard) -> bool {
    let def = cards::get_card_definition(card.id);
    card.id == CardId::SearingBlow
        || (card.upgrades == 0 && !matches!(def.card_type, CardType::Status | CardType::Curse))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with(cards: &[CardId]) -> RunState {
        let mut rs = RunState::new(17, 0, false, "Ironclad");
        rs.master_deck = cards
            .iter()
            .enumerate()
            .map(|(idx, &id)| CombatCard::new(id, idx as u32))
            .collect();
        rs
    }

    #[test]
    fn remove_improvement_is_positive_for_basic_strike_deck() {
        let rs = run_with(&[CardId::Strike, CardId::Strike, CardId::Defend, CardId::Bash]);
        assert!(best_remove_improvement(&rs) > 0);
    }

    #[test]
    fn vampires_exchange_is_positive_for_dense_strike_shell() {
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);
        assert!(vampires_bite_exchange_value(&rs) > 0);
    }

    #[test]
    fn card_add_delta_prefers_shell_completing_card() {
        let rs = run_with(&[
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
            CardId::Strike,
        ]);
        assert!(
            card_add_improvement_delta(&rs, CardId::DarkEmbrace)
                > card_add_improvement_delta(&rs, CardId::Clash)
        );
    }

    #[test]
    fn transform_upgraded_context_is_better_than_plain_transform_context() {
        let rs = run_with(&[CardId::Strike, CardId::Strike, CardId::Defend, CardId::Bash]);
        assert!(
            best_transform_improvement(&rs, 1, true) > best_transform_improvement(&rs, 1, false)
        );
    }

    #[test]
    fn double_transform_improvement_beats_single_when_two_bad_targets_exist() {
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);
        assert!(
            best_transform_improvement(&rs, 2, false) > best_transform_improvement(&rs, 1, false)
        );
    }
}
