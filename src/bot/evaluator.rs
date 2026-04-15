use crate::combat::{CombatCard, CombatState, Intent, PowerId};
use crate::content::cards::{get_card_definition, CardType};
use crate::content::monsters::EnemyId;
use crate::state::{EngineState, RunResult};

/// Static heuristic evaluation of the current Engine and Combat state from the AI's perspective.
/// Returns a score indicating how favorable the state is. Higher is better.
pub fn evaluate_state(engine_state: &EngineState, combat_state: &CombatState) -> f32 {
    match engine_state {
        EngineState::GameOver(RunResult::Defeat) => return -999999.0,
        EngineState::GameOver(RunResult::Victory) => return 999999.0,
        _ => {}
    }

    let mut score = 0.0;

    // Turn penalty to encourage fast kills
    score -= combat_state.turn.turn_count as f32 * 500.0;

    // Player Health is the most precious resource
    score += combat_state.entities.player.current_hp as f32 * 100.0;

    // Score player block (but incoming intent damage will subtract it back)
    score += combat_state.entities.player.block as f32 * 5.0;

    let mut total_monster_expected_damage = 0;
    let mut alive_monster_count = 0;
    let mut alive_monster_hp_sum = 0;
    let reachable_damage = approx_reachable_damage_this_turn(combat_state);

    for m in &combat_state.entities.monsters {
        if m.is_dying || m.is_escaped || m.half_dead || m.current_hp <= 0 {
            continue;
        }

        alive_monster_count += 1;
        alive_monster_hp_sum += m.current_hp.max(0);

        // Massive penalty to compel the AI to deal damage
        score -= m.current_hp as f32 * 50.0;

        // Enemy permanent scaling matters, especially Gremlin Nob style fights.
        let enemy_strength = combat_state.get_power(m.id, PowerId::Strength).max(0);
        if enemy_strength > 0 {
            score -= enemy_strength as f32 * 260.0;
        }
        score -= monster_role_penalty(combat_state, m);
        score -= attack_pressure_penalty(m);
        if reachable_damage > 0 && m.current_hp <= reachable_damage {
            score += kill_window_bonus(combat_state, m, reachable_damage);
        }
        if combat_state.get_power(m.id, PowerId::Split) > 0 && m.current_hp <= 18 {
            score -= 850.0;
        }

        // Calculate expected damage purely from attacks
        match m.current_intent {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => {
                total_monster_expected_damage += m.intent_dmg * (hits as i32);
            }
            _ => {}
        }
    }

    score -= alive_monster_count as f32 * 4500.0;
    score -= alive_monster_hp_sum as f32 * 8.0;
    if combat_state.meta.is_boss_fight {
        score -= alive_monster_count as f32 * 1500.0;
    } else if combat_state.meta.is_elite_fight {
        score -= alive_monster_count as f32 * 750.0;
    }

    // Heavily penalize unblocked incoming damage to prioritize mitigation
    // We penalize this HIGHER than the value of HP, to encourage active blocking over tanking.
    // However, since DFS evaluates the start of the *next* turn, we assume the player will
    // natively generate ~10 block from basic energy usage, softening the blow of large intents.
    let assumed_future_block = 10;
    let expected_net_damage =
        (total_monster_expected_damage - combat_state.entities.player.block).max(0);
    let unblocked_damage = (expected_net_damage - assumed_future_block).max(0);
    score -= unblocked_damage as f32 * 120.0; // soften multiplier slightly

    // Add positive scoring for player powers/buffs
    if let Some(powers) = combat_state
        .entities
        .power_db
        .get(&combat_state.entities.player.id)
    {
        for p in powers {
            // Very naive evaluation: +150 per stack of generic buff to encourage setup plays
            if !crate::content::powers::is_debuff(p.power_type, p.amount) {
                score += p.amount as f32 * 150.0;
            }
        }
    }

    // Add positive scoring for debuffs on enemies
    for m in &combat_state.entities.monsters {
        if let Some(powers) = combat_state.entities.power_db.get(&m.id) {
            for p in powers {
                if crate::content::powers::is_debuff(p.power_type, p.amount) {
                    score += p.amount as f32 * 80.0;
                }
            }
        }
    }

    // Minor score adjustments for deck quality / hand size could go here

    score
}

fn approx_reachable_damage_this_turn(combat_state: &CombatState) -> i32 {
    let mut energy = combat_state.turn.energy as i32;
    let mut damages = combat_state
        .zones
        .hand
        .iter()
        .filter_map(|card| {
            let def = get_card_definition(card.id);
            if def.card_type != CardType::Attack {
                return None;
            }
            let cost = card.get_cost() as i32;
            if cost < 0 || cost > energy {
                return None;
            }
            let base_damage = if card.base_damage_mut > 0 {
                card.base_damage_mut
            } else {
                def.base_damage
            };
            let hits = estimated_attack_hits(card.id);
            Some((cost.max(0), base_damage.max(0) * hits))
        })
        .collect::<Vec<_>>();

    damages.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut total = 0;
    for (cost, damage) in damages {
        if cost > energy {
            continue;
        }
        energy -= cost;
        total += damage;
    }
    total
}

fn estimated_attack_hits(card_id: crate::content::cards::CardId) -> i32 {
    match card_id {
        crate::content::cards::CardId::TwinStrike => 2,
        crate::content::cards::CardId::Pummel => 4,
        crate::content::cards::CardId::SwordBoomerang => 3,
        _ => 1,
    }
}

fn monster_role_penalty(combat_state: &CombatState, monster: &crate::combat::MonsterEntity) -> f32 {
    let Some(enemy) = EnemyId::from_id(monster.monster_type) else {
        return 0.0;
    };

    let mut penalty = 0.0;
    penalty += match enemy {
        EnemyId::GremlinLeader
        | EnemyId::TheCollector
        | EnemyId::Reptomancer
        | EnemyId::BronzeAutomaton
        | EnemyId::TimeEater
        | EnemyId::Hexaghost
        | EnemyId::SlimeBoss
        | EnemyId::TheGuardian => 2800.0,
        EnemyId::Darkling => 1800.0,
        EnemyId::GremlinWizard => 1200.0,
        _ => 0.0,
    };

    if combat_state.meta.is_boss_fight {
        penalty += 900.0;
    } else if combat_state.meta.is_elite_fight {
        penalty += 450.0;
    }

    penalty
}

fn attack_pressure_penalty(monster: &crate::combat::MonsterEntity) -> f32 {
    match monster.current_intent {
        Intent::Attack { hits, .. }
        | Intent::AttackBuff { hits, .. }
        | Intent::AttackDebuff { hits, .. }
        | Intent::AttackDefend { hits, .. } => {
            (monster.intent_dmg.max(0) * hits as i32) as f32 * 42.0
        }
        Intent::Buff => 180.0,
        Intent::StrongDebuff | Intent::Debuff => 120.0,
        _ => 0.0,
    }
}

fn kill_window_bonus(
    combat_state: &CombatState,
    monster: &crate::combat::MonsterEntity,
    reachable_damage: i32,
) -> f32 {
    let mut bonus = 2_400.0 + (reachable_damage - monster.current_hp).max(0) as f32 * 35.0;
    if combat_state.meta.is_boss_fight || combat_state.meta.is_elite_fight {
        bonus += 1_000.0;
    }
    bonus + monster_role_penalty(combat_state, monster) * 0.5
}

// ─── Card Evaluator ──────────────────────────────────────────────────────────

use crate::content::cards::CardId;
use crate::state::run::RunState;

pub(crate) fn curse_remove_severity(card_id: CardId) -> i32 {
    match card_id {
        CardId::Parasite | CardId::Pain | CardId::Normality => 10,
        CardId::Regret | CardId::Writhe | CardId::Decay => 8,
        CardId::CurseOfTheBell => 7,
        CardId::Doubt | CardId::Shame | CardId::Injury | CardId::Clumsy => 5,
        _ => 0,
    }
}

pub struct CardEvaluator;

#[derive(Debug, Default, Clone, Copy)]
pub struct DeckProfile {
    pub attack_count: i32,
    pub skill_count: i32,
    pub power_count: i32,
    pub searing_blow_count: i32,
    pub searing_blow_upgrades: i32,
    pub strength_enablers: i32,
    pub strength_payoffs: i32,
    pub exhaust_engines: i32,
    pub exhaust_outlets: i32,
    pub exhaust_fodder: i32,
    pub block_core: i32,
    pub block_payoffs: i32,
    pub self_damage_sources: i32,
    pub x_cost_payoffs: i32,
    pub draw_sources: i32,
    pub power_scalers: i32,
    pub status_generators: i32,
    pub status_payoffs: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CardRole {
    Engine,
    Support,
    Payoff,
    Filler,
    Generic,
}

#[derive(Debug, Clone, Copy)]
struct MarginalValueProfile {
    role: CardRole,
    presence_value: i32,
    reliability_need: i32,
    collision_risk: i32,
}

impl CardEvaluator {
    /// Score a card purely based on its static tier and how many we already have in the deck.
    /// Returns a heuristic score. If the score is too low, the agent should Skip.
    pub fn evaluate_card(card_id: CardId, run_state: &RunState) -> i32 {
        let base_score = Self::get_base_card_priority(card_id);
        let profile = Self::deck_profile(run_state);

        let mut copies = 0;
        for c in &run_state.master_deck {
            if c.id == card_id {
                copies += 1;
            }
        }

        let cap = Self::get_card_deck_cap(card_id);
        if copies >= cap {
            // We have reached the cap, drastically reduce priority so we never take it
            return base_score - 1000;
        }

        let mut score = base_score;
        score += Self::synergy_adjustment(card_id, &profile);
        score += Self::presence_adjustment(card_id, copies, &profile);
        score -= Self::marginal_duplicate_penalty(card_id, copies, &profile);
        score -= (copies * 3).max(0);

        score
    }

    pub fn evaluate_owned_card(card_id: CardId, run_state: &RunState) -> i32 {
        let base_score = Self::get_base_card_priority(card_id);
        let profile = Self::deck_profile(run_state);
        let copies = run_state
            .master_deck
            .iter()
            .filter(|c| c.id == card_id)
            .count() as i32;

        let mut score = base_score;
        score += Self::synergy_adjustment(card_id, &profile);
        score += Self::presence_adjustment(card_id, 0, &profile);
        score -= Self::marginal_duplicate_penalty(card_id, copies.saturating_sub(1), &profile);
        if copies > 1 {
            score -= (copies - 1) * 3;
        }

        score
    }

    pub fn deck_profile(run_state: &RunState) -> DeckProfile {
        Self::deck_profile_from_cards(run_state.master_deck.iter())
    }

    pub fn combat_profile(combat_state: &CombatState) -> DeckProfile {
        Self::deck_profile_from_cards(
            combat_state
                .zones
                .hand
                .iter()
                .chain(combat_state.zones.draw_pile.iter())
                .chain(combat_state.zones.discard_pile.iter())
                .chain(combat_state.zones.exhaust_pile.iter())
                .chain(combat_state.zones.limbo.iter()),
        )
    }

    pub fn archetype_tags(profile: &DeckProfile) -> Vec<String> {
        let mut tags = Vec::new();

        let strength_online = profile.strength_enablers >= 1 && profile.strength_payoffs >= 2;
        let exhaust_online = profile.exhaust_engines >= 2 && profile.exhaust_outlets >= 1;
        let block_online = profile.block_core >= 3 && profile.block_payoffs >= 1;
        let self_damage_online = profile.self_damage_sources >= 2;
        let draw_cycle_online = profile.draw_sources >= 3;
        let power_scaling_online = profile.power_scalers >= 2 || profile.power_count >= 4;
        let status_online = profile.status_generators >= 1 && profile.status_payoffs >= 1;
        let searing_blow_online = profile.searing_blow_count > 0;

        if strength_online {
            tags.push("strength".to_string());
        }
        if exhaust_online {
            tags.push("exhaust".to_string());
        }
        if block_online {
            tags.push("block".to_string());
        }
        if self_damage_online {
            tags.push("self_damage".to_string());
        }
        if draw_cycle_online {
            tags.push("draw_cycle".to_string());
        }
        if power_scaling_online {
            tags.push("power_scaling".to_string());
        }
        if status_online {
            tags.push("status".to_string());
        }
        if searing_blow_online {
            tags.push("searing_blow".to_string());
        }

        if tags.len() >= 2 {
            tags.push("hybrid".to_string());
        }

        if (profile.strength_enablers > 0 && profile.strength_payoffs == 0)
            || (profile.strength_payoffs >= 2 && profile.strength_enablers == 0)
            || (profile.exhaust_engines > 0 && profile.exhaust_outlets == 0)
            || (profile.exhaust_outlets >= 2 && profile.exhaust_engines == 0)
            || (profile.block_core >= 2 && profile.block_payoffs == 0)
            || (profile.status_generators > 0 && profile.status_payoffs == 0)
        {
            tags.push("shell_incomplete".to_string());
        }

        if tags.is_empty() {
            tags.push("goodstuff".to_string());
        }

        tags.sort();
        tags.dedup();
        tags
    }

    pub fn archetype_summary(profile: &DeckProfile) -> String {
        let tags = Self::archetype_tags(profile).join(",");
        format!(
            "tags=[{}] str={}/{} exh={}/{}/{} block={}/{}",
            tags,
            profile.strength_enablers,
            profile.strength_payoffs,
            profile.exhaust_engines,
            profile.exhaust_outlets,
            profile.exhaust_fodder,
            profile.block_core,
            profile.block_payoffs
        )
    }

    fn deck_profile_from_cards<'a, I>(cards: I) -> DeckProfile
    where
        I: IntoIterator<Item = &'a CombatCard>,
    {
        let mut profile = DeckProfile::default();

        for card in cards {
            match crate::content::cards::get_card_definition(card.id).card_type {
                crate::content::cards::CardType::Attack => profile.attack_count += 1,
                crate::content::cards::CardType::Skill => profile.skill_count += 1,
                crate::content::cards::CardType::Power => profile.power_count += 1,
                _ => {}
            }

            if matches!(
                card.id,
                CardId::Inflame
                    | CardId::SpotWeakness
                    | CardId::DemonForm
                    | CardId::LimitBreak
                    | CardId::Flex
                    | CardId::Rupture
            ) {
                profile.strength_enablers += 1;
            }
            if matches!(
                card.id,
                CardId::HeavyBlade
                    | CardId::SwordBoomerang
                    | CardId::TwinStrike
                    | CardId::Whirlwind
                    | CardId::Pummel
                    | CardId::Reaper
            ) {
                profile.strength_payoffs += 1;
            }
            if card.id == CardId::Whirlwind {
                profile.x_cost_payoffs += 1;
            }
            if card.id == CardId::SearingBlow {
                profile.searing_blow_count += 1;
                profile.searing_blow_upgrades += card.upgrades as i32;
            }
            if matches!(
                card.id,
                CardId::BattleTrance
                    | CardId::PommelStrike
                    | CardId::ShrugItOff
                    | CardId::Offering
                    | CardId::BurningPact
                    | CardId::Finesse
                    | CardId::FlashOfSteel
                    | CardId::MasterOfStrategy
                    | CardId::Brutality
            ) {
                profile.draw_sources += 1;
            }
            if matches!(
                card.id,
                CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
            ) {
                profile.exhaust_engines += 2;
                profile.power_scalers += 1;
            }
            if matches!(
                card.id,
                CardId::SecondWind
                    | CardId::FiendFire
                    | CardId::SeverSoul
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::Exhume
            ) {
                profile.exhaust_outlets += 1;
            }
            if matches!(card.id, CardId::WildStrike | CardId::RecklessCharge) {
                profile.exhaust_fodder += 1;
                profile.status_generators += 1;
            }
            if card.id == CardId::PowerThrough {
                profile.exhaust_fodder += 1;
                profile.block_core += 1;
                profile.status_generators += 1;
            }
            if matches!(
                card.id,
                CardId::ShrugItOff
                    | CardId::FlameBarrier
                    | CardId::Impervious
                    | CardId::GhostlyArmor
                    | CardId::Entrench
                    | CardId::BodySlam
                    | CardId::IronWave
            ) {
                profile.block_core += 1;
            }
            if matches!(card.id, CardId::Barricade | CardId::Juggernaut) {
                profile.block_core += 1;
                profile.block_payoffs += 1;
                profile.power_scalers += 1;
            }
            if matches!(
                card.id,
                CardId::Offering
                    | CardId::Bloodletting
                    | CardId::Hemokinesis
                    | CardId::Combust
                    | CardId::Brutality
                    | CardId::Rupture
            ) {
                profile.self_damage_sources += 1;
            }
            if matches!(
                card.id,
                CardId::DemonForm
                    | CardId::Inflame
                    | CardId::Panache
                    | CardId::Mayhem
                    | CardId::Magnetism
                    | CardId::Rupture
            ) {
                profile.power_scalers += 1;
            }
            if matches!(card.id, CardId::Evolve | CardId::FireBreathing) {
                profile.status_payoffs += 1;
                profile.power_scalers += 1;
            }
        }

        profile
    }

    fn synergy_adjustment(card_id: CardId, profile: &DeckProfile) -> i32 {
        let strength_level = profile.strength_enablers * 2 + profile.strength_payoffs;
        let exhaust_level = profile.exhaust_engines * 2 + profile.exhaust_outlets;
        let block_level = profile.block_core + profile.block_payoffs * 2;

        match card_id {
            CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm => {
                profile.strength_payoffs * 5
            }
            CardId::LimitBreak => {
                if profile.strength_enablers > 0 {
                    18 + profile.strength_enablers * 8 + profile.strength_payoffs * 3
                } else {
                    -28
                }
            }
            CardId::HeavyBlade => {
                if profile.strength_enablers > 0 {
                    10 + profile.strength_enablers * 6
                } else {
                    -8
                }
            }
            CardId::SwordBoomerang | CardId::TwinStrike | CardId::Pummel | CardId::Whirlwind => {
                profile.strength_enablers * 4
            }
            CardId::Reaper => {
                if profile.strength_enablers > 0 {
                    8 + profile.strength_enablers * 4
                } else {
                    0
                }
            }
            CardId::Flex => {
                if profile.strength_payoffs > 0 {
                    8 + profile.strength_payoffs * 3
                } else {
                    -6
                }
            }
            CardId::Rupture => {
                if profile.self_damage_sources > 0 {
                    10 + profile.self_damage_sources * 5
                } else {
                    -18
                }
            }
            CardId::Corruption => {
                10 + profile.exhaust_outlets * 4 + profile.exhaust_fodder * 2 + profile.block_core
            }
            CardId::FeelNoPain => {
                8 + exhaust_level * 4
                    + profile.exhaust_fodder * 2
                    + profile.draw_sources
                    + profile.block_core
            }
            CardId::DarkEmbrace => {
                10 + exhaust_level * 5 + profile.draw_sources * 2 + profile.exhaust_fodder * 2
            }
            CardId::SecondWind | CardId::FiendFire | CardId::SeverSoul | CardId::BurningPact => {
                profile.exhaust_engines * 5
                    + profile.exhaust_fodder * 3
                    + profile.draw_sources
                    + profile.status_generators * 2
            }
            CardId::TrueGrit => 4 + profile.exhaust_engines * 3 + profile.exhaust_fodder * 2,
            CardId::Exhume => {
                if profile.exhaust_outlets > 0 || profile.exhaust_engines > 0 {
                    8 + exhaust_level * 2
                } else {
                    -8
                }
            }
            CardId::BodySlam => {
                if block_level >= 3 {
                    12 + block_level * 3
                } else {
                    -12
                }
            }
            CardId::Barricade => {
                if block_level >= 3 {
                    12 + profile.block_core * 3
                } else {
                    -10
                }
            }
            CardId::Entrench => {
                if block_level >= 4 {
                    10 + block_level * 3
                } else {
                    -14
                }
            }
            CardId::Juggernaut => {
                if profile.block_core >= 3 {
                    8 + profile.block_core * 3
                } else {
                    -6
                }
            }
            CardId::ShrugItOff
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::GhostlyArmor
            | CardId::IronWave => profile.block_payoffs * 3 + profile.exhaust_engines * 2,
            CardId::SearingBlow => {
                if profile.searing_blow_count > 0 {
                    8 + profile.searing_blow_upgrades * 6 + profile.draw_sources * 2
                } else {
                    6
                }
            }
            CardId::Armaments => {
                if profile.searing_blow_count > 0 {
                    10 + profile.searing_blow_upgrades * 3
                } else {
                    0
                }
            }
            CardId::Headbutt | CardId::SeeingRed => {
                if profile.searing_blow_count > 0 {
                    8 + profile.searing_blow_upgrades * 2
                } else {
                    0
                }
            }
            CardId::DoubleTap => {
                if profile.searing_blow_count > 0 {
                    6 + profile.searing_blow_upgrades * 2
                } else {
                    0
                }
            }
            CardId::BattleTrance
            | CardId::PommelStrike
            | CardId::Finesse
            | CardId::FlashOfSteel
            | CardId::MasterOfStrategy => profile.draw_sources * 3,
            CardId::Brutality => profile.draw_sources * 2 + profile.self_damage_sources * 2,
            CardId::Evolve | CardId::FireBreathing => {
                if profile.status_generators > 0 {
                    10 + profile.status_generators * 6
                } else {
                    -10
                }
            }
            CardId::WildStrike | CardId::RecklessCharge => profile.status_payoffs * 4,
            _ if strength_level == 0 && exhaust_level == 0 && block_level == 0 => 0,
            _ => 0,
        }
    }

    fn presence_adjustment(card_id: CardId, copies: i32, profile: &DeckProfile) -> i32 {
        if copies > 0 {
            return 0;
        }

        let marginal = Self::marginal_value_profile(card_id);
        let mut bonus = marginal.presence_value;

        if marginal.role == CardRole::Payoff && !Self::payoff_ready(card_id, profile) {
            bonus /= 2;
        }

        bonus
    }

    fn marginal_duplicate_penalty(card_id: CardId, copies: i32, profile: &DeckProfile) -> i32 {
        if copies <= 0 {
            return 0;
        }

        let marginal = Self::marginal_value_profile(card_id);
        let role_base = match marginal.role {
            CardRole::Engine => 12,
            CardRole::Support => 7,
            CardRole::Payoff => 8,
            CardRole::Filler => 10,
            CardRole::Generic => 6,
        };

        let mut penalty = copies * (role_base + marginal.collision_risk);
        penalty -= marginal.reliability_need * 2;

        if marginal.role == CardRole::Payoff && !Self::payoff_ready(card_id, profile) {
            penalty += copies * 6;
        }

        penalty.max(0)
    }

    fn payoff_ready(card_id: CardId, profile: &DeckProfile) -> bool {
        match card_id {
            CardId::LimitBreak
            | CardId::HeavyBlade
            | CardId::Whirlwind
            | CardId::SwordBoomerang
            | CardId::Pummel => profile.strength_enablers > 0,
            CardId::BodySlam | CardId::Juggernaut => {
                profile.block_core >= 3 || profile.block_payoffs > 0
            }
            _ => true,
        }
    }

    fn marginal_value_profile(card_id: CardId) -> MarginalValueProfile {
        match card_id {
            CardId::Armaments => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 16,
                reliability_need: 0,
                collision_risk: 12,
            },
            CardId::Corruption => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 18,
                reliability_need: 0,
                collision_risk: 14,
            },
            CardId::FeelNoPain => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 16,
                reliability_need: 1,
                collision_risk: 10,
            },
            CardId::DarkEmbrace => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 16,
                reliability_need: 1,
                collision_risk: 10,
            },
            CardId::Barricade => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 16,
                reliability_need: 0,
                collision_risk: 14,
            },
            CardId::DemonForm => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 14,
                reliability_need: 0,
                collision_risk: 12,
            },
            CardId::Apotheosis => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 20,
                reliability_need: 0,
                collision_risk: 16,
            },
            CardId::Panacea => MarginalValueProfile {
                role: CardRole::Engine,
                presence_value: 10,
                reliability_need: 1,
                collision_risk: 8,
            },
            CardId::Offering
            | CardId::BattleTrance
            | CardId::ShrugItOff
            | CardId::PommelStrike
            | CardId::FlameBarrier
            | CardId::BurningPact
            | CardId::SecondWind
            | CardId::TrueGrit => MarginalValueProfile {
                role: CardRole::Support,
                presence_value: 6,
                reliability_need: 3,
                collision_risk: 3,
            },
            CardId::LimitBreak
            | CardId::Juggernaut
            | CardId::BodySlam
            | CardId::HeavyBlade
            | CardId::Whirlwind
            | CardId::SwordBoomerang
            | CardId::Pummel => MarginalValueProfile {
                role: CardRole::Payoff,
                presence_value: 8,
                reliability_need: 1,
                collision_risk: 6,
            },
            CardId::Strike
            | CardId::Defend
            | CardId::StrikeG
            | CardId::DefendG
            | CardId::Clash
            | CardId::WildStrike
            | CardId::Rampage
            | CardId::PerfectedStrike => MarginalValueProfile {
                role: CardRole::Filler,
                presence_value: -2,
                reliability_need: 0,
                collision_risk: 4,
            },
            _ => MarginalValueProfile {
                role: CardRole::Generic,
                presence_value: 0,
                reliability_need: 1,
                collision_risk: 2,
            },
        }
    }

    fn get_base_card_priority(card_id: CardId) -> i32 {
        match card_id {
            // --- Ironclad Top Tier ---
            CardId::Offering => 100,
            CardId::Whirlwind => 90,
            CardId::DemonForm => 85,
            CardId::LimitBreak => 80,
            CardId::Reaper => 80,
            CardId::Immolate => 80,
            CardId::Feed => 80,
            CardId::Corruption => 80,
            CardId::FeelNoPain => 75,
            CardId::DarkEmbrace => 75,

            // --- Ironclad Great Tier ---
            CardId::ShrugItOff => 70,
            CardId::TwinStrike => 65,
            CardId::PommelStrike => 65,
            CardId::Carnage => 65,
            CardId::Shockwave => 65,
            CardId::BattleTrance => 65,
            CardId::FlameBarrier => 65,
            CardId::TrueGrit => 60,
            CardId::Armaments => 60,
            CardId::Inflame => 60,
            CardId::Anger => 60,
            CardId::Uppercut => 60,
            CardId::BodySlam => 55,
            CardId::Clothesline => 55,
            CardId::HeavyBlade => 55,
            CardId::Headbutt => 55,
            CardId::Disarm => 55,

            // --- Ironclad Okay Tier ---
            CardId::Cleave => 40,
            CardId::IronWave => 40,
            CardId::PerfectedStrike => 40,
            CardId::SwordBoomerang => 40,
            CardId::BloodForBlood => 35,
            CardId::Dropkick => 35,
            CardId::ThunderClap => 30,
            CardId::Flex => 30,
            CardId::Warcry => 30,
            CardId::DoubleTap => 30,
            CardId::SeeingRed => 30,
            CardId::GhostlyArmor => 30,
            CardId::Bloodletting => 30,
            CardId::Entrench => 30,
            CardId::Juggernaut => 28,
            CardId::PowerThrough => 28,
            CardId::SecondWind => 30,
            CardId::BurningPact => 32,
            CardId::SeverSoul => 28,
            CardId::Pummel => 28,
            CardId::Rupture => 25,
            CardId::Exhume => 28,

            // --- Colorless tactical cards ---
            CardId::Apotheosis => 95,
            CardId::Panacea => 72,
            CardId::GoodInstincts => 66,
            CardId::Finesse => 62,
            CardId::FlashOfSteel => 62,
            CardId::MasterOfStrategy => 78,
            CardId::SecretTechnique => 62,
            CardId::SecretWeapon => 58,
            CardId::Discovery => 60,
            CardId::Blind => 60,
            CardId::DarkShackles => 64,
            CardId::Trip => 56,
            CardId::Neutralize => 52,
            CardId::Survivor => 56,
            CardId::DeadlyPoison => 54,
            CardId::Prepared => 44,
            CardId::DaggerThrow => 48,
            CardId::PoisonedStab => 52,
            CardId::DaggerSpray => 50,
            CardId::BladeDance => 50,
            CardId::CloakAndDagger => 48,
            CardId::Backflip => 54,
            CardId::Acrobatics => 56,
            CardId::BouncingFlask => 56,
            CardId::Footwork => 58,
            CardId::NoxiousFumes => 58,
            CardId::Catalyst => 46,
            CardId::Adrenaline => 72,
            CardId::AfterImage => 60,
            CardId::Burst => 54,

            // --- Starters & Curses ---
            CardId::Strike | CardId::StrikeG => -10,
            CardId::Defend | CardId::DefendG => -10,
            // Skip tier cards
            CardId::Clash => 10,
            CardId::WildStrike => 10,
            CardId::Havoc => 10,
            CardId::Rampage => 15,
            CardId::SearingBlow => 15,

            _ => 20, // Baseline for implemented but unranked cards
        }
    }

    fn get_card_deck_cap(card_id: CardId) -> i32 {
        match card_id {
            // High limits
            CardId::ShrugItOff => 4,
            CardId::PommelStrike => 3,
            CardId::Offering => 3,
            // Medium limits
            CardId::Whirlwind => 2,
            CardId::LimitBreak => 2,
            CardId::TwinStrike => 2,
            CardId::BattleTrance => 2,
            CardId::TrueGrit => 2,
            CardId::Inflame => 2,
            CardId::Anger => 2,
            CardId::BodySlam => 2,
            CardId::HeavyBlade => 2,
            CardId::Cleave => 2,
            CardId::IronWave => 2,
            CardId::PerfectedStrike => 3,
            CardId::SwordBoomerang => 2,
            CardId::Flex => 2,
            CardId::Finesse => 3,
            CardId::FlashOfSteel => 3,
            // Strict single copies
            CardId::DemonForm => 1,
            CardId::Reaper => 1,
            CardId::Feed => 1,
            CardId::Shockwave => 1,
            CardId::FlameBarrier => 1,
            CardId::Armaments => 1,
            CardId::Uppercut => 1,
            CardId::Clothesline => 1,
            CardId::Corruption => 1,
            CardId::FeelNoPain => 1,
            CardId::DarkEmbrace => 1,
            CardId::Disarm => 1,
            CardId::Apotheosis => 1,
            CardId::Barricade => 1,
            CardId::SearingBlow => 1,
            CardId::Juggernaut => 1,
            CardId::MasterOfStrategy => 1,
            _ => 2, // Default limit is 2 for safety against bloat
        }
    }
}
