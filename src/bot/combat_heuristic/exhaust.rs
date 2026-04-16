use crate::bot::combat_families::apotheosis::apotheosis_hand_shaping_score;
use crate::bot::combat_families::apparition::{
    apparition_hand_shaping_score, ApparitionTimingContext,
};
use crate::bot::combat_families::draw::deck_cycle_thinning_score;
use crate::bot::combat_families::exhaust::{
    exhaust_fuel_value_score, exhaust_future_fuel_reserve_score, exhaust_mass_play_score,
    exhaust_random_core_risk_score, exhaust_random_play_score, mass_exhaust_base_score,
    mass_exhaust_keeper_penalty, mass_exhaust_second_wind_selectivity_score, MassExhaustProfile,
};
use crate::bot::combat_families::survival::{
    exhaust_finish_window_score, reaper_hand_shaping_score, SurvivalTimingContext,
};
use crate::content::cards::{CardId, CardType};

use super::apply::{effective_damage, effective_energy_cost};
use super::sim::{active_hand_cards, SimCard, SimState};

pub(super) fn exhaust_timing_value(state: &SimState, card_idx: usize) -> Option<i32> {
    let card = &state.hand[card_idx];
    let value = match card.card_id {
        CardId::BurningPact => burning_pact_exhaust_value(state, card_idx),
        CardId::SecondWind => second_wind_exhaust_value(state, card_idx),
        CardId::SeverSoul => sever_soul_exhaust_value(state, card_idx),
        CardId::FiendFire => fiend_fire_exhaust_value(state, card_idx),
        CardId::TrueGrit if card.upgrades <= 0 => true_grit_random_exhaust_value(state, card_idx),
        _ => return None,
    };
    Some(value)
}

fn burning_pact_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let targets = exhaust_candidate_indices(state, card_idx);
    let best_fuel = targets
        .iter()
        .map(|idx| exhaust_fuel_value_for_index(state, *idx))
        .max()
        .unwrap_or(-6_000);
    exhaust_mass_play_score(
        best_fuel,
        1,
        state.card_pool_size - 1,
        targets
            .iter()
            .filter(|&&idx| exhaust_fuel_value_for_index(state, idx) > 0)
            .count() as i32
            - i32::from(best_fuel > 0),
        0,
    ) + exhaust_future_fuel_reserve_score(
        remaining_low_value_fuel_after_best_single_exhaust(state, card_idx),
        future_exhaust_demand(state, card_idx),
    ) + deck_cycle_thinning_score(
        state.card_pool_size,
        state.card_pool_size - 1,
        2 + i32::from(state.has_dark_embrace),
        0,
        0,
        0,
    ) + burning_pact_emergency_dig_value(state, card_idx, best_fuel)
}

fn burning_pact_emergency_dig_value(state: &SimState, card_idx: usize, best_fuel: i32) -> i32 {
    let imminent = imminent_unblocked_damage(state);
    let safe_now = imminent <= 0;
    let bad_fuel_in_hand = exhaust_candidate_indices(state, card_idx)
        .into_iter()
        .filter(|&idx| {
            matches!(
                state.hand[idx].card_id,
                CardId::Burn | CardId::Dazed | CardId::Slimed | CardId::Wound | CardId::Injury
            )
        })
        .count() as i32;
    let future_save_cards = (state.draw_pile_size + state.discard_pile_size).min(6)
        + state.status_in_draw.min(2)
        + state.status_in_discard.min(2);

    let mut value = 0;
    if best_fuel <= 0 {
        value -= 4_500;
    }
    if bad_fuel_in_hand > 0 {
        value += bad_fuel_in_hand * 3_000;
    }
    if !safe_now {
        value += 2_000 + imminent.min(24) * 240;
        value += future_save_cards.min(4) * 1_100;
        if bad_fuel_in_hand > 0 {
            value += 2_500;
        }
    } else {
        value += future_save_cards.min(3) * 350;
    }
    if state.has_dark_embrace || state.has_feel_no_pain {
        value += 1_200;
    }
    value
}

fn second_wind_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let exhausted = exhaust_candidate_indices(state, card_idx);
    let finish_window = second_wind_finish_window_bonus(state, card_idx);
    let profile = build_sim_mass_exhaust_profile(
        state,
        card_idx,
        &exhausted,
        finish_window,
        finish_window > 0,
    );
    mass_exhaust_base_score(&profile, state.card_pool_size)
        + mass_exhaust_second_wind_selectivity_score(&profile)
}

fn sever_soul_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let exhausted = exhaust_candidate_indices(state, card_idx);
    let profile = build_sim_mass_exhaust_profile(state, card_idx, &exhausted, 0, false);
    let total = mass_exhaust_base_score(&profile, state.card_pool_size)
        - mass_exhaust_keeper_penalty(&profile, 500, 3_000);
    (total as f32 * 0.8) as i32
}

fn fiend_fire_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let exhausted = exhaust_candidate_indices(state, card_idx);
    let closeout_bonus = fiend_fire_closeout_bonus(state, card_idx, exhausted.len() as i32);
    let profile =
        build_sim_mass_exhaust_profile(state, card_idx, &exhausted, closeout_bonus, false);
    mass_exhaust_base_score(&profile, state.card_pool_size)
        - mass_exhaust_keeper_penalty(&profile, 300, 2_200)
}

fn true_grit_random_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let candidates = exhaust_candidate_indices(state, card_idx);
    let low_value = candidates
        .iter()
        .filter(|&&idx| exhaust_fuel_value_for_index(state, idx) > 0)
        .count() as i32;
    let protected = candidates.len() as i32 - low_value;
    let core = candidates
        .iter()
        .filter(|&&idx| exhaust_fuel_value_for_index(state, idx) <= -8_000)
        .count() as i32;
    let near_core = candidates
        .iter()
        .filter(|&&idx| {
            let value = exhaust_fuel_value_for_index(state, idx);
            value > -8_000 && value <= -2_000
        })
        .count() as i32;
    exhaust_random_play_score(low_value, protected, state.card_pool_size - 1)
        + exhaust_random_core_risk_score(low_value, core, near_core)
        + exhaust_future_fuel_reserve_score(
            remaining_low_value_fuel_after_best_single_exhaust(state, card_idx),
            future_exhaust_demand(state, card_idx),
        )
        + deck_cycle_thinning_score(
            state.card_pool_size,
            state.card_pool_size - 1,
            i32::from(state.has_dark_embrace),
            0,
            0,
            0,
        )
}

fn exhaust_candidate_indices(state: &SimState, card_idx: usize) -> Vec<usize> {
    let card = &state.hand[card_idx];
    active_hand_cards(state)
        .filter(|(idx, _)| *idx != card_idx)
        .filter(|(_, other)| match card.card_id {
            CardId::SecondWind | CardId::SeverSoul => other.card_type != CardType::Attack,
            CardId::FiendFire | CardId::BurningPact | CardId::TrueGrit => true,
            _ => false,
        })
        .map(|(idx, _)| idx)
        .collect()
}

fn exhaust_fuel_value_for_index(state: &SimState, idx: usize) -> i32 {
    let card = &state.hand[idx];
    let can_play_now = effective_energy_cost(state, card) <= state.energy
        && card.card_type != CardType::Curse
        && card.card_type != CardType::Status;
    let safe_block_turn = total_incoming_damage(state) <= state.player_block;

    exhaust_fuel_value_score(
        card.card_id,
        card.card_type,
        card.cost,
        state.energy,
        safe_block_turn,
        can_play_now,
        exhaust_card_timing_hold_score(state, card),
        if state.has_feel_no_pain { 4 } else { 0 },
        state.has_dark_embrace,
    )
}

fn build_sim_mass_exhaust_profile(
    state: &SimState,
    card_idx: usize,
    exhausted: &[usize],
    closeout_bonus: i32,
    exact_stabilize: bool,
) -> MassExhaustProfile {
    let fuel_values: Vec<i32> = exhausted
        .iter()
        .map(|idx| exhaust_fuel_value_for_index(state, *idx))
        .collect();
    let junk_fuel_count = fuel_values.iter().filter(|&&value| value >= 8_000).count() as i32;
    let protected_piece_count = fuel_values.iter().filter(|&&value| value <= -2_000).count() as i32;
    let core_piece_count = fuel_values.iter().filter(|&&value| value <= -8_000).count() as i32;
    let engine_support_level = i32::from(state.has_dark_embrace)
        + i32::from(state.has_feel_no_pain)
        + i32::from(state.has_evolve);
    let unblocked_incoming = imminent_unblocked_damage(state);
    let playable_block_lost = exhausted
        .iter()
        .filter(|idx| {
            let card = &state.hand[**idx];
            effective_energy_cost(state, card) <= state.energy && card.base_block > 0
        })
        .count() as i32;
    let low_pressure_high_hp = unblocked_incoming <= 12
        && state.player_hp >= 30
        && state.player_hp > unblocked_incoming * 2 + 10;

    MassExhaustProfile {
        exhausted_count: exhausted.len() as i32,
        total_fuel: fuel_values.iter().sum(),
        remaining_cards_after: state.card_pool_size - exhausted.len() as i32,
        remaining_low_value_fuel_after: remaining_low_value_fuel_after_mass_exhaust(
            state, card_idx, exhausted,
        ),
        closeout_bonus,
        junk_fuel_count,
        protected_piece_count,
        core_piece_count,
        engine_support_level,
        block_per_exhaust: if state.has_feel_no_pain { 4 } else { 0 },
        imminent_unblocked_damage: unblocked_incoming,
        playable_block_lost,
        exact_stabilize,
        low_pressure_high_hp,
        dark_embrace_draw_count: if state.has_dark_embrace {
            exhausted.len() as i32
        } else {
            0
        },
    }
}

fn exhaust_card_timing_hold_score(state: &SimState, card: &SimCard) -> i32 {
    match card.card_id {
        CardId::Defend | CardId::DefendG => {
            let unblocked = imminent_unblocked_damage(state);
            let junk_fuel_count = active_hand_cards(state)
                .filter(|(_, other)| matches!(other.card_type, CardType::Curse | CardType::Status))
                .count() as i32;
            if unblocked > 0 {
                let mut value = 4_200 + unblocked.min(20) * 260;
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
            if effective_energy_cost(state, card) <= state.energy && state.player_strength <= 3 {
                4_500
            } else {
                0
            }
        }
        CardId::Apotheosis => apotheosis_hand_shaping_score(
            armaments_upgradable_count(state) as i32,
            imminent_unblocked_damage(state),
        ),
        CardId::Apparition => apparition_hand_shaping_score(&ApparitionTimingContext {
            current_hp: state.player_hp,
            current_intangible: state.player_intangible,
            imminent_unblocked_damage: imminent_unblocked_damage(state),
            total_incoming_damage: total_incoming_damage(state),
            apparitions_in_hand: active_hand_cards(state)
                .filter(|(_, c)| c.card_id == CardId::Apparition)
                .count() as i32,
            remaining_apparitions_total: state.remaining_apparitions_total,
            upgraded: card.upgrades > 0,
            has_runic_pyramid: state.has_runic_pyramid,
            encounter_pressure: state.enemy_strength_sum.max(0) * 2
                + alive_monster_count(state).max(0) * 2
                + if state.is_boss_fight {
                    6
                } else if state.is_elite_fight {
                    3
                } else {
                    0
                },
        }),
        CardId::Reaper => reaper_hand_shaping_score(&SurvivalTimingContext {
            current_hp: state.player_hp,
            imminent_unblocked_damage: imminent_unblocked_damage(state),
            missing_hp: missing_hp(state),
        }),
        _ => 0,
    }
}

fn fiend_fire_closeout_bonus(state: &SimState, card_idx: usize, exhausted_cards: i32) -> i32 {
    let card = &state.hand[card_idx];
    let total_damage = effective_damage(state, card).max(0) * exhausted_cards.max(0);
    let best_target_margin = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| (total_damage - state.monsters[i].block).max(0) - state.monsters[i].hp.max(0))
        .max()
        .unwrap_or(i32::MIN / 4);
    let kills = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .filter(|&i| (total_damage - state.monsters[i].block).max(0) >= state.monsters[i].hp.max(0))
        .count() as i32;
    let alive_before = alive_monster_count(state);

    exhaust_finish_window_score(
        best_target_margin >= 0,
        kills,
        estimated_kill_prevention_from_damage(state, total_damage),
        alive_before - kills,
    ) + if best_target_margin >= 0 {
        total_damage * 80
    } else {
        0
    }
}

fn second_wind_finish_window_bonus(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let exhaust_count = exhaust_candidate_indices(state, card_idx).len() as i32;
    let predicted_block = (card.base_block + state.player_dexterity).max(0) * exhaust_count.max(0);
    let prevented = imminent_unblocked_damage(state).min(predicted_block).max(0);
    exhaust_finish_window_score(
        prevented >= imminent_unblocked_damage(state).max(0) && prevented > 0,
        0,
        prevented,
        alive_monster_count(state),
    )
}

fn estimated_kill_prevention_from_damage(state: &SimState, total_damage: i32) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| {
            if (total_damage - state.monsters[i].block).max(0) >= state.monsters[i].hp.max(0) {
                state.monsters[i].intent_dmg.max(0) * state.monsters[i].intent_hits.max(1)
            } else {
                0
            }
        })
        .sum()
}

fn remaining_low_value_fuel_after_best_single_exhaust(state: &SimState, card_idx: usize) -> i32 {
    let targets = exhaust_candidate_indices(state, card_idx);
    let best = targets
        .iter()
        .max_by_key(|&&idx| exhaust_fuel_value_for_index(state, idx))
        .copied();

    targets
        .into_iter()
        .filter(|idx| Some(*idx) != best)
        .filter(|&idx| exhaust_fuel_value_for_index(state, idx) > 0)
        .count() as i32
}

fn remaining_low_value_fuel_after_mass_exhaust(
    state: &SimState,
    card_idx: usize,
    exhausted: &[usize],
) -> i32 {
    let exhausted_set: std::collections::HashSet<usize> = exhausted.iter().copied().collect();
    exhaust_candidate_indices(state, card_idx)
        .into_iter()
        .filter(|idx| !exhausted_set.contains(idx))
        .filter(|&idx| exhaust_fuel_value_for_index(state, idx) > 0)
        .count() as i32
}

fn future_exhaust_demand(state: &SimState, current_card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_card_idx
                && matches!(
                    c.card_id,
                    CardId::SecondWind
                        | CardId::SeverSoul
                        | CardId::FiendFire
                        | CardId::BurningPact
                        | CardId::TrueGrit
                )
        })
        .count() as i32
}

fn alive_monster_count(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .count() as i32
}

fn total_incoming_damage(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_dmg * state.monsters[i].intent_hits.max(1))
        .sum()
}

fn imminent_unblocked_damage(state: &SimState) -> i32 {
    (total_incoming_damage(state) - state.player_block).max(0)
}

fn missing_hp(state: &SimState) -> i32 {
    (state.player_max_hp - state.player_hp).max(0)
}

fn armaments_upgradable_count(state: &SimState) -> usize {
    active_hand_cards(state)
        .filter(|(_, c)| {
            c.upgrades == 0 && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .count()
}
