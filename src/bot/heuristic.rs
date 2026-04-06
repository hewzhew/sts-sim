use std::collections::HashSet;
use crate::state::core::ClientInput;
use crate::combat::{CombatState, Intent, PowerId};
use crate::content::cards::{get_card_definition, CardId, CardType, CardTarget};

const MAX_MONSTERS: usize = 5;
const MAX_HAND: usize = 12;
const MAX_STATES: usize = 50_000;
const DRAW_VALUE: i64 = 550;

// ─── Zero-Allocation Structs (all Copy) ──────────────────────

#[derive(Clone, Copy)]
struct SimMonster {
    entity_id: usize,
    hp: i32,
    block: i32,
    vulnerable: i32,
    weak: i32,
    is_attacking: bool,
    is_gone: bool,
    intent_dmg: i32,
    intent_hits: i32,
}

impl Default for SimMonster {
    fn default() -> Self {
        Self { entity_id: 0, hp: 0, block: 0, vulnerable: 0, weak: 0,
               is_attacking: false, is_gone: true, intent_dmg: 0, intent_hits: 0 }
    }
}

#[derive(Clone, Copy)]
struct SimCard {
    card_id: CardId,
    cost: i32,
    base_damage: i32,
    base_block: i32,
    base_magic: i32,
    card_type: CardType,
    target: CardTarget,
    hits: i32,
}

impl Default for SimCard {
    fn default() -> Self {
        Self { card_id: CardId::Strike, cost: 99, base_damage: 0, base_block: 0,
               base_magic: 0, card_type: CardType::Curse, target: CardTarget::None, hits: 0 }
    }
}

#[derive(Clone, Copy)]
struct SimState {
    energy: i32,
    player_hp: i32,
    player_block: i32,
    player_strength: i32,
    player_dexterity: i32,
    player_weak: bool,
    player_frail: bool,
    player_vulnerable: bool,
    player_entangled: bool,
    draw_bonus: i64,
    monsters: [SimMonster; MAX_MONSTERS],
    monster_count: u8,
    hand: [SimCard; MAX_HAND],
    hand_mask: u16,
}

type Play = (usize, Option<usize>);

// ─── Entry Point ─────────────────────────────────────────────

pub fn decide_heuristic(combat: &CombatState) -> ClientInput {
    let init = build_sim_state(combat);

    if (0..init.monster_count as usize).all(|i| init.monsters[i].is_gone) {
        return ClientInput::EndTurn;
    }

    if let Some(potion_input) = super::potions::should_use_potion(combat) {
        return potion_input;
    }

    let mut best_score = evaluate(&init);
    let mut best_first: Option<Play> = None;
    let mut seen = HashSet::with_capacity(8192);
    seen.insert(fast_hash(&init));

    dfs(&init, None, &mut best_score, &mut best_first, &mut seen);

    match best_first {
        Some((idx, target)) => ClientInput::PlayCard { card_index: idx, target },
        None => ClientInput::EndTurn,
    }
}

// ─── DFS + Heuristic Ordering ────────────────────────────────

fn dfs(
    state: &SimState,
    first_play: Option<Play>,
    best_score: &mut i64,
    best_first: &mut Option<Play>,
    seen: &mut HashSet<u64>,
) {
    if seen.len() >= MAX_STATES { return; }

    let mut plays = get_plays(state);
    plays.sort_unstable_by(|a, b| play_priority(state, b.0).cmp(&play_priority(state, a.0)));

    for &(card_idx, target) in &plays {
        let mut ns = *state; // memcpy — zero heap allocation
        apply_play(&mut ns, card_idx, target);

        let h = fast_hash(&ns);
        if !seen.insert(h) { continue; }

        let real_first = first_play.unwrap_or((card_idx, target));
        let score = evaluate(&ns);
        if score > *best_score {
            *best_score = score;
            *best_first = Some(real_first);
        }

        dfs(&ns, Some(real_first), best_score, best_first, seen);
    }
}

fn play_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p: i32 = 0;

    if card.card_type == CardType::Power { p += 10_000; }

    match card.card_id {
        CardId::Inflame | CardId::SpotWeakness => p += 9_500,
        CardId::Flex => p += 9_000,
        CardId::Offering => p += 8_800,
        CardId::Bloodletting | CardId::SeeingRed => p += 8_500,
        CardId::Bash | CardId::Shockwave => p += 8_000,
        CardId::Uppercut | CardId::ThunderClap => p += 7_800,
        CardId::Clothesline | CardId::Intimidate => p += 7_500,
        _ => {}
    }

    if card.cost == 0 && p == 0 { p += 2_000; }
    if card.card_type == CardType::Attack { p += 5_000 + card.base_damage * card.hits * 10; }
    if card.base_block > 0 { p += 3_000 + card.base_block * 10; }
    p
}

// ─── Build SimState ──────────────────────────────────────────

fn build_sim_state(combat: &CombatState) -> SimState {
    let p_str = combat.get_power(0, PowerId::Strength);
    let p_dex = combat.get_power(0, PowerId::Dexterity);
    let p_vigor = combat.get_power(0, PowerId::Vigor);

    let mut monsters = [SimMonster::default(); MAX_MONSTERS];
    let mc = combat.monsters.len().min(MAX_MONSTERS);
    for (i, m) in combat.monsters.iter().take(MAX_MONSTERS).enumerate() {
        let is_atk = matches!(m.current_intent,
            Intent::Attack{..} | Intent::AttackBuff{..} | Intent::AttackDebuff{..} | Intent::AttackDefend{..});
        let hits = match m.current_intent {
            Intent::Attack{hits,..} | Intent::AttackBuff{hits,..} |
            Intent::AttackDebuff{hits,..} | Intent::AttackDefend{hits,..} => hits as i32,
            _ => 0,
        };
        monsters[i] = SimMonster {
            entity_id: m.id, hp: m.current_hp, block: m.block,
            vulnerable: combat.get_power(m.id, PowerId::Vulnerable),
            weak: combat.get_power(m.id, PowerId::Weak),
            is_attacking: is_atk, is_gone: m.is_dying || m.is_escaped || m.current_hp <= 0,
            intent_dmg: m.intent_dmg, intent_hits: hits,
        };
    }

    let mut hand = [SimCard::default(); MAX_HAND];
    let mut hand_mask: u16 = 0;
    for (i, card) in combat.hand.iter().take(MAX_HAND).enumerate() {
        let def = get_card_definition(card.id);
        let u = card.upgrades as i32;
        let hits = match card.id {
            CardId::TwinStrike => 2,
            CardId::Pummel => 4 + u,
            CardId::SwordBoomerang => def.base_magic + def.upgrade_magic * u,
            _ => 1,
        };
        hand[i] = SimCard {
            card_id: card.id, cost: card.get_cost() as i32,
            base_damage: def.base_damage + def.upgrade_damage * u,
            base_block: def.base_block + def.upgrade_block * u,
            base_magic: def.base_magic + def.upgrade_magic * u,
            card_type: def.card_type, target: def.target, hits,
        };
        hand_mask |= 1 << i;
    }

    SimState {
        energy: combat.energy as i32, player_hp: combat.player.current_hp,
        player_block: combat.player.block, player_strength: p_str + p_vigor,
        player_dexterity: p_dex,
        player_weak: combat.get_power(0, PowerId::Weak) > 0,
        player_frail: combat.get_power(0, PowerId::Frail) > 0,
        player_vulnerable: combat.get_power(0, PowerId::Vulnerable) > 0,
        player_entangled: combat.get_power(0, PowerId::Entangle) > 0,
        draw_bonus: 0, monsters, monster_count: mc as u8, hand, hand_mask,
    }
}

// ─── Legal Plays (bitmask iteration) ─────────────────────────

fn get_plays(state: &SimState) -> Vec<Play> {
    let mut plays = Vec::with_capacity(32);
    let mut seen_ids = [(CardId::Strike, -999i32); MAX_HAND];
    let mut sc = 0usize;

    let mut mask = state.hand_mask;
    while mask != 0 {
        let i = mask.trailing_zeros() as usize;
        mask &= mask - 1;

        let card = &state.hand[i];
        if card.cost > state.energy || card.cost < 0 { continue; }
        if card.card_type == CardType::Curse || card.card_type == CardType::Status { continue; }
        if state.player_entangled && card.card_type == CardType::Attack { continue; }

        if card.card_id == CardId::Clash {
            let mut all_attacks = true;
            let mut check_mask = state.hand_mask;
            while check_mask != 0 {
                let ci = check_mask.trailing_zeros() as usize;
                check_mask &= check_mask - 1;
                if state.hand[ci].card_type != CardType::Attack {
                    all_attacks = false;
                    break;
                }
            }
            if !all_attacks { continue; }
        }

        let key = (card.card_id, card.cost);
        if (0..sc).any(|j| seen_ids[j] == key) { continue; }
        seen_ids[sc] = key;
        sc += 1;

        match card.target {
            CardTarget::Enemy => {
                for mi in 0..state.monster_count as usize {
                    if !state.monsters[mi].is_gone {
                        plays.push((i, Some(state.monsters[mi].entity_id)));
                    }
                }
            }
            _ => plays.push((i, None)),
        }
    }
    plays
}

// ─── Apply Play ──────────────────────────────────────────────

fn apply_play(state: &mut SimState, card_idx: usize, target: Option<usize>) {
    let card = state.hand[card_idx]; // Copy
    state.energy -= card.cost;
    state.hand_mask &= !(1u16 << card_idx);

    // --- Damage ---
    if card.card_type == CardType::Attack {
        let eff_dmg = effective_damage(state, &card);
        if eff_dmg > 0 {
            match card.target {
                CardTarget::Enemy => {
                    if let Some(tid) = target { hit_monster(state, tid, eff_dmg, card.hits); }
                }
                CardTarget::AllEnemy => {
                    let mut ids = [0usize; MAX_MONSTERS];
                    let mut c = 0;
                    for mi in 0..state.monster_count as usize {
                        if !state.monsters[mi].is_gone { ids[c] = state.monsters[mi].entity_id; c += 1; }
                    }
                    for j in 0..c { hit_monster(state, ids[j], eff_dmg, card.hits); }
                }
                _ => {}
            }
        }
    }

    // --- Block ---
    if card.base_block > 0 {
        let mut blk = card.base_block + state.player_dexterity;
        if state.player_frail { blk = (blk as f32 * 0.75).floor() as i32; }
        state.player_block += blk.max(0);
    }

    // --- Special Effects ---
    apply_special(state, &card, target);
}

fn effective_damage(state: &SimState, card: &SimCard) -> i32 {
    let raw = match card.card_id {
        CardId::BodySlam => state.player_block + state.player_strength,
        CardId::HeavyBlade => card.base_damage + state.player_strength * card.base_magic,
        _ => card.base_damage + state.player_strength,
    };
    let mut d = raw;
    if state.player_weak { d = (d as f32 * 0.75).floor() as i32; }
    d.max(0)
}

fn apply_special(state: &mut SimState, card: &SimCard, target: Option<usize>) {
    match card.card_id {
        CardId::Bash => { if let Some(t) = target { add_vuln(state, t, card.base_magic); } }
        CardId::Uppercut => { if let Some(t) = target { add_vuln(state, t, card.base_magic); add_weak(state, t, card.base_magic); } }
        CardId::Clothesline => { if let Some(t) = target { add_weak(state, t, card.base_magic); } }
        CardId::ThunderClap => { for mi in 0..state.monster_count as usize { if !state.monsters[mi].is_gone { state.monsters[mi].vulnerable += 1; } } }
        CardId::Shockwave => { for mi in 0..state.monster_count as usize { if !state.monsters[mi].is_gone { state.monsters[mi].vulnerable += card.base_magic; state.monsters[mi].weak += card.base_magic; } } }
        CardId::Intimidate => { for mi in 0..state.monster_count as usize { if !state.monsters[mi].is_gone { state.monsters[mi].weak += card.base_magic; } } }
        CardId::Disarm => { if let Some(t) = target { if let Some(m) = find_mut(state, t) { m.intent_dmg = (m.intent_dmg - card.base_magic).max(0); } } }
        CardId::Flex | CardId::Inflame => { state.player_strength += card.base_magic; }
        CardId::SpotWeakness => {
            if let Some(t) = target {
                let atk = find(state, t).map_or(false, |m| m.is_attacking);
                if atk { state.player_strength += card.base_magic; }
            }
        }
        CardId::Offering => { state.player_hp -= 6; state.energy += 2; state.draw_bonus += 3 * DRAW_VALUE; }
        CardId::Bloodletting => { state.player_hp -= 3; state.energy += card.base_magic; }
        CardId::SeeingRed => { state.energy += 2; }
        CardId::PommelStrike | CardId::ShrugItOff | CardId::Warcry => { state.draw_bonus += DRAW_VALUE; }
        CardId::BattleTrance => { state.draw_bonus += card.base_magic as i64 * DRAW_VALUE; }
        _ => {}
    }
}

fn hit_monster(state: &mut SimState, tid: usize, dmg: i32, hits: i32) {
    if let Some(m) = find_mut(state, tid) {
        if m.is_gone { return; }
        let mut d = dmg;
        if m.vulnerable > 0 { d = (d as f32 * 1.5).floor() as i32; }
        d = d.max(0);
        for _ in 0..hits {
            if m.is_gone { break; }
            let pierce = (d - m.block).max(0);
            m.block = (m.block - d).max(0);
            m.hp -= pierce;
            if m.hp <= 0 { m.is_gone = true; }
        }
    }
}

fn find(state: &SimState, eid: usize) -> Option<&SimMonster> {
    (0..state.monster_count as usize).find(|&i| state.monsters[i].entity_id == eid).map(|i| &state.monsters[i])
}
fn find_mut(state: &mut SimState, eid: usize) -> Option<&mut SimMonster> {
    (0..state.monster_count as usize).find(|&i| state.monsters[i].entity_id == eid).map(move |i| &mut state.monsters[i])
}
fn add_vuln(s: &mut SimState, t: usize, n: i32) { if let Some(m) = find_mut(s, t) { m.vulnerable += n; } }
fn add_weak(s: &mut SimState, t: usize, n: i32) { if let Some(m) = find_mut(s, t) { m.weak += n; } }

// ─── Evaluation ──────────────────────────────────────────────

fn evaluate(state: &SimState) -> i64 {
    let mut hp = state.player_hp;
    let mut block = state.player_block;

    for mi in 0..state.monster_count as usize {
        let m = &state.monsters[mi];
        if m.is_gone || !m.is_attacking { continue; }
        let mut d = m.intent_dmg;
        if m.weak > 0 { d = (d as f32 * 0.75).floor() as i32; }
        d = d.max(0);
        if state.player_vulnerable { d = (d as f32 * 1.5).floor() as i32; }
        for _ in 0..m.intent_hits {
            let pierce = (d - block).max(0);
            block = (block - d).max(0);
            hp -= pierce;
        }
    }

    let alive = (0..state.monster_count as usize).filter(|&i| !state.monsters[i].is_gone).count() as i64;
    let dead = state.monster_count as i64 - alive;
    let mhp: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone).map(|i| state.monsters[i].hp.max(0) as i64).sum();

    if hp <= 0 { return -1_000_000 + hp as i64; }
    if alive == 0 { return 500_000 + hp as i64 * 100; }

    let mut s: i64 = 0;
    s += dead * 10_000;
    s -= (state.player_hp - hp).max(0) as i64 * 100;
    s -= mhp * 10;

    let vuln: i64 = (0..state.monster_count as usize).filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].vulnerable.min(4) as i64).sum();
    let weak: i64 = (0..state.monster_count as usize).filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].weak.min(4) as i64).sum();
    s += vuln * 500 + weak * 400;
    s += state.draw_bonus;
    s += block.min(30) as i64;
    s += state.energy as i64;
    s
}

// ─── Fast Hash (FNV-1a) ─────────────────────────────────────

fn fast_hash(state: &SimState) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    h = fnv(h, state.energy as u64);
    h = fnv(h, state.player_block as u64);
    h = fnv(h, state.player_strength as u64);
    h = fnv(h, state.player_hp as u64);
    h = fnv(h, state.hand_mask as u64);
    for i in 0..state.monster_count as usize {
        h = fnv(h, state.monsters[i].hp as u64);
        h = fnv(h, state.monsters[i].block as u64);
        h = fnv(h, state.monsters[i].vulnerable as u64);
        h = fnv(h, state.monsters[i].weak as u64);
    }
    h
}

#[inline(always)]
fn fnv(h: u64, v: u64) -> u64 { (h ^ v).wrapping_mul(0x100000001b3) }
