use crate::bot::strategy_families::{
    apotheosis_hand_shaping_score, apotheosis_timing_score, apparition_hand_shaping_score,
    apparition_timing_score, battle_trance_timing_score, body_slam_delay_score,
    deck_cycle_thinning_score, draw_action_timing_score, draw_continuity_score,
    exhaust_engine_payoff_score, exhaust_engine_setup_score, exhaust_finish_window_score,
    exhaust_fuel_value_score, exhaust_future_fuel_reserve_score, exhaust_mass_play_score,
    exhaust_random_core_risk_score, exhaust_random_play_score,
    forced_mass_exhaust_selectivity_score, reaper_hand_shaping_score, reaper_timing_score,
    status_loop_cycle_score,
};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;

use super::apply::{
    effective_damage, effective_energy_cost, effective_hits, expected_reflect_damage,
};
use super::sim::{active_hand_cards, gremlin_nob_enrage_active, SimCard, SimMonster, SimState};

pub(super) fn play_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p: i32 = 0;

    if card.card_type == CardType::Power {
        p += 10_000;
    }

    match card.card_id {
        CardId::Corruption => {
            p += exhaust_engine_setup_score(
                state.has_corruption,
                immediate_exhaust_count(state),
                future_exhaust_source_count(state, card_idx),
                if state.has_feel_no_pain { 4 } else { 0 },
                if state.has_dark_embrace { 1 } else { 0 },
                status_cards_in_hand(state),
                future_status_card_count(state),
                state.sentry_count,
                active_hand_cards(state)
                    .filter(|(idx, c)| {
                        *idx != card_idx && c.card_type == CardType::Skill && c.cost > 0
                    })
                    .count() as i32
                    * 1_000,
            );
        }
        CardId::FeelNoPain => {
            p += exhaust_engine_setup_score(
                state.has_feel_no_pain,
                immediate_exhaust_count(state),
                future_exhaust_source_count(state, card_idx),
                4,
                0,
                status_cards_in_hand(state),
                future_status_card_count(state),
                state.sentry_count,
                0,
            );
        }
        CardId::DarkEmbrace => {
            p += exhaust_engine_setup_score(
                state.has_dark_embrace,
                immediate_exhaust_count(state),
                future_exhaust_source_count(state, card_idx),
                0,
                1,
                status_cards_in_hand(state),
                future_status_card_count(state),
                state.sentry_count,
                0,
            );
        }
        CardId::Rupture => {
            p += if state.has_rupture { -7_500 } else { 8_500 };
            p += self_damage_cards_in_hand(state) * 1_600;
        }
        CardId::Combust => {
            p += if state.has_combust { -7_500 } else { 8_500 };
            p += alive_monster_count(state) * 500;
        }
        CardId::Metallicize => {
            p += if state.has_metallicize { -8_000 } else { 8_500 };
            p += alive_monster_count(state) * 500;
            p += total_incoming_damage(state).min(18) * 120;
        }
        CardId::Evolve => {
            p += if state.has_evolve { -8_000 } else { 8_000 };
            p += status_loop_cycle_score(
                1,
                state.status_in_draw,
                state.status_in_discard,
                true,
                0,
                state.sentry_count,
            );
            if active_hand_cards(state).any(|(_, c)| c.card_id == CardId::DeepBreath) {
                p += 1_500;
            }
        }
        CardId::FireBreathing => {
            p += fire_breathing_play_value(state);
        }
        CardId::Brutality => {
            p += if state.has_brutality { -7_500 } else { 8_000 };
            if state.player_hp * 2 > 0.max(state.player_hp) {
                p += 1_000;
            }
        }
        CardId::Berserk => {
            p += if state.has_berserk { -8_000 } else { 6_000 };
            if state.player_artifact > 0 {
                p += 3_000;
            }
            if state.energy <= 1 {
                p += 2_000;
            }
        }
        CardId::Inflame | CardId::SpotWeakness => p += 9_500,
        CardId::Flex => p += 9_000,
        CardId::Apotheosis => p += apotheosis_timing_value(state),
        CardId::Apparition => p += apparition_timing_value(state, card),
        CardId::Armaments => {
            let upgradable = armaments_upgradable_count(state) as i32;
            if card.upgrades > 0 {
                p += 9_500 + upgradable * 1_200;
                p += armaments_plus_frontload_bonus(state, card_idx);
            } else {
                p += 6_500 + best_armaments_upgrade_value(state, card_idx);
            }
        }
        CardId::PowerThrough => {
            let needed_block = power_through_needed_block(state, card);
            let wound_penalty = power_through_wound_penalty(state);
            p += needed_block * 180;
            p -= wound_penalty;
            if needed_block <= 0 {
                p -= 5_500;
            }
        }
        CardId::FlameBarrier => p += flame_barrier_timing_value(state, card),
        CardId::DoubleTap => {
            if state.double_tap_active {
                p -= 8_000;
            } else {
                let followup = best_followup_attack_value(state, card_idx);
                if followup > 0 {
                    p += 3_000 + followup;
                } else {
                    p -= 4_500;
                }
            }
        }
        CardId::Panache => {
            p += if state.has_panache { -8_000 } else { 9_000 };
            p += active_hand_cards(state)
                .filter(|(_, c)| c.cost == 0 || c.cost == 1)
                .count() as i32
                * 500;
        }
        CardId::Mayhem => p += if state.has_mayhem { -8_000 } else { 8_000 },
        CardId::Magnetism => p += if state.has_magnetism { -8_000 } else { 7_000 },
        CardId::Panacea => {
            p += 7_500;
            let hand_has_flex_combo = active_hand_cards(state).any(|(_, c)| {
                matches!(
                    c.card_id,
                    CardId::Flex
                        | CardId::Bloodletting
                        | CardId::Offering
                        | CardId::Hemokinesis
                        | CardId::Combust
                )
            });
            if hand_has_flex_combo {
                p += 4_000;
            }
            if state.player_artifact == 0 {
                p += 1_500;
            } else {
                p -= 2_500;
            }
            let enemy_might_debuff = (0..state.monster_count as usize)
                .any(|i| !state.monsters[i].is_gone && !state.monsters[i].is_attacking);
            if enemy_might_debuff {
                p += 1_500;
            }
        }
        CardId::Blind => p += threat_reduction_value(state, card.base_magic) + 5_500,
        CardId::DarkShackles => p += threat_reduction_value(state, card.base_magic) + 7_000,
        CardId::Trip => {
            p += best_attack_target_value(state) * card.base_magic + 6_500;
            p += best_followup_attack_value_with_vuln(state, card_idx, card.base_magic);
        }
        CardId::Whirlwind => {
            let hits = effective_hits(card, state.energy);
            p += hits * alive_monster_count(state) * 1_000 + hits * 900;
            if alive_monster_count(state) >= 2 {
                p += 2_500;
            }
        }
        CardId::Reaper => p += reaper_swing_value(state, card),
        CardId::BurningPact => p += burning_pact_exhaust_value(state, card_idx),
        CardId::SecondWind => p += second_wind_exhaust_value(state, card_idx),
        CardId::BodySlam => p += body_slam_timing_value(state, card_idx),
        CardId::SeverSoul => p += sever_soul_exhaust_value(state, card_idx),
        CardId::FiendFire => p += fiend_fire_exhaust_value(state, card_idx),
        CardId::TrueGrit if card.upgrades <= 0 => {
            p += true_grit_random_exhaust_value(state, card_idx)
        }
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
        CardId::DeepBreath => {
            p += draw_continuity_score(state.card_pool_size, 1, 0, state.discard_pile_size);
            p += status_loop_cycle_score(
                i32::from(state.has_evolve),
                state.status_in_draw,
                state.status_in_discard,
                true,
                1,
                state.sentry_count,
            );
        }
        CardId::SecretTechnique | CardId::SecretWeapon | CardId::Discovery => p += 6_500,
        CardId::TheBomb => p += alive_monster_count(state) * 1_100 + 6_000,
        CardId::Offering => p += 8_800 + generic_draw_timing_value(state, card_idx, 3, false),
        CardId::Bloodletting | CardId::SeeingRed => p += 8_500,
        CardId::Bash | CardId::Shockwave => p += 8_000,
        CardId::Uppercut | CardId::ThunderClap => p += 7_800,
        CardId::Clothesline | CardId::Intimidate => p += 7_500,
        CardId::Disarm => {
            let mut threat = total_incoming_damage(state);
            if max_intent_hits(state) >= 2 {
                threat += 6;
            }
            p += threat * 220 + 7_500;
        }
        CardId::LimitBreak => {
            let strength = state.player_strength.max(0);
            let followup = best_followup_attack_value(state, card_idx);
            if strength <= 0 {
                p -= 7_000;
            } else {
                p += strength * 2_000;
                if strength >= 3 {
                    p += 5_500;
                } else if strength == 2 {
                    p += 1_500;
                } else {
                    p -= 1_000;
                }
                p += followup / 2;
                if followup == 0 && strength <= 2 {
                    p -= 2_500;
                }
            }
        }
        _ => {}
    }

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
    if card.card_type == CardType::Attack {
        p += 5_000 + card.base_damage * effective_hits(card, state.energy) * 10;
    }
    if card.base_block > 0 {
        p += 3_000 + card.base_block * 10;
    }
    p
}

pub(super) fn evaluate(state: &SimState) -> i64 {
    let mut hp = state.player_hp;
    let mut block = state.player_block;

    for mi in 0..state.monster_count as usize {
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

    let alive = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .count() as i64;
    let dead = state.monster_count as i64 - alive;
    let mhp: i64 = (0..state.monster_count as usize)
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
    let enemy_strength: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].strength.max(0) as i64)
        .sum();
    let split_quality_bonus: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| slime_boss_split_state_bonus(state, &state.monsters[i]) as i64)
        .sum();
    let flight_pressure: i64 = (0..state.monster_count as usize)
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

    let vuln: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].vulnerable.min(4) as i64)
        .sum();
    let weak: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].weak.min(4) as i64)
        .sum();
    s += vuln * 500 + weak * 400;
    s += state.draw_bonus;
    s += block.min(30) as i64;
    s += state.energy as i64;
    s -= playable_slimed_penalty(state) as i64;
    s -= stranded_double_tap_penalty(state) as i64;
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

pub(super) fn total_incoming_damage(state: &SimState) -> i32 {
    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_dmg * state.monsters[i].intent_hits.max(1))
        .sum()
}

fn max_intent_hits(state: &SimState) -> i32 {
    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| state.monsters[i].intent_hits.max(1))
        .max()
        .unwrap_or(0)
}

fn best_attack_target_value(state: &SimState) -> i32 {
    (0..state.monster_count as usize)
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

fn estimated_reaper_heal(state: &SimState, card: &SimCard) -> i32 {
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return 0;
    }

    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| {
            let m = &state.monsters[i];
            let mut dmg = base;
            if m.vulnerable > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            hp_loss.min(m.hp.max(0))
        })
        .sum()
}

fn estimated_reaper_kills(state: &SimState, card: &SimCard) -> i32 {
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return 0;
    }

    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| {
            let m = &state.monsters[i];
            let mut dmg = base;
            if m.vulnerable > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            i32::from(hp_loss >= m.hp.max(0))
        })
        .sum()
}

fn missing_hp(state: &SimState) -> i32 {
    (state.player_max_hp - state.player_hp).max(0)
}

fn imminent_unblocked_damage(state: &SimState) -> i32 {
    (total_incoming_damage(state) - state.player_block).max(0)
}

fn reaper_swing_value(state: &SimState, card: &SimCard) -> i32 {
    let heal = estimated_reaper_heal(state, card);
    let kills = estimated_reaper_kills(state, card);
    let kill_prevention = estimated_reaper_kill_prevention(state, card);
    reaper_timing_score(
        state.player_hp,
        imminent_unblocked_damage(state),
        missing_hp(state),
        heal,
        kill_prevention,
        kills,
    )
}

fn battle_trance_timing_value(state: &SimState, current_idx: usize) -> i32 {
    let other_draw_sources_in_hand = active_hand_cards(state)
        .filter(|(idx, c)| *idx != current_idx && is_draw_source(c.card_id))
        .count() as i32;

    battle_trance_timing_score(
        state.energy,
        state.player_no_draw,
        state.hand[current_idx].base_magic.max(0),
        active_hand_cards(state).count() as i32,
        state.future_zero_cost_cards,
        state.future_one_cost_cards,
        state.future_two_plus_cost_cards,
        state.future_key_delay_weight,
        state.future_high_cost_key_delay_weight,
        state.status_in_draw + state.status_in_discard,
        other_draw_sources_in_hand,
    )
}

fn generic_draw_timing_value(
    state: &SimState,
    current_idx: usize,
    draw_count: i32,
    applies_no_draw: bool,
) -> i32 {
    let other_draw_sources_in_hand = active_hand_cards(state)
        .filter(|(idx, c)| *idx != current_idx && is_draw_source(c.card_id))
        .count() as i32;

    draw_action_timing_score(
        state.energy,
        state.player_no_draw,
        applies_no_draw,
        draw_count,
        active_hand_cards(state).count() as i32,
        state.future_zero_cost_cards,
        state.future_one_cost_cards,
        state.future_two_plus_cost_cards,
        state.future_key_delay_weight,
        state.future_high_cost_key_delay_weight,
        state.status_in_draw + state.status_in_discard,
        other_draw_sources_in_hand,
    )
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

fn estimated_reaper_kill_prevention(state: &SimState, card: &SimCard) -> i32 {
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return 0;
    }

    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| {
            let m = &state.monsters[i];
            let mut dmg = base;
            if m.vulnerable > 0 {
                dmg = (dmg as f32 * 1.5).floor() as i32;
            }
            let hp_loss = (dmg - m.block).max(0);
            if hp_loss >= m.hp.max(0) {
                m.intent_dmg.max(0) * m.intent_hits.max(1)
            } else {
                0
            }
        })
        .sum()
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
    let mut value = apparition_timing_score(
        state.player_hp,
        state.player_intangible,
        imminent,
        incoming,
        active_hand_cards(state)
            .filter(|(_, c)| c.card_id == CardId::Apparition)
            .count() as i32,
        state.remaining_apparitions_total,
        card.upgrades > 0,
        state.has_runic_pyramid,
        encounter_pressure,
    );

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

fn threat_reduction_value(state: &SimState, amount: i32) -> i32 {
    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| amount.min(state.monsters[i].intent_dmg.max(0)))
        .max()
        .unwrap_or(0)
        * 700
}

fn alive_monster_count(state: &SimState) -> i32 {
    (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .count() as i32
}

fn self_damage_cards_in_hand(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| {
            matches!(
                c.card_id,
                CardId::Bloodletting | CardId::Offering | CardId::Hemokinesis | CardId::Combust
            )
        })
        .count() as i32
}

fn immediate_exhaust_count(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| {
            matches!(
                c.card_id,
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

fn future_exhaust_source_count(state: &SimState, current_card_idx: usize) -> i32 {
    future_exhaust_demand(state, current_card_idx)
}

fn status_cards_in_hand(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, c)| matches!(c.card_type, CardType::Status | CardType::Curse))
        .count() as i32
}

fn future_status_card_count(state: &SimState) -> i32 {
    state.status_in_draw + state.status_in_discard + status_cards_in_hand(state)
}

fn total_cycle_cards(state: &SimState) -> i32 {
    state.card_pool_size
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
        total_cycle_cards(state),
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
    mass_exhaust_value(state, card_idx, finish_window)
        + second_wind_selectivity_bonus(state, &exhausted, finish_window > 0)
        + exhaust_engine_payoff_bonus(state, exhausted.len() as i32)
        + deck_cycle_thinning_score(
            total_cycle_cards(state),
            state.card_pool_size - exhausted.len() as i32,
            if state.has_dark_embrace {
                exhausted.len() as i32
            } else {
                0
            },
            0,
            0,
            0,
        )
}

fn sever_soul_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let exhaust_count = exhaust_candidate_indices(state, card_idx).len() as i32;
    let total = mass_exhaust_value(state, card_idx, 0)
        + exhaust_engine_payoff_bonus(state, exhaust_count)
        + deck_cycle_thinning_score(
            total_cycle_cards(state),
            state.card_pool_size - exhaust_count,
            if state.has_dark_embrace {
                exhaust_count
            } else {
                0
            },
            0,
            0,
            0,
        );
    (total as f32 * 0.8) as i32
}

fn fiend_fire_exhaust_value(state: &SimState, card_idx: usize) -> i32 {
    let exhausted = exhaust_candidate_indices(state, card_idx);
    let closeout_bonus = fiend_fire_closeout_bonus(state, card_idx, exhausted.len() as i32);
    mass_exhaust_value(state, card_idx, closeout_bonus)
        + exhaust_engine_payoff_bonus(state, exhausted.len() as i32)
        + deck_cycle_thinning_score(
            total_cycle_cards(state),
            state.card_pool_size - exhausted.len() as i32,
            if state.has_dark_embrace {
                exhausted.len() as i32
            } else {
                0
            },
            0,
            0,
            0,
        )
}

fn body_slam_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let current_damage = state.player_block.max(0);
    let additional_block = max_additional_block_before_body_slam(state, card_idx);
    let can_kill_now = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .any(|i| current_damage >= state.monsters[i].hp.max(0) + state.monsters[i].block.max(0));
    body_slam_delay_score(
        current_damage,
        additional_block,
        can_kill_now,
        imminent_unblocked_damage(state),
    )
}

fn flame_barrier_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let retaliate_per_hit = card.base_magic.max(4);
    let retaliation_hits = (0..state.monster_count as usize)
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

    let attacking_monsters = (0..state.monster_count as usize)
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
            total_cycle_cards(state),
            state.card_pool_size - 1,
            i32::from(state.has_dark_embrace),
            0,
            0,
            0,
        )
}

fn mass_exhaust_value(state: &SimState, card_idx: usize, closeout_bonus: i32) -> i32 {
    let exhausted = exhaust_candidate_indices(state, card_idx);
    if exhausted.is_empty() {
        return -3_500;
    }

    let total_fuel = exhausted
        .iter()
        .map(|idx| exhaust_fuel_value_for_index(state, *idx))
        .sum::<i32>();
    exhaust_mass_play_score(
        total_fuel,
        exhausted.len() as i32,
        state.card_pool_size - exhausted.len() as i32,
        0,
        closeout_bonus,
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

fn second_wind_selectivity_bonus(
    state: &SimState,
    exhausted: &[usize],
    exact_stabilize: bool,
) -> i32 {
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
    let mut score = forced_mass_exhaust_selectivity_score(
        junk_fuel_count,
        protected_piece_count,
        core_piece_count,
        exact_stabilize,
        imminent_unblocked_damage(state),
        engine_support_level,
    );
    let playable_block_lost = exhausted
        .iter()
        .filter(|idx| {
            let card = &state.hand[**idx];
            effective_energy_cost(state, card) <= state.energy && card.base_block > 0
        })
        .count() as i32;
    if playable_block_lost > 0 {
        let emergency_relief = if exact_stabilize {
            0
        } else {
            imminent_unblocked_damage(state).min(18) * 180
        };
        let preserve_penalty =
            (playable_block_lost * 6_200) - junk_fuel_count * 650 - emergency_relief;
        score -= preserve_penalty.max(0);
        if junk_fuel_count >= 1 && playable_block_lost >= 2 && !exact_stabilize {
            score -= 12_000 + playable_block_lost * 2_500;
        }
    }
    score
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
        CardId::Apparition => apparition_hand_shaping_score(
            state.player_hp,
            state.player_intangible,
            imminent_unblocked_damage(state),
            total_incoming_damage(state),
            active_hand_cards(state)
                .filter(|(_, c)| c.card_id == CardId::Apparition)
                .count() as i32,
            state.remaining_apparitions_total,
            card.upgrades > 0,
            state.has_runic_pyramid,
            state.enemy_strength_sum.max(0) * 2
                + alive_monster_count(state).max(0) * 2
                + if state.is_boss_fight {
                    6
                } else if state.is_elite_fight {
                    3
                } else {
                    0
                },
        ),
        CardId::Reaper => reaper_hand_shaping_score(
            state.player_hp,
            imminent_unblocked_damage(state),
            missing_hp(state),
        ),
        _ => 0,
    }
}

fn fiend_fire_closeout_bonus(state: &SimState, card_idx: usize, exhausted_cards: i32) -> i32 {
    let card = &state.hand[card_idx];
    let total_damage = effective_damage(state, card).max(0) * exhausted_cards.max(0);
    let best_target_margin = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| (total_damage - state.monsters[i].block).max(0) - state.monsters[i].hp.max(0))
        .max()
        .unwrap_or(i32::MIN / 4);
    let kills = (0..state.monster_count as usize)
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
    (0..state.monster_count as usize)
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

fn exhaust_engine_payoff_bonus(state: &SimState, exhaust_count: i32) -> i32 {
    let status_fuel_count = active_hand_cards(state)
        .filter(|(_, c)| matches!(c.card_type, CardType::Status | CardType::Curse))
        .count() as i32;
    exhaust_engine_payoff_score(
        exhaust_count,
        if state.has_feel_no_pain { 4 } else { 0 },
        if state.has_dark_embrace { 1 } else { 0 },
        status_fuel_count,
        if state.has_evolve {
            state.future_status_cards
        } else {
            0
        },
    )
}

fn max_additional_block_before_body_slam(state: &SimState, card_idx: usize) -> i32 {
    let budget = (state.energy - effective_energy_cost(state, &state.hand[card_idx])).max(0);
    if budget <= 0 {
        return 0;
    }

    let mut best = vec![0; budget as usize + 1];
    for (idx, card) in active_hand_cards(state) {
        if idx == card_idx {
            continue;
        }
        let cost = effective_energy_cost(state, card);
        if cost < 0 || cost > budget {
            continue;
        }
        let block_gain = body_slam_followup_block_gain(state, card_idx, idx);
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

fn body_slam_followup_block_gain(state: &SimState, body_slam_idx: usize, idx: usize) -> i32 {
    let card = &state.hand[idx];
    if matches!(card.card_type, CardType::Curse | CardType::Status) {
        return 0;
    }

    let mut block_gain = match card.card_id {
        CardId::SecondWind => {
            let exhaust_count = active_hand_cards(state)
                .filter(|(other_idx, other)| {
                    *other_idx != idx
                        && *other_idx != body_slam_idx
                        && other.card_type != CardType::Attack
                })
                .count() as i32;
            (card.base_block + state.player_dexterity).max(0) * exhaust_count.max(0)
        }
        _ if card.base_block > 0 => (card.base_block + state.player_dexterity).max(0),
        _ => 0,
    };
    if state.player_frail {
        block_gain = (block_gain as f32 * 0.75).floor() as i32;
    }
    block_gain.max(0)
}

fn armaments_upgradable_count(state: &SimState) -> usize {
    active_hand_cards(state)
        .filter(|(_, c)| {
            c.upgrades == 0 && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .count()
}

pub(super) fn best_armaments_upgrade_target(state: &SimState) -> Option<usize> {
    active_hand_cards(state)
        .filter(|(_, c)| {
            c.upgrades == 0 && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .max_by_key(|(_, c)| armaments_upgrade_score(c))
        .map(|(idx, _)| idx)
}

fn best_armaments_upgrade_value(state: &SimState, card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != card_idx
                && c.upgrades == 0
                && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .map(|(_, c)| armaments_upgrade_score(c))
        .max()
        .unwrap_or(0)
}

fn best_followup_attack_value(state: &SimState, current_idx: usize) -> i32 {
    let energy_left = state.energy - effective_energy_cost(state, &state.hand[current_idx]);
    if energy_left <= 0 {
        return 0;
    }

    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_idx
                && c.card_type == CardType::Attack
                && ((c.cost >= 0 && c.cost <= energy_left) || (c.cost < 0 && energy_left > 0))
        })
        .map(|(_, c)| {
            let energy_for_card = if c.cost < 0 { energy_left } else { c.cost };
            let damage = effective_damage(state, c) * effective_hits(c, energy_for_card);
            damage * 250
                + match c.card_id {
                    CardId::Bash
                    | CardId::Uppercut
                    | CardId::Hemokinesis
                    | CardId::BloodForBlood
                    | CardId::SwordBoomerang
                    | CardId::Pummel
                    | CardId::Rampage
                    | CardId::HeavyBlade
                    | CardId::Whirlwind => 2_000,
                    _ => 0,
                }
        })
        .max()
        .unwrap_or(0)
}

fn best_followup_attack_value_with_vuln(
    state: &SimState,
    current_idx: usize,
    vuln_amount: i32,
) -> i32 {
    let energy_left = state.energy - effective_energy_cost(state, &state.hand[current_idx]);
    if energy_left <= 0 {
        return 0;
    }

    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_idx
                && c.card_type == CardType::Attack
                && ((c.cost >= 0 && c.cost <= energy_left) || (c.cost < 0 && energy_left > 0))
        })
        .map(|(_, c)| {
            let energy_for_card = if c.cost < 0 { energy_left } else { c.cost };
            let hits = effective_hits(c, energy_for_card);
            let vulnerable_multiplier = if vuln_amount > 0 { 1.5 } else { 1.0 };
            let damage =
                ((effective_damage(state, c) as f32) * vulnerable_multiplier).floor() as i32 * hits;
            damage * 320
                + match c.card_id {
                    CardId::Bash
                    | CardId::Uppercut
                    | CardId::Hemokinesis
                    | CardId::BloodForBlood
                    | CardId::SwordBoomerang
                    | CardId::Pummel
                    | CardId::Rampage
                    | CardId::HeavyBlade
                    | CardId::Whirlwind
                    | CardId::Dropkick => 2_500,
                    _ => 0,
                }
        })
        .max()
        .unwrap_or(0)
}

fn armaments_plus_frontload_bonus(state: &SimState, card_idx: usize) -> i32 {
    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != card_idx
                && c.upgrades == 0
                && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .map(|(_, c)| {
            let mut score = armaments_upgrade_score(c);
            score += match c.card_type {
                CardType::Power => 2_600,
                CardType::Skill => 500,
                CardType::Attack => 0,
                _ => 0,
            };
            score += match c.card_id {
                CardId::Inflame
                | CardId::DemonForm
                | CardId::FeelNoPain
                | CardId::DarkEmbrace
                | CardId::Corruption
                | CardId::Metallicize
                | CardId::Evolve
                | CardId::Berserk
                | CardId::Shockwave
                | CardId::Whirlwind
                | CardId::LimitBreak
                | CardId::HeavyBlade
                | CardId::BattleTrance => 1_800,
                _ => 0,
            };
            score
        })
        .sum::<i32>()
        .min(11_000)
}

fn armaments_upgrade_score(card: &SimCard) -> i32 {
    let def = get_card_definition(card.card_id);
    let mut score = def.upgrade_damage * 180 + def.upgrade_block * 130 + def.upgrade_magic * 210;
    score += match card.card_id {
        CardId::Inflame
        | CardId::DemonForm
        | CardId::FeelNoPain
        | CardId::DarkEmbrace
        | CardId::Corruption
        | CardId::Metallicize
        | CardId::Evolve
        | CardId::Berserk
        | CardId::Bash
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

fn power_through_needed_block(state: &SimState, card: &SimCard) -> i32 {
    let mut block_gain = card.base_block + state.player_dexterity;
    if state.player_frail {
        block_gain = (block_gain as f32 * 0.75).floor() as i32;
    }
    let incoming = total_incoming_damage(state);
    let covered_now = state.player_block.min(incoming);
    let covered_after = (state.player_block + block_gain.max(0)).min(incoming);
    covered_after - covered_now
}

fn fire_breathing_play_value(state: &SimState) -> i32 {
    let status_now = status_cards_in_hand(state);
    let status_backlog = state.future_status_cards.max(0);
    let monster_count = alive_monster_count(state).max(1);
    let sentry_pressure = state.sentry_count.max(0);

    if state.has_fire_breathing {
        return -8_000;
    }

    let immediate_hits = status_now * monster_count * 900;
    let backlog_hits = status_backlog * monster_count * 420;
    let sentry_bonus = sentry_pressure * 4_500;
    let urgent_hand_bonus = if status_now > 0 { 2_500 } else { 0 };

    2_500 + immediate_hits + backlog_hits + sentry_bonus + urgent_hand_bonus
}

fn power_through_wound_penalty(state: &SimState) -> i32 {
    let has_exhaust_out = active_hand_cards(state).any(|(_, c)| {
        matches!(
            c.card_id,
            CardId::SecondWind
                | CardId::BurningPact
                | CardId::TrueGrit
                | CardId::FiendFire
                | CardId::SeverSoul
        )
    });
    let has_status_payoff = state.has_dark_embrace || state.has_feel_no_pain || state.has_evolve;
    let mut penalty = 6_000 + state.future_status_cards.max(0) * 350;
    if has_exhaust_out {
        penalty -= 2_200;
    }
    if has_status_payoff {
        penalty -= 1_600;
    }
    penalty.max(0)
}

fn gremlin_nob_skill_penalty_value(state: &SimState, card: &SimCard) -> i32 {
    let threatened = total_incoming_damage(state) > state.player_block;
    let mut penalty = 14_000;
    penalty -= match card.card_id {
        CardId::GhostlyArmor | CardId::FlameBarrier | CardId::Impervious if threatened => 8_000,
        CardId::Shockwave | CardId::Disarm if threatened => 5_500,
        CardId::Armaments if threatened => 1_500,
        _ => 0,
    };
    penalty
}
