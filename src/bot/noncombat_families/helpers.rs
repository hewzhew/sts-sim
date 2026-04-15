use crate::map::node::RoomType;
use crate::state::run::RunState;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};

pub(super) fn option_text(option: &Value) -> &str {
    option
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| option.get("label").and_then(|v| v.as_str()))
        .unwrap_or("")
}

pub(super) fn next_purge_cost(rs: &RunState) -> i32 {
    75 + rs.shop_purge_count.max(0) * 25
}

pub(super) fn reachable_room_distance(
    rs: &RunState,
    target: RoomType,
    max_depth: i32,
) -> Option<i32> {
    if rs.map.current_y < 0 || rs.map.current_x < 0 {
        return None;
    }

    let start = (rs.map.current_x as usize, rs.map.current_y as usize);
    let mut q = VecDeque::from([(start, 0i32)]);
    let mut seen = HashSet::from([start]);

    while let Some(((x, y), depth)) = q.pop_front() {
        if depth > 0
            && rs
                .map
                .graph
                .get(y)
                .and_then(|row| row.get(x))
                .and_then(|node| node.class)
                == Some(target)
        {
            return Some(depth);
        }
        if depth >= max_depth {
            continue;
        }

        let Some(node) = rs.map.graph.get(y).and_then(|row| row.get(x)) else {
            continue;
        };
        for edge in &node.edges {
            if edge.dst_x < 0 || edge.dst_y < 0 {
                continue;
            }
            let next = (edge.dst_x as usize, edge.dst_y as usize);
            if seen.insert(next) {
                q.push_back((next, depth + 1));
            }
        }
    }

    None
}

pub(super) fn curse_tractability_score(rs: &RunState) -> i32 {
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let mut score = 0;
    let purge_cost = next_purge_cost(rs);
    if let Some(distance) = reachable_room_distance(rs, RoomType::ShopRoom, 4) {
        if distance <= 2 && rs.gold >= purge_cost {
            score += 4;
        } else if distance <= 4 && rs.gold >= purge_cost {
            score += 2;
        } else if distance <= 2 {
            score += 1;
        }
    }
    if profile.exhaust_outlets >= 1 || profile.exhaust_engines >= 2 {
        score += 1;
    }
    score
}

pub(super) fn nearby_shop_conversion_bonus(rs: &RunState) -> i32 {
    match reachable_room_distance(rs, RoomType::ShopRoom, 3) {
        Some(1) => 380,
        Some(2) => 260,
        Some(3) => 120,
        _ => 0,
    }
}

pub(super) fn generic_remove_value(rs: &RunState) -> i32 {
    1_500
        + count_remove_targets(rs) * 420
        + curse_pressure_score(rs) * 90
        + crate::bot::deck_delta_eval::compare_purge_vs_keep(rs).total * 12
}

pub(super) fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

pub(super) fn first_number(text: &str) -> i32 {
    nth_number(text, 0)
}

pub(super) fn nth_number(text: &str, index: usize) -> i32 {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .nth(index)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

pub(super) fn press_your_luck_continue(
    hp_ratio: f32,
    current_hp: i32,
    immediate_damage: i32,
    chance: i32,
    target_chance: i32,
    min_hp_ratio: f32,
) -> bool {
    hp_ratio >= min_hp_ratio && current_hp > immediate_damage + 10 && chance < target_chance
}

pub(super) fn count_curses(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            matches!(
                crate::content::cards::get_card_definition(card.id).card_type,
                crate::content::cards::CardType::Curse
            )
        })
        .count() as i32
}

pub(super) fn curse_pressure_score(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Curse {
                let severity = crate::bot::evaluator::curse_remove_severity(card.id);
                if severity > 0 {
                    severity
                } else {
                    3
                }
            } else {
                0
            }
        })
        .sum()
}

pub(super) fn count_starter_strikes(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id)
                .tags
                .contains(&crate::content::cards::CardTag::StarterStrike)
        })
        .count() as i32
}

pub(super) fn count_upgradable_cards(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            card.id == crate::content::cards::CardId::SearingBlow
                || (card.upgrades == 0
                    && def.card_type != crate::content::cards::CardType::Status
                    && def.card_type != crate::content::cards::CardType::Curse)
        })
        .count() as i32
}

pub(super) fn count_remove_targets(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type == crate::content::cards::CardType::Curse
                || def.card_type == crate::content::cards::CardType::Status
                || def
                    .tags
                    .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || (def.rarity == crate::content::cards::CardRarity::Basic
                    && !def.tags.contains(&crate::content::cards::CardTag::Healing))
        })
        .count() as i32
}

pub(super) fn count_transform_targets(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type == crate::content::cards::CardType::Curse
                || def
                    .tags
                    .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || def.rarity == crate::content::cards::CardRarity::Basic
                || (def.rarity == crate::content::cards::CardRarity::Common
                    && def.card_type != crate::content::cards::CardType::Power)
        })
        .count() as i32
}

pub(super) fn best_bonfire_fuel_score(rs: &RunState) -> i32 {
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    rs.master_deck
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Curse {
                6
            } else if def.rarity == crate::content::cards::CardRarity::Basic {
                5
            } else if def
                .tags
                .contains(&crate::content::cards::CardTag::StarterStrike)
            {
                5
            } else if def.name == "Defend" {
                4
            } else if def.card_type == crate::content::cards::CardType::Status {
                4
            } else if hp_ratio < 0.45 {
                let owned_value =
                    crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
                if def.rarity != crate::content::cards::CardRarity::Common && owned_value <= 28 {
                    5
                } else if owned_value <= 18 {
                    4
                } else if def.rarity == crate::content::cards::CardRarity::Common {
                    2
                } else {
                    0
                }
            } else if def.rarity == crate::content::cards::CardRarity::Common {
                2
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn best_we_meet_again_card_give_score(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Curse {
                2_500 + crate::bot::evaluator::curse_remove_severity(card.id) * 120
            } else if def
                .tags
                .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || def.rarity == crate::content::cards::CardRarity::Basic
            {
                1_900
            } else {
                let owned_value =
                    crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
                (1_450 - owned_value * 18).max(-200)
            }
        })
        .max()
        .unwrap_or(-200)
}

pub(super) fn potion_keep_value(potion_id: crate::content::potions::PotionId) -> i32 {
    use crate::content::potions::PotionId;
    match potion_id {
        PotionId::AncientPotion => 100,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::BlessingOfTheForge => 84,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 85,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 88,
        _ => 55,
    }
}

pub(super) fn best_we_meet_again_potion_give_score(rs: &RunState) -> i32 {
    rs.potions
        .iter()
        .flatten()
        .map(|p| 1_850 - potion_keep_value(p.id) * 12)
        .max()
        .unwrap_or(-300)
}

pub(super) fn parse_note_card(text: &str) -> Option<crate::content::cards::CardId> {
    use crate::content::cards::{
        colorless_pool_for_rarity, get_card_definition, get_curse_pool, ironclad_pool_for_rarity,
        silent_pool_for_rarity, CardId, CardRarity,
    };

    let lower = text.to_ascii_lowercase();
    let mut candidates: Vec<CardId> = vec![
        CardId::Strike,
        CardId::Defend,
        CardId::Bash,
        CardId::StrikeG,
        CardId::DefendG,
        CardId::Neutralize,
        CardId::Survivor,
    ];
    for pool in [
        ironclad_pool_for_rarity(CardRarity::Common),
        ironclad_pool_for_rarity(CardRarity::Uncommon),
        ironclad_pool_for_rarity(CardRarity::Rare),
        silent_pool_for_rarity(CardRarity::Common),
        silent_pool_for_rarity(CardRarity::Uncommon),
        silent_pool_for_rarity(CardRarity::Rare),
        colorless_pool_for_rarity(CardRarity::Uncommon),
        colorless_pool_for_rarity(CardRarity::Rare),
        get_curse_pool(),
    ] {
        candidates.extend_from_slice(pool);
    }
    candidates.sort_by_key(|id| crate::content::cards::java_id(*id));
    candidates.dedup();

    candidates.into_iter().find(|&id| {
        let name = get_card_definition(id).name.to_ascii_lowercase();
        lower.contains(&name)
    })
}
