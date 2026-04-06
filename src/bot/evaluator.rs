use crate::combat::{CombatState, Intent};
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
    score -= combat_state.turn_count as f32 * 500.0;
    
    // Player Health is the most precious resource
    score += combat_state.player.current_hp as f32 * 100.0;
    
    // Score player block (but incoming intent damage will subtract it back)
    score += combat_state.player.block as f32 * 5.0;

    let mut total_monster_expected_damage = 0;
    
    for m in &combat_state.monsters {
        if m.is_dying || m.is_escaped {
            // Massive bonus for a dead monster
            score += 15000.0;
            continue;
        }
        
        // Massive penalty to compel the AI to deal damage
        score -= m.current_hp as f32 * 50.0;

        // Calculate expected damage purely from attacks
        match m.current_intent {
            Intent::Attack { hits, .. } |
            Intent::AttackBuff { hits, .. } |
            Intent::AttackDebuff { hits, .. } |
            Intent::AttackDefend { hits, .. } => {
                total_monster_expected_damage += m.intent_dmg * (hits as i32);
            },
            _ => {}
        }
    }
    
    // Heavily penalize unblocked incoming damage to prioritize mitigation
    // We penalize this HIGHER than the value of HP, to encourage active blocking over tanking.
    // However, since DFS evaluates the start of the *next* turn, we assume the player will 
    // natively generate ~10 block from basic energy usage, softening the blow of large intents.
    let assumed_future_block = 10;
    let expected_net_damage = (total_monster_expected_damage - combat_state.player.block).max(0);
    let unblocked_damage = (expected_net_damage - assumed_future_block).max(0);
    score -= unblocked_damage as f32 * 120.0; // soften multiplier slightly
    
    // Add positive scoring for player powers/buffs
    if let Some(powers) = combat_state.power_db.get(&combat_state.player.id) {
        for p in powers {
            // Very naive evaluation: +150 per stack of generic buff to encourage setup plays
            if !crate::content::powers::is_debuff(p.power_type, p.amount) {
                score += p.amount as f32 * 150.0;
            }
        }
    }
    
    // Add positive scoring for debuffs on enemies
    for m in &combat_state.monsters {
        if let Some(powers) = combat_state.power_db.get(&m.id) {
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

// ─── Card Evaluator ──────────────────────────────────────────────────────────

use crate::content::cards::CardId;
use crate::state::run::RunState;

pub struct CardEvaluator;

impl CardEvaluator {
    /// Score a card purely based on its static tier and how many we already have in the deck.
    /// Returns a heuristic score. If the score is too low, the agent should Skip.
    pub fn evaluate_card(card_id: CardId, run_state: &RunState) -> i32 {
        let base_score = Self::get_base_card_priority(card_id);
        
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
        
        // Mild penalty for each duplicate even below cap to encourage diversity
        base_score - (copies * 5)
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

            // --- Starters & Curses ---
            CardId::Strike => -10,
            CardId::Defend => -10,
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
            _ => 2, // Default limit is 2 for safety against bloat
        }
    }
}
