use crate::combat::CombatState;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::potions::{get_potion_definition, PotionId};
use crate::content::powers::PowerId;
use crate::engine::targeting;
use crate::state::core::ClientInput;

#[derive(Clone, Copy)]
struct PotionContext {
    player_hp: i32,
    unblocked_incoming: i32,
    low_hp: bool,
    critical_hp: bool,
    imminent_lethal: bool,
    elite_or_boss: bool,
    early_buff_window: bool,
    nob_active: bool,
    alive_monsters: i32,
    attacking_monsters: i32,
    max_intent_hits: i32,
    hand_junk: i32,
    statuses_in_hand: i32,
    playable_attacks: i32,
    playable_blocks: i32,
    expensive_unplayable_cards: i32,
    energy_hungry_cards: i32,
    total_enemy_hp: i32,
    upgradable_cards_in_hand: i32,
    hand_has_searing_blow: bool,
    discard_recovery_score: i32,
    exhaustable_junk: i32,
    player_has_artifact: bool,
    hand_has_flex: bool,
    hand_has_x_cost: bool,
    likely_long_fight: bool,
    boss_stalling_window: bool,
}

#[derive(Clone, Copy)]
struct PotionPlan {
    score: i32,
    potion_index: usize,
    target: Option<usize>,
}

const DONT_PLAY_POTIONS: &[PotionId] = &[PotionId::FairyPotion];

/// Checks if we should immediately consume a potion.
/// Returns the ClientInput to play the potion if appropriate, otherwise None.
pub fn should_use_potion(state: &CombatState) -> Option<ClientInput> {
    let ctx = build_context(state);
    if !should_consider_potions(&ctx) {
        return None;
    }

    let mut best: Option<PotionPlan> = None;

    for (potion_index, potion) in state
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(idx, slot)| slot.as_ref().map(|p| (idx, p)))
    {
        if DONT_PLAY_POTIONS.contains(&potion.id) {
            continue;
        }

        let Some(plan) = plan_for_potion(state, &ctx, potion_index, potion.id) else {
            continue;
        };

        match best {
            Some(current) if current.score >= plan.score => {}
            _ => best = Some(plan),
        }
    }

    let best = best?;
    let minimum_score = if ctx.imminent_lethal {
        34
    } else if ctx.elite_or_boss && (ctx.low_hp || ctx.unblocked_incoming > 0) {
        44
    } else {
        55
    };
    if best.score < minimum_score {
        return None;
    }

    Some(ClientInput::UsePotion {
        potion_index: best.potion_index,
        target: best.target,
    })
}

fn build_context(state: &CombatState) -> PotionContext {
    let mut incoming_damage = 0;
    let mut attacking_monsters = 0;
    let mut max_intent_hits = 0;
    let mut alive_monsters = 0;
    let mut total_enemy_hp = 0;

    for monster in &state.entities.monsters {
        if monster.is_dying || monster.is_escaped || monster.current_hp <= 0 {
            continue;
        }
        alive_monsters += 1;
        total_enemy_hp += monster.current_hp + monster.block;
        let hits = match monster.current_intent {
            crate::combat::Intent::Attack { hits, .. }
            | crate::combat::Intent::AttackBuff { hits, .. }
            | crate::combat::Intent::AttackDebuff { hits, .. }
            | crate::combat::Intent::AttackDefend { hits, .. } => hits as i32,
            _ => 0,
        };
        if hits > 0 {
            attacking_monsters += 1;
            incoming_damage += monster.intent_dmg.max(0) * hits.max(1);
            max_intent_hits = max_intent_hits.max(hits.max(1));
        }
    }

    let mut hand_junk = 0;
    let mut statuses_in_hand = 0;
    let mut playable_attacks = 0;
    let mut playable_blocks = 0;
    let mut expensive_unplayable_cards = 0;
    let mut energy_hungry_cards = 0;
    let mut hand_has_x_cost = false;
    let mut upgradable_cards_in_hand = 0;
    let mut hand_has_searing_blow = false;
    let mut exhaustable_junk = 0;

    for card in &state.zones.hand {
        let def = get_card_definition(card.id);
        let can_play_now = crate::content::cards::can_play_card(card, state).is_ok();
        if card.id == CardId::SearingBlow {
            hand_has_searing_blow = true;
        }
        if !matches!(def.card_type, CardType::Status | CardType::Curse)
            && (card.id == CardId::SearingBlow || card.upgrades == 0)
        {
            upgradable_cards_in_hand += 1;
        }
        if matches!(def.card_type, CardType::Curse | CardType::Status) {
            hand_junk += 1;
            statuses_in_hand += 1;
            exhaustable_junk += 1;
            continue;
        }
        if can_play_now {
            if def.card_type == CardType::Attack {
                playable_attacks += 1;
            }
            if def.base_block > 0 {
                playable_blocks += 1;
            }
        }
        let cost = card.get_cost() as i32;
        if cost < 0 {
            hand_has_x_cost = true;
            energy_hungry_cards += 1;
        } else if cost > 0 {
            if cost > state.turn.energy as i32 {
                expensive_unplayable_cards += 1;
            }
            if can_play_now {
                energy_hungry_cards += 1;
            }
        }
        if !can_play_now
            && matches!(
                card.id,
                CardId::Strike | CardId::Defend | CardId::DefendG | CardId::GoodInstincts
            )
        {
            hand_junk += 1;
        }
    }

    let discard_recovery_score = state
        .zones
        .discard_pile
        .iter()
        .map(|card| match card.id {
            CardId::Offering => 42,
            CardId::Apparition => 38,
            CardId::Impervious => 34,
            CardId::Reaper => 30,
            CardId::SearingBlow => 28 + card.upgrades as i32 * 6,
            CardId::FlameBarrier | CardId::GhostlyArmor | CardId::PowerThrough => 22,
            CardId::Disarm | CardId::Shockwave | CardId::Uppercut => 20,
            CardId::ShrugItOff | CardId::BattleTrance | CardId::BurningPact => 16,
            _ => {
                let def = get_card_definition(card.id);
                let mut score = 0;
                if matches!(def.card_type, CardType::Power) {
                    score += 18;
                }
                if def.base_damage >= 12 {
                    score += 10;
                }
                if def.base_block >= 10 {
                    score += 10;
                }
                score
            }
        })
        .max()
        .unwrap_or(0);

    let hp_per =
        (state.entities.player.current_hp as f32 / state.entities.player.max_hp as f32) * 100.0;
    let unblocked_incoming = (incoming_damage - state.entities.player.block).max(0);

    PotionContext {
        player_hp: state.entities.player.current_hp,
        unblocked_incoming,
        low_hp: hp_per <= 50.0,
        critical_hp: hp_per <= 30.0,
        imminent_lethal: incoming_damage
            >= (state.entities.player.current_hp + state.entities.player.block),
        elite_or_boss: state.meta.is_elite_fight || state.meta.is_boss_fight,
        early_buff_window: state.turn.turn_count <= 2,
        nob_active: state
            .entities
            .monsters
            .iter()
            .any(|m| !m.is_dying && !m.is_escaped && state.get_power(m.id, PowerId::Anger) != 0),
        alive_monsters,
        attacking_monsters,
        max_intent_hits,
        hand_junk,
        statuses_in_hand,
        playable_attacks,
        playable_blocks,
        expensive_unplayable_cards,
        energy_hungry_cards,
        total_enemy_hp,
        upgradable_cards_in_hand,
        hand_has_searing_blow,
        discard_recovery_score,
        exhaustable_junk,
        player_has_artifact: state.get_power(0, PowerId::Artifact) > 0,
        hand_has_flex: state.zones.hand.iter().any(|c| c.id == CardId::Flex),
        hand_has_x_cost,
        likely_long_fight: state.meta.is_boss_fight
            || alive_monsters == 1
                && state
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
                    .map(|m| m.current_hp + m.block)
                    .max()
                    .unwrap_or(0)
                    >= 80,
        boss_stalling_window: state.meta.is_boss_fight
            && alive_monsters == 1
            && incoming_damage <= state.entities.player.block + 12,
    }
}

fn should_consider_potions(ctx: &PotionContext) -> bool {
    (ctx.elite_or_boss && ctx.early_buff_window)
        || ctx.low_hp
        || ctx.imminent_lethal
        || ctx.nob_active
        || ctx.unblocked_incoming >= 12
        || (ctx.elite_or_boss && (ctx.unblocked_incoming > 0 || ctx.total_enemy_hp >= 90))
        || ctx.discard_recovery_score >= 26
        || ctx.exhaustable_junk >= 2
}

fn plan_for_potion(
    state: &CombatState,
    ctx: &PotionContext,
    potion_index: usize,
    potion_id: PotionId,
) -> Option<PotionPlan> {
    let def = get_potion_definition(potion_id);
    let (target, target_score) =
        if let Some(validation) = targeting::validation_for_potion_target(def.target_required) {
            let candidates = targeting::candidate_targets(state, validation);
            let target = best_potion_target(state, ctx, potion_id, &candidates)?;
            let score = potion_target_score(state, ctx, potion_id, target);
            (Some(target), score)
        } else {
            (None, 0)
        };

    let score = potion_base_score(state, ctx, potion_id) + target_score;
    if score <= 0 {
        return None;
    }

    Some(PotionPlan {
        score,
        potion_index,
        target,
    })
}

fn potion_base_score(state: &CombatState, ctx: &PotionContext, potion_id: PotionId) -> i32 {
    match potion_id {
        PotionId::AncientPotion => {
            if ctx.hand_has_flex && !ctx.player_has_artifact {
                98
            } else if ctx.elite_or_boss && !ctx.player_has_artifact && ctx.early_buff_window {
                90
            } else if !ctx.player_has_artifact && (ctx.low_hp || ctx.attacking_monsters > 0) {
                78
            } else {
                12
            }
        }
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::DuplicationPotion
        | PotionId::HeartOfIron
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze => {
            if ctx.elite_or_boss && ctx.early_buff_window {
                92
            } else if ctx.imminent_lethal || ctx.nob_active {
                90
            } else if ctx.low_hp {
                82
            } else {
                76
            }
        }
        PotionId::CultistPotion => {
            if ctx.boss_stalling_window {
                112
            } else if ctx.likely_long_fight && ctx.early_buff_window && !ctx.imminent_lethal {
                92
            } else if ctx.imminent_lethal || ctx.alive_monsters >= 3 {
                26
            } else {
                44
            }
        }
        PotionId::EnergyPotion => {
            if ctx.hand_has_x_cost && ctx.energy_hungry_cards > 0 {
                96
            } else if ctx.energy_hungry_cards >= 2 {
                90
            } else if ctx.expensive_unplayable_cards > 0 && ctx.unblocked_incoming > 0 {
                84
            } else {
                54
            }
        }
        PotionId::PowerPotion | PotionId::ColorlessPotion => {
            if ctx.elite_or_boss || ctx.unblocked_incoming > 0 {
                80
            } else {
                64
            }
        }
        PotionId::DistilledChaosPotion => {
            if ctx.imminent_lethal {
                92
            } else if ctx.unblocked_incoming > 0 && ctx.playable_blocks == 0 {
                86
            } else if ctx.elite_or_boss {
                74
            } else {
                38
            }
        }
        PotionId::AttackPotion | PotionId::SkillPotion | PotionId::SwiftPotion => {
            if ctx.imminent_lethal || ctx.unblocked_incoming > 0 {
                84
            } else if ctx.elite_or_boss {
                74
            } else {
                60
            }
        }
        PotionId::FearPotion => {
            if ctx.playable_attacks <= 0 {
                40
            } else if ctx.elite_or_boss || ctx.nob_active {
                92
            } else if ctx.attacking_monsters > 0 {
                78
            } else {
                66
            }
        }
        PotionId::WeakenPotion => {
            if ctx.imminent_lethal {
                96
            } else if ctx.unblocked_incoming > 0 {
                86
            } else {
                60
            }
        }
        PotionId::FirePotion => {
            if ctx.imminent_lethal || ctx.alive_monsters == 1 {
                82
            } else {
                64
            }
        }
        PotionId::ExplosivePotion => {
            if ctx.alive_monsters >= 3 {
                90
            } else if ctx.alive_monsters == 2 {
                78
            } else {
                42
            }
        }
        PotionId::PoisonPotion => {
            if ctx.elite_or_boss && ctx.alive_monsters == 1 {
                80
            } else {
                58
            }
        }
        PotionId::BlockPotion => {
            if ctx.imminent_lethal {
                100
            } else if ctx.unblocked_incoming > 0 || ctx.low_hp {
                86
            } else {
                36
            }
        }
        PotionId::RegenPotion => {
            if ctx.low_hp && ctx.elite_or_boss {
                82
            } else {
                42
            }
        }
        PotionId::GhostInAJar => {
            if ctx.imminent_lethal {
                110
            } else if ctx.max_intent_hits >= 2 && (ctx.low_hp || ctx.unblocked_incoming > 0) {
                102
            } else if ctx.unblocked_incoming >= ctx.player_hp / 2 {
                94
            } else if ctx.low_hp || ctx.elite_or_boss {
                82
            } else {
                50
            }
        }
        PotionId::LiquidMemories => {
            if ctx.discard_recovery_score >= 34 {
                104
            } else if ctx.discard_recovery_score >= 24 && (ctx.elite_or_boss || ctx.low_hp) {
                88
            } else if ctx.discard_recovery_score >= 16 {
                62
            } else {
                18
            }
        }
        PotionId::BloodPotion | PotionId::FruitJuice => {
            if ctx.critical_hp {
                if ctx.elite_or_boss {
                    74
                } else {
                    58
                }
            } else {
                18
            }
        }
        PotionId::GamblersBrew => gambler_brew_score(state, ctx),
        PotionId::Elixir => {
            if ctx.exhaustable_junk >= 2 {
                92
            } else if ctx.exhaustable_junk >= 1 && (ctx.unblocked_incoming > 0 || ctx.elite_or_boss)
            {
                76
            } else {
                18
            }
        }
        PotionId::StancePotion => {
            if ctx.imminent_lethal || (ctx.unblocked_incoming > 0 && ctx.low_hp) {
                82
            } else if ctx.elite_or_boss && ctx.playable_attacks > 0 {
                68
            } else {
                24
            }
        }
        PotionId::BlessingOfTheForge => {
            if ctx.elite_or_boss && ctx.hand_has_searing_blow {
                104
            } else if ctx.elite_or_boss && ctx.upgradable_cards_in_hand >= 2 {
                88
            } else if ctx.upgradable_cards_in_hand >= 3 {
                68
            } else {
                24
            }
        }
        PotionId::SneckoOil => {
            if ctx.expensive_unplayable_cards >= 2
                || (ctx.energy_hungry_cards >= 2 && ctx.unblocked_incoming > 0)
            {
                86
            } else if ctx.hand_junk >= 2 {
                62
            } else {
                20
            }
        }
        PotionId::SmokeBomb => {
            if !ctx.elite_or_boss
                && (ctx.imminent_lethal || (ctx.low_hp && ctx.unblocked_incoming > 0))
            {
                140
            } else if ctx.elite_or_boss {
                -10_000
            } else {
                0
            }
        }
        PotionId::Ambrosia => {
            if ctx.imminent_lethal {
                96
            } else if ctx.elite_or_boss && ctx.playable_attacks >= 1 {
                82
            } else {
                30
            }
        }
        PotionId::EssenceOfDarkness => {
            if ctx.elite_or_boss && ctx.likely_long_fight {
                70
            } else {
                24
            }
        }
        _ => 28,
    }
}

fn gambler_brew_score(state: &CombatState, ctx: &PotionContext) -> i32 {
    let discardable_junk = state
        .zones
        .hand
        .iter()
        .filter(|card| {
            let def = get_card_definition(card.id);
            matches!(def.card_type, CardType::Curse | CardType::Status)
                || (card.get_cost() as i32) > state.turn.energy as i32
        })
        .count() as i32;

    if discardable_junk <= 0 && ctx.hand_junk <= 0 && ctx.expensive_unplayable_cards <= 0 {
        return 18;
    }

    let mut score = 40
        + discardable_junk * 16
        + ctx.statuses_in_hand * 22
        + ctx.expensive_unplayable_cards * 10;
    if ctx.imminent_lethal {
        score += 22;
    } else if ctx.unblocked_incoming > 0 {
        score += 14;
    }
    if ctx.elite_or_boss {
        score += 8;
    }
    if ctx.playable_blocks == 0 && ctx.unblocked_incoming > 0 {
        score += 10;
    }
    score
}

fn best_potion_target(
    state: &CombatState,
    ctx: &PotionContext,
    potion_id: PotionId,
    candidates: &[usize],
) -> Option<usize> {
    candidates
        .iter()
        .copied()
        .max_by_key(|target| potion_target_score(state, ctx, potion_id, *target))
}

fn potion_target_score(
    state: &CombatState,
    ctx: &PotionContext,
    potion_id: PotionId,
    target: usize,
) -> i32 {
    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target)
        .map(|monster| {
            let artifact = state.get_power(monster.id, PowerId::Artifact);
            let intent_hits = match monster.current_intent {
                crate::combat::Intent::Attack { hits, .. }
                | crate::combat::Intent::AttackBuff { hits, .. }
                | crate::combat::Intent::AttackDebuff { hits, .. }
                | crate::combat::Intent::AttackDefend { hits, .. } => hits as i32,
                _ => 0,
            }
            .max(1);
            match potion_id {
                PotionId::WeakenPotion => {
                    monster.intent_dmg.max(0) * intent_hits * 12 - artifact * 1_500
                }
                PotionId::FearPotion => {
                    ctx.playable_attacks * 2_000 + monster.current_hp - artifact * 1_000
                }
                PotionId::FirePotion => {
                    let potency = get_potion_definition(potion_id).base_potency;
                    let lethal_bonus = if monster.current_hp <= potency {
                        8_000
                    } else {
                        0
                    };
                    lethal_bonus + monster.current_hp
                }
                PotionId::PoisonPotion => {
                    let boss_bonus = i32::from(ctx.elite_or_boss) * 2_000;
                    boss_bonus + monster.current_hp - artifact * 1_000
                }
                _ => monster.current_hp,
            }
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{
        CombatCard, CombatMeta, CombatPhase, CombatRng, EngineRuntime, EntityState, Intent,
        MonsterEntity, PlayerEntity, RelicBuses, StanceId, TurnRuntime,
    };
    use crate::content::potions::Potion;
    use std::collections::{HashMap, VecDeque};

    fn combat_with_potions(hand: &[CardId], potions: &[PotionId]) -> CombatState {
        CombatState {
            meta: CombatMeta {
                ascension_level: 0,
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 3,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: crate::combat::CardZones {
                draw_pile: Vec::new(),
                hand: hand
                    .iter()
                    .enumerate()
                    .map(|(idx, &id)| CombatCard::new(id, 100 + idx as u32))
                    .collect(),
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 500,
            },
            entities: EntityState {
                player: PlayerEntity {
                    id: 0,
                    current_hp: 40,
                    max_hp: 80,
                    block: 0,
                    gold_delta_this_combat: 0,
                    gold: 99,
                    max_orbs: 0,
                    orbs: Vec::new(),
                    stance: StanceId::Neutral,
                    relics: Vec::new(),
                    relic_buses: RelicBuses::default(),
                    energy_master: 3,
                },
                monsters: vec![MonsterEntity {
                    id: 1,
                    monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
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
                    hexaghost: Default::default(),
                    darkling: Default::default(),
                }],
                potions: potions
                    .iter()
                    .enumerate()
                    .map(|(idx, &id)| Some(Potion::new(id, idx as u32 + 1)))
                    .chain(std::iter::repeat(None))
                    .take(3)
                    .collect(),
                power_db: HashMap::new(),
            },
            engine: EngineRuntime {
                action_queue: VecDeque::new(),
            },
            rng: CombatRng::new(crate::rng::RngPool::new(123)),
        }
    }

    #[test]
    fn fear_potion_is_preferred_over_explosive_in_single_target_attack_window() {
        let combat = combat_with_potions(
            &[CardId::Bash, CardId::Strike],
            &[PotionId::ExplosivePotion, PotionId::FearPotion],
        );
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 1,
                target: Some(1),
            })
        ));
    }

    #[test]
    fn gamblers_brew_is_used_when_hand_is_clogged_with_statuses_under_pressure() {
        let mut combat = combat_with_potions(
            &[CardId::Dazed, CardId::Slimed, CardId::Strike],
            &[PotionId::GamblersBrew],
        );
        combat.entities.player.current_hp = 20;
        combat.entities.monsters[0].intent_dmg = 18;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 18,
            hits: 1,
        };
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn ghost_in_a_jar_spikes_in_multi_hit_frail_window() {
        let mut combat =
            combat_with_potions(&[CardId::Strike, CardId::Defend], &[PotionId::GhostInAJar]);
        combat.entities.player.current_hp = 18;
        combat.entities.player.max_hp = 42;
        combat.entities.monsters[0].intent_dmg = 10;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 10,
            hits: 2,
        };
        combat.entities.power_db.insert(
            0,
            vec![crate::combat::Power {
                power_type: PowerId::Frail,
                amount: 5,
                extra_data: 0,
                just_applied: false,
            }],
        );
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn cultist_potion_is_held_in_short_multi_enemy_fight() {
        let mut combat = combat_with_potions(
            &[CardId::Strike, CardId::Defend],
            &[PotionId::CultistPotion],
        );
        combat.entities.monsters = vec![
            MonsterEntity {
                id: 1,
                monster_type: crate::content::monsters::EnemyId::GremlinWarrior as usize,
                current_hp: 21,
                max_hp: 21,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Attack { damage: 4, hits: 1 },
                move_history: VecDeque::new(),
                intent_dmg: 4,
                logical_position: 0,
                hexaghost: Default::default(),
                darkling: Default::default(),
            },
            MonsterEntity {
                id: 2,
                monster_type: crate::content::monsters::EnemyId::GremlinFat as usize,
                current_hp: 14,
                max_hp: 14,
                block: 0,
                slot: 1,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::AttackDebuff { damage: 4, hits: 1 },
                move_history: VecDeque::new(),
                intent_dmg: 4,
                logical_position: 1,
                hexaghost: Default::default(),
                darkling: Default::default(),
            },
            MonsterEntity {
                id: 3,
                monster_type: crate::content::monsters::EnemyId::GremlinWizard as usize,
                current_hp: 21,
                max_hp: 21,
                block: 0,
                slot: 2,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Unknown,
                move_history: VecDeque::new(),
                intent_dmg: 0,
                logical_position: 2,
                hexaghost: Default::default(),
                darkling: Default::default(),
            },
            MonsterEntity {
                id: 4,
                monster_type: crate::content::monsters::EnemyId::GremlinThief as usize,
                current_hp: 12,
                max_hp: 12,
                block: 0,
                slot: 3,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Attack { damage: 9, hits: 1 },
                move_history: VecDeque::new(),
                intent_dmg: 9,
                logical_position: 3,
                hexaghost: Default::default(),
                darkling: Default::default(),
            },
        ];

        assert!(should_use_potion(&combat).is_none());
    }

    #[test]
    fn cultist_potion_is_used_in_stallable_boss_window() {
        let mut combat = combat_with_potions(
            &[CardId::Defend, CardId::Inflame],
            &[PotionId::CultistPotion],
        );
        combat.meta.is_boss_fight = true;
        combat.entities.monsters[0].monster_type =
            crate::content::monsters::EnemyId::TheGuardian as usize;
        combat.entities.monsters[0].current_hp = 240;
        combat.entities.monsters[0].max_hp = 240;
        combat.entities.monsters[0].current_intent = Intent::Defend;
        combat.entities.monsters[0].intent_dmg = 0;

        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn blessing_of_the_forge_is_used_with_searing_blow_in_boss_hand() {
        let mut combat = combat_with_potions(
            &[CardId::SearingBlow, CardId::Strike, CardId::Defend],
            &[PotionId::BlessingOfTheForge],
        );
        combat.meta.is_boss_fight = true;
        combat.entities.monsters[0].monster_type =
            crate::content::monsters::EnemyId::Hexaghost as usize;
        combat.entities.monsters[0].current_hp = 220;
        combat.entities.monsters[0].max_hp = 220;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 12,
            hits: 2,
        };
        combat.entities.monsters[0].intent_dmg = 12;

        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn elixir_is_used_when_hand_has_multiple_statuses() {
        let mut combat = combat_with_potions(
            &[CardId::Slimed, CardId::Burn, CardId::Strike],
            &[PotionId::Elixir],
        );
        combat.entities.player.current_hp = 22;
        combat.entities.monsters[0].intent_dmg = 16;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 16,
            hits: 1,
        };
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn liquid_memories_is_used_when_discard_contains_offering() {
        let mut combat = combat_with_potions(&[CardId::Strike], &[PotionId::LiquidMemories]);
        combat.meta.is_boss_fight = true;
        combat
            .zones
            .discard_pile
            .push(CombatCard::new(CardId::Offering, 999));
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn snecko_oil_is_used_when_hand_is_clogged_with_expensive_cards() {
        let mut combat = combat_with_potions(
            &[CardId::Impervious, CardId::Barricade, CardId::Bludgeon],
            &[PotionId::SneckoOil],
        );
        combat.turn.energy = 1;
        combat.entities.player.current_hp = 24;
        combat.entities.monsters[0].intent_dmg = 14;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 14,
            hits: 1,
        };
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn stance_potion_is_used_in_low_hp_attack_window() {
        let mut combat =
            combat_with_potions(&[CardId::Strike, CardId::Defend], &[PotionId::StancePotion]);
        combat.entities.player.current_hp = 18;
        combat.entities.player.max_hp = 70;
        combat.entities.monsters[0].intent_dmg = 20;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 20,
            hits: 1,
        };
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }

    #[test]
    fn smoke_bomb_is_used_in_hopeless_non_boss_fight() {
        let mut combat = combat_with_potions(&[CardId::Strike], &[PotionId::SmokeBomb]);
        combat.entities.player.current_hp = 9;
        combat.entities.player.max_hp = 70;
        combat.entities.monsters[0].intent_dmg = 24;
        combat.entities.monsters[0].current_intent = Intent::Attack {
            damage: 24,
            hits: 1,
        };
        assert!(matches!(
            should_use_potion(&combat),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            })
        ));
    }
}