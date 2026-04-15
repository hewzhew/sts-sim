use crate::bot::card_taxonomy::taxonomy;
use crate::content::cards::CardId;

use super::apply::{effective_damage, effective_energy_cost, effective_hits};
use super::sim::{active_hand_cards, SimState};

pub(super) fn debuff_timing_value(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let tags = taxonomy(card.card_id);
    let mut value = match card.card_id {
        CardId::Blind => 5_500,
        CardId::DarkShackles => 7_000,
        CardId::Bash | CardId::Shockwave => 8_000,
        CardId::Uppercut | CardId::ThunderClap => 7_800,
        CardId::Clothesline | CardId::Intimidate => 7_500,
        CardId::Trip => 6_500,
        _ => 0,
    };

    if matches!(card.card_id, CardId::Blind | CardId::DarkShackles) || tags.is_weak_enabler() {
        value += threat_reduction_value(state, card.base_magic);
    }
    if card.card_id == CardId::Trip {
        value += best_attack_target_value(state) * card.base_magic;
        value += best_followup_attack_value_with_vuln(state, card_idx, card.base_magic);
    }

    value
}

fn threat_reduction_value(state: &SimState, amount: i32) -> i32 {
    (0..state.monsters.len())
        .filter(|&i| !state.monsters[i].is_gone && state.monsters[i].is_attacking)
        .map(|i| amount.min(state.monsters[i].intent_dmg.max(0)))
        .max()
        .unwrap_or(0)
        * 700
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
                && c.card_type == crate::content::cards::CardType::Attack
                && ((c.cost >= 0 && c.cost <= energy_left) || (c.cost < 0 && energy_left > 0))
        })
        .map(|(_, c)| {
            let energy_for_card = if c.cost < 0 { energy_left } else { c.cost };
            let hits = effective_hits(c, energy_for_card);
            let vulnerable_multiplier = if vuln_amount > 0 { 1.5 } else { 1.0 };
            let damage =
                ((effective_damage(state, c) as f32) * vulnerable_multiplier).floor() as i32 * hits;
            damage * 320 + followup_attack_vuln_bonus(c.card_id)
        })
        .max()
        .unwrap_or(0)
}

fn followup_attack_vuln_bonus(card_id: CardId) -> i32 {
    let base = if taxonomy(card_id).is_attack_followup_priority() {
        2_000
    } else {
        0
    };
    base + if card_id == CardId::Dropkick { 500 } else { 0 }
}
