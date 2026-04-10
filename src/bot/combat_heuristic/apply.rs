use crate::content::cards::{get_card_definition, CardId, CardTarget, CardType};

use super::sim::{active_hand_cards, SimCard, SimState, MAX_MONSTERS};

pub(super) fn apply_play(state: &mut SimState, card_idx: usize, target: Option<usize>) {
    let card = state.hand[card_idx];
    let energy_spent = effective_energy_cost(state, &card);
    state.energy -= energy_spent;
    state.hand_mask &= !(1u16 << card_idx);
    let repeat_attack = card.card_type == CardType::Attack && state.double_tap_active;
    if repeat_attack {
        state.double_tap_active = false;
    }

    if card.card_type == CardType::Attack {
        apply_attack_damage(state, &card, target, energy_spent);
    }

    if card.base_block > 0 {
        let mut blk = card.base_block + state.player_dexterity;
        if state.player_frail {
            blk = (blk as f32 * 0.75).floor() as i32;
        }
        state.player_block += blk.max(0);
    }

    apply_special(state, &card, target);

    if repeat_attack {
        apply_attack_damage(state, &card, target, energy_spent);
        apply_special(state, &card, target);
    }
}

pub(super) fn effective_energy_cost(state: &SimState, card: &SimCard) -> i32 {
    if card.cost < 0 {
        state.energy.max(0)
    } else {
        card.cost.max(0)
    }
}

pub(super) fn effective_hits(card: &SimCard, energy_spent: i32) -> i32 {
    match card.card_id {
        CardId::Whirlwind => energy_spent.max(0),
        _ => card.hits.max(0),
    }
}

pub(super) fn effective_damage(state: &SimState, card: &SimCard) -> i32 {
    let raw = match card.card_id {
        CardId::BodySlam => state.player_block + state.player_strength,
        CardId::HeavyBlade => card.base_damage + state.player_strength * card.base_magic,
        _ => card.base_damage + state.player_strength,
    };
    let mut d = raw;
    if state.player_weak {
        d = (d as f32 * 0.75).floor() as i32;
    }
    d.max(0)
}

fn apply_attack_damage(
    state: &mut SimState,
    card: &SimCard,
    target: Option<usize>,
    energy_spent: i32,
) {
    let eff_dmg = effective_damage(state, card);
    let hits = effective_hits(card, energy_spent);
    if eff_dmg <= 0 || hits <= 0 {
        return;
    }

    match card.target {
        CardTarget::Enemy => {
            if let Some(tid) = target {
                hit_monster(state, tid, eff_dmg, hits);
                apply_reflect_damage(state, card, Some(tid), hits);
            }
        }
        CardTarget::AllEnemy => {
            let mut ids = [0usize; MAX_MONSTERS];
            let mut count = 0;
            for mi in 0..state.monster_count as usize {
                if !state.monsters[mi].is_gone {
                    ids[count] = state.monsters[mi].entity_id;
                    count += 1;
                }
            }
            for target_id in ids.into_iter().take(count) {
                hit_monster(state, target_id, eff_dmg, hits);
            }
            apply_reflect_damage(state, card, None, hits);
        }
        _ => {}
    }
}

fn apply_special(state: &mut SimState, card: &SimCard, target: Option<usize>) {
    match card.card_id {
        CardId::Bash => {
            if let Some(t) = target {
                add_vuln(state, t, card.base_magic);
            }
        }
        CardId::Uppercut => {
            if let Some(t) = target {
                add_vuln(state, t, card.base_magic);
                add_weak(state, t, card.base_magic);
            }
        }
        CardId::Clothesline => {
            if let Some(t) = target {
                add_weak(state, t, card.base_magic);
            }
        }
        CardId::ThunderClap => {
            for mi in 0..state.monster_count as usize {
                if !state.monsters[mi].is_gone {
                    state.monsters[mi].vulnerable += 1;
                }
            }
        }
        CardId::Shockwave => {
            for mi in 0..state.monster_count as usize {
                if !state.monsters[mi].is_gone {
                    state.monsters[mi].vulnerable += card.base_magic;
                    state.monsters[mi].weak += card.base_magic;
                }
            }
        }
        CardId::Intimidate => {
            for mi in 0..state.monster_count as usize {
                if !state.monsters[mi].is_gone {
                    state.monsters[mi].weak += card.base_magic;
                }
            }
        }
        CardId::Blind => {
            if let Some(t) = target {
                add_weak(state, t, card.base_magic);
            }
        }
        CardId::DarkShackles => {
            if let Some(t) = target {
                if let Some(m) = find_mut(state, t) {
                    m.intent_dmg = (m.intent_dmg - card.base_magic).max(0);
                }
            }
        }
        CardId::Trip => {
            if let Some(t) = target {
                add_vuln(state, t, card.base_magic);
            }
        }
        CardId::Disarm => {
            if let Some(t) = target {
                if let Some(m) = find_mut(state, t) {
                    m.intent_dmg = (m.intent_dmg - card.base_magic).max(0);
                }
            }
        }
        CardId::Flex | CardId::Inflame => {
            state.player_strength += card.base_magic;
        }
        CardId::LimitBreak => {
            if state.player_strength > 0 {
                state.player_strength *= 2;
            }
        }
        CardId::Armaments => {
            if card.upgrades > 0 {
                let indices: Vec<usize> = active_hand_cards(state).map(|(idx, _)| idx).collect();
                for idx in indices {
                    upgrade_sim_card(&mut state.hand[idx]);
                }
            } else if let Some(idx) = super::scoring::best_armaments_upgrade_target(state) {
                upgrade_sim_card(&mut state.hand[idx]);
            }
        }
        CardId::Panacea => {
            state.player_artifact += card.base_magic;
        }
        CardId::DoubleTap => state.double_tap_active = true,
        CardId::Corruption => state.has_corruption = true,
        CardId::FeelNoPain => state.has_feel_no_pain = true,
        CardId::DarkEmbrace => state.has_dark_embrace = true,
        CardId::Metallicize => state.has_metallicize = true,
        CardId::Evolve => state.has_evolve = true,
        CardId::Rupture => state.has_rupture = true,
        CardId::Combust => state.has_combust = true,
        CardId::Brutality => state.has_brutality = true,
        CardId::Berserk => state.has_berserk = true,
        CardId::Panache => state.has_panache = true,
        CardId::Mayhem => state.has_mayhem = true,
        CardId::Magnetism => state.has_magnetism = true,
        CardId::SpotWeakness => {
            if let Some(t) = target {
                let atk = find(state, t).map_or(false, |m| m.is_attacking);
                if atk {
                    state.player_strength += card.base_magic;
                }
            }
        }
        CardId::Offering => {
            state.player_hp -= 6;
            state.energy += 2;
            add_draw_bonus(state, super::sim::DRAW_VALUE * 3);
        }
        CardId::PowerThrough => {
            state.future_status_cards += 2;
        }
        CardId::Bloodletting => {
            state.player_hp -= 3;
            state.energy += card.base_magic;
        }
        CardId::SeeingRed => {
            state.energy += 2;
        }
        CardId::PommelStrike | CardId::ShrugItOff | CardId::Warcry => {
            add_draw_bonus(state, super::sim::DRAW_VALUE);
        }
        CardId::Finesse | CardId::FlashOfSteel | CardId::GoodInstincts => {
            add_draw_bonus(
                state,
                if matches!(card.card_id, CardId::Finesse | CardId::FlashOfSteel) {
                    super::sim::DRAW_VALUE
                } else {
                    0
                },
            );
        }
        CardId::BattleTrance => {
            add_draw_bonus(state, card.base_magic as i64 * super::sim::DRAW_VALUE);
            state.player_no_draw = true;
        }
        CardId::MasterOfStrategy => {
            add_draw_bonus(state, card.base_magic as i64 * super::sim::DRAW_VALUE);
        }
        CardId::DeepBreath => {
            if state.discard_pile_size > 0 {
                state.draw_pile_size += state.discard_pile_size;
                state.status_in_draw += state.status_in_discard;
                state.discard_pile_size = 0;
                state.status_in_discard = 0;
            }
            add_draw_bonus(state, super::sim::DRAW_VALUE);
        }
        _ => {}
    }

    if card.card_type == CardType::Skill {
        for mi in 0..state.monster_count as usize {
            let monster = &mut state.monsters[mi];
            if monster.is_gone || !monster.nob_enrage {
                continue;
            }
            monster.strength += 2;
        }
    }
}

fn add_draw_bonus(state: &mut SimState, draw_bonus: i64) {
    if !state.player_no_draw && draw_bonus > 0 {
        state.draw_bonus += draw_bonus;
    }
}

pub(super) fn hit_monster(state: &mut SimState, tid: usize, dmg: i32, hits: i32) {
    if let Some(m) = find_mut(state, tid) {
        if m.is_gone {
            return;
        }
        let mut d = dmg;
        if m.vulnerable > 0 {
            d = (d as f32 * 1.5).floor() as i32;
        }
        d = d.max(0);
        for _ in 0..hits {
            if m.is_gone {
                break;
            }
            let mut dealt = d;
            if m.flight > 0 {
                dealt = (dealt as f32 / 2.0).round() as i32;
            }
            let pierce = (dealt - m.block).max(0);
            m.block = (m.block - dealt).max(0);
            m.hp -= pierce;
            if dealt > 0 && m.flight > 0 && m.hp > 0 {
                m.flight -= 1;
                if m.flight <= 0 {
                    m.flight = 0;
                    m.is_attacking = false;
                    m.intent_dmg = 0;
                    m.intent_hits = 0;
                }
            }
            if m.hp <= 0 {
                m.is_gone = true;
            }
        }
    }
}

pub(super) fn expected_reflect_damage(
    state: &SimState,
    card: &SimCard,
    target: Option<usize>,
    hits: i32,
) -> i32 {
    if card.card_type != CardType::Attack || hits <= 0 {
        return 0;
    }

    match card.target {
        CardTarget::Enemy => {
            if let Some(tid) = target {
                find(state, tid)
                    .map(|m| m.sharp_hide.max(0) + m.thorns.max(0) * hits.max(0))
                    .unwrap_or(0)
            } else {
                (0..state.monster_count as usize)
                    .filter_map(|i| {
                        let m = &state.monsters[i];
                        (!m.is_gone).then_some(m.sharp_hide.max(0) + m.thorns.max(0) * hits.max(0))
                    })
                    .max()
                    .unwrap_or(0)
            }
        }
        CardTarget::AllEnemy => (0..state.monster_count as usize)
            .filter_map(|i| {
                let m = &state.monsters[i];
                (!m.is_gone).then_some(m.sharp_hide.max(0) + m.thorns.max(0) * hits.max(0))
            })
            .sum(),
        _ => 0,
    }
}

fn apply_reflect_damage(state: &mut SimState, card: &SimCard, target: Option<usize>, hits: i32) {
    let reflect = expected_reflect_damage(state, card, target, hits);
    if reflect <= 0 {
        return;
    }

    let blocked = reflect.min(state.player_block.max(0));
    state.player_block = (state.player_block - blocked).max(0);
    state.player_hp -= reflect - blocked;
}

pub(super) fn find(state: &SimState, eid: usize) -> Option<&super::sim::SimMonster> {
    (0..state.monster_count as usize)
        .find(|&i| state.monsters[i].entity_id == eid)
        .map(|i| &state.monsters[i])
}

fn find_mut(state: &mut SimState, eid: usize) -> Option<&mut super::sim::SimMonster> {
    (0..state.monster_count as usize)
        .find(|&i| state.monsters[i].entity_id == eid)
        .map(move |i| &mut state.monsters[i])
}

fn add_vuln(s: &mut SimState, t: usize, n: i32) {
    if let Some(m) = find_mut(s, t) {
        m.vulnerable += n;
    }
}

fn add_weak(s: &mut SimState, t: usize, n: i32) {
    if let Some(m) = find_mut(s, t) {
        m.weak += n;
    }
}

fn upgrade_sim_card(card: &mut SimCard) {
    if card.upgrades > 0 {
        return;
    }
    let def = get_card_definition(card.card_id);
    card.upgrades += 1;
    card.base_damage += def.upgrade_damage;
    card.base_block += def.upgrade_block;
    card.base_magic += def.upgrade_magic;
    match card.card_id {
        CardId::BloodForBlood | CardId::Havoc | CardId::SeeingRed | CardId::Exhume => {
            card.cost = (card.cost - 1).max(0);
        }
        _ => {}
    }
}
