use crate::combat::{CombatState, Intent, PowerId};
use crate::content::cards::{get_card_definition, CardId, CardTarget, CardType};
use crate::content::monsters::EnemyId;
use crate::state::core::ClientInput;
use std::collections::HashSet;

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
    strength: i32,
    vulnerable: i32,
    weak: i32,
    is_attacking: bool,
    is_gone: bool,
    nob_enrage: bool,
    intent_dmg: i32,
    intent_hits: i32,
}

impl Default for SimMonster {
    fn default() -> Self {
        Self {
            entity_id: 0,
            hp: 0,
            block: 0,
            strength: 0,
            vulnerable: 0,
            weak: 0,
            is_attacking: false,
            is_gone: true,
            nob_enrage: false,
            intent_dmg: 0,
            intent_hits: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct SimCard {
    card_id: CardId,
    upgrades: i32,
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
        Self {
            card_id: CardId::Strike,
            upgrades: 0,
            cost: 99,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            card_type: CardType::Curse,
            target: CardTarget::None,
            hits: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct SimState {
    energy: i32,
    player_hp: i32,
    player_block: i32,
    player_strength: i32,
    player_dexterity: i32,
    player_artifact: i32,
    player_weak: bool,
    player_frail: bool,
    player_vulnerable: bool,
    player_entangled: bool,
    has_corruption: bool,
    has_feel_no_pain: bool,
    has_dark_embrace: bool,
    has_rupture: bool,
    has_combust: bool,
    has_brutality: bool,
    has_panache: bool,
    has_mayhem: bool,
    has_magnetism: bool,
    has_metallicize: bool,
    has_evolve: bool,
    has_berserk: bool,
    double_tap_active: bool,
    future_status_cards: i32,
    sentry_count: i32,
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
        Some((idx, target)) => ClientInput::PlayCard {
            card_index: idx,
            target,
        },
        None => ClientInput::EndTurn,
    }
}

pub fn describe_end_turn_options(combat: &CombatState) -> Vec<String> {
    let state = build_sim_state(combat);
    let end_score = evaluate(&state);
    let plays = get_plays(&state);
    if plays.is_empty() {
        return vec![format!("END score={} no_legal_plays", end_score)];
    }

    let mut lines = vec![format!("END score={} legal_plays={}", end_score, plays.len())];
    let mut scored: Vec<(i64, i32, usize, Option<usize>)> = plays
        .into_iter()
        .map(|(card_idx, target)| {
            let mut next = state;
            apply_play(&mut next, card_idx, target);
            (
                evaluate(&next),
                play_priority(&state, card_idx),
                card_idx,
                target,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.cmp(a));
    for (score, priority, card_idx, target) in scored.into_iter().take(8) {
        let card = &combat.hand[card_idx];
        let target_label = target
            .map(|t| t.to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "play idx={} card={} target={} score={} priority={}",
            card_idx, card.id as u16, target_label, score, priority
        ));
    }
    lines
}

// ─── DFS + Heuristic Ordering ────────────────────────────────

fn dfs(
    state: &SimState,
    first_play: Option<Play>,
    best_score: &mut i64,
    best_first: &mut Option<Play>,
    seen: &mut HashSet<u64>,
) {
    if seen.len() >= MAX_STATES {
        return;
    }

    let mut plays = get_plays(state);
    plays.sort_unstable_by(|a, b| play_priority(state, b.0).cmp(&play_priority(state, a.0)));

    for &(card_idx, target) in &plays {
        let mut ns = *state; // memcpy — zero heap allocation
        apply_play(&mut ns, card_idx, target);

        let h = fast_hash(&ns);
        if !seen.insert(h) {
            continue;
        }

        let real_first = first_play.unwrap_or((card_idx, target));
        let score = evaluate(&ns);
        if score > *best_score || (score == *best_score && best_first.is_none()) {
            *best_score = score;
            *best_first = Some(real_first);
        }

        dfs(&ns, Some(real_first), best_score, best_first, seen);
    }
}

fn play_priority(state: &SimState, card_idx: usize) -> i32 {
    let card = &state.hand[card_idx];
    let mut p: i32 = 0;

    if card.card_type == CardType::Power {
        p += 10_000;
    }

    match card.card_id {
        CardId::Corruption => {
            p += if state.has_corruption { -8_000 } else { 12_000 };
            p += active_hand_cards(state)
                .filter(|(_, c)| c.card_type == CardType::Skill && c.cost > 0)
                .count() as i32
                * 1_000;
        }
        CardId::FeelNoPain => {
            p += if state.has_feel_no_pain {
                -8_000
            } else {
                10_000
            };
            p += exhaust_synergy_cards_in_hand(state) * 1_100;
            p += state.future_status_cards * 900;
            p += state.sentry_count * 2_500;
        }
        CardId::DarkEmbrace => {
            p += if state.has_dark_embrace {
                -8_000
            } else {
                10_000
            };
            p += exhaust_synergy_cards_in_hand(state) * 950;
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
            p += state.future_status_cards * 1_100;
            p += state.sentry_count * 2_000;
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
        CardId::Apotheosis => p += 11_500,
        CardId::Armaments => {
            let upgradable = armaments_upgradable_count(state) as i32;
            if card.upgrades > 0 {
                p += 9_500 + upgradable * 1_200;
            } else {
                p += 6_500 + best_armaments_upgrade_value(state, card_idx);
            }
        }
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
        CardId::GoodInstincts | CardId::Finesse => {
            p += if total_incoming_damage(state) > state.player_block {
                7_500
            } else {
                4_500
            };
        }
        CardId::FlashOfSteel => p += best_attack_target_value(state) * 350 + 5_500,
        CardId::MasterOfStrategy => p += 7_500,
        CardId::DeepBreath => p += 4_000,
        CardId::SecretTechnique | CardId::SecretWeapon | CardId::Discovery => p += 6_500,
        CardId::TheBomb => p += alive_monster_count(state) * 1_100 + 6_000,
        CardId::Offering => p += 8_800,
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

    if gremlin_nob_enrage_active(state) && card.card_type == CardType::Skill {
        p -= gremlin_nob_skill_penalty_value(state, &card);
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

fn active_hand_cards(state: &SimState) -> impl Iterator<Item = (usize, &SimCard)> {
    state
        .hand
        .iter()
        .enumerate()
        .filter(move |(i, _)| (state.hand_mask & (1u16 << i)) != 0)
}

// ─── Build SimState ──────────────────────────────────────────

fn build_sim_state(combat: &CombatState) -> SimState {
    let p_str = combat.get_power(0, PowerId::Strength);
    let p_dex = combat.get_power(0, PowerId::Dexterity);
    let p_vigor = combat.get_power(0, PowerId::Vigor);

    let mut monsters = [SimMonster::default(); MAX_MONSTERS];
    let mc = combat.monsters.len().min(MAX_MONSTERS);
    for (i, m) in combat.monsters.iter().take(MAX_MONSTERS).enumerate() {
        let is_atk = matches!(
            m.current_intent,
            Intent::Attack { .. }
                | Intent::AttackBuff { .. }
                | Intent::AttackDebuff { .. }
                | Intent::AttackDefend { .. }
        );
        let hits = match m.current_intent {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => hits as i32,
            _ => 0,
        };
        monsters[i] = SimMonster {
            entity_id: m.id,
            hp: m.current_hp,
            block: m.block,
            strength: combat.get_power(m.id, PowerId::Strength),
            vulnerable: combat.get_power(m.id, PowerId::Vulnerable),
            weak: combat.get_power(m.id, PowerId::Weak),
            is_attacking: is_atk,
            is_gone: m.is_dying || m.is_escaped || m.current_hp <= 0,
            nob_enrage: combat.get_power(m.id, PowerId::Anger) != 0,
            intent_dmg: m.intent_dmg,
            intent_hits: hits,
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
            card_id: card.id,
            upgrades: card.upgrades as i32,
            cost: card.get_cost() as i32,
            base_damage: def.base_damage + def.upgrade_damage * u,
            base_block: def.base_block + def.upgrade_block * u,
            base_magic: def.base_magic + def.upgrade_magic * u,
            card_type: def.card_type,
            target: def.target,
            hits,
        };
        hand_mask |= 1 << i;
    }

    SimState {
        energy: combat.energy as i32,
        player_hp: combat.player.current_hp,
        player_block: combat.player.block,
        player_strength: p_str + p_vigor,
        player_dexterity: p_dex,
        player_artifact: combat.get_power(0, PowerId::Artifact),
        player_weak: combat.get_power(0, PowerId::Weak) > 0,
        player_frail: combat.get_power(0, PowerId::Frail) > 0,
        player_vulnerable: combat.get_power(0, PowerId::Vulnerable) > 0,
        player_entangled: combat.get_power(0, PowerId::Entangle) > 0,
        has_corruption: combat.get_power(0, PowerId::Corruption) != 0,
        has_feel_no_pain: combat.get_power(0, PowerId::FeelNoPain) > 0,
        has_dark_embrace: combat.get_power(0, PowerId::DarkEmbrace) > 0,
        has_rupture: combat.get_power(0, PowerId::Rupture) > 0,
        has_combust: combat.get_power(0, PowerId::Combust) > 0,
        has_brutality: combat.get_power(0, PowerId::Brutality) > 0,
        has_panache: combat.get_power(0, PowerId::PanachePower) > 0,
        has_mayhem: combat.get_power(0, PowerId::MayhemPower) > 0,
        has_magnetism: combat.get_power(0, PowerId::MagnetismPower) > 0,
        has_metallicize: combat.get_power(0, PowerId::Metallicize) > 0,
        has_evolve: combat.get_power(0, PowerId::Evolve) > 0,
        has_berserk: combat.get_power(0, PowerId::Berserk) > 0,
        double_tap_active: combat.get_power(0, PowerId::DoubleTap) > 0,
        future_status_cards: combat
            .draw_pile
            .iter()
            .chain(combat.discard_pile.iter())
            .chain(combat.hand.iter())
            .filter(|card| {
                matches!(
                    get_card_definition(card.id).card_type,
                    CardType::Status | CardType::Curse
                )
            })
            .count() as i32,
        sentry_count: combat
            .monsters
            .iter()
            .filter(|m| !m.is_dying && !m.is_escaped && m.monster_type == EnemyId::Sentry as usize)
            .count() as i32,
        draw_bonus: 0,
        monsters,
        monster_count: mc as u8,
        hand,
        hand_mask,
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
        let effective_cost = effective_energy_cost(state, card);
        if effective_cost > state.energy {
            continue;
        }
        if card.card_type == CardType::Curse || card.card_type == CardType::Status {
            continue;
        }
        if state.player_entangled && card.card_type == CardType::Attack {
            continue;
        }

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
            if !all_attacks {
                continue;
            }
        }

        let key = (card.card_id, effective_cost);
        if (0..sc).any(|j| seen_ids[j] == key) {
            continue;
        }
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
    if gremlin_nob_enrage_active(state) {
        let has_non_skill_play = plays
            .iter()
            .any(|(idx, _)| state.hand[*idx].card_type != CardType::Skill);
        if has_non_skill_play {
            plays.retain(|(idx, _)| {
                let card = &state.hand[*idx];
                card.card_type != CardType::Skill || skill_allowed_vs_nob(state, card)
            });
        }
    }
    plays
}

// ─── Apply Play ──────────────────────────────────────────────

fn apply_play(state: &mut SimState, card_idx: usize, target: Option<usize>) {
    let card = state.hand[card_idx]; // Copy
    let energy_spent = effective_energy_cost(state, &card);
    state.energy -= energy_spent;
    state.hand_mask &= !(1u16 << card_idx);
    let repeat_attack = card.card_type == CardType::Attack && state.double_tap_active;
    if repeat_attack {
        state.double_tap_active = false;
    }

    // --- Damage ---
    if card.card_type == CardType::Attack {
        apply_attack_damage(state, &card, target, energy_spent);
    }

    // --- Block ---
    if card.base_block > 0 {
        let mut blk = card.base_block + state.player_dexterity;
        if state.player_frail {
            blk = (blk as f32 * 0.75).floor() as i32;
        }
        state.player_block += blk.max(0);
    }

    // --- Special Effects ---
    apply_special(state, &card, target);

    if repeat_attack {
        apply_attack_damage(state, &card, target, energy_spent);
        apply_special(state, &card, target);
    }
}

fn effective_energy_cost(state: &SimState, card: &SimCard) -> i32 {
    if card.cost < 0 {
        state.energy.max(0)
    } else {
        card.cost.max(0)
    }
}

fn effective_hits(card: &SimCard, energy_spent: i32) -> i32 {
    match card.card_id {
        CardId::Whirlwind => energy_spent.max(0),
        _ => card.hits.max(0),
    }
}

fn effective_damage(state: &SimState, card: &SimCard) -> i32 {
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

fn apply_attack_damage(state: &mut SimState, card: &SimCard, target: Option<usize>, energy_spent: i32) {
    let eff_dmg = effective_damage(state, card);
    let hits = effective_hits(card, energy_spent);
    if eff_dmg <= 0 || hits <= 0 {
        return;
    }

    match card.target {
        CardTarget::Enemy => {
            if let Some(tid) = target {
                hit_monster(state, tid, eff_dmg, hits);
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
            } else if let Some(idx) = best_armaments_upgrade_target(state) {
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
            state.draw_bonus += 3 * DRAW_VALUE;
        }
        CardId::Bloodletting => {
            state.player_hp -= 3;
            state.energy += card.base_magic;
        }
        CardId::SeeingRed => {
            state.energy += 2;
        }
        CardId::PommelStrike | CardId::ShrugItOff | CardId::Warcry => {
            state.draw_bonus += DRAW_VALUE;
        }
        CardId::Finesse | CardId::FlashOfSteel | CardId::GoodInstincts => {
            state.draw_bonus += if matches!(card.card_id, CardId::Finesse | CardId::FlashOfSteel) {
                DRAW_VALUE
            } else {
                0
            };
        }
        CardId::BattleTrance => {
            state.draw_bonus += card.base_magic as i64 * DRAW_VALUE;
        }
        CardId::MasterOfStrategy => {
            state.draw_bonus += card.base_magic as i64 * DRAW_VALUE;
        }
        CardId::DeepBreath => {
            state.draw_bonus += DRAW_VALUE;
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

fn hit_monster(state: &mut SimState, tid: usize, dmg: i32, hits: i32) {
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
            let pierce = (d - m.block).max(0);
            m.block = (m.block - d).max(0);
            m.hp -= pierce;
            if m.hp <= 0 {
                m.is_gone = true;
            }
        }
    }
}

fn find(state: &SimState, eid: usize) -> Option<&SimMonster> {
    (0..state.monster_count as usize)
        .find(|&i| state.monsters[i].entity_id == eid)
        .map(|i| &state.monsters[i])
}
fn find_mut(state: &mut SimState, eid: usize) -> Option<&mut SimMonster> {
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

// ─── Evaluation ──────────────────────────────────────────────

fn evaluate(state: &SimState) -> i64 {
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
        .map(|i| state.monsters[i].hp.max(0) as i64)
        .sum();
    let enemy_strength: i64 = (0..state.monster_count as usize)
        .filter(|&i| !state.monsters[i].is_gone)
        .map(|i| state.monsters[i].strength.max(0) as i64)
        .sum();

    if hp <= 0 {
        return -1_000_000 + hp as i64;
    }
    if alive == 0 {
        return 500_000 + hp as i64 * 100;
    }

    let mut s: i64 = 0;
    s += dead * 10_000;
    s -= (state.player_hp - hp).max(0) as i64 * 100;
    s -= mhp * if alive <= 1 { 35 } else { 10 };
    s -= enemy_strength * 260;

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

fn total_incoming_damage(state: &SimState) -> i32 {
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
        .map(|i| state.monsters[i].hp.max(0))
        .max()
        .unwrap_or(0)
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

fn exhaust_synergy_cards_in_hand(state: &SimState) -> i32 {
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
                    | CardId::Panacea
            )
        })
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

fn armaments_upgradable_count(state: &SimState) -> usize {
    active_hand_cards(state)
        .filter(|(_, c)| {
            c.upgrades == 0 && !matches!(c.card_type, CardType::Status | CardType::Curse)
        })
        .count()
}

fn best_armaments_upgrade_target(state: &SimState) -> Option<usize> {
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
            let damage = ((effective_damage(state, c) as f32) * vulnerable_multiplier).floor() as i32
                * hits;
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

fn armaments_upgrade_score(card: &SimCard) -> i32 {
    let def = get_card_definition(card.card_id);
    let mut score = def.upgrade_damage * 180 + def.upgrade_block * 130 + def.upgrade_magic * 210;
    score += match card.card_id {
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

fn gremlin_nob_enrage_active(state: &SimState) -> bool {
    (0..state.monster_count as usize)
        .any(|i| !state.monsters[i].is_gone && state.monsters[i].nob_enrage)
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

fn skill_allowed_vs_nob(state: &SimState, card: &SimCard) -> bool {
    let incoming = total_incoming_damage(state);
    let lethalish = incoming >= state.player_block + state.player_hp.saturating_sub(4);
    match card.card_id {
        CardId::Impervious => lethalish,
        CardId::GhostlyArmor | CardId::FlameBarrier => lethalish,
        CardId::Shockwave | CardId::Disarm => lethalish && incoming > state.player_block,
        _ => false,
    }
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
fn fnv(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x100000001b3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CombatCard, CombatPhase, MonsterEntity};
    use crate::content::monsters::EnemyId;
    use std::collections::{HashMap, VecDeque};

    fn combat_with_hand(hand: &[CardId], draw: &[CardId]) -> CombatState {
        let mut rs = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        rs.master_deck = draw
            .iter()
            .enumerate()
            .map(|(idx, &id)| CombatCard::new(id, idx as u32 + 1))
            .collect();
        CombatState {
            ascension_level: 0,
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            draw_pile: draw
                .iter()
                .enumerate()
                .map(|(idx, &id)| CombatCard::new(id, 200 + idx as u32))
                .collect(),
            hand: hand
                .iter()
                .enumerate()
                .map(|(idx, &id)| CombatCard::new(id, 100 + idx as u32))
                .collect(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            player: rs.build_combat_player(0),
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: EnemyId::JawWorm as usize,
                current_hp: 40,
                max_hp: 40,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Attack {
                    damage: 10,
                    hits: 1,
                },
                move_history: VecDeque::new(),
                intent_dmg: 10,
                logical_position: 0,
            }],
            potions: vec![None, None, None],
            power_db: HashMap::new(),
            action_queue: VecDeque::new(),
            counters: crate::combat::EphemeralCounters::default(),
            card_uuid_counter: 999,
            rng: crate::rng::RngPool::new(123),
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        }
    }

    #[test]
    fn double_tap_priority_rises_with_good_followup_attack() {
        let combat = combat_with_hand(
            &[CardId::DoubleTap, CardId::Bash, CardId::Strike],
            &[CardId::DoubleTap, CardId::Bash, CardId::Strike],
        );
        let state = build_sim_state(&combat);
        assert!(play_priority(&state, 0) > 0);
    }

    #[test]
    fn feel_no_pain_priority_rises_against_sentries_with_status_backlog() {
        let mut combat = combat_with_hand(
            &[CardId::FeelNoPain, CardId::Strike],
            &[
                CardId::FeelNoPain,
                CardId::Strike,
                CardId::Dazed,
                CardId::Dazed,
            ],
        );
        combat.monsters[0].monster_type = EnemyId::Sentry as usize;
        let state = build_sim_state(&combat);
        assert!(play_priority(&state, 0) > 10_000);
    }

    #[test]
    fn limit_break_priority_requires_meaningful_strength() {
        let mut combat = combat_with_hand(
            &[CardId::LimitBreak, CardId::HeavyBlade],
            &[CardId::LimitBreak, CardId::HeavyBlade],
        );
        let low = build_sim_state(&combat);
        combat.power_db.insert(
            0,
            vec![crate::combat::Power {
                power_type: PowerId::Strength,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );
        let high = build_sim_state(&combat);
        assert!(play_priority(&high, 0) > play_priority(&low, 0));
    }

    #[test]
    fn whirlwind_priority_scales_with_available_energy() {
        let mut low = combat_with_hand(&[CardId::Whirlwind], &[CardId::Whirlwind]);
        low.energy = 1;
        let low_state = build_sim_state(&low);

        let mut high = combat_with_hand(&[CardId::Whirlwind], &[CardId::Whirlwind]);
        high.energy = 3;
        let high_state = build_sim_state(&high);

        assert!(play_priority(&high_state, 0) > play_priority(&low_state, 0));
    }

    #[test]
    fn lone_monster_attacks_are_preferred_over_end_turn() {
        let mut combat = combat_with_hand(
            &[CardId::Anger, CardId::Strike, CardId::Anger],
            &[CardId::Anger, CardId::Strike, CardId::Anger],
        );
        combat.monsters[0].current_hp = 17;
        combat.monsters[0].intent_dmg = 7;
        combat.monsters[0].current_intent = Intent::Attack { damage: 7, hits: 1 };

        let decision = decide_heuristic(&combat);
        assert!(matches!(decision, ClientInput::PlayCard { .. }));
    }

    #[test]
    fn x_cost_cards_are_treated_as_legal_plays() {
        let mut combat = combat_with_hand(&[CardId::Whirlwind], &[CardId::Whirlwind]);
        combat.energy = 1;
        let state = build_sim_state(&combat);

        let plays = get_plays(&state);
        assert_eq!(plays.len(), 1);
        assert_eq!(plays[0].0, 0);
    }
}
