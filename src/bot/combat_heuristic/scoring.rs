use crate::bot::card_taxonomy::taxonomy;
use crate::bot::strategy_families::{
    apotheosis_timing_score, apparition_timing_score, assess_branch_opening, assess_turn_action,
    branch_family_for_card, classify_turn_action, default_chance_profile,
    default_ordering_constraint, default_ordering_hint, default_risk_profile,
    ApparitionTimingContext, BranchOpeningContext, BranchOpeningEstimate, DrawTimingContext,
    TurnRiskContext, TurnSequencingContext,
};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;

use super::apply::{
    effective_damage, effective_energy_cost, effective_hits, expected_reflect_damage,
};
use super::attack::attack_timing_value;
use super::debuff::debuff_timing_value;
use super::draw::{
    battle_trance_timing_value, deep_breath_timing_value, generic_draw_timing_value,
};
use super::exhaust::exhaust_timing_value;
use super::posture::posture_features;
use super::power::{power_posture_adjustment, power_timing_value};
use super::sim::{
    active_hand_cards, active_monsters, gremlin_nob_enrage_active, SimCard, SimMonster, SimState,
};
use super::support::{support_posture_adjustment, support_timing_value};

pub(super) fn play_priority(state: &SimState, card_idx: usize) -> i32 {
    base_priority(state, card_idx)
        + timing_priority(state, card_idx)
        + posture_priority(state, card_idx)
        + sequencing_priority(state, card_idx)
        + shared_priority_adjustments(state, card_idx)
}

fn base_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p = 0;

    if card.card_type == CardType::Power {
        p += 10_000;
    }
    if card.card_type == CardType::Attack {
        p += 5_000 + card.base_damage * effective_hits(card, state.energy) * 10;
    }
    if card.base_block > 0 {
        p += 3_000 + card.base_block * 10;
    }

    p
}

fn timing_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p: i32 = 0;

    if let Some(value) = power_timing_value(state, card_idx) {
        return value;
    }
    if let Some(value) = support_timing_value(state, card_idx) {
        return value;
    }
    if let Some(value) = exhaust_timing_value(state, card_idx) {
        return value;
    }
    if let Some(value) = attack_timing_value(state, card_idx) {
        return value;
    }

    match card.card_id {
        CardId::Flex => p += 9_000,
        CardId::Apotheosis => p += apotheosis_timing_value(state),
        CardId::Apparition => p += apparition_timing_value(state, card),
        CardId::Armaments => p += armaments_timing_value(state, card_idx),
        CardId::FlameBarrier => p += flame_barrier_timing_value(state, card),
        CardId::Blind
        | CardId::DarkShackles
        | CardId::Trip
        | CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Clothesline
        | CardId::Intimidate => p += debuff_timing_value(state, card_idx),
        CardId::PommelStrike => p += generic_draw_timing_value(state, card_idx, 1, false),
        CardId::ShrugItOff => p += generic_draw_timing_value(state, card_idx, 1, false),
        CardId::Warcry => p += generic_draw_timing_value(state, card_idx, 1, false),
        CardId::GoodInstincts | CardId::Finesse => {
            p += if total_incoming_damage(state) > state.player_block {
                7_500
            } else {
                4_500
            };
            p += generic_draw_timing_value(state, card_idx, 1, false);
        }
        CardId::FlashOfSteel => {
            p += best_attack_target_value(state) * 350 + 5_500;
            p += generic_draw_timing_value(state, card_idx, 1, false);
        }
        CardId::BattleTrance => p += battle_trance_timing_value(state, card_idx),
        CardId::MasterOfStrategy => {
            p += 7_500 + generic_draw_timing_value(state, card_idx, card.base_magic.max(0), false)
        }
        CardId::DeepBreath => p += deep_breath_timing_value(state),
        CardId::SecretTechnique | CardId::SecretWeapon | CardId::Discovery => p += 6_500,
        CardId::TheBomb => p += alive_monster_count(state) * 1_100 + 6_000,
        CardId::Offering => p += 8_800 + generic_draw_timing_value(state, card_idx, 3, false),
        CardId::Bloodletting => p += resource_conversion_timing_value(state, card_idx, 3),
        CardId::SeeingRed => p += resource_conversion_timing_value(state, card_idx, 0),
        CardId::Disarm => p += disarm_timing_value(state),
        _ => {}
    }

    p
}

fn posture_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let posture = posture_features(state);

    power_posture_adjustment(card.card_id, &posture)
        + support_posture_adjustment(card.card_id, &posture)
}

fn shared_priority_adjustments(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p = 0;

    if card.base_block > 0 {
        let reflect = best_reflect_threat_in_hand_followup(state, card_idx);
        if reflect > 0 {
            p += reflect * 260;
        }
    }
    if card.card_type == CardType::Attack {
        let energy_spent = effective_energy_cost(state, card);
        let reflect =
            expected_reflect_damage(state, card, None, effective_hits(card, energy_spent));
        if reflect > 0 {
            let absorbed = reflect.min(state.player_block.max(0));
            let unblocked = reflect - absorbed;
            p -= unblocked * 320 + absorbed * 80;
        }
    }

    if gremlin_nob_enrage_active(state) && card.card_type == CardType::Skill {
        p -= gremlin_nob_skill_penalty_value(state, card);
    }

    if card.cost == 0 && p == 0 {
        p += 2_000;
    }
    p
}

fn sequencing_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let role = classify_turn_action(card.card_id, card.card_type);
    let risk_ctx = build_turn_risk_context(state);
    let sequencing = TurnSequencingContext {
        role,
        ordering_hint: default_ordering_hint(card.card_id, role),
        chance_profile: default_chance_profile(card.card_id),
        risk_profile: default_risk_profile(card.card_id, role),
        ordering_constraint: default_ordering_constraint(card.card_id),
        immediate_payoff: immediate_sequencing_payoff(state, card_idx),
        followup_payoff: followup_payoff_estimate(state, card_idx),
        growth_window: growth_window_available(state, card_idx),
    };
    let branch = branch_opening_estimate(state, card_idx, &risk_ctx);
    assess_turn_action(&sequencing, &risk_ctx, branch.as_ref()).total_delta()
}

fn build_turn_risk_context(state: &SimState) -> TurnRiskContext {
    let unblocked_damage = total_incoming_damage(state);
    TurnRiskContext {
        current_hp: state.player_hp,
        unblocked_damage,
        defense_gap: (unblocked_damage - state.player_block).max(0),
        lethal_pressure: unblocked_damage >= state.player_hp,
        urgent_pressure: unblocked_damage > 0 || state.is_elite_fight || state.is_boss_fight,
        current_energy: state.energy,
        remaining_actions: active_hand_cards(state).count().saturating_sub(1) as i32,
        has_safe_line: unblocked_damage <= state.player_block,
    }
}

fn branch_opening_estimate(
    state: &SimState,
    card_idx: usize,
    risk: &TurnRiskContext,
) -> Option<BranchOpeningEstimate> {
    let card = &state.hand[card_idx];
    let branch_family = branch_family_for_card(card.card_id)?;
    let draw_count = match card.card_id {
        CardId::PommelStrike | CardId::ShrugItOff | CardId::Warcry => 1,
        CardId::BattleTrance => card.base_magic.max(0),
        CardId::MasterOfStrategy => card.base_magic.max(0),
        CardId::Offering => 3,
        CardId::BurningPact => 2,
        CardId::DeepBreath => 1,
        CardId::Discovery | CardId::InfernalBlade => 1,
        _ => 0,
    };
    if draw_count <= 0 {
        return None;
    }
    let draw_ctx = DrawTimingContext {
        current_energy: state.energy,
        player_no_draw: state.player_no_draw,
        current_hand_size: active_hand_cards(state).count() as i32,
        future_zero_cost_cards: state.future_zero_cost_cards,
        future_one_cost_cards: state.future_one_cost_cards,
        future_two_plus_cost_cards: state.future_two_plus_cost_cards,
        future_key_delay_weight: state.future_key_delay_weight,
        future_high_cost_key_delay_weight: state.future_high_cost_key_delay_weight,
        future_status_cards: state.status_in_draw + state.status_in_discard,
        other_draw_sources_in_hand: active_hand_cards(state)
            .filter(|(idx, c)| *idx != card_idx && is_draw_like(c.card_id))
            .count() as i32,
    };
    let immediate_action_value = immediate_sequencing_payoff(state, card_idx);
    let current_defensive_floor = immediate_defensive_floor(state, card_idx);
    let energy_after_play = state.energy - effective_energy_cost(state, card);
    let hand_space_after_play = (10_i32 - (active_hand_cards(state).count() as i32 - 1)).max(0);
    let remaining_attack_followups = count_followups(state, card_idx, true);
    let remaining_defensive_followups = count_followups(state, card_idx, false);
    Some(assess_branch_opening(&BranchOpeningContext {
        draw: draw_ctx,
        risk: *risk,
        draw_count,
        applies_no_draw: matches!(card.card_id, CardId::BattleTrance),
        current_safe_line_exists: risk.has_safe_line,
        current_defensive_floor,
        energy_after_play,
        hand_space_after_play,
        immediate_action_value,
        remaining_attack_followups,
        remaining_defensive_followups,
        branch_family,
    }))
}

fn count_followups(state: &SimState, exclude_idx: usize, attacks: bool) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, card)| {
            *idx != exclude_idx
                && if attacks {
                    card.card_type == CardType::Attack
                } else {
                    card.base_block > 0
                        || matches!(
                            card.card_id,
                            CardId::Disarm
                                | CardId::Shockwave
                                | CardId::Uppercut
                                | CardId::Clothesline
                                | CardId::ThunderClap
                                | CardId::Intimidate
                                | CardId::Blind
                                | CardId::DarkShackles
                                | CardId::Trip
                        )
                }
        })
        .count() as i32
}

fn immediate_defensive_floor(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    card.base_block.max(0)
        + match card.card_id {
            CardId::Disarm => 6,
            CardId::Shockwave | CardId::Uppercut => 7,
            CardId::Clothesline | CardId::Intimidate | CardId::Blind | CardId::Trip => 4,
            CardId::ThunderClap => 2,
            _ => 0,
        }
}

fn immediate_sequencing_payoff(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    match card.card_type {
        CardType::Attack => effective_damage(state, card) * effective_hits(card, state.energy),
        CardType::Skill => card.base_block.max(0),
        _ => 0,
    }
}

fn followup_payoff_estimate(state: &SimState, current_idx: usize) -> i32 {
    let current = &state.hand[current_idx];
    if matches!(
        current.card_id,
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption
    ) {
        let exhaust_sources = active_hand_cards(state)
            .filter(|(idx, card)| {
                *idx != current_idx
                    && (taxonomy(card.card_id).is_exhaust_outlet()
                        || matches!(
                            card.card_id,
                            CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                        ))
            })
            .count() as i32;
        let junk_fuel = active_hand_cards(state)
            .filter(|(idx, card)| {
                *idx != current_idx
                    && matches!(
                        card.card_id,
                        CardId::Burn
                            | CardId::Dazed
                            | CardId::Slimed
                            | CardId::Wound
                            | CardId::Injury
                    )
            })
            .count() as i32;
        return exhaust_sources * 14 + junk_fuel * 8;
    }
    let energy_after = state.energy - effective_energy_cost(state, current);
    let mut best_attack_followup = 0;
    let mut best_high_cost_followup = 0;
    let mut best_multi_hit_followup = 0;
    for (idx, card) in active_hand_cards(state) {
        if idx == current_idx || card.card_type != CardType::Attack {
            continue;
        }
        let energy_for_card = if card.cost < 0 {
            energy_after.max(0)
        } else {
            card.cost
        };
        if energy_for_card > energy_after {
            continue;
        }
        let damage = effective_damage(state, card) * effective_hits(card, energy_for_card);
        best_attack_followup = best_attack_followup.max(damage);
        if card.cost >= 2 || card.cost < 0 {
            best_high_cost_followup = best_high_cost_followup.max(damage);
        }
        if taxonomy(card.card_id).is_multi_hit()
            || taxonomy(card.card_id).is_attack_followup_priority()
        {
            best_multi_hit_followup = best_multi_hit_followup.max(damage);
        }
    }
    match current.card_id {
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => best_high_cost_followup,
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip => best_multi_hit_followup.max(best_attack_followup),
        _ if taxonomy(current.card_id).is_setup_power()
            || matches!(current.card_id, CardId::Rage | CardId::Flex) =>
        {
            best_attack_followup
        }
        _ => best_attack_followup / 2,
    }
}

fn growth_window_available(state: &SimState, card_idx: usize) -> bool {
    let card = &state.hand[card_idx];
    if !matches!(card.card_id, CardId::Feed | CardId::Reaper) {
        return false;
    }
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return false;
    }
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .any(|i| {
            let m = &state.monsters[i];
            let mut dmg = base;
            if m.vulnerable > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            (dmg - m.block.max(0)).max(0) >= m.hp.max(0)
        })
}

fn is_draw_like(card_id: CardId) -> bool {
    taxonomy(card_id).is_draw_core() && !matches!(card_id, CardId::DarkEmbrace | CardId::Evolve)
}

pub(super) fn evaluate(state: &SimState) -> i64 {
    let mut hp = state.player_hp;
    let mut block = state.player_block;

    for mi in 0..state.monsters.len() {
        let m = &state.monsters[mi];
        if m.is_gone || !m.is_attacking {
            continue;
        }
        let mut d = m.intent_dmg;
        if m.weak > 0 {
            d = (d as f32 * 0.75).floor() as i32;
        }
        d = d.max(0);
        if state.player_vulnerable {
            d = (d as f32 * 1.5).floor() as i32;
        }
        for _ in 0..m.intent_hits {
            let pierce = (d - block).max(0);
            block = (block - d).max(0);
            hp -= pierce;
        }
    }

    let alive = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .count() as i64;
    let dead = state.monsters.len() as i64 - alive;
    let mhp: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| {
            let m = &state.monsters[i];
            let block_progress = if m.persistent_block {
                m.block.max(0)
            } else {
                0
            };
            (m.hp.max(0) + block_progress) as i64
        })
        .sum();
    let enemy_strength: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].strength.max(0) as i64)
        .sum();
    let split_quality_bonus: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| slime_boss_split_state_bonus(state, &state.monsters[i]) as i64)
        .sum();
    let flight_pressure: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].flight > 0)
        .map(|i| {
            let m = &state.monsters[i];
            let threat = if m.is_attacking {
                (m.intent_dmg.max(0) * m.intent_hits.max(1)) as i64
            } else {
                0
            };
            (m.flight as i64) * (220 + threat * 12)
        })
        .sum();

    if hp <= 0 {
        return -1_000_000 + hp as i64;
    }
    if alive == 0 {
        return 500_000 + hp as i64 * 100;
    }

    let mut s: i64 = 0;
    s += dead * 10_000;
    s += state.player_hp as i64 * 8;
    s -= (state.player_hp - hp).max(0) as i64 * 100;
    s -= mhp * if alive <= 1 { 35 } else { 10 };
    s -= enemy_strength * 260;
    s -= flight_pressure;
    s += split_quality_bonus;

    let vuln: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].vulnerable.min(4) as i64)
        .sum();
    let weak: i64 = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].weak.min(4) as i64)
        .sum();
    s += vuln * 500 + weak * 400;
    s += state.draw_bonus;
    s += block.min(30) as i64;
    s += state.energy as i64;
    s -= playable_slimed_penalty(state) as i64;
    s -= stranded_double_tap_penalty(state) as i64;
    s -= stranded_energy_penalty(state) as i64;
    s += (state.player_artifact.min(3) as i64) * 450;
    if state.has_corruption {
        s += 2_400;
    }
    if state.has_feel_no_pain {
        s += 1_800;
    }
    if state.has_dark_embrace {
        s += 1_700;
    }
    if state.has_rupture {
        s += 1_000;
    }
    if state.has_combust {
        s += 1_300;
    }
    if state.has_metallicize {
        s += 1_400 + alive * 250;
    }
    if state.has_evolve {
        s += 1_000 + (state.future_status_cards as i64) * 220;
    }
    if state.has_brutality {
        s += 1_100;
    }
    if state.has_berserk {
        s += 900 + state.energy as i64 * 120;
    }
    if state.has_panache {
        s += 1_000;
    }
    if state.has_mayhem {
        s += 1_200;
    }
    if state.has_magnetism {
        s += 800;
    }
    s += state.future_growth_value as i64;
    s
}

fn slime_boss_split_state_bonus(state: &SimState, monster: &SimMonster) -> i32 {
    if monster.monster_type != EnemyId::SlimeBoss as usize || monster.hp <= 0 {
        return 0;
    }

    const SPLIT_THRESHOLD: i32 = 70;
    if monster.hp > SPLIT_THRESHOLD {
        return 0;
    }

    let hp = monster.hp.max(0);
    let slam_pressure = if monster.is_attacking {
        monster.intent_dmg.max(0) * monster.intent_hits.max(1)
    } else {
        0
    };
    let imminent = imminent_unblocked_damage(state);
    let forced_split = imminent >= state.player_hp.saturating_sub(6)
        || slam_pressure >= 30 && imminent >= state.player_hp.saturating_sub(12);

    let mut bonus = -(hp * 180);
    if hp <= 35 {
        bonus += 8_000;
    } else if hp <= 45 {
        bonus += 3_000;
    } else if hp >= 60 {
        bonus -= 7_000;
    } else if hp >= 50 {
        bonus -= 2_500;
    }

    if forced_split {
        bonus += 7_000 + (SPLIT_THRESHOLD - hp).max(0) * 140;
    }

    bonus
}

fn playable_slimed_penalty(state: &SimState) -> i32 {
    let slimed_count = active_hand_cards(state)
        .filter(|(_, c)| {
            c.card_id == CardId::Slimed && effective_energy_cost(state, c) <= state.energy
        })
        .count() as i32;
    if slimed_count <= 0 {
        return 0;
    }

    let other_non_status_plays = active_hand_cards(state)
        .filter(|(_, c)| c.card_id != CardId::Slimed)
        .filter(|(_, c)| c.card_type != CardType::Curse && c.card_type != CardType::Status)
        .filter(|(_, c)| effective_energy_cost(state, c) <= state.energy)
        .count() as i32;

    let mut penalty = slimed_count * 2_400;
    if other_non_status_plays == 0 {
        penalty += 4_500 + state.energy.max(0) * 400;
    }
    penalty
}

fn stranded_double_tap_penalty(state: &SimState) -> i32 {
    if !state.double_tap_active {
        return 0;
    }

    let has_followup = active_hand_cards(state).any(|(_, c)| {
        c.card_type == CardType::Attack && effective_energy_cost(state, c) <= state.energy
    });
    if has_followup {
        0
    } else {
        5_500
    }
}

fn stranded_energy_penalty(state: &SimState) -> i32 {
    if state.energy <= 0 {
        return 0;
    }

    let playable_non_conversion = active_hand_cards(state)
        .filter(|(_, c)| {
            effective_energy_cost(state, c) <= state.energy
                && !matches!(c.card_type, CardType::Curse | CardType::Status)
                && !matches!(c.card_id, CardId::Bloodletting | CardId::SeeingRed)
        })
        .count() as i32;

    if playable_non_conversion > 0 {
        return 0;
    }

    4_000 + state.energy.max(0) * 650
}

pub(super) fn total_incoming_damage(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_dmg * state.monsters[i].intent_hits.max(1))
        .sum()
}

fn imminent_unblocked_damage(state: &SimState) -> i32 {
    (total_incoming_damage(state) - state.player_block).max(0)
}

fn resource_conversion_timing_value(state: &SimState, card_idx: usize, hp_cost: i32) -> i32 {
    let card = &state.hand[card_idx];
    let cost = effective_energy_cost(state, card);
    let post_energy = (state.energy - cost + card.base_magic).max(0);
    let imminent = imminent_unblocked_damage(state);

    let mut playable_before = 0;
    let mut playable_after = 0;
    let mut newly_unlocked = 0;
    let mut unlocked_damage = 0;
    let mut unlocked_block = 0;
    let mut key_followup_weight = 0;

    for (idx, other) in active_hand_cards(state) {
        if idx == card_idx || matches!(other.card_type, CardType::Curse | CardType::Status) {
            continue;
        }

        let other_cost = effective_energy_cost(state, other);
        let before = other_cost <= state.energy;
        let after = other_cost <= post_energy;
        if before {
            playable_before += 1;
        }
        if after {
            playable_after += 1;
        }
        if !before && after {
            newly_unlocked += 1;
            unlocked_damage += other.base_damage.max(0) * effective_hits(other, other_cost).max(1);
            unlocked_block += other.base_block.max(0);
            key_followup_weight += match other.card_id {
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

    let mut value = 1_200 + newly_unlocked * 3_500;
    value += unlocked_damage.min(30) * 220;
    value += unlocked_block.min(imminent.max(0)).min(20) * 260;
    value += key_followup_weight * 1_800;

    if newly_unlocked <= 0 {
        value -= 5_500;
    }
    if playable_after <= 0 {
        value -= 8_500;
    }
    if playable_before > 0 && newly_unlocked <= 0 {
        value -= 2_500;
    }
    if hp_cost > 0 {
        value -= hp_cost * 1_000;
        if imminent >= state.player_hp.saturating_sub(4) {
            value -= hp_cost * 1_300 + 1_800;
        } else if state.player_hp <= 12 {
            value -= hp_cost * 900;
        }
    }

    value
}

fn max_intent_hits(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_hits.max(1))
        .max()
        .unwrap_or(0)
}

fn best_attack_target_value(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| {
            let m = &state.monsters[i];
            m.hp.max(0)
                + if m.persistent_block {
                    m.block.max(0)
                } else {
                    0
                }
        })
        .max()
        .unwrap_or(0)
}

fn best_reflect_threat_in_hand_followup(state: &SimState, current_idx: usize) -> i32 {
    let energy_left = state.energy - effective_energy_cost(state, &state.hand[current_idx]);
    if energy_left < 0 {
        return 0;
    }

    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_idx
                && c.card_type == CardType::Attack
                && effective_energy_cost(state, c) <= energy_left
        })
        .map(|(_, c)| {
            let hits = effective_hits(c, effective_energy_cost(state, c));
            expected_reflect_damage(state, c, None, hits)
        })
        .max()
        .unwrap_or(0)
}

fn apotheosis_timing_value(state: &SimState) -> i32 {
    apotheosis_timing_score(
        armaments_upgradable_count(state) as i32,
        imminent_unblocked_damage(state),
    )
}

fn apparition_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let incoming = total_incoming_damage(state);
    let imminent = imminent_unblocked_damage(state);
    let hits = max_intent_hits(state);
    let alive = alive_monster_count(state);
    let encounter_pressure = state.enemy_strength_sum.max(0) * 2
        + alive.max(0) * 2
        + if state.is_boss_fight {
            6
        } else if state.is_elite_fight {
            3
        } else {
            0
        };
    let mut value = apparition_timing_score(&ApparitionTimingContext {
        current_hp: state.player_hp,
        current_intangible: state.player_intangible,
        imminent_unblocked_damage: imminent,
        total_incoming_damage: incoming,
        apparitions_in_hand: active_hand_cards(state)
            .filter(|(_, c)| c.card_id == CardId::Apparition)
            .count() as i32,
        remaining_apparitions_total: state.remaining_apparitions_total,
        upgraded: card.upgrades > 0,
        has_runic_pyramid: state.has_runic_pyramid,
        encounter_pressure,
    });

    if imminent >= state.player_hp / 2 {
        value += 4_500;
    }
    if hits >= 2 && incoming > 0 {
        value += 2_500 + incoming * 140;
    }
    if state.player_frail && incoming > state.player_block {
        value += 3_000 + imminent.min(state.player_hp.max(0)) * 120;
    }
    if state.player_hp <= 35 && incoming >= 14 {
        value += 2_500;
    }

    value
}

fn alive_monster_count(state: &SimState) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .count() as i32
}

fn flame_barrier_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let retaliate_per_hit = card.base_magic.max(4);
    let retaliation_hits = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_hits.max(1))
        .sum::<i32>();
    if retaliation_hits <= 0 {
        return if total_incoming_damage(state) <= state.player_block {
            -3_500
        } else {
            0
        };
    }

    let attacking_monsters = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .count() as i32;
    let mut score = retaliation_hits * retaliate_per_hit * 180 + attacking_monsters * 900;
    if max_intent_hits(state) >= 2 {
        score += 3_500 + retaliation_hits * retaliate_per_hit * 60;
    }
    if imminent_unblocked_damage(state) > 0 {
        score += 2_000;
    }
    score
}

fn armaments_upgradable_count(state: &SimState) -> usize {
    active_hand_cards(state)
        .filter(|(_, c)| is_armaments_upgrade_candidate(c))
        .count()
}

pub(super) fn best_armaments_upgrade_target(state: &SimState) -> Option<usize> {
    active_hand_cards(state)
        .filter(|(_, c)| is_armaments_upgrade_candidate(c))
        .max_by_key(|(_, c)| armaments_upgrade_score(c))
        .map(|(idx, _)| idx)
}

fn best_armaments_upgrade_value(state: &SimState, card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| *idx != card_idx && is_armaments_upgrade_candidate(c))
        .map(|(_, c)| armaments_upgrade_score(c))
        .max()
        .unwrap_or(0)
}

fn armaments_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let upgradable = armaments_upgradable_count(state) as i32;
    if card.upgrades > 0 {
        9_500 + upgradable * 1_200 + armaments_plus_frontload_bonus(state, card_idx)
    } else {
        6_500 + best_armaments_upgrade_value(state, card_idx)
    }
}

fn armaments_plus_frontload_bonus(state: &SimState, card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| *idx != card_idx && is_armaments_upgrade_candidate(c))
        .map(|(_, c)| {
            let mut score = armaments_upgrade_score(c);
            score += match c.card_type {
                CardType::Power => 2_600,
                CardType::Skill => 500,
                CardType::Attack => 0,
                _ => 0,
            };
            if is_armaments_frontload_priority_card(c.card_id) {
                score += 1_800;
            }
            score
        })
        .sum::<i32>()
        .min(11_000)
}

fn armaments_upgrade_score(card: &SimCard) -> i32 {
    let def = get_card_definition(card.card_id);
    let mut score = def.upgrade_damage * 180 + def.upgrade_block * 130 + def.upgrade_magic * 210;
    if is_armaments_upgrade_priority_card(card.card_id) {
        score += 1_500;
    }
    score
}

fn is_armaments_upgrade_candidate(card: &SimCard) -> bool {
    card.upgrades == 0 && !matches!(card.card_type, CardType::Status | CardType::Curse)
}

fn is_armaments_frontload_priority_card(card_id: CardId) -> bool {
    taxonomy(card_id).is_armaments_frontload_priority()
}

fn is_armaments_upgrade_priority_card(card_id: CardId) -> bool {
    taxonomy(card_id).is_armaments_upgrade_priority()
}

fn disarm_timing_value(state: &SimState) -> i32 {
    let incoming = total_incoming_damage(state);
    let imminent = imminent_unblocked_damage(state);
    let attacking_monsters = active_monsters(state)
        .filter(|(_, m)| m.is_attacking)
        .count() as i32;
    let multi_hit_pressure = active_monsters(state)
        .filter(|(_, m)| m.is_attacking)
        .map(|(_, m)| (m.intent_hits - 1).max(0))
        .sum::<i32>();
    let strength_pressure = active_monsters(state)
        .filter(|(_, m)| m.is_attacking)
        .map(|(_, m)| m.strength.max(0))
        .sum::<i32>();
    let mut value = 1_800
        + imminent * 220
        + attacking_monsters * 450
        + multi_hit_pressure * 900
        + strength_pressure * 320;

    if imminent <= 0 && multi_hit_pressure == 0 && strength_pressure == 0 {
        value -= 5_000;
    }
    if attacking_monsters <= 1 && incoming <= 8 {
        value -= 2_400;
    }
    if state.player_hp >= 24 && imminent <= 0 {
        value -= 1_600;
    }

    value
}

fn gremlin_nob_skill_relief(card_id: CardId, threatened: bool) -> i32 {
    if !threatened {
        return 0;
    }

    match card_id {
        CardId::GhostlyArmor | CardId::FlameBarrier | CardId::Impervious => 8_000,
        CardId::Shockwave | CardId::Disarm => 5_500,
        CardId::Armaments => 1_500,
        _ => 0,
    }
}

fn gremlin_nob_skill_penalty_value(state: &SimState, card: &SimCard) -> i32 {
    let threatened = total_incoming_damage(state) > state.player_block;
    let mut penalty = 11_000;
    penalty -= gremlin_nob_skill_relief(card.card_id, threatened);
    if best_attack_target_value(state) >= 12 {
        penalty -= 1_500;
    }
    if threatened && imminent_unblocked_damage(state) > 0 {
        penalty += 1_000;
    }
    penalty
}

