use crate::bot::monster_belief::build_combat_belief_state;
use crate::content::cards::{get_card_definition, CardId, CardTarget, CardType};
use crate::content::monsters::EnemyId;
use crate::content::relics::RelicId;
use crate::runtime::combat::{CombatState, Intent, PowerId};

const MAX_HAND: usize = 12;
pub(super) const MAX_STATES: usize = 50_000;
pub(super) const DRAW_VALUE: i64 = 550;

#[derive(Clone, Copy)]
pub(super) struct SimMonster {
    pub entity_id: usize,
    pub monster_type: usize,
    pub hp: i32,
    pub block: i32,
    pub flight: i32,
    pub strength: i32,
    pub vulnerable: i32,
    pub weak: i32,
    pub is_attacking: bool,
    pub is_gone: bool,
    pub nob_enrage: bool,
    pub persistent_block: bool,
    pub sharp_hide: i32,
    pub thorns: i32,
    pub intent_dmg: i32,
    pub intent_hits: i32,
}

impl Default for SimMonster {
    fn default() -> Self {
        Self {
            entity_id: 0,
            monster_type: 0,
            hp: 0,
            block: 0,
            flight: 0,
            strength: 0,
            vulnerable: 0,
            weak: 0,
            is_attacking: false,
            is_gone: true,
            nob_enrage: false,
            persistent_block: false,
            sharp_hide: 0,
            thorns: 0,
            intent_dmg: 0,
            intent_hits: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SimCard {
    pub card_id: CardId,
    pub upgrades: i32,
    pub cost: i32,
    pub base_damage: i32,
    pub base_block: i32,
    pub base_magic: i32,
    pub card_type: CardType,
    pub target: CardTarget,
    pub hits: i32,
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

#[derive(Clone)]
pub(super) struct SimState {
    pub energy: i32,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub player_strength: i32,
    pub player_dexterity: i32,
    pub player_artifact: i32,
    pub player_intangible: i32,
    pub player_weak: bool,
    pub player_frail: bool,
    pub player_vulnerable: bool,
    pub player_entangled: bool,
    pub player_no_draw: bool,
    pub has_corruption: bool,
    pub has_feel_no_pain: bool,
    pub has_dark_embrace: bool,
    pub has_rupture: bool,
    pub has_combust: bool,
    pub has_brutality: bool,
    pub has_panache: bool,
    pub has_mayhem: bool,
    pub has_magnetism: bool,
    pub has_metallicize: bool,
    pub has_evolve: bool,
    pub has_fire_breathing: bool,
    pub has_berserk: bool,
    pub has_runic_pyramid: bool,
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
    pub double_tap_active: bool,
    pub future_growth_value: i32,
    pub future_status_cards: i32,
    pub future_zero_cost_cards: i32,
    pub future_one_cost_cards: i32,
    pub future_two_plus_cost_cards: i32,
    pub future_key_delay_weight: i32,
    pub future_high_cost_key_delay_weight: i32,
    pub remaining_apparitions_total: i32,
    pub enemy_strength_sum: i32,
    pub sentry_count: i32,
    pub card_pool_size: i32,
    pub draw_pile_size: i32,
    pub discard_pile_size: i32,
    pub status_in_draw: i32,
    pub status_in_discard: i32,
    pub draw_bonus: i64,
    pub monsters: Vec<SimMonster>,
    pub hand: [SimCard; MAX_HAND],
    pub hand_mask: u16,
}

pub(super) type Play = (usize, Option<usize>);

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

pub(super) fn active_hand_cards(state: &SimState) -> impl Iterator<Item = (usize, &SimCard)> {
    state
        .hand
        .iter()
        .enumerate()
        .filter(move |(i, _)| (state.hand_mask & (1u16 << i)) != 0)
}

pub(super) fn active_monsters(state: &SimState) -> impl Iterator<Item = (usize, &SimMonster)> {
    state
        .monsters
        .iter()
        .enumerate()
        .filter(|(_, monster)| !monster.is_gone)
}

fn build_sim_monsters(combat: &CombatState) -> Vec<SimMonster> {
    let belief = build_combat_belief_state(combat);
    let mut monsters: Vec<_> = combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead && m.current_hp > 0)
        .map(|m| {
            let belief_entry = belief
                .monsters
                .iter()
                .find(|belief_monster| belief_monster.entity_id == m.id);
            let (is_attacking, intent_dmg, intent_hits) = if belief.hidden_intent_active {
                let attack_probability = belief_entry
                    .map(|belief_monster| belief_monster.attack_probability)
                    .unwrap_or(0.0);
                let expected_total = belief_entry
                    .map(|belief_monster| belief_monster.expected_incoming_damage.round() as i32)
                    .unwrap_or(0);
                (
                    attack_probability > 0.25,
                    expected_total.max(0),
                    if expected_total > 0 { 1 } else { 0 },
                )
            } else {
                let is_attacking = matches!(
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
                (is_attacking, m.intent_dmg, hits)
            };
            (
                !is_attacking,
                m.protocol_identity.draw_x.is_none(),
                m.protocol_identity.draw_x.unwrap_or_default(),
                m.protocol_identity.group_index.unwrap_or(usize::MAX),
                m.slot as usize,
                m.id,
                SimMonster {
                    entity_id: m.id,
                    monster_type: m.monster_type,
                    hp: m.current_hp,
                    block: m.block,
                    flight: combat.get_power(m.id, PowerId::Flight),
                    strength: combat.get_power(m.id, PowerId::Strength),
                    vulnerable: combat.get_power(m.id, PowerId::Vulnerable),
                    weak: combat.get_power(m.id, PowerId::Weak),
                    is_attacking,
                    is_gone: false,
                    nob_enrage: combat.get_power(m.id, PowerId::Anger) != 0,
                    persistent_block: combat.get_power(m.id, PowerId::Barricade) != 0,
                    sharp_hide: combat.get_power(m.id, PowerId::SharpHide),
                    thorns: combat.get_power(m.id, PowerId::Thorns),
                    intent_dmg,
                    intent_hits,
                },
            )
        })
        .collect();
    monsters.sort_by_key(|entry| (entry.0, entry.1, entry.2, entry.3, entry.4, entry.5));
    monsters.into_iter().map(|entry| entry.6).collect()
}

pub(super) fn build_sim_state(combat: &CombatState) -> SimState {
    let p_str = combat.get_power(0, PowerId::Strength);
    let p_dex = combat.get_power(0, PowerId::Dexterity);
    let p_vigor = combat.get_power(0, PowerId::Vigor);
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
    let future_key_delay_weight = future_drawable_cards
        .iter()
        .map(|card| key_card_delay_weight(card.id))
        .sum();
    let future_high_cost_key_delay_weight = future_drawable_cards
        .iter()
        .filter(|card| card.get_cost() >= 1)
        .map(|card| key_card_delay_weight(card.id))
        .sum();

    let monsters = build_sim_monsters(combat);

    let mut hand = [SimCard::default(); MAX_HAND];
    let mut hand_mask: u16 = 0;
    for (i, card) in combat.zones.hand.iter().take(MAX_HAND).enumerate() {
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
            target: crate::content::cards::effective_target(card),
            hits,
        };
        hand_mask |= 1 << i;
    }

    SimState {
        energy: combat.turn.energy as i32,
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        player_strength: p_str + p_vigor,
        player_dexterity: p_dex,
        player_artifact: combat.get_power(0, PowerId::Artifact),
        player_intangible: combat
            .get_power(0, PowerId::Intangible)
            .max(combat.get_power(0, PowerId::IntangiblePlayer)),
        player_weak: combat.get_power(0, PowerId::Weak) > 0,
        player_frail: combat.get_power(0, PowerId::Frail) > 0,
        player_vulnerable: combat.get_power(0, PowerId::Vulnerable) > 0,
        player_entangled: combat.get_power(0, PowerId::Entangle) > 0,
        player_no_draw: combat.get_power(0, PowerId::NoDraw) != 0,
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
        has_fire_breathing: combat.get_power(0, PowerId::FireBreathing) > 0,
        has_berserk: combat.get_power(0, PowerId::Berserk) > 0,
        has_runic_pyramid: combat.entities.player.has_relic(RelicId::RunicPyramid),
        is_boss_fight: combat.meta.is_boss_fight,
        is_elite_fight: combat.meta.is_elite_fight,
        double_tap_active: combat.get_power(0, PowerId::DoubleTap) > 0,
        future_growth_value: 0,
        future_status_cards: combat
            .zones
            .draw_pile
            .iter()
            .chain(combat.zones.discard_pile.iter())
            .chain(combat.zones.hand.iter())
            .filter(|card| {
                matches!(
                    get_card_definition(card.id).card_type,
                    CardType::Status | CardType::Curse
                )
            })
            .count() as i32,
        future_zero_cost_cards,
        future_one_cost_cards,
        future_two_plus_cost_cards,
        future_key_delay_weight,
        future_high_cost_key_delay_weight,
        remaining_apparitions_total: combat
            .zones
            .hand
            .iter()
            .chain(combat.zones.draw_pile.iter())
            .chain(combat.zones.discard_pile.iter())
            .filter(|card| card.id == CardId::Apparition)
            .count() as i32,
        enemy_strength_sum: combat
            .entities
            .monsters
            .iter()
            .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead && m.current_hp > 0)
            .map(|m| combat.get_power(m.id, PowerId::Strength).max(0))
            .sum(),
        sentry_count: combat
            .entities
            .monsters
            .iter()
            .filter(|m| {
                !m.is_dying
                    && !m.is_escaped
                    && !m.half_dead
                    && m.current_hp > 0
                    && m.monster_type == EnemyId::Sentry as usize
            })
            .count() as i32,
        card_pool_size: (combat.zones.draw_pile.len()
            + combat.zones.discard_pile.len()
            + combat.zones.hand.len()) as i32,
        draw_pile_size: combat.zones.draw_pile.len() as i32,
        discard_pile_size: combat.zones.discard_pile.len() as i32,
        status_in_draw: combat
            .zones
            .draw_pile
            .iter()
            .filter(|card| {
                matches!(
                    get_card_definition(card.id).card_type,
                    CardType::Status | CardType::Curse
                )
            })
            .count() as i32,
        status_in_discard: combat
            .zones
            .discard_pile
            .iter()
            .filter(|card| {
                matches!(
                    get_card_definition(card.id).card_type,
                    CardType::Status | CardType::Curse
                )
            })
            .count() as i32,
        draw_bonus: 0,
        monsters,
        hand,
        hand_mask,
    }
}

pub(super) fn get_plays(state: &SimState) -> Vec<Play> {
    let mut plays = Vec::with_capacity(32);
    let mut seen_ids = [(CardId::Strike, -999i32); MAX_HAND];
    let mut sc = 0usize;

    let mut mask = state.hand_mask;
    while mask != 0 {
        let i = mask.trailing_zeros() as usize;
        mask &= mask - 1;

        let card = &state.hand[i];
        let effective_cost = super::apply::effective_energy_cost(state, card);
        if effective_cost > state.energy {
            continue;
        }
        if card.card_type == CardType::Curse
            || (card.card_type == CardType::Status && card.card_id != CardId::Slimed)
        {
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
                for (_, monster) in active_monsters(state) {
                    plays.push((i, Some(monster.entity_id)));
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

pub(super) fn gremlin_nob_enrage_active(state: &SimState) -> bool {
    active_monsters(state).any(|(_, monster)| monster.nob_enrage)
}

fn skill_allowed_vs_nob(state: &SimState, card: &SimCard) -> bool {
    let incoming = super::scoring::total_incoming_damage(state);
    let lethalish = incoming >= state.player_block + state.player_hp.saturating_sub(4);
    match card.card_id {
        CardId::Impervious => lethalish,
        CardId::GhostlyArmor | CardId::FlameBarrier => lethalish,
        CardId::Shockwave | CardId::Disarm => lethalish && incoming > state.player_block,
        _ => false,
    }
}

pub(super) fn fast_hash(state: &SimState) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    h = fnv(h, state.energy as u64);
    h = fnv(h, state.player_block as u64);
    h = fnv(h, state.player_strength as u64);
    h = fnv(h, state.player_hp as u64);
    h = fnv(h, state.player_no_draw as u64);
    h = fnv(h, state.player_intangible as u64);
    h = fnv(h, state.draw_pile_size as u64);
    h = fnv(h, state.discard_pile_size as u64);
    h = fnv(h, state.status_in_draw as u64);
    h = fnv(h, state.status_in_discard as u64);
    h = fnv(h, state.draw_bonus as u64);
    h = fnv(h, state.hand_mask as u64);
    for (idx, card) in active_hand_cards(state) {
        h = fnv(h, idx as u64);
        h = fnv(h, card.card_id as u16 as u64);
        h = fnv(h, card.upgrades as u64);
        h = fnv(h, card.cost as u64);
        h = fnv(h, card.base_damage as u64);
        h = fnv(h, card.base_block as u64);
        h = fnv(h, card.base_magic as u64);
    }
    for monster in &state.monsters {
        h = fnv(h, monster.entity_id as u64);
        h = fnv(h, monster.monster_type as u64);
        h = fnv(h, monster.hp as u64);
        h = fnv(h, monster.block as u64);
        h = fnv(h, monster.flight as u64);
        h = fnv(h, monster.strength as u64);
        h = fnv(h, monster.vulnerable as u64);
        h = fnv(h, monster.weak as u64);
        h = fnv(h, monster.is_attacking as u64);
        h = fnv(h, monster.is_gone as u64);
        h = fnv(h, monster.intent_dmg as u64);
        h = fnv(h, monster.intent_hits as u64);
    }
    h
}

#[inline(always)]
fn fnv(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x100000001b3)
}
