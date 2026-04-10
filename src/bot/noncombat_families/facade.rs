use super::deck_surgery::{choose_best_deck_surgery_option, deck_surgery_option_score};
use super::helpers::*;
use crate::state::run::RunState;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EventCostClass {
    None = 0,
    MinorHp = 1,
    MaxHp = 2,
    Curse = 3,
}

pub(crate) fn choose_event_choice(
    gs: &Value,
    rs: &RunState,
    choice_list: &[&str],
) -> Option<usize> {
    let screen_state = gs.get("screen_state")?;
    let event_id = screen_state
        .get("event_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let event_name = screen_state
        .get("event_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let options = screen_state.get("options").and_then(|v| v.as_array())?;

    if let Some(idx) = choose_event_specific(event_id, event_name, rs, options) {
        return Some(idx);
    }

    choose_by_cost_family(options, choice_list)
}

fn choose_event_specific(
    event_id: &str,
    event_name: &str,
    rs: &RunState,
    options: &[Value],
) -> Option<usize> {
    if event_id.eq_ignore_ascii_case("Neow Event") || event_name.eq_ignore_ascii_case("Neow") {
        return choose_neow_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Golden Idol")
        || event_name.eq_ignore_ascii_case("Golden Idol")
    {
        return choose_golden_idol_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Big Fish") || event_name.eq_ignore_ascii_case("Big Fish") {
        return choose_big_fish_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Woman in Blue")
        || event_name.eq_ignore_ascii_case("Woman in Blue")
    {
        return choose_woman_in_blue_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Cleric") || event_name.eq_ignore_ascii_case("Cleric") {
        return choose_cleric_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Golden Shrine")
        || event_name.eq_ignore_ascii_case("Golden Shrine")
    {
        return choose_golden_shrine_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Cursed Tome")
        || event_name.eq_ignore_ascii_case("Cursed Tome")
    {
        return choose_cursed_tome_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Forgotten Altar")
        || event_name.eq_ignore_ascii_case("Forgotten Altar")
    {
        return choose_forgotten_altar_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Shining Light")
        || event_name.eq_ignore_ascii_case("Shining Light")
    {
        return choose_shining_light_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Living Wall")
        || event_name.eq_ignore_ascii_case("Living Wall")
    {
        return choose_living_wall_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Designer") || event_name.eq_ignore_ascii_case("Designer") {
        return choose_designer_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Beggar") || event_name.eq_ignore_ascii_case("Beggar") {
        return choose_beggar_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Moai Head") || event_name.eq_ignore_ascii_case("Moai Head") {
        return choose_moai_head_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Transmogrifier")
        || event_name.eq_ignore_ascii_case("Transmogrifier")
    {
        return choose_transmogrifier_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Purification Shrine")
        || event_name.eq_ignore_ascii_case("Purification Shrine")
    {
        return choose_purification_shrine_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Upgrade Shrine")
        || event_name.eq_ignore_ascii_case("Upgrade Shrine")
    {
        return choose_upgrade_shrine_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Back to Basics")
        || event_name.eq_ignore_ascii_case("Back to Basics")
    {
        return choose_back_to_basics_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("We Meet Again")
        || event_name.eq_ignore_ascii_case("We Meet Again")
    {
        return choose_we_meet_again_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Note For Yourself")
        || event_name.eq_ignore_ascii_case("Note For Yourself")
    {
        return choose_note_for_yourself_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Drug Dealer")
        || event_name.eq_ignore_ascii_case("Drug Dealer")
    {
        return choose_drug_dealer_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Vampires") || event_name.eq_ignore_ascii_case("Vampires") {
        return choose_vampires_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Ghosts") || event_name.eq_ignore_ascii_case("Ghosts") {
        return choose_ghosts_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Winding Halls")
        || event_name.eq_ignore_ascii_case("Winding Halls")
    {
        return choose_winding_halls_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Golden Wing")
        || event_name.eq_ignore_ascii_case("Golden Wing")
    {
        return choose_golden_wing_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Face Trader")
        || event_name.eq_ignore_ascii_case("Face Trader")
    {
        return choose_face_trader_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Scrap Ooze") || event_name.eq_ignore_ascii_case("Scrap Ooze")
    {
        return choose_scrap_ooze_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Mushrooms") || event_name.eq_ignore_ascii_case("Mushrooms") {
        return choose_mushrooms_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Dead Adventurer")
        || event_name.eq_ignore_ascii_case("Dead Adventurer")
    {
        return choose_dead_adventurer_option(rs, options);
    }
    if event_id.eq_ignore_ascii_case("Bonfire Spirits")
        || event_name.eq_ignore_ascii_case("Bonfire Spirits")
        || event_id.eq_ignore_ascii_case("Bonfire Elementals")
        || event_name.eq_ignore_ascii_case("Bonfire Elementals")
    {
        return choose_bonfire_option(rs, options);
    }

    None
}

fn choose_neow_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    if labels
        .iter()
        .all(|t| contains_any(t, &["proceed", "talk", "leave"]))
    {
        return Some(0);
    }

    let current_gold = rs.gold.max(0);
    let hp_bonus = ((rs.max_hp as f32) * 0.1) as i32;
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let curse_count = count_curses(rs);

    let mut best: Option<(usize, i32)> = None;
    for (idx, label) in labels.iter().enumerate() {
        let lower = label.to_ascii_lowercase();
        let mut score = 0;

        if lower.contains("remove 2 cards") {
            score += 5_200;
        } else if lower.contains("remove a card") {
            score += 4_200;
        } else if lower.contains("transform 2 cards") {
            score += 4_100;
        } else if lower.contains("transform a card") {
            score += 3_400;
        } else if lower.contains("random boss relic") {
            score += 4_000;
        } else if lower.contains("rare relic") {
            score += 3_700;
        } else if lower.contains("common relic") {
            score += 2_300;
        } else if lower.contains("250 gold") {
            score += 3_000;
        } else if lower.contains("100 gold") {
            score += 1_500;
        } else if lower.contains("rare card") {
            score += 2_100;
        } else if lower.contains("colorless") {
            score += if lower.contains("rare") { 1_900 } else { 1_300 };
        } else if lower.contains("upgrade a card") {
            score += 3_000;
        } else if lower.contains("max hp") {
            score += if lower.contains("+20") || lower.contains("20%") {
                hp_bonus * 2 * 220
            } else {
                hp_bonus * 220
            };
        } else if lower.contains("100 gold") {
            score += 1_500;
        } else if lower.contains("next three combats") {
            score += 900;
        } else if lower.contains("3 random potions") {
            score += 700;
        }

        if lower.contains("lose all gold") {
            score -= current_gold * 18;
        }
        if lower.contains("lose your starting relic") {
            score -= 1_800;
        }
        if lower.contains("obtain a curse") || lower.contains("curse") {
            score -= 4_000 + curse_count * 700;
        }
        if lower.contains("lose ") && lower.contains("max hp") && !lower.contains("gain") {
            score -= hp_bonus.max(1) * 260;
        }
        if lower.contains("take ") && lower.contains("damage") {
            score -= if hp_ratio < 0.50 { 4_000 } else { 1_800 };
        }

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_golden_idol_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();

    if labels
        .iter()
        .any(|t| contains_any(t, &["run", "fight", "lose max hp"]))
    {
        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let run_idx = labels
            .iter()
            .position(|t| contains_any(t, &["run", "injury", "curse"]));
        let fight_idx = labels
            .iter()
            .position(|t| contains_any(t, &["fight", "take", "damage"]));
        let max_hp_idx = labels
            .iter()
            .position(|t| contains_any(t, &["lose max hp", "max hp"]));

        if hp_ratio >= 0.70 {
            fight_idx.or(max_hp_idx).or(run_idx)
        } else {
            max_hp_idx.or(fight_idx).or(run_idx)
        }
    } else if labels
        .iter()
        .any(|t| contains_any(t, &["take", "obtain golden idol"]))
    {
        labels
            .iter()
            .position(|t| contains_any(t, &["take", "obtain golden idol"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        None
    }
}

fn choose_big_fish_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let missing_hp = rs.max_hp - rs.current_hp;
    let tractability = curse_tractability_score(rs);
    let curse_pressure = curse_pressure_score(rs);

    let mut best: Option<(usize, i32)> = None;
    for (idx, label) in labels.iter().enumerate() {
        let mut score = 0;
        if contains_any(label, &["banana", "heal"]) {
            let heal_gain = missing_hp.min(26).max(0);
            score += heal_gain * 52;
            if hp_ratio < 0.40 {
                score += 650;
            } else if hp_ratio < 0.55 {
                score += 280;
            }
        } else if contains_any(label, &["donut", "max hp"]) {
            score += 1_750;
            if hp_ratio < 0.55 {
                score += 140;
            }
        } else if contains_any(label, &["box", "relic"]) {
            score += 1_950;
            score += tractability * 380;
            score -= 1_050 + curse_pressure * 45;
            if hp_ratio < 0.40 {
                score -= 450;
            }
        }

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_woman_in_blue_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let empty_slots = rs.potions.iter().filter(|p| p.is_none()).count() as i32;
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;

    if empty_slots >= 2 && rs.gold >= 30 {
        labels
            .iter()
            .position(|t| contains_any(t, &["2 potions", "lose 30 gold"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["1 potion", "lose 20 gold"]))
            })
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else if empty_slots >= 1 && rs.gold >= 20 {
        labels
            .iter()
            .position(|t| contains_any(t, &["1 potion", "lose 20 gold"]))
            .or_else(|| {
                if hp_ratio > 0.70 {
                    labels
                        .iter()
                        .position(|t| contains_any(t, &["2 potions", "lose 30 gold"]))
                } else {
                    None
                }
            })
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        labels.iter().position(|t| contains_any(t, &["leave"]))
    }
}

fn choose_cleric_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let missing_hp = rs.max_hp - rs.current_hp;

    let mut best: Option<(usize, i32)> = None;
    for (idx, label) in labels.iter().enumerate() {
        let mut score = if contains_any(label, &["leave"]) {
            -100
        } else {
            0
        };
        if contains_any(label, &["purify", "remove a card"]) {
            score += generic_remove_value(rs);
            score -= first_number(label) * 15;
        } else if contains_any(label, &["heal"]) {
            let heal_gain = ((rs.max_hp as f32) * 0.25).round() as i32;
            score += missing_hp.min(heal_gain.max(0)) * 45;
            if hp_ratio < 0.40 {
                score += 900;
            } else if hp_ratio < 0.55 {
                score += 260;
            }
            score -= first_number(label) * 14;
        }

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_golden_wing_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;

    if hp_ratio >= 0.45 {
        labels
            .iter()
            .position(|t| contains_any(t, &["remove a card"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["attack", "gold"]))
            })
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["attack", "gold"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["remove a card"]))
            })
    }
}

fn choose_golden_shrine_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let tractability = curse_tractability_score(rs);
    let curse_pressure = curse_pressure_score(rs);
    let shop_bonus = nearby_shop_conversion_bonus(rs);
    let mut best: Option<(usize, i32)> = None;

    for (idx, label) in labels.iter().enumerate() {
        let mut score = if contains_any(label, &["leave"]) {
            -100
        } else {
            0
        };
        if contains_any(label, &["pray"]) {
            score += first_number(label).max(50) * 15;
        } else if contains_any(label, &["desecrate", "curse"]) {
            score += first_number(label).max(200) * 11;
            score += tractability * 320 + shop_bonus;
            score -= 1_650 + curse_pressure * 40;
        }

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_face_trader_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;

    if hp_ratio > 0.25 {
        labels
            .iter()
            .position(|t| contains_any(t, &["trade", "face relic"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["touch", "gain", "gold"]))
            })
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["trade", "face relic"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    }
}

fn choose_forgotten_altar_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    if rs
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::GoldenIdol)
    {
        return labels
            .iter()
            .position(|t| contains_any(t, &["offer", "bloody idol"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["pray"])))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["desecrate"])));
    }

    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    if hp_ratio >= 0.65 {
        labels
            .iter()
            .position(|t| contains_any(t, &["pray", "gain 5 max hp"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["desecrate"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["desecrate"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["pray"])))
    }
}

fn choose_shining_light_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let upgradable_cards = count_upgradable_cards(rs);
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let enter_idx = labels
        .iter()
        .position(|t| contains_any(t, &["enter the light", "upgrade 2 random cards"]));
    let leave_idx = labels.iter().position(|t| contains_any(t, &["leave"]));

    if upgradable_cards >= 2 && hp_ratio >= 0.55 {
        enter_idx.or(leave_idx)
    } else if upgradable_cards >= 1 && hp_ratio >= 0.75 {
        enter_idx.or(leave_idx)
    } else {
        leave_idx.or(enter_idx)
    }
}

fn choose_living_wall_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    choose_best_deck_surgery_option(rs, options)
}

fn choose_designer_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let leave_like = labels
        .iter()
        .all(|t| contains_any(t, &["proceed", "leave"]));
    if leave_like {
        return Some(0);
    }

    let mut best: Option<(usize, i32)> = None;
    for (idx, label) in labels.iter().enumerate() {
        if options[idx]
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }

        let mut score = deck_surgery_option_score(rs, label);
        let gold_cost = first_number(label);
        if gold_cost > 0 {
            score -= gold_cost * 20;
        }
        if contains_any(label, &["remove 1 card + upgrade 1 random"]) {
            score += 1_600;
        } else if contains_any(label, &["transform 2 cards"]) {
            score += 180;
        } else if contains_any(label, &["upgrade 2 random cards"]) {
            score += 120;
        }
        if contains_any(label, &["lose", "hp", "punch"]) {
            score -= 1_500;
        }

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_beggar_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let donate_idx = labels
        .iter()
        .position(|t| contains_any(t, &["donate", "remove a card"]));
    let leave_idx = labels.iter().position(|t| contains_any(t, &["leave"]));
    let gold_cost = labels
        .get(donate_idx.unwrap_or(usize::MAX))
        .map(|t| first_number(t))
        .unwrap_or(75);

    if count_remove_targets(rs) >= 1 && rs.gold >= gold_cost + 40 {
        donate_idx.or(leave_idx)
    } else {
        leave_idx.or(donate_idx)
    }
}

fn choose_moai_head_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let trade_idx = labels
        .iter()
        .position(|t| contains_any(t, &["trade", "golden idol", "333 gold"]));
    let enter_idx = labels
        .iter()
        .position(|t| contains_any(t, &["enter", "heal to full"]));
    let leave_idx = labels.iter().position(|t| contains_any(t, &["leave"]));

    if rs
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::GoldenIdol)
    {
        return trade_idx.or(enter_idx).or(leave_idx);
    }

    let hp_loss_pct = if rs.ascension_level >= 15 {
        0.18
    } else {
        0.125
    };
    let hp_loss = (rs.max_hp as f32 * hp_loss_pct).round() as i32;
    let missing_hp = rs.max_hp - rs.current_hp;
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let healing_gain = missing_hp - hp_loss;
    let should_heal = (hp_ratio <= 0.45 && healing_gain > 8) || healing_gain > rs.max_hp / 3;

    if should_heal {
        enter_idx.or(leave_idx)
    } else {
        leave_idx.or(enter_idx)
    }
}

fn choose_transmogrifier_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    choose_best_deck_surgery_option(rs, options)
}

fn choose_purification_shrine_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    choose_best_deck_surgery_option(rs, options)
}

fn choose_upgrade_shrine_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    choose_best_deck_surgery_option(rs, options)
}

fn choose_back_to_basics_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    choose_best_deck_surgery_option(rs, options)
}

fn choose_we_meet_again_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let potion_idx = labels
        .iter()
        .position(|t| contains_any(t, &["give potion"]));
    let gold_idx = labels.iter().position(|t| contains_any(t, &["give gold"]));
    let card_idx = labels.iter().position(|t| contains_any(t, &["give card"]));
    let attack_idx = labels.iter().position(|t| contains_any(t, &["attack"]));

    let card_give_value = best_we_meet_again_card_give_score(rs);
    let potion_give_value = best_we_meet_again_potion_give_score(rs);
    let gold_cost = gold_idx
        .and_then(|idx| labels.get(idx))
        .map(|t| first_number(t))
        .unwrap_or(0);
    let gold_value = 1_450 - gold_cost * 18 - nearby_shop_conversion_bonus(rs);

    let mut best: Option<(usize, i32)> = None;
    for (idx, _) in labels.iter().enumerate() {
        if options[idx]
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let score = if Some(idx) == card_idx {
            card_give_value
        } else if Some(idx) == potion_idx {
            potion_give_value
        } else if Some(idx) == gold_idx {
            gold_value
        } else if Some(idx) == attack_idx {
            0
        } else {
            -100
        };

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_note_for_yourself_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let take_idx = labels
        .iter()
        .position(|t| contains_any(t, &["take card", "obtain", "take"]));
    let ignore_idx = labels.iter().position(|t| contains_any(t, &["ignore"]));

    let take_score = take_idx
        .and_then(|idx| labels.get(idx))
        .and_then(|text| parse_note_card(text))
        .map(|card_id| crate::bot::evaluator::CardEvaluator::evaluate_card(card_id, rs))
        .unwrap_or(-50);

    if take_score >= 35 {
        take_idx.or(ignore_idx)
    } else {
        ignore_idx.or(take_idx)
    }
}

fn choose_drug_dealer_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let jax_idx = labels
        .iter()
        .position(|t| contains_any(t, &["j.a.x", "jax"]));
    let transform_idx = labels
        .iter()
        .position(|t| contains_any(t, &["transform 2 cards", "test subject"]));
    let relic_idx = labels
        .iter()
        .position(|t| contains_any(t, &["mutagenic strength", "inject mutagens"]));

    let transform_targets = count_transform_targets(rs);
    let has_strength_scaling = rs.master_deck.iter().any(|card| {
        matches!(
            card.id,
            crate::content::cards::CardId::HeavyBlade
                | crate::content::cards::CardId::SwordBoomerang
                | crate::content::cards::CardId::TwinStrike
                | crate::content::cards::CardId::Pummel
                | crate::content::cards::CardId::Reaper
        )
    });

    let mut best: Option<(usize, i32)> = None;
    for (idx, _) in labels.iter().enumerate() {
        if options[idx]
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let score = if Some(idx) == transform_idx {
            1_900 + transform_targets * 320
        } else if Some(idx) == relic_idx {
            if has_strength_scaling {
                2_400
            } else {
                2_050
            }
        } else if Some(idx) == jax_idx {
            if has_strength_scaling {
                1_850
            } else {
                1_350
            }
        } else {
            -100
        };

        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((idx, score)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn choose_cursed_tome_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    if labels.iter().any(|t| contains_any(t, &["read"])) {
        return Some(0);
    }
    if labels.iter().all(|t| contains_any(t, &["continue"])) {
        return Some(0);
    }
    if labels
        .iter()
        .any(|t| contains_any(t, &["take the book", "book relic"]))
    {
        let final_cost = if rs.ascension_level >= 15 { 15 } else { 10 };
        let safe_to_pay = rs.current_hp > final_cost + 12;
        if safe_to_pay {
            labels
                .iter()
                .position(|t| contains_any(t, &["take the book", "book relic"]))
                .or_else(|| {
                    labels
                        .iter()
                        .position(|t| contains_any(t, &["stop reading"]))
                })
        } else {
            labels
                .iter()
                .position(|t| contains_any(t, &["stop reading"]))
                .or_else(|| {
                    labels
                        .iter()
                        .position(|t| contains_any(t, &["take the book"]))
                })
        }
    } else {
        None
    }
}

fn choose_by_cost_family(options: &[Value], choice_list: &[&str]) -> Option<usize> {
    let mut best: Option<(usize, EventCostClass)> = None;

    for (idx, option) in options.iter().enumerate() {
        if option
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let text = option_text(option);
        let cost = classify_event_cost(text, choice_list.get(idx).copied().unwrap_or(""));
        match best {
            Some((_, best_cost)) if cost >= best_cost => {}
            _ => best = Some((idx, cost)),
        }
    }

    best.map(|(idx, _)| idx)
}

fn classify_event_cost(text: &str, choice_name: &str) -> EventCostClass {
    let merged = format!("{} {}", text, choice_name).to_ascii_lowercase();
    if contains_any(
        &merged,
        &[
            "curse", "injury", "doubt", "regret", "pain", "writhe", "parasite",
        ],
    ) {
        EventCostClass::Curse
    } else if contains_any(&merged, &["lose max hp", "max hp"]) {
        EventCostClass::MaxHp
    } else if contains_any(&merged, &["damage", "lose hp", "hp loss", "take "]) {
        EventCostClass::MinorHp
    } else {
        EventCostClass::None
    }
}

fn choose_vampires_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    if rs
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::BloodVial)
    {
        return labels
            .iter()
            .position(|t| contains_any(t, &["give vial", "blood vial"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["accept"])))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["refuse", "leave"]))
            });
    }

    let strike_count = count_starter_strikes(rs);
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    if strike_count >= 4 && hp_ratio >= 0.60 {
        labels
            .iter()
            .position(|t| contains_any(t, &["accept", "replace all strikes with 5 bites"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["refuse", "leave"]))
            })
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["refuse", "leave"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["accept"])))
    }
}

fn choose_ghosts_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let early_enough = rs.act_num <= 2;
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let route_support = profile.draw_sources * 2
        + profile.exhaust_engines * 3
        + profile.exhaust_outlets * 2
        + profile.block_core
        + profile.block_payoffs
        + profile.power_scalers
        + profile.self_damage_sources;
    let strong_support = route_support >= 5;
    let medium_support = route_support >= 3;

    if early_enough
        && ((strong_support && hp_ratio >= 0.45) || (medium_support && hp_ratio >= 0.60))
    {
        labels
            .iter()
            .position(|t| contains_any(t, &["accept", "apparitions"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["refuse"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["refuse"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["accept"])))
    }
}

fn choose_winding_halls_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    if labels.iter().any(|t| contains_any(t, &["proceed"])) {
        return Some(0);
    }

    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    if hp_ratio >= 0.65 {
        labels
            .iter()
            .position(|t| contains_any(t, &["embrace", "madness"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["accept", "max hp"]))
            })
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["retrace", "writhe"]))
            })
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["accept", "max hp"]))
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["embrace", "madness"]))
            })
            .or_else(|| {
                labels
                    .iter()
                    .position(|t| contains_any(t, &["retrace", "writhe"]))
            })
    }
}

fn choose_scrap_ooze_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let text = labels.first().copied().unwrap_or("");
    let chance = first_number(text);
    let damage = nth_number(text, 0);
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let should_continue =
        press_your_luck_continue(hp_ratio, rs.current_hp, damage, chance, 55, 0.55);

    if should_continue {
        labels
            .iter()
            .position(|t| contains_any(t, &["reach in"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["leave"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["reach in"])))
    }
}

fn choose_mushrooms_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let curse_pressure = curse_pressure_score(rs);
    let has_parasite = rs
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Parasite);
    let stomp_idx = labels
        .iter()
        .position(|t| contains_any(t, &["stomp", "fight the mushrooms"]));
    let eat_idx = labels
        .iter()
        .position(|t| contains_any(t, &["eat", "cursed", "heal"]));

    let stomp_score = 2_200 + curse_pressure * 40 + if has_parasite { 900 } else { 0 };
    let mut eat_score = 0;
    if hp_ratio < 0.45 {
        eat_score += 2_800;
    } else if hp_ratio < 0.60 {
        eat_score += 1_200;
    }
    eat_score -= curse_pressure * 85;
    if has_parasite {
        eat_score -= 2_400;
    }

    if eat_score > stomp_score {
        eat_idx.or(stomp_idx)
    } else {
        stomp_idx.or(eat_idx)
    }
}

fn choose_dead_adventurer_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let text = labels.first().copied().unwrap_or("");
    let chance = first_number(text);
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let should_continue = press_your_luck_continue(hp_ratio, rs.current_hp, 0, chance, 60, 0.60);

    if should_continue {
        labels
            .iter()
            .position(|t| contains_any(t, &["search"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["leave"])))
    } else {
        labels
            .iter()
            .position(|t| contains_any(t, &["leave"]))
            .or_else(|| labels.iter().position(|t| contains_any(t, &["search"])))
    }
}

fn choose_bonfire_option(rs: &RunState, options: &[Value]) -> Option<usize> {
    let labels = options.iter().map(option_text).collect::<Vec<_>>();
    let approach_idx = labels.iter().position(|t| contains_any(t, &["approach"]));
    let offer_idx = labels.iter().position(|t| contains_any(t, &["offer"]));
    let leave_idx = labels.iter().position(|t| contains_any(t, &["leave"]));

    let fuel_score = best_bonfire_fuel_score(rs);

    if let Some(idx) = offer_idx {
        return if fuel_score >= 1 {
            Some(idx)
        } else {
            leave_idx.or(Some(idx))
        };
    }

    if let Some(idx) = approach_idx {
        return if fuel_score >= 1 || (rs.current_hp < rs.max_hp && fuel_score >= 0) {
            Some(idx)
        } else {
            leave_idx.or(Some(idx))
        };
    }

    leave_idx
}

#[cfg(test)]
mod tests {
    use super::choose_event_choice;
    use crate::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;
    use crate::state::run::RunState;
    use serde_json::json;

    fn run_state(hp: i32, max_hp: i32) -> RunState {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.current_hp = hp;
        rs.max_hp = max_hp;
        rs
    }

    fn attach_two_step_shop(rs: &mut RunState) {
        let mut row0 = vec![MapRoomNode::new(0, 0)];
        let mut row1 = vec![MapRoomNode::new(0, 1)];
        let mut row2 = vec![MapRoomNode::new(0, 2)];
        row0[0].class = Some(RoomType::EventRoom);
        row1[0].class = Some(RoomType::MonsterRoom);
        row2[0].class = Some(RoomType::ShopRoom);
        row0[0].edges.insert(MapEdge::new(0, 0, 0, 1));
        row1[0].edges.insert(MapEdge::new(0, 1, 0, 2));
        rs.map = MapState {
            graph: vec![row0, row1, row2],
            current_y: 0,
            current_x: 0,
            boss_node_available: false,
            has_emerald_key: false,
        };
    }

    #[test]
    fn golden_idol_trap_avoids_curse_when_hp_is_high() {
        let gs = json!({
            "screen_state": {
                "event_id": "Golden Idol",
                "event_name": "Golden Idol",
                "options": [
                    {"text": "[Run] Obtain Injury curse.", "label": "Run", "disabled": false},
                    {"text": "[Fight] Take 20 damage.", "label": "Fight", "disabled": false},
                    {"text": "[Lose Max HP] Lose 6 Max HP.", "label": "Lose Max HP", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(70, 80), &["run", "fight", "lose max hp"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn golden_idol_trap_prefers_max_hp_over_curse_when_hp_is_low() {
        let gs = json!({
            "screen_state": {
                "event_id": "Golden Idol",
                "event_name": "Golden Idol",
                "options": [
                    {"text": "[Run] Obtain Injury curse.", "label": "Run", "disabled": false},
                    {"text": "[Fight] Take 20 damage.", "label": "Fight", "disabled": false},
                    {"text": "[Lose Max HP] Lose 6 Max HP.", "label": "Lose Max HP", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(25, 80), &["run", "fight", "lose max hp"]);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn generic_event_cost_family_prefers_non_curse_option() {
        let gs = json!({
            "screen_state": {
                "event_id": "Some Event",
                "event_name": "Some Event",
                "options": [
                    {"text": "[Accept] Obtain Injury.", "label": "Accept", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(60, 80), &["accept", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn big_fish_prefers_donut_when_not_low_hp() {
        let gs = json!({
            "screen_state": {
                "event_id": "Big Fish",
                "event_name": "Big Fish",
                "options": [
                    {"text": "[Banana] Heal 26 HP.", "label": "Banana", "disabled": false},
                    {"text": "[Donut] Gain 5 Max HP.", "label": "Donut", "disabled": false},
                    {"text": "[Box] Obtain a random Relic. Become Cursed - Regret.", "label": "Box", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(70, 80), &["banana", "donut", "box"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn big_fish_can_take_box_when_regret_is_tractable() {
        let gs = json!({
            "screen_state": {
                "event_id": "Big Fish",
                "event_name": "Big Fish",
                "options": [
                    {"text": "[Banana] Heal 26 HP.", "label": "Banana", "disabled": false},
                    {"text": "[Donut] Gain 5 Max HP.", "label": "Donut", "disabled": false},
                    {"text": "[Box] Obtain a random Relic. Become Cursed - Regret.", "label": "Box", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(68, 80);
        rs.gold = 150;
        attach_two_step_shop(&mut rs);
        rs.add_card_to_deck(crate::content::cards::CardId::BurningPact);
        let idx = choose_event_choice(&gs, &rs, &["banana", "donut", "box"]);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn golden_shrine_prefers_pray_over_curse() {
        let gs = json!({
            "screen_state": {
                "event_id": "Golden Shrine",
                "event_name": "Golden Shrine",
                "options": [
                    {"text": "[Pray] Gain 100 Gold.", "label": "Pray", "disabled": false},
                    {"text": "[Desecrate] Gain 275 Gold. Become Cursed - Regret.", "label": "Desecrate", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(60, 80), &["pray", "desecrate", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn golden_shrine_can_take_desecrate_when_shop_conversion_is_high() {
        let gs = json!({
            "screen_state": {
                "event_id": "Golden Shrine",
                "event_name": "Golden Shrine",
                "options": [
                    {"text": "[Pray] Gain 100 Gold.", "label": "Pray", "disabled": false},
                    {"text": "[Desecrate] Gain 275 Gold. Become Cursed - Regret.", "label": "Desecrate", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(67, 80);
        rs.gold = 40;
        attach_two_step_shop(&mut rs);
        let idx = choose_event_choice(&gs, &rs, &["pray", "desecrate", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn cleric_prefers_purify_when_curse_present() {
        let mut rs = run_state(60, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Regret);
        let gs = json!({
            "screen_state": {
                "event_id": "Cleric",
                "event_name": "Cleric",
                "options": [
                    {"text": "[Heal] Lose 35 Gold. Heal 25% of your Max HP.", "label": "Heal", "disabled": false},
                    {"text": "[Purify] Lose 50 Gold. Remove a card from your deck.", "label": "Purify", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &rs, &["heal", "purify", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn cleric_purify_is_good_even_without_curse_when_remove_targets_exist() {
        let gs = json!({
            "screen_state": {
                "event_id": "Cleric",
                "event_name": "Cleric",
                "options": [
                    {"text": "[Heal] Lose 35 Gold. Heal 25% of your Max HP.", "label": "Heal", "disabled": false},
                    {"text": "[Purify] Lose 50 Gold. Remove a card from your deck.", "label": "Purify", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(61, 80);
        rs.gold = 120;
        rs.add_card_to_deck(crate::content::cards::CardId::Strike);
        let idx = choose_event_choice(&gs, &rs, &["heal", "purify", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn neow_prefers_remove_over_small_gold() {
        let gs = json!({
            "screen_state": {
                "event_id": "Neow Event",
                "event_name": "Neow",
                "options": [
                    {"text": "[Remove a Card]", "label": "Remove a Card", "disabled": false},
                    {"text": "[Obtain 100 Gold]", "label": "Obtain 100 Gold", "disabled": false},
                    {"text": "[Lose 8 Max HP Gain 250 Gold]", "label": "Lose 8 Max HP Gain 250 Gold", "disabled": false},
                    {"text": "[Lose your starting Relic Obtain a random Boss Relic]", "label": "Lose your starting Relic Obtain a random Boss Relic", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(
            &gs,
            &run_state(80, 80),
            &["remove", "gold", "max hp gold", "boss relic"],
        );
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn woman_in_blue_prefers_two_potions_when_slots_open() {
        let gs = json!({
            "screen_state": {
                "event_id": "Woman in Blue",
                "event_name": "Woman in Blue",
                "options": [
                    {"text": "[1 Potion] Lose 20 Gold.", "label": "1 Potion", "disabled": false},
                    {"text": "[2 Potions] Lose 30 Gold.", "label": "2 Potions", "disabled": false},
                    {"text": "[3 Potions] Lose 40 Gold.", "label": "3 Potions", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.gold = 100;
        let idx = choose_event_choice(&gs, &rs, &["1 potion", "2 potions", "3 potions", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn golden_wing_prefers_remove_when_hp_is_safe() {
        let gs = json!({
            "screen_state": {
                "event_id": "Golden Wing",
                "event_name": "Golden Wing",
                "options": [
                    {"text": "[Remove a card] Take 7 damage. Remove a card from your deck.", "label": "Remove", "disabled": false},
                    {"text": "[Attack] Gain 50-80 Gold.", "label": "Attack", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(70, 80), &["remove", "attack", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn face_trader_prefers_trade() {
        let gs = json!({
            "screen_state": {
                "event_id": "Face Trader",
                "event_name": "Face Trader",
                "options": [
                    {"text": "[Touch] Lose 8 HP. Gain 75 Gold.", "label": "Touch", "disabled": false},
                    {"text": "[Trade] Obtain a face Relic.", "label": "Trade", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(60, 80), &["touch", "trade", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn scrap_ooze_stops_when_risk_is_high() {
        let gs = json!({
            "screen_state": {
                "event_id": "Scrap Ooze",
                "event_name": "Scrap Ooze",
                "options": [
                    {"text": "[Reach In] Take 7 damage. 65% chance to obtain a Relic.", "label": "Reach In", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(30, 80), &["reach in", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn mushrooms_prefers_fight_when_hp_is_safe() {
        let gs = json!({
            "screen_state": {
                "event_id": "Mushrooms",
                "event_name": "Mushrooms",
                "options": [
                    {"text": "[Stomp] Fight the mushrooms!", "label": "Stomp", "disabled": false},
                    {"text": "[Eat] Heal 20 HP. Become Cursed - Parasite.", "label": "Eat", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(70, 80), &["stomp", "eat"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn mushrooms_avoids_second_parasite_even_when_hp_is_low() {
        let gs = json!({
            "screen_state": {
                "event_id": "Mushrooms",
                "event_name": "Mushrooms",
                "options": [
                    {"text": "[Stomp] Fight the mushrooms!", "label": "Stomp", "disabled": false},
                    {"text": "[Eat] Heal 20 HP. Become Cursed - Parasite.", "label": "Eat", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(24, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Parasite);
        let idx = choose_event_choice(&gs, &rs, &["stomp", "eat"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn dead_adventurer_stops_search_when_chance_is_high_and_hp_is_low() {
        let gs = json!({
            "screen_state": {
                "event_id": "Dead Adventurer",
                "event_name": "Dead Adventurer",
                "options": [
                    {"text": "[Search] 75% chance of a fight.", "label": "Search", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(28, 80), &["search", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn forgotten_altar_prefers_offer_with_golden_idol() {
        let gs = json!({
            "screen_state": {
                "event_id": "Forgotten Altar",
                "event_name": "Forgotten Altar",
                "options": [
                    {"text": "[Offer] Trade Golden Idol for Bloody Idol.", "label": "Offer", "disabled": false},
                    {"text": "[Pray] Gain 5 Max HP. Lose 20 HP.", "label": "Pray", "disabled": false},
                    {"text": "[Desecrate] Become Cursed - Decay.", "label": "Desecrate", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(60, 80);
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::GoldenIdol,
        ));
        let idx = choose_event_choice(&gs, &rs, &["offer", "pray", "desecrate"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn vampires_prefers_give_vial_when_available() {
        let gs = json!({
            "screen_state": {
                "event_id": "Vampires",
                "event_name": "Vampires",
                "options": [
                    {"text": "[Accept] Lose 24 Max HP. Replace all Strikes with 5 Bites.", "label": "Accept", "disabled": false},
                    {"text": "[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites.", "label": "Give Vial", "disabled": false},
                    {"text": "[Refuse] Leave.", "label": "Refuse", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(65, 80);
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::BloodVial,
        ));
        let idx = choose_event_choice(&gs, &rs, &["accept", "give vial", "refuse"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn ghosts_accepts_when_early_and_healthy() {
        let gs = json!({
            "screen_state": {
                "event_id": "Ghosts",
                "event_name": "Ghosts",
                "options": [
                    {"text": "[Accept] Lose 40 Max HP. Obtain 5 Apparitions.", "label": "Accept", "disabled": false},
                    {"text": "[Refuse]", "label": "Refuse", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.act_num = 2;
        rs.master_deck.push(crate::combat::CombatCard::new(
            crate::content::cards::CardId::BurningPact,
            1,
        ));
        rs.master_deck.push(crate::combat::CombatCard::new(
            crate::content::cards::CardId::DarkEmbrace,
            2,
        ));
        let idx = choose_event_choice(&gs, &rs, &["accept", "refuse"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn ghosts_refuses_unsupported_deck_even_when_healthy() {
        let gs = json!({
            "screen_state": {
                "event_id": "Ghosts",
                "event_name": "Ghosts",
                "options": [
                    {"text": "[Accept] Lose 40 Max HP. Obtain 5 Apparitions.", "label": "Accept", "disabled": false},
                    {"text": "[Refuse]", "label": "Refuse", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.act_num = 2;
        rs.master_deck.push(crate::combat::CombatCard::new(
            crate::content::cards::CardId::Strike,
            1,
        ));
        rs.master_deck.push(crate::combat::CombatCard::new(
            crate::content::cards::CardId::Defend,
            2,
        ));
        let idx = choose_event_choice(&gs, &rs, &["accept", "refuse"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn winding_halls_prefers_max_hp_loss_when_low_hp() {
        let gs = json!({
            "screen_state": {
                "event_id": "Winding Halls",
                "event_name": "Winding Halls",
                "options": [
                    {"text": "[Embrace] Lose 10 HP. Obtain 2 Madness.", "label": "Embrace", "disabled": false},
                    {"text": "[Retrace] Heal 20 HP. Become Cursed - Writhe.", "label": "Retrace", "disabled": false},
                    {"text": "[Accept] Lose 4 Max HP.", "label": "Accept", "disabled": false}
                ]
            }
        });
        let idx = choose_event_choice(&gs, &run_state(24, 80), &["embrace", "retrace", "accept"]);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn shining_light_prefers_upgrade_when_healthy_and_deck_has_targets() {
        let gs = json!({
            "screen_state": {
                "event_id": "Shining Light",
                "event_name": "Shining Light",
                "options": [
                    {"text": "[Enter the Light] Take 16 damage. Upgrade 2 random cards.", "label": "Enter the Light", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Strike);
        rs.add_card_to_deck(crate::content::cards::CardId::Bash);
        let idx = choose_event_choice(&gs, &rs, &["enter", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn moai_head_prefers_trade_with_golden_idol() {
        let gs = json!({
            "screen_state": {
                "event_id": "Moai Head",
                "event_name": "Moai Head",
                "options": [
                    {"text": "[Enter] Lose 10 Max HP. Heal to full.", "label": "Enter", "disabled": false},
                    {"text": "[Trade] Give Golden Idol. Gain 333 Gold.", "label": "Trade", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(50, 80);
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::GoldenIdol,
        ));
        let idx = choose_event_choice(&gs, &rs, &["enter", "trade", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn bonfire_prefers_offer_when_deck_has_curse_to_burn() {
        let gs = json!({
            "screen_state": {
                "event_id": "Bonfire Spirits",
                "event_name": "Bonfire Spirits",
                "options": [
                    {"text": "[Offer] Select a card to offer.", "label": "Offer", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(60, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Regret);
        let idx = choose_event_choice(&gs, &rs, &["offer", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn bonfire_can_leave_when_no_reasonable_fuel_exists() {
        let gs = json!({
            "screen_state": {
                "event_id": "Bonfire Elementals",
                "event_name": "Bonfire Elementals",
                "options": [
                    {"text": "[Offer] Sacrifice a card to the spirits.", "label": "Offer", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(80, 80);
        rs.master_deck.clear();
        rs.add_card_to_deck(crate::content::cards::CardId::Reaper);
        rs.add_card_to_deck(crate::content::cards::CardId::DemonForm);
        let idx = choose_event_choice(&gs, &rs, &["offer", "leave"]);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn living_wall_prefers_forget_when_deck_has_bad_starters() {
        let gs = json!({
            "screen_state": {
                "event_id": "Living Wall",
                "event_name": "Living Wall",
                "options": [
                    {"text": "[Forget] Remove a card from your deck.", "label": "Forget", "disabled": false},
                    {"text": "[Change] Transform a card in your deck.", "label": "Change", "disabled": false},
                    {"text": "[Grow] Upgrade a card in your deck.", "label": "Grow", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Strike);
        rs.add_card_to_deck(crate::content::cards::CardId::Defend);
        let idx = choose_event_choice(&gs, &rs, &["forget", "change", "grow"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn designer_prefers_full_service_when_affordable_and_deck_has_targets() {
        let gs = json!({
            "screen_state": {
                "event_id": "Designer",
                "event_name": "Designer",
                "options": [
                    {"text": "[Adjust] 40 Gold. Upgrade 1 card.", "label": "Adjust", "disabled": false},
                    {"text": "[Clean Up] 60 Gold. Remove 1 card.", "label": "Clean Up", "disabled": false},
                    {"text": "[Full Service] 90 Gold. Remove 1 card + upgrade 1 random.", "label": "Full Service", "disabled": false},
                    {"text": "[Punch] Lose 3 HP.", "label": "Punch", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.gold = 120;
        rs.add_card_to_deck(crate::content::cards::CardId::Strike);
        rs.add_card_to_deck(crate::content::cards::CardId::Bash);
        let idx = choose_event_choice(&gs, &rs, &["adjust", "clean", "full", "punch"]);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn purification_shrine_prefers_pray_when_deck_has_remove_target() {
        let gs = json!({
            "screen_state": {
                "event_id": "Purification Shrine",
                "event_name": "Purification Shrine",
                "options": [
                    {"text": "[Pray] Remove a card from your deck.", "label": "Pray", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(60, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Regret);
        let idx = choose_event_choice(&gs, &rs, &["pray", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn purification_shrine_strongly_prefers_pray_with_parasite() {
        let gs = json!({
            "screen_state": {
                "event_id": "Purification Shrine",
                "event_name": "Purification Shrine",
                "options": [
                    {"text": "[Pray] Remove a card from your deck.", "label": "Pray", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(62, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Parasite);
        let idx = choose_event_choice(&gs, &rs, &["pray", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn upgrade_shrine_prefers_upgrade_when_targets_exist() {
        let gs = json!({
            "screen_state": {
                "event_id": "Upgrade Shrine",
                "event_name": "Upgrade Shrine",
                "options": [
                    {"text": "[Pray] Upgrade a card.", "label": "Pray", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Bash);
        let idx = choose_event_choice(&gs, &rs, &["pray", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn beggar_prefers_donate_when_gold_and_remove_target_exist() {
        let gs = json!({
            "screen_state": {
                "event_id": "Beggar",
                "event_name": "Beggar",
                "options": [
                    {"text": "[Donate] Lose 75 Gold. Remove a card.", "label": "Donate", "disabled": false},
                    {"text": "[Leave]", "label": "Leave", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.gold = 150;
        rs.add_card_to_deck(crate::content::cards::CardId::Regret);
        let idx = choose_event_choice(&gs, &rs, &["donate", "leave"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn we_meet_again_prefers_card_trade_when_bad_card_exists() {
        let gs = json!({
            "screen_state": {
                "event_id": "We Meet Again",
                "event_name": "We Meet Again",
                "options": [
                    {"text": "[Give Potion] Obtain a Relic.", "label": "Give Potion", "disabled": false},
                    {"text": "[Give Gold] Lose 50 Gold. Obtain a Relic.", "label": "Give Gold", "disabled": false},
                    {"text": "[Give Card] Remove a card. Obtain a Relic.", "label": "Give Card", "disabled": false},
                    {"text": "[Attack]", "label": "Attack", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Defend);
        let idx = choose_event_choice(&gs, &rs, &["potion", "gold", "card", "attack"]);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn note_for_yourself_prefers_take_when_removal_is_good() {
        let gs = json!({
            "screen_state": {
                "event_id": "Note For Yourself",
                "event_name": "Note For Yourself",
                "options": [
                    {"text": "[Take Card] Obtain Iron Wave. Remove a card.", "label": "Take Card", "disabled": false},
                    {"text": "[Ignore]", "label": "Ignore", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.add_card_to_deck(crate::content::cards::CardId::Strike);
        let idx = choose_event_choice(&gs, &rs, &["take", "ignore"]);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn drug_dealer_prefers_mutagenic_strength_when_strength_synergy_exists() {
        let gs = json!({
            "screen_state": {
                "event_id": "Drug Dealer",
                "event_name": "Drug Dealer",
                "options": [
                    {"text": "[Ingest Mutagens] Obtain J.A.X.", "label": "JAX", "disabled": false},
                    {"text": "[Become a Test Subject] Transform 2 cards.", "label": "Transform", "disabled": false},
                    {"text": "[Inject Mutagens] Obtain Mutagenic Strength relic.", "label": "Relic", "disabled": false}
                ]
            }
        });
        let mut rs = run_state(70, 80);
        rs.master_deck.clear();
        rs.add_card_to_deck(crate::content::cards::CardId::HeavyBlade);
        let idx = choose_event_choice(&gs, &rs, &["jax", "transform", "relic"]);
        assert_eq!(idx, Some(2));
    }
}
