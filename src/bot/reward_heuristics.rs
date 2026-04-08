use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::bot::evaluator::CardEvaluator;
use crate::content::cards::{self, CardId};
use crate::state::run::RunState;

const DEFAULT_PICK_RATE: f32 = 0.05;
const HISTORY_WEIGHT: f32 = 25.0;
const ACT1_FORCE_PICK_FLOOR: i32 = 16;
const ACT1_FORCE_PICK_DECK_SIZE: usize = 14;

#[derive(Debug, Clone, Copy)]
pub struct CardStatistics {
    pub card_id: CardId,
    /// Pick probability normalized to the `[0.0, 1.0]` range.
    pub pick_rate: f32,
}

#[derive(Debug, Deserialize)]
struct RawCardStatistics {
    card_id: String,
    pick_rate: String,
}

static CARD_NAME_MAP: OnceLock<HashMap<String, CardId>> = OnceLock::new();
static CARD_STATISTICS: OnceLock<HashMap<CardId, CardStatistics>> = OnceLock::new();

pub fn evaluate_reward_screen(offered_cards: &[CardId]) -> Option<usize> {
    if offered_cards.is_empty() {
        return None;
    }

    let mut best_idx = 0usize;
    let mut best_pick_rate = 0.0f32;
    let mut skip_probability = 1.0f32;

    for (idx, &card_id) in offered_cards.iter().enumerate() {
        let pick_rate = pick_probability(card_id);
        skip_probability *= 1.0 - pick_rate;

        if pick_rate > best_pick_rate {
            best_pick_rate = pick_rate;
            best_idx = idx;
        }
    }

    if skip_probability >= best_pick_rate {
        None
    } else {
        Some(best_idx)
    }
}

pub fn evaluate_reward_screen_for_run(
    offered_cards: &[CardId],
    run_state: &RunState,
) -> Option<usize> {
    if offered_cards.is_empty() {
        return None;
    }

    let profile = CardEvaluator::deck_profile(run_state);
    let mut best_idx = 0usize;
    let mut best_pick_rate = 0.0f32;
    let mut best_local_score = i32::MIN;
    let mut best_combined_score = f32::MIN;
    let mut skip_probability = 1.0f32;

    for (idx, &card_id) in offered_cards.iter().enumerate() {
        let pick_rate = pick_probability(card_id);
        let local_score = CardEvaluator::evaluate_card(card_id, run_state)
            + reward_shell_bonus(card_id, &profile);
        let combined_score = local_score as f32 + pick_rate * HISTORY_WEIGHT;

        skip_probability *= 1.0 - pick_rate;

        if combined_score > best_combined_score {
            best_idx = idx;
            best_combined_score = combined_score;
            best_pick_rate = pick_rate;
            best_local_score = local_score;
        }
    }

    if should_force_pick_in_act1(run_state) {
        return Some(best_idx);
    }

    if should_force_pick_for_shell(offered_cards, &profile) {
        return Some(best_idx);
    }

    let skip_margin = if run_state.act_num <= 1 {
        0.35
    } else if run_state.master_deck.len() <= 15 {
        0.20
    } else {
        0.08
    };

    let should_skip = best_local_score < 15 && skip_probability > best_pick_rate + skip_margin;
    if should_skip {
        None
    } else {
        Some(best_idx)
    }
}

fn reward_shell_bonus(card_id: CardId, profile: &crate::bot::evaluator::DeckProfile) -> i32 {
    match card_id {
        CardId::LimitBreak if profile.strength_enablers >= 1 => 18,
        CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm
            if profile.strength_payoffs >= 2 =>
        {
            12
        }
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel | CardId::Whirlwind
            if profile.strength_enablers >= 2 =>
        {
            8
        }
        CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
            if profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1 =>
        {
            16
        }
        CardId::SecondWind | CardId::BurningPact | CardId::SeverSoul | CardId::FiendFire
            if profile.exhaust_engines >= 2 =>
        {
            10
        }
        CardId::Barricade | CardId::Entrench if profile.block_core >= 3 => 16,
        CardId::BodySlam | CardId::FlameBarrier | CardId::Impervious
            if profile.block_payoffs >= 1 =>
        {
            10
        }
        _ => 0,
    }
}

fn should_force_pick_for_shell(
    offered_cards: &[CardId],
    profile: &crate::bot::evaluator::DeckProfile,
) -> bool {
    offered_cards
        .iter()
        .any(|&card_id| reward_shell_bonus(card_id, profile) >= 14)
}

pub fn pick_probability(card_id: CardId) -> f32 {
    card_statistics()
        .get(&card_id)
        .map(|stats| stats.pick_rate)
        .unwrap_or(DEFAULT_PICK_RATE)
}

fn should_force_pick_in_act1(run_state: &RunState) -> bool {
    run_state.act_num == 1
        && run_state.floor_num <= ACT1_FORCE_PICK_FLOOR
        && run_state.master_deck.len() <= ACT1_FORCE_PICK_DECK_SIZE
}

fn card_statistics() -> &'static HashMap<CardId, CardStatistics> {
    CARD_STATISTICS.get_or_init(|| {
        let records: Vec<RawCardStatistics> =
            serde_json::from_str(include_str!("data/card_pick_records.json"))
                .expect("card_pick_records.json must be valid JSON");
        let name_map = card_name_map();
        let mut stats = HashMap::with_capacity(records.len());

        for record in records {
            let normalized_name = normalize_card_name(&record.card_id);
            let Some(&card_id) = name_map.get(&normalized_name) else {
                continue;
            };
            let Some(pick_rate) = parse_pick_rate(&record.pick_rate) else {
                continue;
            };

            stats.insert(card_id, CardStatistics { card_id, pick_rate });
        }

        stats
    })
}

fn card_name_map() -> &'static HashMap<String, CardId> {
    CARD_NAME_MAP.get_or_init(|| {
        let java_id_map = cards::build_java_id_map();
        let mut unique_ids = HashSet::with_capacity(java_id_map.len());
        let mut map = HashMap::with_capacity(java_id_map.len() * 3);

        for (name, &card_id) in &java_id_map {
            map.insert(normalize_card_name(name), card_id);
            unique_ids.insert(card_id);
        }

        for card_id in unique_ids {
            let display_name = cards::get_card_definition(card_id).name;
            map.insert(normalize_card_name(display_name), card_id);
            map.insert(normalize_card_name(&format!("{card_id:?}")), card_id);
        }

        map.insert(normalize_card_name("J.A.X."), CardId::JAX);
        map.insert(normalize_card_name("Hand Of Greed"), CardId::HandOfGreed);
        map.insert(normalize_card_name("HandOfGreed"), CardId::HandOfGreed);

        map
    })
}

fn normalize_card_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn parse_pick_rate(value: &str) -> Option<f32> {
    value
        .parse::<f32>()
        .ok()
        .map(|percent| (percent / 100.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CombatCard;

    fn sample_run_state() -> RunState {
        let mut rs = RunState::new(123, 0, false, "Ironclad");
        rs.master_deck = (0..10)
            .map(|i| CombatCard::new(CardId::Strike, i))
            .collect();
        rs
    }

    #[test]
    fn high_pick_rate_card_beats_skip() {
        let offered = [CardId::Offering, CardId::Clash, CardId::Clothesline];
        assert_eq!(evaluate_reward_screen(&offered), Some(0));
    }

    #[test]
    fn low_pick_rate_bundle_prefers_skip() {
        let offered = [CardId::Clash, CardId::Clothesline, CardId::IronWave];
        assert_eq!(evaluate_reward_screen(&offered), None);
    }

    #[test]
    fn missing_cards_fall_back_to_default_pick_rate() {
        assert_eq!(pick_probability(CardId::Burn), DEFAULT_PICK_RATE);
    }

    #[test]
    fn special_card_names_are_mapped() {
        let stats = card_statistics();
        assert!(stats
            .values()
            .any(|record| record.card_id == CardId::HandOfGreed));
    }

    #[test]
    fn act1_early_game_forces_a_pick() {
        let mut rs = sample_run_state();
        rs.act_num = 1;
        rs.floor_num = 5;

        let offered = [CardId::Clash, CardId::WildStrike, CardId::Havoc];
        assert_eq!(evaluate_reward_screen_for_run(&offered, &rs), Some(2));
    }

    #[test]
    fn later_run_can_still_skip_bad_rewards() {
        let mut rs = sample_run_state();
        rs.act_num = 2;
        rs.floor_num = 25;

        let offered = [CardId::Clash, CardId::WildStrike, CardId::Havoc];
        assert_eq!(evaluate_reward_screen_for_run(&offered, &rs), None);
    }

    #[test]
    fn shell_completion_card_is_not_skipped_late() {
        let mut rs = sample_run_state();
        rs.act_num = 2;
        rs.floor_num = 25;
        rs.master_deck = vec![
            CombatCard::new(CardId::Barricade, 1),
            CombatCard::new(CardId::ShrugItOff, 2),
            CombatCard::new(CardId::FlameBarrier, 3),
            CombatCard::new(CardId::Defend, 4),
            CombatCard::new(CardId::Defend, 5),
        ];

        let offered = [CardId::Entrench, CardId::Clash, CardId::Havoc];
        assert_eq!(evaluate_reward_screen_for_run(&offered, &rs), Some(0));
    }
}
