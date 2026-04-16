use crate::bot::card_disposition::{
    best_exhaust_candidate_uuid, combat_exhaust_score_for_uuid, combat_retention_score_for_uuid,
    count_remaining_low_value_exhaust_candidates, exhaust_disposition_stats,
};
use crate::bot::card_taxonomy::{is_multi_attack_payoff, is_strength_payoff};
use crate::bot::combat_families::apotheosis::apotheosis_timing_score;
use crate::bot::combat_families::apparition::{
    apparition_hand_shaping_score, apparition_timing_score, ApparitionTimingContext,
};
use crate::bot::combat_families::draw::{
    battle_trance_timing_score, deck_cycle_thinning_score, draw_action_timing_score,
    draw_continuity_score, status_loop_cycle_score, DrawTimingContext,
};
use crate::bot::combat_families::exhaust::{
    exhaust_engine_setup_score, exhaust_fuel_value_score, exhaust_future_fuel_reserve_score,
    exhaust_mass_play_score, exhaust_random_core_risk_score, exhaust_random_play_score,
    mass_exhaust_base_score, mass_exhaust_keeper_penalty,
    mass_exhaust_second_wind_selectivity_score, MassExhaustProfile,
};
use crate::bot::combat_families::survival::{
    body_slam_delay_score, exhaust_finish_window_score, flight_break_progress_score,
    persistent_block_progress_score, reaper_hand_shaping_score, reaper_timing_score,
    SurvivalTimingContext,
};
use crate::bot::combat_posture::posture_features;
use crate::bot::monster_belief::build_combat_belief_state;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatState;
use crate::runtime::combat::PowerId;
use crate::state::core::ClientInput;
use serde_json::{json, Value};

use super::intent_hits;
use super::root_policy::same_turn_exhaust_setup_bonus_excluding;

pub(crate) fn tactical_move_bonus(combat: &CombatState, chosen_move: &ClientInput) -> f32 {
    tactical_bonus_breakdown(combat, chosen_move)
        .iter()
        .map(|(_, value)| *value)
        .sum()
}

pub(crate) fn decision_audit_json(combat: &CombatState, chosen_move: &ClientInput) -> Value {
    let components = tactical_bonus_breakdown(combat, chosen_move)
        .into_iter()
        .map(|(name, value)| {
            json!({
                "name": name,
                "value": value,
            })
        })
        .collect::<Vec<_>>();
    let total = components
        .iter()
        .map(|component| component["value"].as_f64().unwrap_or(0.0) as f32)
        .sum::<f32>();
    json!({
        "total": total,
        "components": components,
    })
}

fn tactical_bonus_breakdown(
    combat: &CombatState,
    chosen_move: &ClientInput,
) -> Vec<(&'static str, f32)> {
    match chosen_move {
        ClientInput::PlayCard { card_index, target } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return Vec::new();
            };
            vec![
                ("armaments", armaments_move_bonus(combat, *card_index)),
                ("double_tap", double_tap_move_bonus(combat, *card_index)),
                ("feel_no_pain", feel_no_pain_move_bonus(combat, *card_index)),
                ("dark_embrace", dark_embrace_move_bonus(combat, *card_index)),
                ("corruption", corruption_move_bonus(combat, *card_index)),
                ("evolve", evolve_move_bonus(combat, *card_index)),
                ("deep_breath", deep_breath_move_bonus(combat, *card_index)),
                (
                    "exhaust_timing",
                    exhaust_timing_move_bonus(combat, *card_index),
                ),
                (
                    "battle_trance",
                    battle_trance_move_bonus(combat, *card_index),
                ),
                ("generic_draw", generic_draw_move_bonus(combat, *card_index)),
                (
                    "resource_conversion",
                    resource_conversion_move_bonus(combat, *card_index),
                ),
                ("body_slam", body_slam_move_bonus(combat, *card_index)),
                (
                    "flame_barrier",
                    flame_barrier_move_bonus(combat, *card_index),
                ),
                ("limit_break", limit_break_move_bonus(combat, *card_index)),
                ("hold_commit", hold_commit_timing_bonus(combat, *card_index)),
                (
                    "survival_swing",
                    survival_swing_move_bonus(combat, *card_index),
                ),
                (
                    "target_progress",
                    target_progress_move_bonus(combat, *card_index, *target),
                ),
                ("posture", posture_move_bonus(combat, *card_index)),
                (
                    "gremlin_nob_penalty",
                    gremlin_nob_skill_penalty(combat, card),
                ),
                ("slimed_cleanup", slimed_cleanup_move_bonus(combat, card)),
                (
                    "sharp_hide_penalty",
                    sharp_hide_attack_penalty(combat, card, *target),
                ),
                (
                    "slime_boss_split",
                    slime_boss_split_timing_bonus(combat, card, *target),
                ),
                (
                    "guardian_phase",
                    guardian_phase_timing_bonus(combat, card, *target),
                ),
            ]
            .into_iter()
            .filter(|(_, value)| *value != 0.0)
            .collect()
        }
        _ => Vec::new(),
    }
}

fn armaments_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::Armaments {
        return 0.0;
    }

    let candidates: Vec<_> = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, c)| {
            *idx != card_index
                && c.upgrades == 0
                && !matches!(
                    get_card_definition(c.id).card_type,
                    CardType::Status | CardType::Curse
                )
        })
        .collect();

    if candidates.is_empty() {
        return 0.0;
    }

    if card.upgrades > 0 {
        return 7_500.0 + candidates.len() as f32 * 1_500.0;
    }

    let best_upgrade_value = candidates
        .iter()
        .map(|(_, c)| armaments_upgrade_value(c.id))
        .max()
        .unwrap_or(0);

    3_500.0 + best_upgrade_value as f32
}

fn double_tap_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::DoubleTap {
        return 0.0;
    }
    if combat.get_power(0, PowerId::DoubleTap) > 0 {
        return -7_000.0;
    }

    let energy_left = combat.turn.energy as i32 - card.get_cost() as i32;
    if energy_left <= 0 {
        return -4_000.0;
    }

    let best_followup = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, other)| {
            *idx != card_index
                && get_card_definition(other.id).card_type == CardType::Attack
                && other.get_cost() as i32 >= 0
                && other.get_cost() as i32 <= energy_left
        })
        .map(|(_, other)| double_tap_followup_value(combat, other))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    if best_followup <= 0.0 {
        -4_500.0
    } else {
        2_500.0 + best_followup
    }
}

fn feel_no_pain_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::FeelNoPain {
        return 0.0;
    }
    if combat.get_power(0, PowerId::FeelNoPain) > 0 {
        return -8_000.0;
    }

    exhaust_engine_setup_score(
        false,
        immediate_exhaust_count(combat),
        future_exhaust_source_count(combat, card_index),
        4,
        0,
        status_cards_in_hand(combat),
        future_status_card_count(combat),
        sentry_count(combat),
        0,
    ) as f32
}

fn dark_embrace_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::DarkEmbrace {
        return 0.0;
    }
    if combat.get_power(0, PowerId::DarkEmbrace) > 0 {
        return -8_000.0;
    }

    exhaust_engine_setup_score(
        false,
        immediate_exhaust_count(combat),
        future_exhaust_source_count(combat, card_index),
        0,
        1,
        status_cards_in_hand(combat),
        future_status_card_count(combat),
        sentry_count(combat),
        same_turn_exhaust_setup_bonus(combat, card_index, true),
    ) as f32
}

fn resource_conversion_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    match card.id {
        CardId::Bloodletting => resource_conversion_bonus_inner(combat, card_index, 3),
        CardId::SeeingRed => resource_conversion_bonus_inner(combat, card_index, 0),
        _ => 0.0,
    }
}

fn posture_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    let posture = posture_features(combat);

    match card.id {
        CardId::FireBreathing => {
            posture.future_pollution_risk as f32 * 700.0
                + posture.expected_fight_length_bucket as f32 * 1_500.0
                + posture.setup_payoff_density as f32 * 500.0
                - posture.immediate_survival_pressure.min(24) as f32 * 120.0
        }
        CardId::DarkEmbrace => {
            posture.future_pollution_risk as f32 * 320.0
                + posture.expected_fight_length_bucket as f32 * 900.0
                + posture.setup_payoff_density as f32 * 420.0
                - posture.immediate_survival_pressure.min(20) as f32 * 140.0
        }
        CardId::PowerThrough => {
            posture.immediate_survival_pressure.min(30) as f32 * 250.0
                - posture.resource_preservation_pressure as f32 * 820.0
        }
        _ => 0.0,
    }
}

fn corruption_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::Corruption {
        return 0.0;
    }
    if combat.get_power(0, PowerId::Corruption) > 0 {
        return -8_000.0;
    }

    let skill_targets = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, c)| {
            *idx != card_index
                && get_card_definition(c.id).card_type == CardType::Skill
                && c.get_cost() as i32 > 0
        })
        .count() as i32;

    exhaust_engine_setup_score(
        false,
        immediate_exhaust_count(combat),
        future_exhaust_source_count(combat, card_index),
        combat.get_power(0, PowerId::FeelNoPain),
        i32::from(combat.get_power(0, PowerId::DarkEmbrace) > 0),
        status_cards_in_hand(combat),
        future_status_card_count(combat),
        sentry_count(combat),
        skill_targets * 1_000,
    ) as f32
}

fn resource_conversion_bonus_inner(combat: &CombatState, card_index: usize, hp_cost: i32) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    let post_energy = (combat.turn.energy as i32 - card.get_cost() as i32
        + get_card_definition(card.id).base_magic
        + get_card_definition(card.id).upgrade_magic * card.upgrades as i32)
        .max(0);
    let imminent = combat_unblocked_incoming_damage(combat);

    let mut playable_before = 0;
    let mut playable_after = 0;
    let mut newly_unlocked = 0;
    let mut unlocked_damage = 0;
    let mut unlocked_block = 0;
    let mut key_followups = 0;

    for (idx, other) in combat.zones.hand.iter().enumerate() {
        if idx == card_index
            || matches!(
                get_card_definition(other.id).card_type,
                CardType::Curse | CardType::Status
            )
        {
            continue;
        }
        let other_cost = other.get_cost() as i32;
        let before = other_cost >= 0 && other_cost <= combat.turn.energy as i32;
        let after = other_cost >= 0 && other_cost <= post_energy;
        if before {
            playable_before += 1;
        }
        if after {
            playable_after += 1;
        }
        if !before && after {
            newly_unlocked += 1;
            let def = get_card_definition(other.id);
            let upgrades = other.upgrades as i32;
            unlocked_damage += (def.base_damage + def.upgrade_damage * upgrades).max(0)
                * intent_hits_like(other.id, upgrades);
            unlocked_block += (def.base_block + def.upgrade_block * upgrades).max(0);
            key_followups += match other.id {
                CardId::Carnage
                | CardId::Offering
                | CardId::Impervious
                | CardId::DarkEmbrace
                | CardId::FeelNoPain
                | CardId::Corruption
                | CardId::Shockwave
                | CardId::FlameBarrier
                | CardId::Bash
                | CardId::Clothesline
                | CardId::Uppercut => 1,
                _ => 0,
            };
        }
    }

    let mut bonus = 1_000.0
        + newly_unlocked as f32 * 3_200.0
        + unlocked_damage.min(30) as f32 * 220.0
        + unlocked_block.min(imminent.max(0)).min(20) as f32 * 250.0
        + key_followups as f32 * 1_500.0;

    if newly_unlocked <= 0 {
        bonus -= 5_500.0;
    }
    if playable_after <= 0 {
        bonus -= 8_000.0;
    }
    if playable_before > 0 && newly_unlocked <= 0 {
        bonus -= 2_000.0;
    }
    if hp_cost > 0 {
        bonus -= hp_cost as f32 * 1_000.0;
        if imminent >= combat.entities.player.current_hp.saturating_sub(4) {
            bonus -= hp_cost as f32 * 1_300.0 + 1_800.0;
        } else if combat.entities.player.current_hp <= 12 {
            bonus -= hp_cost as f32 * 900.0;
        }
    }

    bonus
}

fn evolve_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::Evolve {
        return 0.0;
    }
    if combat.get_power(0, PowerId::Evolve) > 0 {
        return -8_000.0;
    }

    let status_loop = status_loop_cycle_score(
        1,
        status_cards_in_draw_pile(combat),
        status_cards_in_discard_pile(combat),
        true,
        0,
        sentry_count(combat),
    );
    let deep_breath_synergy = i32::from(
        combat
            .zones
            .hand
            .iter()
            .chain(combat.zones.draw_pile.iter())
            .any(|c| c.id == CardId::DeepBreath),
    ) * 1_600;

    (4_000 + status_loop + deep_breath_synergy) as f32
}

fn deep_breath_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::DeepBreath {
        return 0.0;
    }

    let status_loop = status_loop_cycle_score(
        i32::from(combat.get_power(0, PowerId::Evolve) > 0),
        status_cards_in_draw_pile(combat),
        status_cards_in_discard_pile(combat),
        true,
        1,
        sentry_count(combat),
    );
    let cycle = draw_continuity_score(
        total_cycle_cards(combat),
        1,
        0,
        combat.zones.discard_pile.len() as i32,
    );

    (status_loop + cycle) as f32
}

fn battle_trance_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::BattleTrance {
        return 0.0;
    }

    let future_drawable_cards: Vec<_> = combat
        .zones
        .draw_pile
        .iter()
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            !matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .collect();
    let future_zero_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() <= 0)
        .count() as i32;
    let future_one_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() == 1)
        .count() as i32;
    let future_two_plus_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() >= 2)
        .count() as i32;
    let other_draw_sources_in_hand = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, other)| *idx != card_index && is_draw_source(other.id))
        .count() as i32;

    battle_trance_timing_score(
        &DrawTimingContext {
            current_energy: combat.turn.energy as i32,
            player_no_draw: combat.get_power(0, PowerId::NoDraw) != 0,
            current_hand_size: combat.zones.hand.len() as i32,
            future_zero_cost_cards,
            future_one_cost_cards,
            future_two_plus_cost_cards,
            future_key_delay_weight: future_drawable_cards
                .iter()
                .map(|card| key_card_delay_weight(card.id))
                .sum(),
            future_high_cost_key_delay_weight: future_drawable_cards
                .iter()
                .filter(|card| card.get_cost() >= 1)
                .map(|card| key_card_delay_weight(card.id))
                .sum(),
            future_status_cards: status_cards_in_draw_pile(combat)
                + status_cards_in_discard_pile(combat),
            other_draw_sources_in_hand,
        },
        get_card_definition(card.id).base_magic
            + get_card_definition(card.id).upgrade_magic * card.upgrades as i32,
    ) as f32
}

fn generic_draw_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    let draw_count = match card.id {
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::Finesse
        | CardId::FlashOfSteel => 1,
        CardId::MasterOfStrategy => {
            let def = get_card_definition(card.id);
            def.base_magic + def.upgrade_magic * card.upgrades as i32
        }
        CardId::Offering => 3,
        _ => return 0.0,
    };

    let future_drawable_cards: Vec<_> = combat
        .zones
        .draw_pile
        .iter()
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            !matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .collect();
    let future_zero_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() <= 0)
        .count() as i32;
    let future_one_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() == 1)
        .count() as i32;
    let future_two_plus_cost_cards = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() >= 2)
        .count() as i32;
    let other_draw_sources_in_hand = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, other)| *idx != card_index && is_draw_source(other.id))
        .count() as i32;

    draw_action_timing_score(
        &DrawTimingContext {
            current_energy: combat.turn.energy as i32,
            player_no_draw: combat.get_power(0, PowerId::NoDraw) != 0,
            current_hand_size: combat.zones.hand.len() as i32,
            future_zero_cost_cards,
            future_one_cost_cards,
            future_two_plus_cost_cards,
            future_key_delay_weight: future_drawable_cards
                .iter()
                .map(|card| key_card_delay_weight(card.id))
                .sum(),
            future_high_cost_key_delay_weight: future_drawable_cards
                .iter()
                .filter(|card| card.get_cost() >= 1)
                .map(|card| key_card_delay_weight(card.id))
                .sum(),
            future_status_cards: status_cards_in_draw_pile(combat)
                + status_cards_in_discard_pile(combat),
            other_draw_sources_in_hand,
        },
        false,
        draw_count,
    ) as f32
}

fn is_draw_source(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BattleTrance
            | CardId::MasterOfStrategy
            | CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::Warcry
            | CardId::Finesse
            | CardId::FlashOfSteel
            | CardId::Offering
            | CardId::DeepBreath
    )
}

fn key_card_delay_weight(card_id: CardId) -> i32 {
    match card_id {
        CardId::Apparition
        | CardId::LimitBreak
        | CardId::Corruption
        | CardId::Barricade
        | CardId::DemonForm
        | CardId::Impervious
        | CardId::Reaper
        | CardId::SearingBlow => 4,
        CardId::Offering => 1,
        CardId::DarkEmbrace
        | CardId::FeelNoPain
        | CardId::BurningPact
        | CardId::BodySlam
        | CardId::PowerThrough
        | CardId::FlameBarrier
        | CardId::GhostlyArmor
        | CardId::HeavyBlade
        | CardId::Exhume
        | CardId::BattleTrance => 3,
        CardId::ShrugItOff
        | CardId::PommelStrike
        | CardId::Disarm
        | CardId::Shockwave
        | CardId::Armaments
        | CardId::Warcry
        | CardId::SeeingRed => 2,
        _ => 0,
    }
}

fn intent_hits_like(card_id: CardId, upgrades: i32) -> i32 {
    match card_id {
        CardId::TwinStrike => 2,
        CardId::Pummel => 4 + upgrades,
        CardId::SwordBoomerang => {
            let def = get_card_definition(card_id);
            def.base_magic + def.upgrade_magic * upgrades
        }
        _ => 1,
    }
}

fn same_turn_exhaust_setup_bonus(
    combat: &CombatState,
    card_index: usize,
    draw_engine: bool,
) -> i32 {
    same_turn_exhaust_setup_bonus_excluding(combat, card_index, draw_engine)
}

fn exhaust_timing_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };

    match card.id {
        CardId::BurningPact => {
            let targets = exhaust_candidate_uuids_for_card(combat, card_index);
            let best_fuel = exhaust_candidate_uuids_for_card(combat, card_index)
                .into_iter()
                .map(|uuid| exhaust_fuel_value_for_uuid(combat, uuid))
                .max()
                .unwrap_or(-6_000);
            let bad_fuel_in_hand = targets
                .iter()
                .filter_map(|uuid| combat.zones.hand.iter().find(|c| c.uuid == *uuid))
                .filter(|c| {
                    matches!(
                        c.id,
                        CardId::Burn
                            | CardId::Dazed
                            | CardId::Slimed
                            | CardId::Wound
                            | CardId::Injury
                    )
                })
                .count() as i32;
            let remaining_cards_after = combat.zones.draw_pile.len() as i32
                + combat.zones.discard_pile.len() as i32
                + combat.zones.hand.len() as i32
                - 1;
            let future_save_cards = combat
                .zones
                .draw_pile
                .iter()
                .chain(combat.zones.discard_pile.iter())
                .filter(|c| {
                    matches!(
                        c.id,
                        CardId::Defend
                            | CardId::DefendG
                            | CardId::ShrugItOff
                            | CardId::FlameBarrier
                            | CardId::GhostlyArmor
                            | CardId::PowerThrough
                            | CardId::Impervious
                            | CardId::Apparition
                            | CardId::Offering
                            | CardId::BurningPact
                            | CardId::SecondWind
                    )
                })
                .count() as i32;
            let imminent = combat_unblocked_incoming_damage(combat);
            let mut value = exhaust_mass_play_score(
                best_fuel,
                1,
                remaining_cards_after,
                remaining_low_value_fuel_after_best_single_exhaust(combat, card_index),
                0,
            ) as f32
                + exhaust_future_fuel_reserve_score(
                    remaining_low_value_fuel_after_best_single_exhaust(combat, card_index),
                    future_exhaust_demand(combat, card_index),
                ) as f32
                + deck_cycle_thinning_score(
                    total_cycle_cards(combat),
                    remaining_cards_after,
                    2 + i32::from(combat.get_power(0, PowerId::DarkEmbrace) > 0),
                    0,
                    0,
                    0,
                ) as f32;
            if best_fuel <= 0 {
                value -= 4_500.0;
            }
            if bad_fuel_in_hand > 0 {
                value += (bad_fuel_in_hand * 3_000) as f32;
            }
            if imminent > 0 {
                value += (2_000 + imminent.min(24) * 240) as f32;
                value += (future_save_cards.min(4) * 1_100) as f32;
                if bad_fuel_in_hand > 0 {
                    value += 2_500.0;
                }
            } else {
                value += (future_save_cards.min(3) * 350) as f32;
            }
            if combat.get_power(0, PowerId::DarkEmbrace) > 0
                || combat.get_power(0, PowerId::FeelNoPain) > 0
            {
                value += 1_200.0;
            }
            value
        }
        CardId::SecondWind => {
            let targets = exhaust_candidate_uuids_for_card(combat, card_index);
            if targets.is_empty() {
                return -4_500.0;
            }
            let finish_window = second_wind_finish_window_bonus(combat, targets.len() as i32);
            let profile = build_mass_exhaust_profile(
                combat,
                card_index,
                &targets,
                finish_window,
                finish_window > 0,
            );
            (mass_exhaust_base_score(&profile, total_cycle_cards(combat))
                + mass_exhaust_second_wind_selectivity_score(&profile)) as f32
        }
        CardId::SeverSoul => {
            let targets = exhaust_candidate_uuids_for_card(combat, card_index);
            if targets.is_empty() {
                return -2_500.0;
            }
            let profile = build_mass_exhaust_profile(combat, card_index, &targets, 0, false);
            ((mass_exhaust_base_score(&profile, total_cycle_cards(combat))
                - mass_exhaust_keeper_penalty(&profile, 500, 3_000)) as f32)
                * 0.8
        }
        CardId::FiendFire => {
            let targets = exhaust_candidate_uuids_for_card(combat, card_index);
            if targets.is_empty() {
                return -3_500.0;
            }
            let closeout_bonus = fiend_fire_closeout_bonus(combat, card, targets.len() as i32);
            let profile =
                build_mass_exhaust_profile(combat, card_index, &targets, closeout_bonus, false);
            (mass_exhaust_base_score(&profile, total_cycle_cards(combat))
                - mass_exhaust_keeper_penalty(&profile, 300, 2_200)) as f32
        }
        CardId::TrueGrit if card.upgrades == 0 => {
            let candidates = exhaust_candidate_uuids_for_card(combat, card_index);
            let stats = exhaust_disposition_stats(combat, &candidates);
            let remaining_cards_after = combat.zones.draw_pile.len() as i32
                + combat.zones.discard_pile.len() as i32
                + combat.zones.hand.len() as i32
                - 1;
            exhaust_random_play_score(
                stats.junk_count,
                stats.protected_count,
                remaining_cards_after,
            ) as f32
                + exhaust_random_core_risk_score(
                    stats.junk_count,
                    stats.core_count,
                    stats.near_core_count,
                ) as f32
                + exhaust_future_fuel_reserve_score(
                    remaining_low_value_fuel_after_best_single_exhaust(combat, card_index),
                    future_exhaust_demand(combat, card_index),
                ) as f32
                + deck_cycle_thinning_score(
                    total_cycle_cards(combat),
                    remaining_cards_after,
                    i32::from(combat.get_power(0, PowerId::DarkEmbrace) > 0),
                    0,
                    0,
                    0,
                ) as f32
        }
        _ => 0.0,
    }
}

fn limit_break_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::LimitBreak {
        return 0.0;
    }

    let strength = combat.get_power(0, PowerId::Strength);
    if strength <= 0 {
        return -7_000.0;
    }

    let energy_left = combat.turn.energy as i32 - card.get_cost() as i32;
    let best_followup = if energy_left > 0 {
        combat
            .zones
            .hand
            .iter()
            .enumerate()
            .filter(|(idx, other)| {
                *idx != card_index
                    && get_card_definition(other.id).card_type == CardType::Attack
                    && other.get_cost() as i32 >= 0
                    && other.get_cost() as i32 <= energy_left
            })
            .map(|(_, other)| double_tap_followup_value(combat, other))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let mut bonus = strength as f32 * 1_800.0;
    if strength >= 3 {
        bonus += 5_000.0;
    } else if strength == 2 {
        bonus += 1_500.0;
    } else {
        bonus -= 1_000.0;
    }
    bonus += best_followup * 0.5;
    if best_followup <= 0.0 && strength <= 2 {
        bonus -= 2_500.0;
    }
    bonus
}

fn body_slam_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::BodySlam {
        return 0.0;
    }

    let current_damage = combat.entities.player.block.max(0);
    let additional_block = max_additional_block_before_body_slam(combat, card_index);
    let can_kill_now = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .any(|m| current_damage >= m.current_hp.max(0) + m.block.max(0));
    body_slam_delay_score(
        current_damage,
        additional_block,
        can_kill_now,
        combat_unblocked_incoming_damage(combat),
    ) as f32
}

fn flame_barrier_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::FlameBarrier {
        return 0.0;
    }

    let retaliate_per_hit = card.base_magic_num_mut.max(4);
    let retaliation_hits = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| intent_hits(&m.current_intent))
        .sum::<i32>();
    if retaliation_hits <= 0 {
        return if combat_total_incoming_damage(combat) <= combat.entities.player.block {
            -3_500.0
        } else {
            0.0
        };
    }

    let attacking_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .filter(|m| intent_hits(&m.current_intent) > 0)
        .count() as i32;
    let mut bonus = (retaliation_hits * retaliate_per_hit * 180 + attacking_monsters * 900) as f32;
    if combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| intent_hits(&m.current_intent))
        .max()
        .unwrap_or(0)
        >= 2
    {
        bonus += 3_500.0 + (retaliation_hits * retaliate_per_hit * 60) as f32;
    }
    if combat_unblocked_incoming_damage(combat) > 0 {
        bonus += 2_000.0;
    }
    bonus
}

fn hold_commit_timing_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    match card.id {
        CardId::Apotheosis => apotheosis_timing_score(
            combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(idx, c)| {
                    *idx != card_index
                        && c.upgrades == 0
                        && !matches!(
                            get_card_definition(c.id).card_type,
                            CardType::Status | CardType::Curse
                        )
                })
                .count() as i32,
            combat_unblocked_incoming_damage(combat),
        ) as f32,
        CardId::Apparition => {
            apparition_timing_score(&combat_apparition_timing_context(combat, card.upgrades > 0))
                as f32
        }
        CardId::BurningPact => {
            let imminent = combat_unblocked_incoming_damage(combat);
            let bad_fuel = exhaust_candidate_uuids_for_card(combat, card_index)
                .into_iter()
                .filter(|uuid| combat_exhaust_score_for_uuid(combat, *uuid) >= 8_000)
                .count() as i32;
            let future_save_cards = combat
                .zones
                .draw_pile
                .iter()
                .chain(combat.zones.discard_pile.iter())
                .filter(|c| {
                    matches!(
                        c.id,
                        CardId::Defend
                            | CardId::DefendG
                            | CardId::ShrugItOff
                            | CardId::FlameBarrier
                            | CardId::GhostlyArmor
                            | CardId::PowerThrough
                            | CardId::Impervious
                            | CardId::Apparition
                            | CardId::Offering
                            | CardId::BurningPact
                            | CardId::SecondWind
                    )
                })
                .count() as i32;
            let mut score = 0.0;
            if imminent > 0 {
                score += (1_500 + imminent.min(24) * 220) as f32;
                score += (future_save_cards.min(4) * 900) as f32;
            }
            if bad_fuel > 0 {
                score += (bad_fuel * 2_000) as f32;
            }
            score
        }
        CardId::SpotWeakness => {
            let attacking_target_exists = combat.entities.monsters.iter().any(|m| {
                !m.is_dying && !m.is_escaped && !m.half_dead && intent_hits(&m.current_intent) > 0
            });
            if !attacking_target_exists {
                return -5_500.0;
            }
            let followup_attack_count = combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(idx, other)| {
                    *idx != card_index
                        && crate::content::cards::can_play_card(other, combat).is_ok()
                        && get_card_definition(other.id).card_type == CardType::Attack
                })
                .count() as i32;
            let followup_payoff_count = combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(idx, other)| {
                    *idx != card_index
                        && crate::content::cards::can_play_card(other, combat).is_ok()
                        && is_strength_payoff(other.id)
                })
                .count() as i32;
            4_500.0
                + (followup_attack_count * 1_200) as f32
                + (followup_payoff_count * 1_500) as f32
                + if combat.meta.is_boss_fight || combat.meta.is_elite_fight {
                    1_200.0
                } else {
                    0.0
                }
        }
        CardId::Rage => {
            let followup_attack_count = combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(idx, other)| {
                    *idx != card_index
                        && crate::content::cards::can_play_card(other, combat).is_ok()
                        && get_card_definition(other.id).card_type == CardType::Attack
                })
                .count() as i32;
            let multi_attack_count = combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(idx, other)| {
                    *idx != card_index
                        && crate::content::cards::can_play_card(other, combat).is_ok()
                        && is_multi_attack_payoff(other.id)
                })
                .count() as i32;
            if followup_attack_count <= 0 {
                -4_500.0
            } else {
                3_500.0
                    + (followup_attack_count * 1_600) as f32
                    + (multi_attack_count * 2_000) as f32
                    + if combat_unblocked_incoming_damage(combat) > 0 {
                        2_000.0
                    } else {
                        0.0
                    }
            }
        }
        _ => 0.0,
    }
}

fn survival_swing_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    match card.id {
        CardId::Reaper => {
            let heal = estimated_reaper_heal(combat, card);
            let kills = estimated_reaper_kills(combat, card);
            let prevented = estimated_reaper_kill_prevention(combat, card);
            reaper_timing_score(
                &combat_survival_timing_context(combat),
                heal,
                prevented,
                kills,
            ) as f32
        }
        _ => 0.0,
    }
}

fn target_progress_move_bonus(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
) -> f32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0.0;
    };
    if get_card_definition(card.id).card_type != CardType::Attack {
        return 0.0;
    }

    let mut bonus = 0.0;
    if let Some(target_id) = target {
        if let Some(monster) = combat
            .entities
            .monsters
            .iter()
            .find(|m| m.id == target_id && !m.is_dying && !m.is_escaped && !m.half_dead)
        {
            let target_damage = estimated_attack_damage(combat, card) * card_hits(card);
            let flight = combat.get_power(target_id, PowerId::Flight);
            if flight > 0 && target_damage > 0 {
                let prevented = monster.intent_dmg * intent_hits(&monster.current_intent);
                bonus += flight_break_progress_score(card_hits(card), flight, prevented);
            }

            if combat.get_power(target_id, PowerId::Barricade) > 0 && monster.block > 0 {
                bonus += persistent_block_progress_score(target_damage.min(monster.block.max(0)));
            }
        }
    }

    bonus
}

fn double_tap_followup_value(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
) -> f32 {
    let base = estimated_attack_damage(combat, card) * card_hits(card);
    (base as f32 * 300.0)
        + match card.id {
            CardId::Bash
            | CardId::Uppercut
            | CardId::Hemokinesis
            | CardId::BloodForBlood
            | CardId::SwordBoomerang
            | CardId::Pummel
            | CardId::Rampage
            | CardId::HeavyBlade
            | CardId::Whirlwind => 2_000.0,
            _ => 0.0,
        }
}

fn armaments_upgrade_value(card_id: CardId) -> i32 {
    let def = get_card_definition(card_id);
    let mut score = def.upgrade_damage * 180 + def.upgrade_block * 130 + def.upgrade_magic * 210;

    score += match card_id {
        CardId::Bash
        | CardId::Uppercut
        | CardId::Shockwave
        | CardId::BattleTrance
        | CardId::GhostlyArmor
        | CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::FlameBarrier
        | CardId::BodySlam
        | CardId::HeavyBlade
        | CardId::TrueGrit
        | CardId::SecondWind
        | CardId::BurningPact
        | CardId::LimitBreak
        | CardId::SeeingRed
        | CardId::Havoc
        | CardId::BloodForBlood
        | CardId::Exhume => 1_500,
        _ => 0,
    };

    score
}

fn gremlin_nob_skill_penalty(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Skill {
        return 0.0;
    }

    let nob_is_enraged = combat
        .entities
        .monsters
        .iter()
        .any(|m| !m.is_dying && !m.is_escaped && combat.get_power(m.id, PowerId::Anger) != 0);
    if !nob_is_enraged {
        return 0.0;
    }

    let mut penalty = -9_000.0;
    let incoming_damage: i32 = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped)
        .map(|m| match m.current_intent {
            crate::runtime::combat::Intent::Attack { hits, .. }
            | crate::runtime::combat::Intent::AttackBuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDefend { hits, .. } => {
                m.intent_dmg * hits as i32
            }
            _ => 0,
        })
        .sum();
    let threatened = incoming_damage > combat.entities.player.block;

    penalty += match card.id {
        CardId::GhostlyArmor | CardId::ShrugItOff | CardId::FlameBarrier | CardId::Impervious
            if threatened =>
        {
            6_500.0
        }
        CardId::Shockwave | CardId::Disarm if threatened => 4_500.0,
        CardId::Armaments if threatened => 2_500.0,
        _ => 0.0,
    };

    penalty
}

fn sharp_hide_attack_penalty(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Attack {
        return 0.0;
    }

    let Some((
        sharp_hide_owner,
        thorns_damage,
        owner_effective_hp,
        owner_monster_type,
        owner_next_move,
        owner_intent,
    )) = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .find_map(|m| {
            let amount = combat.get_power(m.id, PowerId::SharpHide);
            (amount > 0).then_some((
                m.id,
                amount,
                m.current_hp + m.block,
                m.monster_type,
                m.next_move_byte,
                m.current_intent.clone(),
            ))
        })
    else {
        return 0.0;
    };

    let incoming_damage: i32 = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| match m.current_intent {
            crate::runtime::combat::Intent::Attack { hits, .. }
            | crate::runtime::combat::Intent::AttackBuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDefend { hits, .. } => {
                m.intent_dmg * hits as i32
            }
            _ => 0,
        })
        .sum();
    let attack_damage = estimated_attack_damage(combat, card) * card_hits(card);
    let lethal = target
        .filter(|&tid| tid == sharp_hide_owner)
        .map(|_| attack_damage >= owner_effective_hp)
        .unwrap_or(false);
    let projected_hp = combat.entities.player.current_hp
        - thorns_damage
        - (incoming_damage - combat.entities.player.block).max(0);
    let progress_ratio = if owner_effective_hp > 0 {
        attack_damage as f32 / owner_effective_hp as f32
    } else {
        1.0
    };
    let has_playable_non_attack_alternative = combat.zones.hand.iter().any(|other| {
        other.uuid != card.uuid
            && get_card_definition(other.id).card_type != CardType::Attack
            && other.get_cost() >= 0
            && other.get_cost() as i32 <= combat.turn.energy as i32
    });

    let mut penalty = -6_000.0;
    if lethal {
        penalty += 5_000.0;
    }
    if projected_hp <= 0 && !lethal {
        penalty -= 20_000.0;
    } else if projected_hp <= 8 && !lethal {
        penalty -= 10_000.0;
    } else if combat.entities.player.current_hp <= thorns_damage + 6 && !lethal {
        penalty -= 6_000.0;
    }
    if !lethal && progress_ratio < 0.33 {
        penalty -= 4_500.0;
    }
    if !lethal && has_playable_non_attack_alternative {
        penalty -= 3_000.0;
    }
    if !lethal && owner_monster_type == EnemyId::TheGuardian as usize {
        penalty += guardian_sharp_hide_extra_penalty(
            combat,
            thorns_damage,
            projected_hp,
            owner_next_move,
            &owner_intent,
        );
    }

    penalty
}

fn slimed_cleanup_move_bonus(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
) -> f32 {
    if card.id != CardId::Slimed {
        return 0.0;
    }

    let playable_non_status_count = combat
        .zones
        .hand
        .iter()
        .filter(|other| other.uuid != card.uuid)
        .filter(|other| {
            !matches!(
                get_card_definition(other.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .filter(|other| {
            other.get_cost() >= 0 && other.get_cost() as i32 <= combat.turn.energy as i32
        })
        .count() as i32;
    let slimed_in_hand = combat
        .zones
        .hand
        .iter()
        .filter(|other| other.id == CardId::Slimed)
        .count() as i32;

    let mut bonus = 2_500.0 + (slimed_in_hand.max(1) * 1_200) as f32;
    if playable_non_status_count == 0 {
        bonus += 5_000.0 + combat.turn.energy as f32 * 400.0;
    } else {
        bonus -= 2_000.0;
    }

    bonus
}

fn slime_boss_split_timing_bonus(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Attack {
        return 0.0;
    }

    let Some(target_id) = target else {
        return 0.0;
    };
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target_id && !m.is_dying && !m.is_escaped && !m.half_dead)
    else {
        return 0.0;
    };
    if monster.monster_type != EnemyId::SlimeBoss as usize {
        return 0.0;
    }

    let attack_damage = estimated_attack_damage(combat, card) * card_hits(card);
    if attack_damage <= 0 {
        return 0.0;
    }

    const SPLIT_THRESHOLD: i32 = 70;
    let projected_hp = (monster.current_hp - (attack_damage - monster.block.max(0)).max(0)).max(0);
    let current_hp = monster.current_hp.max(0);
    let imminent = combat_unblocked_incoming_damage(combat);
    let slam_pressure = if matches!(
        monster.current_intent,
        crate::runtime::combat::Intent::Attack { .. }
            | crate::runtime::combat::Intent::AttackBuff { .. }
            | crate::runtime::combat::Intent::AttackDebuff { .. }
            | crate::runtime::combat::Intent::AttackDefend { .. }
    ) {
        monster.intent_dmg * intent_hits(&monster.current_intent)
    } else {
        0
    };
    let forced_split = imminent >= combat.entities.player.current_hp.saturating_sub(6)
        || (slam_pressure >= 30
            && imminent >= combat.entities.player.current_hp.saturating_sub(12));

    if current_hp > SPLIT_THRESHOLD && projected_hp > SPLIT_THRESHOLD {
        return 0.0;
    }

    let mut bonus = 0.0;
    if projected_hp <= SPLIT_THRESHOLD {
        if forced_split {
            bonus += 10_000.0 + (SPLIT_THRESHOLD - projected_hp).max(0) as f32 * 170.0;
            if projected_hp <= 45 {
                bonus += 2_000.0;
            } else if projected_hp >= 60 {
                bonus -= 1_500.0;
            }
        } else if projected_hp <= 35 {
            bonus += 8_500.0;
        } else if projected_hp <= 45 {
            bonus += 3_500.0;
        } else if projected_hp >= 60 {
            bonus -= 8_000.0;
        } else if projected_hp >= 50 {
            bonus -= 3_000.0;
        }
    }

    bonus += (current_hp - projected_hp).max(0) as f32 * 60.0;
    bonus
}

fn guardian_phase_timing_bonus(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Attack {
        return 0.0;
    }

    let Some(target_id) = target else {
        return 0.0;
    };
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target_id && !m.is_dying && !m.is_escaped && !m.half_dead)
    else {
        return 0.0;
    };
    if monster.monster_type != EnemyId::TheGuardian as usize {
        return 0.0;
    }

    let mode_shift_remaining = combat.get_power(target_id, PowerId::ModeShift);
    if mode_shift_remaining <= 0 {
        return 0.0;
    }

    let attack_damage = estimated_attack_damage(combat, card) * card_hits(card);
    if attack_damage <= 0 || attack_damage < mode_shift_remaining {
        return 0.0;
    }

    let guardian_effective_hp = monster.current_hp + monster.block;
    if attack_damage >= guardian_effective_hp {
        return 0.0;
    }

    let incoming = monster.intent_dmg * intent_hits(&monster.current_intent);
    let player_hp = combat.entities.player.current_hp;
    let unblocked_incoming = (incoming - combat.entities.player.block).max(0);
    let shift_hp_buffer = player_hp - unblocked_incoming;

    if incoming >= 20 {
        6_500.0
    } else if incoming >= 10 && shift_hp_buffer <= 18 {
        3_000.0
    } else if shift_hp_buffer <= 10 {
        1_000.0
    } else {
        -4_000.0
    }
}

fn guardian_sharp_hide_extra_penalty(
    combat: &CombatState,
    thorns_damage: i32,
    projected_hp: i32,
    owner_next_move: u8,
    owner_intent: &crate::runtime::combat::Intent,
) -> f32 {
    let mut penalty = -2_500.0;
    let incoming = combat_total_incoming_damage(combat);
    let unblocked = combat_unblocked_incoming_damage(combat);

    if owner_next_move == 4
        || matches!(
            owner_intent,
            crate::runtime::combat::Intent::AttackBuff { .. }
        )
    {
        // Twin Slam is the clearest "just wait one enemy turn and reflect disappears" window.
        penalty -= 5_000.0;
    } else if owner_next_move == 3
        || matches!(owner_intent, crate::runtime::combat::Intent::Attack { .. })
    {
        penalty -= 2_000.0;
    }

    if projected_hp <= 12 {
        penalty -= 4_000.0;
    }
    if thorns_damage >= 3 && combat.entities.player.current_hp <= incoming + 12 {
        penalty -= 2_500.0;
    }
    if unblocked <= 0 {
        // If we can comfortably block this turn, eating reflect is even less attractive.
        penalty -= 1_500.0;
    }

    penalty
}

fn estimated_attack_damage(combat: &CombatState, card: &crate::runtime::combat::CombatCard) -> i32 {
    let def = get_card_definition(card.id);
    let mut damage = match card.id {
        CardId::BodySlam => combat.entities.player.block,
        CardId::HeavyBlade => {
            let mult = card.base_magic_num_mut.max(def.base_magic);
            card.base_damage_mut + combat.get_power(0, PowerId::Strength) * mult
        }
        _ => card.base_damage_mut + combat.get_power(0, PowerId::Strength),
    };
    if combat.get_power(0, PowerId::Weak) > 0 {
        damage = (damage as f32 * 0.75).floor() as i32;
    }
    damage.max(0)
}

fn exhaust_candidate_uuids_for_card(combat: &CombatState, card_index: usize) -> Vec<u32> {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return Vec::new();
    };

    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != card_index)
        .filter(|(_, other)| match card.id {
            CardId::SecondWind | CardId::SeverSoul => {
                get_card_definition(other.id).card_type != CardType::Attack
            }
            CardId::FiendFire | CardId::BurningPact | CardId::TrueGrit => true,
            _ => false,
        })
        .map(|(_, other)| other.uuid)
        .collect()
}

fn exhaust_fuel_value_for_uuid(combat: &CombatState, uuid: u32) -> i32 {
    let Some(card) = combat.zones.hand.iter().find(|c| c.uuid == uuid) else {
        return -10_000;
    };
    let def = get_card_definition(card.id);
    let incoming = combat_total_incoming_damage(combat);
    let unblocked = combat_unblocked_incoming_damage(combat);
    let safe_block_turn = incoming <= combat.entities.player.block;
    let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();

    exhaust_fuel_value_score(
        card.id,
        def.card_type,
        def.cost as i32,
        combat.turn.energy as i32,
        safe_block_turn,
        can_play_now,
        exhaust_card_timing_hold_score(combat, card, unblocked, incoming),
        combat.get_power(0, PowerId::FeelNoPain),
        combat.get_power(0, PowerId::DarkEmbrace) > 0,
    ) + combat_exhaust_score_for_uuid(combat, uuid)
}

fn build_mass_exhaust_profile(
    combat: &CombatState,
    card_index: usize,
    exhausted_targets: &[u32],
    closeout_bonus: i32,
    exact_stabilize: bool,
) -> MassExhaustProfile {
    let exhausted_count = exhausted_targets.len() as i32;
    let total_fuel = exhausted_targets
        .iter()
        .map(|uuid| exhaust_fuel_value_for_uuid(combat, *uuid))
        .sum::<i32>();
    let stats = exhaust_disposition_stats(combat, exhausted_targets);
    let engine_support_level = i32::from(combat.get_power(0, PowerId::DarkEmbrace) > 0)
        + i32::from(combat.get_power(0, PowerId::FeelNoPain) > 0)
        + i32::from(combat.get_power(0, PowerId::Evolve) > 0);
    let unblocked_incoming = combat_unblocked_incoming_damage(combat);
    let remaining_cards_after = combat.zones.draw_pile.len() as i32
        + combat.zones.discard_pile.len() as i32
        + combat.zones.hand.len() as i32
        - exhausted_count;
    let playable_block_lost = exhausted_targets
        .iter()
        .filter_map(|uuid| combat.zones.hand.iter().find(|c| c.uuid == *uuid))
        .filter(|card| {
            crate::content::cards::can_play_card(card, combat).is_ok()
                && combat_retention_score_for_uuid(combat, card.uuid) >= 2_500
                && get_card_definition(card.id).base_block > 0
        })
        .count() as i32;
    let low_pressure_high_hp = unblocked_incoming <= 12
        && combat.entities.player.current_hp >= 30
        && combat.entities.player.current_hp > unblocked_incoming * 2 + 10;
    MassExhaustProfile {
        exhausted_count,
        total_fuel,
        remaining_cards_after,
        remaining_low_value_fuel_after: remaining_low_value_fuel_after_mass_exhaust(
            combat,
            card_index,
            exhausted_targets,
        ),
        closeout_bonus,
        junk_fuel_count: stats.junk_count,
        protected_piece_count: stats.protected_count,
        core_piece_count: stats.core_count,
        engine_support_level,
        block_per_exhaust: combat.get_power(0, PowerId::FeelNoPain),
        imminent_unblocked_damage: unblocked_incoming,
        playable_block_lost,
        exact_stabilize,
        low_pressure_high_hp,
        dark_embrace_draw_count: if combat.get_power(0, PowerId::DarkEmbrace) > 0 {
            exhausted_count
        } else {
            0
        },
    }
}

fn exhaust_card_timing_hold_score(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    unblocked_incoming: i32,
    incoming: i32,
) -> i32 {
    let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
    match card.id {
        CardId::Defend | CardId::DefendG => {
            let junk_fuel_count = combat
                .zones
                .hand
                .iter()
                .filter(|other| {
                    other.uuid != card.uuid
                        && matches!(
                            get_card_definition(other.id).card_type,
                            CardType::Curse | CardType::Status
                        )
                })
                .count() as i32;
            if unblocked_incoming > 0 {
                let mut value = 4_200 + unblocked_incoming.min(20) * 260;
                if junk_fuel_count == 0 {
                    value += 1_800;
                } else {
                    value -= junk_fuel_count.min(2) * 450;
                }
                value
            } else {
                0
            }
        }
        CardId::Inflame => {
            if can_play_now && combat.get_power(0, PowerId::Strength) <= 3 {
                4_500
            } else {
                0
            }
        }
        CardId::Disarm | CardId::Shockwave | CardId::Uppercut | CardId::Intimidate => {
            if incoming > 0 && can_play_now {
                5_400 + incoming.min(24) * 180
            } else if can_play_now {
                1_800
            } else {
                0
            }
        }
        CardId::BattleTrance
        | CardId::BurningPact
        | CardId::Offering
        | CardId::SeeingRed
        | CardId::Warcry => {
            if can_play_now {
                3_200 + unblocked_incoming.min(18) * 120
            } else {
                0
            }
        }
        CardId::Apotheosis => {
            let upgrade_targets = combat
                .zones
                .hand
                .iter()
                .filter(|c| c.uuid != card.uuid)
                .filter(|c| {
                    c.upgrades == 0
                        && !matches!(
                            get_card_definition(c.id).card_type,
                            CardType::Curse | CardType::Status
                        )
                })
                .count() as i32;
            crate::bot::combat_families::apotheosis::apotheosis_hand_shaping_score(
                upgrade_targets,
                unblocked_incoming,
            )
        }
        CardId::Reaper => reaper_hand_shaping_score(&SurvivalTimingContext {
            current_hp: combat.entities.player.current_hp,
            imminent_unblocked_damage: unblocked_incoming,
            missing_hp: combat_missing_hp(combat),
        }),
        CardId::Apparition => apparition_hand_shaping_score(&ApparitionTimingContext {
            current_hp: combat.entities.player.current_hp,
            current_intangible: combat
                .get_power(0, PowerId::Intangible)
                .max(combat.get_power(0, PowerId::IntangiblePlayer)),
            imminent_unblocked_damage: unblocked_incoming,
            total_incoming_damage: incoming,
            apparitions_in_hand: combat
                .zones
                .hand
                .iter()
                .filter(|c| c.id == CardId::Apparition)
                .count() as i32,
            remaining_apparitions_total: combat
                .zones
                .hand
                .iter()
                .chain(combat.zones.draw_pile.iter())
                .chain(combat.zones.discard_pile.iter())
                .filter(|c| c.id == CardId::Apparition)
                .count() as i32,
            upgraded: card.upgrades > 0,
            has_runic_pyramid: combat.entities.player.has_relic(RelicId::RunicPyramid),
            encounter_pressure: combat
                .entities
                .monsters
                .iter()
                .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
                .map(|m| combat.get_power(m.id, PowerId::Strength).max(0) * 2 + 2)
                .sum::<i32>()
                + if combat.meta.is_boss_fight {
                    6
                } else if combat.meta.is_elite_fight {
                    3
                } else {
                    0
                },
        }),
        _ => 0,
    }
}

fn combat_survival_timing_context(combat: &CombatState) -> SurvivalTimingContext {
    SurvivalTimingContext {
        current_hp: combat.entities.player.current_hp,
        imminent_unblocked_damage: combat_unblocked_incoming_damage(combat),
        missing_hp: combat_missing_hp(combat),
    }
}

fn combat_apparition_timing_context(
    combat: &CombatState,
    upgraded: bool,
) -> ApparitionTimingContext {
    ApparitionTimingContext {
        current_hp: combat.entities.player.current_hp,
        current_intangible: combat
            .get_power(0, PowerId::Intangible)
            .max(combat.get_power(0, PowerId::IntangiblePlayer)),
        imminent_unblocked_damage: combat_unblocked_incoming_damage(combat),
        total_incoming_damage: combat_total_incoming_damage(combat),
        apparitions_in_hand: combat
            .zones
            .hand
            .iter()
            .filter(|c| c.id == CardId::Apparition)
            .count() as i32,
        remaining_apparitions_total: combat
            .zones
            .hand
            .iter()
            .chain(combat.zones.draw_pile.iter())
            .chain(combat.zones.discard_pile.iter())
            .filter(|c| c.id == CardId::Apparition)
            .count() as i32,
        upgraded,
        has_runic_pyramid: combat.entities.player.has_relic(RelicId::RunicPyramid),
        encounter_pressure: combat
            .entities
            .monsters
            .iter()
            .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
            .map(|m| combat.get_power(m.id, PowerId::Strength).max(0) * 2 + 2)
            .sum::<i32>()
            + if combat.meta.is_boss_fight {
                6
            } else if combat.meta.is_elite_fight {
                3
            } else {
                0
            },
    }
}

fn remaining_low_value_fuel_after_best_single_exhaust(
    combat: &CombatState,
    card_index: usize,
) -> i32 {
    let targets = exhaust_candidate_uuids_for_card(combat, card_index);
    let excluded = best_exhaust_candidate_uuid(combat, &targets)
        .map(|uuid| vec![uuid])
        .unwrap_or_default();
    count_remaining_low_value_exhaust_candidates(combat, &targets, &excluded)
}

fn future_exhaust_demand(combat: &CombatState, current_card_index: usize) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            *idx != current_card_index
                && matches!(
                    card.id,
                    CardId::SecondWind
                        | CardId::SeverSoul
                        | CardId::FiendFire
                        | CardId::BurningPact
                        | CardId::TrueGrit
                )
        })
        .count() as i32
}

fn remaining_low_value_fuel_after_mass_exhaust(
    combat: &CombatState,
    card_index: usize,
    exhausted_targets: &[u32],
) -> i32 {
    let candidates = exhaust_candidate_uuids_for_card(combat, card_index);
    count_remaining_low_value_exhaust_candidates(combat, &candidates, exhausted_targets)
}

fn fiend_fire_closeout_bonus(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    exhausted_cards: i32,
) -> i32 {
    let per_hit = estimated_attack_damage(combat, card).max(0);
    let total_damage = per_hit * exhausted_cards.max(0);
    let best_target = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| (total_damage - m.block).max(0) - m.current_hp.max(0))
        .max()
        .unwrap_or(i32::MIN / 4);

    let kills = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .filter(|m| (total_damage - m.block).max(0) >= m.current_hp.max(0))
        .count() as i32;
    let alive_before = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .count() as i32;
    exhaust_finish_window_score(
        best_target >= 0,
        kills,
        estimated_kill_prevention_from_damage(combat, total_damage),
        alive_before - kills,
    ) + if best_target >= 0 {
        total_damage * 80
    } else {
        0
    }
}

fn total_cycle_cards(combat: &CombatState) -> i32 {
    (combat.zones.draw_pile.len() + combat.zones.discard_pile.len() + combat.zones.hand.len())
        as i32
}

fn max_additional_block_before_body_slam(combat: &CombatState, card_index: usize) -> i32 {
    let Some(body_slam) = combat.zones.hand.get(card_index) else {
        return 0;
    };
    let budget = (combat.turn.energy as i32 - body_slam.get_cost() as i32).max(0);
    if budget <= 0 {
        return 0;
    }

    let mut best = vec![0; budget as usize + 1];
    for (idx, card) in combat.zones.hand.iter().enumerate() {
        if idx == card_index {
            continue;
        }
        let cost = card.get_cost() as i32;
        if cost < 0 || cost > budget {
            continue;
        }
        let block_gain = body_slam_followup_block_gain(combat, card_index, idx);
        if block_gain <= 0 {
            continue;
        }
        for energy in (cost..=budget).rev() {
            let energy_idx = energy as usize;
            let prev_idx = (energy - cost) as usize;
            best[energy_idx] = best[energy_idx].max(best[prev_idx] + block_gain);
        }
    }

    best.into_iter().max().unwrap_or(0)
}

fn body_slam_followup_block_gain(combat: &CombatState, body_slam_index: usize, idx: usize) -> i32 {
    let Some(card) = combat.zones.hand.get(idx) else {
        return 0;
    };
    let def = get_card_definition(card.id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return 0;
    }

    let mut block_gain = match card.id {
        CardId::SecondWind => {
            let exhaust_count = combat
                .zones
                .hand
                .iter()
                .enumerate()
                .filter(|(other_idx, other)| {
                    *other_idx != idx
                        && *other_idx != body_slam_index
                        && get_card_definition(other.id).card_type != CardType::Attack
                })
                .count() as i32;
            card.base_block_mut.max(5) * exhaust_count.max(0)
        }
        _ if card.base_block_mut > 0 => {
            (card.base_block_mut + combat.get_power(0, PowerId::Dexterity)).max(0)
        }
        _ => 0,
    };
    if combat.get_power(0, PowerId::Frail) > 0 {
        block_gain = (block_gain as f32 * 0.75).floor() as i32;
    }
    block_gain.max(0)
}

fn immediate_exhaust_count(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| {
            matches!(
                card.id,
                CardId::SecondWind
                    | CardId::SeverSoul
                    | CardId::FiendFire
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::Offering
                    | CardId::SeeingRed
            )
        })
        .count() as i32
}

fn future_exhaust_source_count(combat: &CombatState, current_card_index: usize) -> i32 {
    future_exhaust_demand(combat, current_card_index)
}

fn status_cards_in_hand(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32
}

fn future_status_card_count(combat: &CombatState) -> i32 {
    status_cards_in_draw_pile(combat)
        + status_cards_in_discard_pile(combat)
        + status_cards_in_hand(combat)
}

fn sentry_count(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && m.monster_type == EnemyId::Sentry as usize)
        .count() as i32
}

fn status_cards_in_draw_pile(combat: &CombatState) -> i32 {
    combat
        .zones
        .draw_pile
        .iter()
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32
}

fn status_cards_in_discard_pile(combat: &CombatState) -> i32 {
    combat
        .zones
        .discard_pile
        .iter()
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32
}

fn second_wind_finish_window_bonus(combat: &CombatState, exhausted_cards: i32) -> i32 {
    let block_per = combat
        .zones
        .hand
        .iter()
        .find(|c| c.id == CardId::SecondWind)
        .map(|c| c.base_block_mut.max(5))
        .unwrap_or(5);
    let predicted_block = block_per * exhausted_cards.max(0);
    let prevented = combat_unblocked_incoming_damage(combat)
        .min(predicted_block)
        .max(0);
    let exact_stabilize =
        prevented >= combat_unblocked_incoming_damage(combat).max(0) && prevented > 0;
    let alive_before = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .count() as i32;
    exhaust_finish_window_score(exact_stabilize, 0, prevented, alive_before)
}

fn estimated_kill_prevention_from_damage(combat: &CombatState, total_damage: i32) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .filter(|m| intent_hits(&m.current_intent) > 0)
        .map(|m| {
            if (total_damage - m.block).max(0) >= m.current_hp.max(0) {
                m.intent_dmg * intent_hits(&m.current_intent)
            } else {
                0
            }
        })
        .sum()
}

fn combat_total_incoming_damage(combat: &CombatState) -> i32 {
    build_combat_belief_state(combat)
        .expected_incoming_damage
        .round() as i32
}

fn combat_unblocked_incoming_damage(combat: &CombatState) -> i32 {
    (combat_total_incoming_damage(combat) - combat.entities.player.block).max(0)
}

fn combat_missing_hp(combat: &CombatState) -> i32 {
    (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0)
}

fn estimated_reaper_heal(combat: &CombatState, card: &crate::runtime::combat::CombatCard) -> i32 {
    let base = estimated_attack_damage(combat, card).max(0);
    if base <= 0 {
        return 0;
    }

    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| {
            let mut dmg = base;
            if combat.get_power(m.id, PowerId::Vulnerable) > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            hp_loss.min(m.current_hp.max(0))
        })
        .sum()
}

fn estimated_reaper_kills(combat: &CombatState, card: &crate::runtime::combat::CombatCard) -> i32 {
    let base = estimated_attack_damage(combat, card).max(0);
    if base <= 0 {
        return 0;
    }

    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| {
            let mut dmg = base;
            if combat.get_power(m.id, PowerId::Vulnerable) > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            i32::from(hp_loss >= m.current_hp.max(0))
        })
        .sum()
}

fn estimated_reaper_kill_prevention(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
) -> i32 {
    let base = estimated_attack_damage(combat, card).max(0);
    if base <= 0 {
        return 0;
    }

    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .filter(|m| intent_hits(&m.current_intent) > 0)
        .map(|m| {
            let mut dmg = base;
            if combat.get_power(m.id, PowerId::Vulnerable) > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            if hp_loss >= m.current_hp.max(0) {
                m.intent_dmg * intent_hits(&m.current_intent)
            } else {
                0
            }
        })
        .sum()
}

fn card_hits(card: &crate::runtime::combat::CombatCard) -> i32 {
    match card.id {
        CardId::TwinStrike => 2,
        CardId::Pummel => card.base_magic_num_mut.max(4),
        CardId::SwordBoomerang => card.base_magic_num_mut.max(3),
        _ => 1,
    }
}
