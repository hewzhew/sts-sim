use crate::bot::deck_profile::{deck_profile, DeckProfile};
use crate::bot::deck_scoring::{curse_remove_severity, score_owned_card};
use crate::bot::shared::{analyze_run_needs, score_reward_potion, RunNeedSnapshot};
use crate::content::cards;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug)]
pub(super) struct BossRelicContext {
    pub need: RunNeedSnapshot,
    pub profile: DeckProfile,
    pub next_act: u8,
    pub rest_distance: Option<i32>,
    pub elite_distance: Option<i32>,
    pub missing_keys: u8,
    pub is_final_act_available: bool,
    pub two_plus_cost_count: i32,
    pub avg_cost_times_10: i32,
    pub expensive_cards: i32,
    pub starter_cards: i32,
    pub empty_potion_slots: i32,
    pub draw_sources: i32,
    pub exhaust_outlets: i32,
    pub attack_count: i32,
    pub block_core: i32,
    pub energy_sink_value: i32,
    pub high_action_dependence: i32,
    pub hand_clog_risk: i32,
    pub pyramid_retention_quality: i32,
    pub pyramid_cleanup_capacity: i32,
    pub pyramid_clog_liability: i32,
    pub snecko_draw_bonus: i32,
    pub snecko_cost_randomization_benefit: i32,
    pub snecko_cost_randomization_risk: i32,
    pub reward_dependence: i32,
    pub deck_maturity: i32,
    pub crown_reward_dependency: i32,
    pub crown_maturity_buffer: i32,
    pub potion_dependence: i32,
    pub current_potion_quality: i32,
    pub curse_tolerance: i32,
    pub volatility_tolerance: i32,
    pub pandora_rebuild_value: i32,
    pub pandora_plan_disruption_risk: i32,
    pub sustain_strength: i32,
    pub campfire_heal_dependence: i32,
    pub upgrade_backlog: i32,
    pub shop_value_pressure: i32,
    pub potion_lock_pressure: i32,
    pub enemy_buff_risk: i32,
    pub wound_absorption: i32,
    pub remove_targets_value: i32,
    pub transform_targets_value: i32,
    pub has_omamori: bool,
}

pub(super) fn build_context(run_state: &RunState) -> BossRelicContext {
    let need = analyze_run_needs(run_state);
    let profile = deck_profile(run_state);
    let next_act = run_state.act_num.saturating_add(1);
    let rest_distance = need.rest_distance;
    let shop_distance = need.shop_distance;
    let elite_distance = need.elite_distance;
    let missing_keys = need.missing_keys;

    let mut zero_cost_count = 0;
    let mut one_cost_count = 0;
    let mut two_plus_cost_count = 0;
    let mut x_cost_count = 0;
    let mut expensive_cards = 0;
    let mut starter_cards = 0;
    let mut total_cost_times_10 = 0;
    let mut removable_scores = Vec::with_capacity(run_state.master_deck.len());

    for card in &run_state.master_deck {
        let cost = cards::upgraded_base_cost_override(card)
            .unwrap_or_else(|| cards::get_card_definition(card.id).cost);
        if cost < 0 {
            x_cost_count += 1;
            total_cost_times_10 += 20;
        } else if cost == 0 {
            zero_cost_count += 1;
        } else if cost == 1 {
            one_cost_count += 1;
            total_cost_times_10 += 10;
        } else {
            two_plus_cost_count += 1;
            expensive_cards += 1;
            total_cost_times_10 += cost as i32 * 10;
        }

        if cards::is_starter_basic(card.id) {
            starter_cards += 1;
        }

        removable_scores.push(removable_pain(run_state, card.id));
    }

    removable_scores.sort_by(|lhs, rhs| rhs.cmp(lhs));
    let remove_targets_value = removable_scores.iter().take(2).sum::<i32>();
    let transform_targets_value = removable_scores.iter().take(3).sum::<i32>();

    let held_potions = run_state
        .potions
        .iter()
        .filter_map(|slot| slot.as_ref().map(|potion| potion.id))
        .collect::<Vec<_>>();
    let empty_potion_slots = run_state
        .potions
        .iter()
        .filter(|slot| slot.is_none())
        .count() as i32;
    let current_potion_quality = if held_potions.is_empty() {
        0
    } else {
        held_potions
            .iter()
            .map(|potion_id| score_reward_potion(run_state, *potion_id))
            .sum::<i32>()
            / held_potions.len() as i32
    };

    let avg_cost_times_10 = if run_state.master_deck.is_empty() {
        10
    } else {
        total_cost_times_10 / run_state.master_deck.len() as i32
    };

    let high_action_dependence = clamp_feature(
        zero_cost_count * 12
            + one_cost_count * 5
            + profile.draw_sources * 4
            + profile.skill_count * 2
            - two_plus_cost_count * 3
            - profile.x_cost_payoffs * 4,
    );
    let cost_precision_dependence = clamp_feature(
        zero_cost_count * 10
            + one_cost_count * 4
            + profile.draw_sources * 3
            + profile.status_generators * 2
            - expensive_cards * 4
            - x_cost_count * 6,
    );
    let hand_clog_risk = clamp_feature(
        two_plus_cost_count * 6 + profile.status_generators * 10 + starter_cards * 2
            - profile.exhaust_outlets * 7
            - profile.draw_sources * 4
            - profile.block_core * 2,
    );
    let hand_conversion_ability = clamp_feature(
        profile.draw_sources * 5
            + profile.exhaust_outlets * 12
            + profile.block_core * 4
            + profile.exhaust_engines * 6
            + profile.power_scalers * 2,
    );
    let retention_quality = clamp_feature(
        profile.block_core * 5
            + profile.draw_sources * 4
            + expensive_cards * 5
            + x_cost_count * 6
            + profile.exhaust_outlets * 3
            - profile.status_generators * 4,
    );
    let energy_sink_value = clamp_feature(
        expensive_cards * 7
            + x_cost_count * 10
            + profile.draw_sources * 3
            + profile.strength_payoffs * 4
            + profile.block_payoffs * 3,
    );
    let pyramid_retention_quality = compute_pyramid_retention_quality(
        retention_quality,
        expensive_cards,
        profile.block_core,
        profile.status_generators,
        x_cost_count,
    );
    let pyramid_cleanup_capacity = compute_pyramid_cleanup_capacity(
        hand_conversion_ability,
        profile.exhaust_outlets,
        profile.draw_sources,
        zero_cost_count,
        one_cost_count,
        starter_cards,
    );
    let pyramid_clog_liability = compute_pyramid_clog_liability(
        hand_clog_risk,
        profile.status_generators,
        two_plus_cost_count,
        profile.exhaust_outlets,
        profile.draw_sources,
        x_cost_count,
    );
    let snecko_draw_bonus = compute_snecko_draw_bonus(profile.draw_sources, profile.attack_count);
    let snecko_cost_randomization_benefit = compute_snecko_cost_randomization_benefit(
        energy_sink_value,
        x_cost_count,
        two_plus_cost_count,
        avg_cost_times_10,
        profile.draw_sources,
    );
    let snecko_cost_randomization_risk = compute_snecko_cost_randomization_risk(
        cost_precision_dependence,
        zero_cost_count,
        one_cost_count,
        starter_cards,
        expensive_cards,
        x_cost_count,
    );

    let gap_pressure = need.damage_gap + need.block_gap + need.control_gap;
    let shell_closure_signal = shell_closure_signal(&profile);
    let starter_drag = starter_cards * 3;
    let consistency_support =
        profile.draw_sources * 2 + profile.exhaust_outlets * 2 + profile.block_core * 2;
    let deck_maturity = compute_deck_maturity(
        gap_pressure,
        shell_closure_signal,
        starter_drag,
        consistency_support,
    );
    let reward_dependence = compute_reward_dependence(gap_pressure, deck_maturity, next_act);
    let crown_reward_dependency =
        compute_crown_reward_dependency(reward_dependence, deck_maturity, next_act);
    let crown_maturity_buffer =
        compute_crown_maturity_buffer(deck_maturity, gap_pressure, &profile);
    let potion_dependence = compute_potion_dependence(
        need.survival_pressure,
        gap_pressure,
        current_potion_quality,
        empty_potion_slots,
    );
    let has_omamori = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Omamori && !relic.used_up && relic.counter > 0);
    let curse_tolerance = clamp_feature(
        20 + profile.exhaust_outlets * 10
            + profile.draw_sources * 3
            + if has_omamori { 20 } else { 0 }
            - need.purge_pressure / 4,
    );
    let volatility_tolerance = compute_volatility_tolerance(
        starter_cards,
        deck_maturity,
        need.hp_ratio,
        need.survival_pressure,
    );
    let pandora_rebuild_value = compute_pandora_rebuild_value(
        transform_targets_value,
        starter_cards,
        deck_maturity,
        next_act,
    );
    let pandora_plan_disruption_risk = compute_pandora_plan_disruption_risk(
        deck_maturity,
        volatility_tolerance,
        hand_clog_risk,
        starter_cards,
    );
    let sustain_strength = clamp_feature(
        sustain_relic_bonus(run_state)
            + if run_state.player_class == "Ironclad" {
                12
            } else {
                0
            }
            + sustain_potion_bonus(&held_potions),
    );
    let campfire_heal_dependence = clamp_feature(
        need.survival_pressure / 2
            + match rest_distance {
                Some(distance) if distance <= 2 => 16,
                Some(distance) if distance <= 4 => 8,
                _ => 2,
            }
            + if need.hp_ratio < 0.60 { 12 } else { 0 }
            - sustain_strength / 3,
    );
    let upgrade_backlog = clamp_feature(
        need.upgrade_pressure
            + match rest_distance {
                Some(distance) if distance <= 2 => 12,
                Some(distance) if distance <= 4 => 6,
                _ => 0,
            }
            - deck_maturity / 5,
    );
    let shop_value_pressure = clamp_feature(
        need.gold_reserve / 2
            + reward_dependence / 3
            + match shop_distance {
                Some(distance) if distance <= 2 => 16,
                Some(distance) if distance <= 4 => 8,
                _ => 0,
            },
    );
    let potion_lock_pressure = clamp_feature(
        potion_dependence
            + current_potion_quality / 3
            + if empty_potion_slots == 0 { 6 } else { 0 },
    );
    let enemy_buff_risk = clamp_feature(
        need.survival_pressure / 3
            + if need.hp_ratio < 0.60 { 14 } else { 0 }
            + if next_act >= 3 { 8 } else { 0 }
            - profile.attack_count.min(10) * 2,
    );
    let wound_absorption = clamp_feature(
        profile.exhaust_outlets * 12 + hand_conversion_ability / 3 + profile.draw_sources * 3
            - hand_clog_risk / 2,
    );

    BossRelicContext {
        need,
        profile,
        next_act,
        rest_distance,
        elite_distance,
        missing_keys,
        is_final_act_available: run_state.is_final_act_available,
        two_plus_cost_count,
        avg_cost_times_10,
        expensive_cards,
        starter_cards,
        empty_potion_slots,
        draw_sources: profile.draw_sources,
        exhaust_outlets: profile.exhaust_outlets,
        attack_count: profile.attack_count,
        block_core: profile.block_core,
        energy_sink_value,
        high_action_dependence,
        hand_clog_risk,
        pyramid_retention_quality,
        pyramid_cleanup_capacity,
        pyramid_clog_liability,
        snecko_draw_bonus,
        snecko_cost_randomization_benefit,
        snecko_cost_randomization_risk,
        reward_dependence,
        deck_maturity,
        crown_reward_dependency,
        crown_maturity_buffer,
        potion_dependence,
        current_potion_quality,
        curse_tolerance,
        volatility_tolerance,
        pandora_rebuild_value,
        pandora_plan_disruption_risk,
        sustain_strength,
        campfire_heal_dependence,
        upgrade_backlog,
        shop_value_pressure,
        potion_lock_pressure,
        enemy_buff_risk,
        wound_absorption,
        remove_targets_value,
        transform_targets_value,
        has_omamori,
    }
}

fn removable_pain(run_state: &RunState, card_id: crate::content::cards::CardId) -> i32 {
    let score_penalty = (24 - score_owned_card(card_id, run_state)).max(0);
    curse_remove_severity(card_id) * 24
        + i32::from(cards::is_starter_basic(card_id)) * 18
        + score_penalty
}

fn sustain_relic_bonus(run_state: &RunState) -> i32 {
    run_state
        .relics
        .iter()
        .map(|relic| match relic.id {
            RelicId::BurningBlood => 12,
            RelicId::BlackBlood => 22,
            RelicId::BloodVial => 8,
            RelicId::EternalFeather => 8,
            RelicId::MealTicket => 6,
            RelicId::MeatOnTheBone => 12,
            RelicId::ToyOrnithopter => 6,
            _ => 0,
        })
        .sum()
}

fn sustain_potion_bonus(held_potions: &[PotionId]) -> i32 {
    held_potions
        .iter()
        .map(|potion_id| match potion_id {
            PotionId::BloodPotion | PotionId::FairyPotion | PotionId::FruitJuice => 8,
            PotionId::RegenPotion => 10,
            _ => 0,
        })
        .sum()
}

fn shell_closure_signal(profile: &DeckProfile) -> i32 {
    profile.draw_sources * 5
        + profile.block_core * 4
        + profile.power_count * 3
        + profile.strength_payoffs * 4
        + profile.status_payoffs * 3
}

fn compute_deck_maturity(
    gap_pressure: i32,
    shell_closure_signal: i32,
    starter_drag: i32,
    consistency_support: i32,
) -> i32 {
    clamp_feature(80 - gap_pressure + shell_closure_signal + consistency_support - starter_drag)
}

fn compute_reward_dependence(gap_pressure: i32, deck_maturity: i32, next_act: u8) -> i32 {
    let act_pressure = if next_act <= 2 { 12 } else { 4 };
    let immaturity_pressure = (55 - deck_maturity).max(0);
    clamp_feature(18 + gap_pressure + immaturity_pressure + act_pressure)
}

fn compute_potion_dependence(
    survival_pressure: i32,
    gap_pressure: i32,
    current_potion_quality: i32,
    empty_potion_slots: i32,
) -> i32 {
    clamp_feature(
        survival_pressure / 3 + gap_pressure / 2 + current_potion_quality / 4
            - empty_potion_slots * 10,
    )
}

fn compute_volatility_tolerance(
    starter_cards: i32,
    deck_maturity: i32,
    hp_ratio: f32,
    survival_pressure: i32,
) -> i32 {
    let hp_buffer_bonus = if hp_ratio >= 0.65 { 10 } else { 0 };
    let immaturity_bonus = (50 - deck_maturity).max(0);
    clamp_feature(
        20 + starter_cards * 7 + immaturity_bonus + hp_buffer_bonus - survival_pressure / 5,
    )
}

fn compute_pyramid_retention_quality(
    retention_quality: i32,
    expensive_cards: i32,
    block_core: i32,
    status_generators: i32,
    x_cost_count: i32,
) -> i32 {
    clamp_feature(
        retention_quality + expensive_cards * 2 + block_core * 3 + x_cost_count * 5
            - status_generators * 2,
    )
}

fn compute_pyramid_cleanup_capacity(
    hand_conversion_ability: i32,
    exhaust_outlets: i32,
    draw_sources: i32,
    zero_cost_count: i32,
    one_cost_count: i32,
    starter_cards: i32,
) -> i32 {
    clamp_feature(
        hand_conversion_ability
            + exhaust_outlets * 8
            + draw_sources * 4
            + zero_cost_count * 3
            + one_cost_count
            - starter_cards * 2,
    )
}

fn compute_pyramid_clog_liability(
    hand_clog_risk: i32,
    status_generators: i32,
    two_plus_cost_count: i32,
    exhaust_outlets: i32,
    draw_sources: i32,
    x_cost_count: i32,
) -> i32 {
    clamp_feature(
        hand_clog_risk + status_generators * 6 + two_plus_cost_count * 3 + x_cost_count * 2
            - exhaust_outlets * 8
            - draw_sources * 3,
    )
}

fn compute_snecko_draw_bonus(draw_sources: i32, attack_count: i32) -> i32 {
    clamp_feature(20 + (6 - draw_sources).max(0) * 8 + attack_count.min(8))
}

fn compute_snecko_cost_randomization_benefit(
    energy_sink_value: i32,
    x_cost_count: i32,
    two_plus_cost_count: i32,
    avg_cost_times_10: i32,
    draw_sources: i32,
) -> i32 {
    clamp_feature(
        energy_sink_value
            + x_cost_count * 6
            + two_plus_cost_count * 4
            + avg_cost_times_10
            + (5 - draw_sources).max(0) * 3,
    )
}

fn compute_snecko_cost_randomization_risk(
    cost_precision_dependence: i32,
    zero_cost_count: i32,
    one_cost_count: i32,
    starter_cards: i32,
    expensive_cards: i32,
    x_cost_count: i32,
) -> i32 {
    clamp_feature(
        cost_precision_dependence + zero_cost_count * 8 + one_cost_count * 4 + starter_cards * 2
            - expensive_cards * 3
            - x_cost_count * 4,
    )
}

fn compute_crown_reward_dependency(
    reward_dependence: i32,
    deck_maturity: i32,
    next_act: u8,
) -> i32 {
    let act_pressure = if next_act <= 2 { 10 } else { 4 };
    let maturity_gap = (60 - deck_maturity).max(0);
    clamp_feature(reward_dependence + maturity_gap / 2 + act_pressure)
}

fn compute_crown_maturity_buffer(
    deck_maturity: i32,
    gap_pressure: i32,
    profile: &DeckProfile,
) -> i32 {
    clamp_feature(
        deck_maturity
            + profile.draw_sources * 3
            + profile.exhaust_outlets * 3
            + profile.block_core * 2
            - gap_pressure / 2,
    )
}

fn compute_pandora_rebuild_value(
    transform_targets_value: i32,
    starter_cards: i32,
    deck_maturity: i32,
    next_act: u8,
) -> i32 {
    let act_window_bonus = if next_act <= 2 { 10 } else { 4 };
    let immaturity_bonus = (55 - deck_maturity).max(0) / 2;
    clamp_feature(
        transform_targets_value / 2 + starter_cards * 10 + immaturity_bonus + act_window_bonus,
    )
}

fn compute_pandora_plan_disruption_risk(
    deck_maturity: i32,
    volatility_tolerance: i32,
    hand_clog_risk: i32,
    starter_cards: i32,
) -> i32 {
    clamp_feature(
        deck_maturity / 2 + hand_clog_risk / 2 + (2 - starter_cards).max(0) * 16
            - volatility_tolerance / 3,
    )
}

fn clamp_feature(value: i32) -> i32 {
    value.clamp(0, 100)
}
