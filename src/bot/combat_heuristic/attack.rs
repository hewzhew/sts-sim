use crate::bot::card_taxonomy::{is_multi_attack_payoff, is_strength_payoff, taxonomy};
use crate::bot::strategy_families::{
    body_slam_delay_score, reaper_timing_score, SurvivalTimingContext,
};
use crate::content::cards::{CardId, CardType};

use super::apply::{effective_damage, effective_energy_cost, effective_hits};
use super::sim::{active_hand_cards, SimCard, SimState};

pub(super) fn attack_timing_value(state: &SimState, card_idx: usize) -> Option<i32> {
    let card = &state.hand[card_idx];
    let value = match card.card_id {
        CardId::SpotWeakness => spot_weakness_timing_value(state, card_idx),
        CardId::Rage => rage_timing_value(state, card_idx),
        CardId::DoubleTap => double_tap_timing_value(state, card_idx),
        CardId::Whirlwind => whirlwind_timing_value(state, card),
        CardId::Reaper => reaper_swing_value(state, card),
        CardId::Feed => feed_timing_value(state, card),
        CardId::BodySlam => body_slam_timing_value(state, card_idx),
        CardId::LimitBreak => limit_break_timing_value(state, card_idx),
        _ => return None,
    };
    Some(value)
}

fn estimated_reaper_heal(state: &SimState, card: &SimCard) -> i32 {
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return 0;
    }

    (0..state.monsters.len())
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

    (0..state.monsters.len())
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

fn estimated_reaper_kill_prevention(state: &SimState, card: &SimCard) -> i32 {
    let base = effective_damage(state, card).max(0);
    if base <= 0 {
        return 0;
    }

    (0..state.monsters.len())
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

fn feed_damage_on_target(state: &SimState, card: &SimCard, monster_idx: usize) -> i32 {
    let m = &state.monsters[monster_idx];
    let mut dmg = effective_damage(state, card).max(0);
    if dmg <= 0 {
        return 0;
    }
    if m.vulnerable > 0 {
        dmg = (dmg as f32 * 1.5).floor() as i32;
    }
    (dmg - m.block.max(0)).max(0)
}

fn feed_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let mut best_kill_value = i32::MIN;
    let mut best_gap = i32::MAX;
    let imminent = imminent_unblocked_damage(state);

    for idx in 0..state.monsters.len() {
        let monster = &state.monsters[idx];
        if monster.is_gone {
            continue;
        }
        let hp_loss = feed_damage_on_target(state, card, idx);
        let remaining = (monster.hp.max(0) - hp_loss).max(0);
        best_gap = best_gap.min(remaining);

        if remaining == 0 {
            let prevented_damage = if monster.is_attacking {
                monster.intent_dmg.max(0) * monster.intent_hits.max(1)
            } else {
                0
            };
            let mut value = 9_500 + prevented_damage * 180;
            if imminent == 0 {
                value += 1_600;
            }
            if state.is_boss_fight || state.is_elite_fight {
                value += 1_200;
            }
            if state.player_hp <= state.player_max_hp.saturating_sub(12) {
                value += 1_000;
            }
            best_kill_value = best_kill_value.max(value);
        }
    }

    if best_kill_value != i32::MIN {
        return best_kill_value;
    }

    if best_gap <= 4 && imminent == 0 {
        1_200 - best_gap * 180
    } else {
        0
    }
}

fn reaper_swing_value(state: &SimState, card: &SimCard) -> i32 {
    let heal = estimated_reaper_heal(state, card);
    let kills = estimated_reaper_kills(state, card);
    let kill_prevention = estimated_reaper_kill_prevention(state, card);
    reaper_timing_score(
        &SurvivalTimingContext {
            current_hp: state.player_hp,
            imminent_unblocked_damage: imminent_unblocked_damage(state),
            missing_hp: missing_hp(state),
        },
        heal,
        kill_prevention,
        kills,
    )
}

fn body_slam_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let current_damage = state.player_block.max(0);
    let additional_block = max_additional_block_before_body_slam(state, card_idx);
    let can_kill_now = (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone)
        .any(|i| current_damage >= state.monsters[i].hp.max(0) + state.monsters[i].block.max(0));
    body_slam_delay_score(
        current_damage,
        additional_block,
        can_kill_now,
        imminent_unblocked_damage(state),
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
            damage * 250 + followup_attack_bonus(c.card_id)
        })
        .max()
        .unwrap_or(0)
}

fn double_tap_timing_value(state: &SimState, card_idx: usize) -> i32 {
    if state.double_tap_active {
        return -8_000;
    }

    let followup = best_followup_attack_value(state, card_idx);
    if followup > 0 {
        3_000 + followup
    } else {
        -4_500
    }
}

fn followup_attack_count(state: &SimState, current_idx: usize) -> i32 {
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
        .count() as i32
}

fn followup_strength_payoff_count(state: &SimState, current_idx: usize) -> i32 {
    let energy_left = state.energy - effective_energy_cost(state, &state.hand[current_idx]);
    if energy_left <= 0 {
        return 0;
    }

    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_idx
                && ((c.cost >= 0 && c.cost <= energy_left) || (c.cost < 0 && energy_left > 0))
                && is_strength_payoff(c.card_id)
        })
        .count() as i32
}

fn followup_multi_attack_payoff_count(state: &SimState, current_idx: usize) -> i32 {
    let energy_left = state.energy - effective_energy_cost(state, &state.hand[current_idx]);
    if energy_left <= 0 {
        return 0;
    }

    active_hand_cards(state)
        .filter(|(idx, c)| {
            *idx != current_idx
                && ((c.cost >= 0 && c.cost <= energy_left) || (c.cost < 0 && energy_left > 0))
                && is_multi_attack_payoff(c.card_id)
        })
        .count() as i32
}

fn spot_weakness_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let has_attacking_target = (0..state.monsters.len())
        .any(|idx| !state.monsters[idx].is_gone && state.monsters[idx].is_attacking);
    if !has_attacking_target {
        return -6_500;
    }

    let followup_attacks = followup_attack_count(state, card_idx);
    let followup_payoffs = followup_strength_payoff_count(state, card_idx);
    let followup_value = best_followup_attack_value(state, card_idx);
    let mut score = 6_500 + followup_attacks * 1_500 + followup_payoffs * 1_800;
    if followup_value > 0 {
        score += followup_value / 2;
    } else if state.player_strength <= 0 {
        score -= 2_500;
    }
    if state.is_boss_fight || state.is_elite_fight {
        score += 1_200;
    }
    score
}

fn rage_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let followup_attacks = followup_attack_count(state, card_idx);
    if followup_attacks <= 0 {
        return -5_500;
    }

    let multi_hit_payoffs = followup_multi_attack_payoff_count(state, card_idx);
    let threatened = total_incoming_damage(state) > state.player_block;
    let mut score = 4_500 + followup_attacks * 1_900 + multi_hit_payoffs * 2_200;
    if threatened {
        score += 3_000 + imminent_unblocked_damage(state).min(20) * 120;
    }
    score
}

fn followup_attack_bonus(card_id: CardId) -> i32 {
    if taxonomy(card_id).is_attack_followup_priority() {
        2_000
    } else {
        0
    }
}

fn whirlwind_timing_value(state: &SimState, card: &SimCard) -> i32 {
    let hits = effective_hits(card, state.energy);
    let mut value = hits * alive_monster_count(state) * 1_000 + hits * 900;
    if alive_monster_count(state) >= 2 {
        value += 2_500;
    }
    value
}

fn limit_break_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let strength = state.player_strength.max(0);
    let followup = best_followup_attack_value(state, card_idx);
    if strength <= 0 {
        return -7_000;
    }

    let mut value = strength * 2_000;
    if strength >= 3 {
        value += 5_500;
    } else if strength == 2 {
        value += 1_500;
    } else {
        value -= 1_000;
    }
    value += followup / 2;
    if followup == 0 && strength <= 2 {
        value -= 2_500;
    }
    value
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
