use super::helpers::{
    best_bonfire_fuel_score, contains_any, count_remove_targets, count_transform_targets,
    count_upgradable_cards, curse_pressure_score, first_number,
};
use crate::state::run::RunState;
use serde_json::Value;

pub(super) fn choose_best_deck_surgery_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;

    for (idx, option) in options.iter().enumerate() {
        if option
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }

        let text = super::helpers::option_text(option);
        let score = deck_surgery_option_score(rs, text);
        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

pub(super) fn deck_surgery_option_score(rs: &RunState, text: &str) -> i32 {
    let lower = text.to_ascii_lowercase();
    if contains_any(&lower, &["leave", "proceed"]) {
        return -100;
    }

    let remove_targets = count_remove_targets(rs);
    let transform_targets = count_transform_targets(rs);
    let upgradable_cards = count_upgradable_cards(rs);
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let missing_hp = rs.max_hp - rs.current_hp;
    let curse_pressure = curse_pressure_score(rs);

    let mut score = 0;

    if contains_any(
        &lower,
        &[
            "remove a card",
            "remove 1 card",
            "purify",
            "forget",
            "simplicity",
        ],
    ) {
        score += 3_000 + remove_targets * 420 + curse_pressure * 55;
    }
    if contains_any(&lower, &["transform a card"]) {
        score += 2_200 + transform_targets * 320;
    }
    if contains_any(&lower, &["transform 2 cards"]) {
        score += 2_800 + transform_targets * 450;
    }
    if contains_any(&lower, &["upgrade a card", "upgrade 1 card", "grow"]) {
        score += 2_000 + upgradable_cards * 280;
    }
    if contains_any(&lower, &["upgrade 2 random cards"]) {
        score += 2_300 + upgradable_cards * 220;
    }
    if contains_any(&lower, &["heal to full", "heal"]) {
        score += missing_hp * 35;
    }
    if contains_any(&lower, &["sacrifice", "offer"]) {
        score += best_bonfire_fuel_score(rs) * 380;
    }
    if contains_any(&lower, &["max hp"]) && !contains_any(&lower, &["lose max hp"]) {
        score += 900;
    }

    if contains_any(&lower, &["lose max hp"]) {
        score -= if hp_ratio < 0.55 { 1_600 } else { 900 };
    }
    if contains_any(&lower, &["damage", "lose hp", "take "]) && !contains_any(&lower, &["heal"]) {
        score -= if hp_ratio < 0.50 { 1_800 } else { 950 };
    }
    if contains_any(
        &lower,
        &[
            "curse",
            "regret",
            "writhe",
            "injury",
            "doubt",
            "parasite",
            "pain",
            "normality",
            "decay",
        ],
    ) {
        score -= 3_600 + curse_pressure * 40;
        if contains_any(&lower, &["parasite", "pain", "normality"]) {
            score -= 1_800;
        }
    }
    if contains_any(&lower, &["gold"]) {
        score -= first_number(&lower) * 18;
    }

    score
}
