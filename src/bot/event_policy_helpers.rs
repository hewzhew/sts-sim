use crate::map::node::RoomType;
use crate::state::run::RunState;

pub(crate) fn knowing_skull_colorless_value(rs: &RunState) -> f32 {
    use crate::content::cards::{colorless_pool_for_rarity, CardRarity};

    let pool = colorless_pool_for_rarity(CardRarity::Uncommon);
    if pool.is_empty() {
        return 0.0;
    }

    let expected_delta = pool
        .iter()
        .map(|&card_id| crate::bot::deck_delta_eval::compare_pick_vs_skip(rs, card_id).total)
        .sum::<i32>() as f32
        / pool.len() as f32;

    expected_delta * 10.0
}

pub(crate) fn random_potion_offer_value(
    rs: &RunState,
    empty_potion_slots: usize,
    potion_blocked: bool,
    potion_count: usize,
    open_slot_multiplier: f32,
    full_slot_multiplier: f32,
    blocked_penalty_per_potion: f32,
) -> f32 {
    if potion_blocked {
        return blocked_penalty_per_potion * potion_count as f32;
    }

    let expected_value = random_potion_expectation(rs);
    let open_slot_count = potion_count.min(empty_potion_slots);
    let overflow_count = potion_count.saturating_sub(open_slot_count);

    expected_value
        * (open_slot_count as f32 * open_slot_multiplier
            + overflow_count as f32 * full_slot_multiplier)
}

pub(crate) fn cursed_tome_book_relic_value(rs: &RunState) -> f32 {
    use crate::content::relics::RelicId;

    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let high_cost_attacks = rs
        .master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type == crate::content::cards::CardType::Attack && def.cost >= 2
        })
        .count() as f32;
    let base = relic_equity_value(rs) * 0.88;

    let available = [
        RelicId::Necronomicon,
        RelicId::Enchiridion,
        RelicId::NilrysCodex,
    ]
    .into_iter()
    .filter(|relic_id| !rs.relics.iter().any(|owned| owned.id == *relic_id))
    .collect::<Vec<_>>();

    if available.is_empty() {
        return base * 0.35;
    }

    let available_count = available.len() as f32;
    let total = available
        .into_iter()
        .map(|relic_id| match relic_id {
            RelicId::Necronomicon => {
                base + high_cost_attacks * 180.0 + profile.attack_count.min(8) as f32 * 28.0
            }
            RelicId::Enchiridion => {
                base + profile.power_scalers as f32 * 95.0 + profile.draw_sources as f32 * 35.0
            }
            RelicId::NilrysCodex => {
                base + profile.draw_sources as f32 * 70.0
                    + profile.exhaust_engines as f32 * 45.0
                    + profile.block_payoffs as f32 * 30.0
            }
            _ => base,
        })
        .sum::<f32>();

    total / available_count
}

pub(crate) fn cursed_tome_remaining_commitment_damage(rs: &RunState, current_screen: usize) -> i32 {
    let final_damage = if rs.ascension_level >= 15 { 15 } else { 10 };
    match current_screen {
        0 => 1 + 2 + 3 + final_damage,
        1 => 2 + 3 + final_damage,
        2 => 3 + final_damage,
        3 => final_damage,
        _ => 0,
    }
}

pub(crate) fn context_curse_drag(rs: &RunState) -> f32 {
    40.0 + crate::bot::noncombat_families::helpers::curse_pressure_score(rs) as f32 * 14.0
}

pub(crate) fn scrap_ooze_continue_score(rs: &RunState, damage: i32, chance: i32) -> f32 {
    let success = chance as f32 / 100.0;
    let hp_cost = hp_point_value(rs);
    let relic_value = relic_equity_value(rs);
    let immediate_cost = damage.max(0) as f32 * hp_cost;
    let failure_drag = (1.0 - success) * (damage + 1).max(0) as f32 * hp_cost * 0.55;
    let safety_penalty = safety_gap_penalty(rs, damage);
    success * relic_value - immediate_cost - failure_drag - safety_penalty
}

pub(crate) fn world_of_goop_gather_score(rs: &RunState, gain: i32, damage: i32) -> f32 {
    gain.max(0) as f32 * gold_value_per_gold(rs)
        - damage.max(0) as f32 * hp_point_value(rs)
        - safety_gap_penalty(rs, damage)
}

pub(crate) fn dead_adventurer_continue_score(rs: &RunState, encounter_chance: i32) -> f32 {
    let fight_p = encounter_chance as f32 / 100.0;
    let safe_loot_value = 680.0 + relic_equity_value(rs) * 0.18;
    let combat_reward_value = 900.0 + gold_value_per_gold(rs) * 20.0;
    let combat_risk = hp_point_value(rs) * (7.0 + route_hostility_scalar(rs) * 0.6);
    let safety_penalty = safety_gap_penalty(rs, 9);
    (1.0 - fight_p) * safe_loot_value + fight_p * (combat_reward_value - combat_risk)
        - safety_penalty * 0.45
}

pub(crate) fn relic_equity_value(rs: &RunState) -> f32 {
    let mut value = 1_450.0;
    if rs.act_num == 1 {
        value += 150.0;
    }
    if rs.floor_num <= 5 {
        value += 120.0;
    }
    if matches!(
        crate::bot::noncombat_families::helpers::reachable_room_distance(
            rs,
            RoomType::MonsterRoomElite,
            3,
        ),
        Some(1 | 2)
    ) {
        value += 90.0;
    }
    value
}

pub(crate) fn gold_value_per_gold(rs: &RunState) -> f32 {
    let mut value = 11.5;
    match crate::bot::noncombat_families::helpers::reachable_room_distance(
        rs,
        RoomType::ShopRoom,
        3,
    ) {
        Some(1) => value += 2.5,
        Some(2) => value += 1.5,
        Some(3) => value += 0.5,
        _ => {}
    }
    if rs.gold < 75 {
        value += 1.0;
    }
    value
}

pub(crate) fn hp_point_value(rs: &RunState) -> f32 {
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let mut value = 42.0 + (1.0 - hp_ratio).max(0.0) * 78.0;
    if rs.current_hp <= 45 {
        value += 8.0;
    }
    if rs.current_hp <= 30 {
        value += 14.0;
    }
    if rs.current_hp <= 20 {
        value += 22.0;
    }
    match crate::bot::noncombat_families::helpers::reachable_room_distance(
        rs,
        RoomType::MonsterRoomElite,
        4,
    ) {
        Some(1) => value += 18.0,
        Some(2) => value += 10.0,
        _ => {}
    }
    match crate::bot::noncombat_families::helpers::reachable_room_distance(
        rs,
        RoomType::RestRoom,
        3,
    ) {
        Some(1) => value -= 12.0,
        Some(2) => value -= 6.0,
        _ => {}
    }
    value.max(18.0)
}

pub(crate) fn safety_gap_penalty(rs: &RunState, immediate_damage: i32) -> f32 {
    let after_hp = (rs.current_hp - immediate_damage).max(0);
    let mut safety_floor = 14;
    if matches!(
        crate::bot::noncombat_families::helpers::reachable_room_distance(
            rs,
            RoomType::MonsterRoomElite,
            3,
        ),
        Some(1 | 2)
    ) {
        safety_floor += 10;
    }
    if matches!(
        crate::bot::noncombat_families::helpers::reachable_room_distance(rs, RoomType::RestRoom, 3),
        None | Some(3)
    ) {
        safety_floor += 4;
    }
    let gap = (safety_floor - after_hp).max(0) as f32;
    gap * hp_point_value(rs) * 0.65
}

fn random_potion_expectation(rs: &RunState) -> f32 {
    use crate::content::potions::{get_potion_definition, potions_for_class, PotionRarity};

    let agent = crate::bot::Agent::new_policy_model();
    let pool = potions_for_class(rs.potion_class());
    if pool.is_empty() {
        return 0.0;
    }

    let mut common_scores = Vec::new();
    let mut uncommon_scores = Vec::new();
    let mut rare_scores = Vec::new();
    for potion_id in pool {
        let score = agent.reward_potion_score(rs, potion_id) as f32;
        match get_potion_definition(potion_id).rarity {
            PotionRarity::Common => common_scores.push(score),
            PotionRarity::Uncommon => uncommon_scores.push(score),
            PotionRarity::Rare => rare_scores.push(score),
        }
    }

    weighted_bucket_average(&common_scores, 0.65)
        + weighted_bucket_average(&uncommon_scores, 0.25)
        + weighted_bucket_average(&rare_scores, 0.10)
}

fn route_hostility_scalar(rs: &RunState) -> f32 {
    let mut scalar: f32 = 0.0;
    if matches!(
        crate::bot::noncombat_families::helpers::reachable_room_distance(
            rs,
            RoomType::MonsterRoom,
            2
        ),
        Some(1)
    ) {
        scalar += 2.0;
    }
    match crate::bot::noncombat_families::helpers::reachable_room_distance(
        rs,
        RoomType::MonsterRoomElite,
        4,
    ) {
        Some(1) => scalar += 3.0,
        Some(2) => scalar += 2.0,
        Some(3) => scalar += 1.0,
        _ => {}
    }
    match crate::bot::noncombat_families::helpers::reachable_room_distance(
        rs,
        RoomType::RestRoom,
        3,
    ) {
        Some(1) => scalar -= 1.5,
        Some(2) => scalar -= 0.8,
        _ => {}
    }
    scalar.max(0.0)
}

fn weighted_bucket_average(scores: &[f32], weight: f32) -> f32 {
    if scores.is_empty() {
        return 0.0;
    }
    weight * (scores.iter().sum::<f32>() / scores.len() as f32)
}
