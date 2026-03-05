//! MCTS Combat Search
//! 
//! Rollout-based combat action evaluation for Slay the Spire.
//! For each legal combat action, we clone GameState, apply the action,
//! then do random rollouts to estimate the HP outcome.
//!
//! Uses "quotient set" dedup: identical cards are grouped, only one
//! representative evaluated per group.

use crate::core::state::{GameState, GamePhase};
use crate::engine;
use crate::core::loader::{CardLibrary, MonsterLibrary};

/// Result of evaluating a single combat action via MCTS rollouts.
#[derive(Debug, Clone)]
pub struct ActionEvaluation {
    pub action_id: i32,
    pub avg_score: f32,
    pub avg_turns: f32,
    pub survival_rate: f32,
    pub n_rollouts: usize,
}

/// Result of MCTS evaluation for all valid actions in a combat state.
#[derive(Debug, Clone)]
pub struct MctsResult {
    pub actions: Vec<ActionEvaluation>,
    pub best_action: i32,
    pub total_sims: usize,
}

/// Get valid combat action IDs from a GameState.
/// Action Space Encoding:
/// 10: End Turn
/// 0..9: Play card from hand with NO specific target (Self, AllEnemies, Random)
/// 100..149: Play card from hand with SPECIFIC target (100 + hand_idx * 5 + enemy_idx)
fn get_valid_combat_actions(state: &GameState, library: &CardLibrary) -> Vec<i32> {
    let mut actions = Vec::new();
    
    // End turn (10) — always valid in combat
    actions.push(10);
    
    let has_living_enemies = state.enemies.iter().any(|e| !e.is_dead());
    if !has_living_enemies {
        return actions; // Only end turn if no enemies alive
    }

    // Cards (0-9 or 100-149): playable if in hand, affordable, correct type
    for (i, card) in state.hand.iter().enumerate().take(10) {
        // Status and Curse cards are never playable here
        if card.card_type == crate::core::schema::CardType::Status ||
           card.card_type == crate::core::schema::CardType::Curse {
            continue;
        }
        // Must have enough energy
        if card.current_cost < 0 || card.current_cost > state.player.energy {
            continue;
        }
        
        // Check target type
        if let Ok(def) = library.get(&card.definition_id) {
            if def.logic.target_type == crate::core::schema::TargetType::Enemy {
                // Must explicitly target a living enemy
                for (enemy_idx, enemy) in state.enemies.iter().enumerate().take(5) {
                    if !enemy.is_dead() {
                        actions.push(100 + (i as i32 * 5) + enemy_idx as i32);
                    }
                }
            } else {
                // No specific target needed
                actions.push(i as i32);
            }
        } else {
            // Fallback if library lookup fails: assume self/no target
            actions.push(i as i32);
        }
    }
    
    // Potions (11-14): valid if slot has a potion
    for i in 0..state.potions.capacity().min(4) {
        if let Ok(Some(_)) = state.potions.get(i) {
            // Potions currently don't enumerate targets in our simple space
            actions.push(11 + i as i32);
        }
    }
    
    actions
}

/// Execute one combat action on a state.
/// Returns true if combat ended (won or lost).
fn execute_combat_action(
    state: &mut GameState,
    action_id: i32,
    card_library: &CardLibrary,
    monster_library: &MonsterLibrary,
) -> bool {
    match action_id {
        // End turn (10)
        10 => {
            state.end_turn();
            engine::execute_enemy_turn(state, monster_library);
            if state.player.current_hp <= 0 {
                state.screen = GamePhase::GameOver;
                return true;
            }
            // Start next player turn
            engine::on_turn_start(state, card_library, None);
            engine::on_turn_start_post_draw(state, card_library);
            false
        }
        // Play card WITHOUT specific target (0-9)
        0..=9 => {
            let hand_index = action_id as usize;
            if hand_index >= state.hand.len() {
                return false;
            }
            // Engine defaults to first alive if TargetType::Enemy is passed None, 
            // but we shouldn't hit this path for TargetType::Enemy now.
            let _ = engine::play_card_from_hand(state, card_library, hand_index, None);
            
            engine::all_enemies_dead(state)
        }
        // Play card WITH specific target (100-149)
        100..=149 => {
            let hand_index = ((action_id - 100) / 5) as usize;
            let target_index = ((action_id - 100) % 5) as usize;
            
            if hand_index >= state.hand.len() || target_index >= state.enemies.len() {
                return false;
            }
            
            let _ = engine::play_card_from_hand(state, card_library, hand_index, Some(target_index));
            
            engine::all_enemies_dead(state)
        }
        // Skip potions in MCTS rollouts
        _ => false,
    }
}

/// Random rollout from current combat state until depth reached.
/// Returns (final_score, turns_taken).
fn random_combat_rollout(
    state: &mut GameState,
    card_library: &CardLibrary,
    monster_library: &MonsterLibrary,
    max_turns: u32,
) -> (f32, u32) {
    use rand::Rng;
    let start_turn = state.turn;
    
    for _ in 0..max_turns {
        if state.player.current_hp <= 0 {
            return (-10000.0, state.turn.saturating_sub(start_turn));
        }
        if engine::all_enemies_dead(state) || state.screen != GamePhase::Combat {
            return (10000.0, state.turn.saturating_sub(start_turn));
        }
        
        let actions = get_valid_combat_actions(state, card_library);
        if actions.is_empty() {
            break;
        }
        
        let card_actions: Vec<i32> = actions.iter().copied().filter(|&a| a < 10 || a >= 100).collect();
        let action = if !card_actions.is_empty() {
            let mut weights = Vec::with_capacity(card_actions.len());
            let mut total_weight = 0u64;
            
            for &a in &card_actions {
                let hand_index = if a < 10 { a as usize } else { ((a - 100) / 5) as usize };
                let card = &state.hand[hand_index];
                
                // Heuristic Playout Priority:
                // Powers are extremely good to play early.
                // Attacks are universally good for ending the fight.
                // Skills are situational (often block), give them lower weight unless we refine this later.
                let w = match card.card_type {
                    crate::core::schema::CardType::Power => 100,
                    crate::core::schema::CardType::Attack => 40,
                    crate::core::schema::CardType::Skill => 10,
                    _ => 1,
                };
                weights.push(w);
                total_weight += w;
            }
            
            if total_weight > 0 {
                let mut val = state.rng.random_range(0..total_weight);
                let mut chosen = card_actions[0];
                for (i, &w) in weights.iter().enumerate() {
                    if val < w {
                        chosen = card_actions[i];
                        break;
                    }
                    val -= w;
                }
                chosen
            } else {
                card_actions[0]
            }
        } else {
            10 // End turn
        };
        
        let combat_over = execute_combat_action(state, action, card_library, monster_library);
        if combat_over {
            break;
        }
    }
    
    let score = crate::ai::heuristic::HeuristicEvaluator::score(state);
    (score, state.turn.saturating_sub(start_turn))
}

/// Evaluate all valid combat actions via MCTS rollouts.
/// 
/// Uses "quotient set" optimization: identical cards (same definition_id,
/// cost, upgraded) are grouped — only one representative is evaluated,
/// then results are shared across all duplicates.
pub fn mcts_evaluate_combat(
    state: &GameState,
    card_library: &CardLibrary,
    monster_library: &MonsterLibrary,
    n_rollouts: usize,
    max_rollout_turns: u32,
) -> MctsResult {
    let valid_actions = get_valid_combat_actions(state, card_library);
    
    // ── Deduplication: group card actions by identity ──
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<i32>> = HashMap::new();
    
    for &action_id in &valid_actions {
        let key = if action_id >= 0 && action_id <= 9 {
            let card = &state.hand[action_id as usize];
            format!("card:{}:{}:{}::TARGET_NONE", card.definition_id, card.current_cost, card.upgraded)
        } else if action_id >= 100 && action_id <= 149 {
            let hand_index = ((action_id - 100) / 5) as usize;
            let target_index = ((action_id - 100) % 5) as usize;
            let card = &state.hand[hand_index];
            format!("card:{}:{}:{}::TARGET_{}", card.definition_id, card.current_cost, card.upgraded, target_index)
        } else {
            format!("action:{}", action_id)
        };
        groups.entry(key).or_default().push(action_id);
    }
    
    let unique_count = groups.len();
    let rollouts_per_unique = (n_rollouts / unique_count.max(1)).max(1);
    let mut evaluations = Vec::with_capacity(valid_actions.len());
    let mut total_sims = 0;
    
    for (_key, action_ids) in &groups {
        let representative = action_ids[0];
        let mut score_sum = 0.0f32;
        let mut turns_sum = 0.0f32;
        let mut survived = 0usize;
        
        for r in 0..rollouts_per_unique {
            let mut sim_state = state.clone();
            use rand::SeedableRng;
            sim_state.rng = rand_xoshiro::Xoshiro256StarStar::seed_from_u64(
                state.run_seed.wrapping_add(representative as u64 * 1000 + r as u64)
            );
            
            let combat_over = execute_combat_action(
                &mut sim_state, representative, card_library, monster_library
            );
            
            if combat_over || sim_state.screen != GamePhase::Combat {
                let score = crate::ai::heuristic::HeuristicEvaluator::score(&sim_state);
                score_sum += score;
                if sim_state.player.current_hp > 0 { survived += 1; }
            } else {
                let (final_score, turns) = random_combat_rollout(
                    &mut sim_state, card_library, monster_library, max_rollout_turns
                );
                score_sum += final_score;
                turns_sum += turns as f32;
                if final_score > -9000.0 { survived += 1; }
            }
            
            total_sims += 1;
        }
        
        let n = rollouts_per_unique as f32;
        let eval = ActionEvaluation {
            action_id: representative,
            avg_score: score_sum / n,
            avg_turns: turns_sum / n,
            survival_rate: survived as f32 / n,
            n_rollouts: rollouts_per_unique,
        };
        
        for &aid in action_ids {
            evaluations.push(ActionEvaluation {
                action_id: aid,
                ..eval.clone()
            });
        }
    }
    
    evaluations.sort_by(|a, b| b.avg_score.partial_cmp(&a.avg_score).unwrap_or(std::cmp::Ordering::Equal));
    let best_action = evaluations.first().map(|e| e.action_id).unwrap_or(10);
    
    MctsResult {
        actions: evaluations,
        best_action,
        total_sims,
    }
}
