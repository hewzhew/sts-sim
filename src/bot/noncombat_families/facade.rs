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
        return event_option_choice_index(options, idx);
    }

    choose_by_cost_family(options, choice_list)
        .and_then(|idx| event_option_choice_index(options, idx))
}

fn event_option_choice_index(options: &[Value], option_idx: usize) -> Option<usize> {
    let option = options.get(option_idx)?;
    if option
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return None;
    }

    if let Some(choice_index) = option.get("choice_index").and_then(|v| v.as_u64()) {
        return Some(choice_index as usize);
    }

    Some(
        options
            .iter()
            .take(option_idx + 1)
            .filter(|option| {
                !option
                    .get("disabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count()
            .saturating_sub(1),
    )
}

fn event_choice_name_for_option<'a>(
    choice_list: &'a [&str],
    options: &[Value],
    option_idx: usize,
) -> &'a str {
    event_option_choice_index(options, option_idx)
        .and_then(|choice_idx| choice_list.get(choice_idx).copied())
        .unwrap_or("")
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
    let transform_value = crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, 2, false);
    let jax_delta =
        crate::bot::deck_delta_eval::compare_pick_vs_skip(rs, crate::content::cards::CardId::JAX);
    let jax_value =
        1_150 + jax_delta.prior_delta * 10 + jax_delta.rollout_delta * 6 + jax_delta.suite_bias * 3;
    let relic_value = if has_strength_scaling { 2_400 } else { 2_050 };

    if let Some(idx) = transform_idx {
        if transform_targets >= 2 && !has_strength_scaling {
            return Some(idx);
        }
    }

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
            1_800
                + transform_targets * 260
                + transform_value.prior_delta * 10
                + transform_value.rollout_delta * 8
                + transform_value.suite_bias * 4
        } else if Some(idx) == relic_idx {
            relic_value
        } else if Some(idx) == jax_idx {
            if has_strength_scaling {
                jax_value + 220
            } else {
                jax_value
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
        let cost = classify_event_cost(
            text,
            event_choice_name_for_option(choice_list, options, idx),
        );
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
    let nearby_shop_bonus = nearby_shop_conversion_bonus(rs);
    let max_hp_loss = labels
        .iter()
        .find(|t| contains_any(t, &["lose", "max hp"]))
        .map(|t| first_number(t))
        .unwrap_or_else(|| {
            if rs.ascension_level >= 15 {
                ((rs.max_hp as f32) * 0.35).round() as i32
            } else {
                ((rs.max_hp as f32) * 0.30).round() as i32
            }
        });
    let bite_exchange = crate::bot::deck_delta_eval::compare_vampires_vs_refuse(rs);
    let bite_trade_score =
        bite_exchange.total * 12 + strike_count * 85 + i32::from(rs.act_num <= 2) * 220
            - nearby_shop_bonus
            - max_hp_loss * 42
            - i32::from(hp_ratio < 0.55) * 260;

    if bite_trade_score >= 1_150 && strike_count >= 4 {
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

