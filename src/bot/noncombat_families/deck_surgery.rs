use super::helpers::{
    best_bonfire_fuel_score, contains_any, count_remove_targets, count_transform_targets,
    count_upgradable_cards, curse_pressure_score, first_number,
};
use super::model::{build_noncombat_need_snapshot_for_run, NoncombatNeedSnapshot};
use crate::state::run::RunState;
use serde_json::Value;

pub(super) fn choose_best_deck_surgery_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let need = build_noncombat_need_snapshot_for_run(rs);
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
        let score = deck_surgery_option_score(rs, &need, text);
        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

pub(super) fn deck_surgery_option_score(
    rs: &RunState,
    need: &NoncombatNeedSnapshot,
    text: &str,
) -> i32 {
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
    let immediate_need_pressure =
        need.survival_pressure + need.key_urgency / 2 + need.best_upgrade_value / 5;

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
        score += 1_800 + need.purge_value * 8 + remove_targets * 180 + curse_pressure * 55;
        score += crate::bot::deck_delta_eval::compare_purge_vs_keep(rs).total * 12;
    }
    if contains_any(&lower, &["transform a card"]) {
        score += 1_400
            + transform_targets * 180
            + need.purge_value * 3 / 2
            + need.best_upgrade_value / 3;
        score += crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, 1, false).total * 10;
    }
    if contains_any(&lower, &["transform 2 cards"]) {
        score += 1_900
            + transform_targets * 240
            + need.purge_value * 2
            + need.best_upgrade_value / 2;
        score += crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, 2, false).total * 10;
    }
    if contains_any(&lower, &["upgrade a card", "upgrade 1 card", "grow"]) {
        score += 1_300 + need.best_upgrade_value * 8 + upgradable_cards * 120;
        score += crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, 1).total * 10;
    }
    if contains_any(&lower, &["upgrade 2 random cards"]) {
        score += 1_600 + need.best_upgrade_value * 5 + upgradable_cards * 100;
        score += crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, 2).total * 8;
    }
    if contains_any(&lower, &["duplicate", "copy a card"]) {
        score += 1_400
            + crate::bot::deck_delta_eval::compare_duplicate_vs_decline(rs).total * 10
            + need.long_term_meta_value / 2
            + need.best_upgrade_value / 4
            - need.purge_value / 5;
    }
    if contains_any(&lower, &["heal to full", "heal"]) {
        score += missing_hp * 35 + need.survival_pressure * 3;
    }
    if contains_any(&lower, &["sacrifice", "offer"]) {
        score += best_bonfire_fuel_score(rs) * 380;
    }
    if contains_any(&lower, &["max hp"]) && !contains_any(&lower, &["lose max hp"]) {
        score += 900;
    }

    if contains_any(&lower, &["lose max hp"]) {
        score -= if hp_ratio < 0.55 { 1_600 } else { 900 };
        score -= immediate_need_pressure / 6;
    }
    if contains_any(&lower, &["damage", "lose hp", "take "]) && !contains_any(&lower, &["heal"]) {
        score -= if hp_ratio < 0.50 { 1_800 } else { 950 };
        score -= immediate_need_pressure / 5;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use serde_json::json;

    #[test]
    fn deck_surgery_prefers_remove_when_purge_need_dominates() {
        let mut rs = RunState::new(2, 0, true, "Ironclad");
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            31_001,
        ));
        let need = build_noncombat_need_snapshot_for_run(&rs);

        assert!(
            deck_surgery_option_score(&rs, &need, "Remove a card")
                > deck_surgery_option_score(&rs, &need, "Upgrade a card")
        );
    }

    #[test]
    fn choose_best_deck_surgery_option_prefers_upgrade_when_upgrade_value_is_premium() {
        let mut rs = RunState::new(2, 0, true, "Ironclad");
        rs.current_hp = 60;
        rs.max_hp = 80;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Shockwave,
            31_002,
        ));

        let options = vec![
            json!({"text":"[Upgrade] Upgrade a card", "disabled":false}),
            json!({"text":"[Leave] Leave", "disabled":false}),
        ];

        assert_eq!(choose_best_deck_surgery_option(&rs, &options), Some(0));
    }
}
