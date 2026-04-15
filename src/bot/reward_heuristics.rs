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
    /// Pick probability normalized to the `[0.0, 1.0]` range.
    pub pick_rate: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RewardCardScore {
    pub card_id: CardId,
    pub pick_rate: f32,
    pub local_score: i32,
    pub delta_suite: crate::bot::encounter_suite::EncounterSuiteId,
    pub delta_prior: i32,
    pub delta_bias: i32,
    pub delta_rollout: i32,
    pub delta_context: i32,
    pub delta_context_rationale_key: Option<&'static str>,
    pub delta_rule_context_summary: Option<&'static str>,
    pub delta_score: i32,
    pub combined_score: f32,
}

#[derive(Debug, Clone)]
pub struct RewardScreenEvaluation {
    pub offered_cards: Vec<RewardCardScore>,
    pub recommended_choice: Option<usize>,
    pub best_pick_rate: f32,
    pub best_local_score: i32,
    pub best_combined_score: f32,
    pub skip_probability: f32,
    pub skip_margin: f32,
    pub force_pick_in_act1: bool,
    pub force_pick_for_shell: bool,
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
    evaluate_reward_screen_for_run_detailed(offered_cards, run_state).recommended_choice
}

pub fn evaluate_reward_screen_for_run_detailed(
    offered_cards: &[CardId],
    run_state: &RunState,
) -> RewardScreenEvaluation {
    if offered_cards.is_empty() {
        return RewardScreenEvaluation {
            offered_cards: Vec::new(),
            recommended_choice: None,
            best_pick_rate: 0.0,
            best_local_score: i32::MIN,
            best_combined_score: f32::MIN,
            skip_probability: 1.0,
            skip_margin: skip_margin_for_run(run_state),
            force_pick_in_act1: false,
            force_pick_for_shell: false,
        };
    }

    let profile = CardEvaluator::deck_profile(run_state);
    let mut best_idx = 0usize;
    let mut best_pick_rate = 0.0f32;
    let mut best_local_score = i32::MIN;
    let mut best_combined_score = f32::MIN;
    let mut skip_probability = 1.0f32;
    let mut scored_cards = Vec::with_capacity(offered_cards.len());

    for (idx, &card_id) in offered_cards.iter().enumerate() {
        let pick_rate = pick_probability(card_id);
        let delta = crate::bot::deck_delta_eval::compare_pick_vs_skip(run_state, card_id);
        let local_score =
            adjusted_reward_local_score(card_id, run_state, &profile) + delta.total.clamp(-20, 36);
        let combined_score = local_score as f32 + pick_rate * HISTORY_WEIGHT;

        skip_probability *= 1.0 - pick_rate;
        scored_cards.push(RewardCardScore {
            card_id,
            pick_rate,
            local_score,
            delta_suite: delta.suite,
            delta_prior: delta.prior_delta,
            delta_bias: delta.suite_bias,
            delta_rollout: delta.rollout_delta,
            delta_context: delta.context_delta,
            delta_context_rationale_key: delta.context_rationale_key,
            delta_rule_context_summary: delta.rule_context_summary,
            delta_score: delta.total,
            combined_score,
        });

        if combined_score > best_combined_score {
            best_idx = idx;
            best_combined_score = combined_score;
            best_pick_rate = pick_rate;
            best_local_score = local_score;
        }
    }

    let force_pick_in_act1 = should_force_pick_in_act1(run_state);
    let force_pick_for_shell = should_force_pick_for_shell(offered_cards, &profile);
    let skip_margin = skip_margin_for_run(run_state);
    let best_card_id = offered_cards[best_idx];

    let recommended_choice = if force_pick_in_act1 || force_pick_for_shell {
        Some(best_idx)
    } else {
        let should_skip = should_skip_reward(
            run_state,
            &profile,
            best_card_id,
            best_local_score,
            skip_probability,
            best_pick_rate,
            skip_margin,
        );
        if should_skip {
            None
        } else {
            Some(best_idx)
        }
    };

    RewardScreenEvaluation {
        offered_cards: scored_cards,
        recommended_choice,
        best_pick_rate,
        best_local_score,
        best_combined_score,
        skip_probability,
        skip_margin,
        force_pick_in_act1,
        force_pick_for_shell,
    }
}

fn adjusted_reward_local_score(
    card_id: CardId,
    run_state: &RunState,
    profile: &crate::bot::evaluator::DeckProfile,
) -> i32 {
    let raw = CardEvaluator::evaluate_card(card_id, run_state);
    let capped = if raw < -200 { -120 } else { raw };
    capped
        + reward_shell_bonus(card_id, profile)
        + reward_stage_adjustment(card_id, run_state, profile)
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
        CardId::DarkEmbrace
            if profile.exhaust_engines >= 1
                || (profile.exhaust_outlets >= 1 && profile.draw_sources >= 1) =>
        {
            14
        }
        CardId::SecondWind | CardId::BurningPact | CardId::SeverSoul | CardId::FiendFire
            if profile.exhaust_engines >= 2 =>
        {
            10
        }
        CardId::BurningPact
            if profile.exhaust_engines >= 1
                || (profile.exhaust_outlets >= 1 && profile.exhaust_fodder >= 1) =>
        {
            14
        }
        CardId::Offering if profile.exhaust_engines >= 1 || profile.draw_sources >= 2 => 10,
        CardId::Armaments
            if profile.power_scalers >= 1
                || profile.block_core >= 2
                || (profile.exhaust_engines >= 1 && profile.draw_sources >= 1) =>
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

fn reward_stage_adjustment(
    card_id: CardId,
    run_state: &RunState,
    profile: &crate::bot::evaluator::DeckProfile,
) -> i32 {
    let late_game = run_state.act_num >= 2;
    let no_strength_shell = profile.strength_enablers == 0;
    let larger_deck = run_state.master_deck.len() >= 16;

    let mut adj = 0;

    match card_id {
        CardId::Warcry => {
            adj += 12;
            if late_game {
                adj += 6;
            }
            if profile.draw_sources >= 1 {
                adj += 3;
            }
        }
        CardId::SecondWind => {
            if run_state.act_num == 1 && run_state.master_deck.len() <= 12 {
                adj += 10;
            }
            if profile.exhaust_engines >= 1 || profile.status_generators >= 1 {
                adj += 6;
            }
        }
        CardId::BurningPact => {
            if profile.exhaust_engines >= 1 || profile.exhaust_fodder >= 1 {
                adj += 10;
            }
            if profile.draw_sources >= 2 {
                adj += 4;
            }
        }
        CardId::DarkEmbrace => {
            if profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1 {
                adj += 10;
            }
            if profile.draw_sources >= 2 {
                adj += 4;
            }
        }
        CardId::Offering => {
            if profile.exhaust_engines >= 1 || profile.power_scalers >= 1 {
                adj += 6;
            }
        }
        CardId::Armaments => {
            if profile.power_scalers >= 1 || profile.block_core >= 2 {
                adj += 6;
            }
        }
        CardId::FireBreathing => {
            if profile.status_generators >= 1 {
                adj += 10;
            } else if run_state.act_num == 1 {
                adj += 4;
            }
        }
        CardId::Havoc => {
            if run_state.act_num == 1 && run_state.floor_num <= 10 {
                adj += 26;
            } else if late_game {
                adj -= 10;
            }
        }
        CardId::WildStrike | CardId::Clash => {
            if late_game {
                adj -= if larger_deck { 24 } else { 18 };
            }
        }
        CardId::TwinStrike => {
            if late_game && no_strength_shell {
                adj -= 16;
            }
        }
        CardId::SwordBoomerang => {
            if late_game && no_strength_shell {
                adj -= 18;
            }
        }
        CardId::HeavyBlade => {
            if no_strength_shell {
                adj -= if late_game { 24 } else { 14 };
            }
        }
        CardId::Clothesline => {
            if late_game {
                adj -= 14;
            }
        }
        CardId::Headbutt => {
            if late_game && larger_deck {
                adj -= 12;
            }
        }
        CardId::Cleave => {
            if late_game && larger_deck {
                adj -= 24;
            }
        }
        CardId::PerfectedStrike => {
            if late_game {
                adj -= 12;
            }
        }
        CardId::IronWave => {
            if late_game && larger_deck {
                adj -= 22;
            }
        }
        _ => {}
    }

    adj
}

fn should_skip_reward(
    run_state: &RunState,
    profile: &crate::bot::evaluator::DeckProfile,
    best_card_id: CardId,
    best_local_score: i32,
    skip_probability: f32,
    best_pick_rate: f32,
    skip_margin: f32,
) -> bool {
    if best_local_score < 15 && skip_probability > best_pick_rate + skip_margin {
        return true;
    }

    let late_game = run_state.act_num >= 2;
    let mediocre_attack = matches!(
        best_card_id,
        CardId::IronWave
            | CardId::SwordBoomerang
            | CardId::Cleave
            | CardId::Headbutt
            | CardId::Clothesline
            | CardId::HeavyBlade
            | CardId::TwinStrike
            | CardId::PerfectedStrike
    );
    let low_quality_bundle_card = matches!(
        best_card_id,
        CardId::Clash
            | CardId::WildStrike
            | CardId::Havoc
            | CardId::IronWave
            | CardId::SwordBoomerang
            | CardId::Cleave
            | CardId::Clothesline
            | CardId::HeavyBlade
            | CardId::TwinStrike
            | CardId::PerfectedStrike
    );
    let no_strength_shell = profile.strength_enablers == 0;

    if late_game && low_quality_bundle_card && best_local_score < 66 && skip_probability > 0.55 {
        return true;
    }

    late_game
        && mediocre_attack
        && (best_card_id != CardId::Headbutt || run_state.master_deck.len() >= 16)
        && (best_card_id != CardId::HeavyBlade || no_strength_shell)
        && (best_card_id != CardId::TwinStrike || no_strength_shell)
        && (best_card_id != CardId::SwordBoomerang || no_strength_shell)
        && best_local_score < 58
        && skip_probability > 0.60
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

fn skip_margin_for_run(run_state: &RunState) -> f32 {
    if run_state.act_num <= 1 {
        0.35
    } else if run_state.master_deck.len() <= 15 {
        0.20
    } else {
        0.08
    }
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

            stats.insert(card_id, CardStatistics { pick_rate });
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
