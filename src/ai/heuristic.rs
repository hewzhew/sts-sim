//! Heuristic Evaluation of Game State
//!
//! Provides a scoring function for evaluating non-terminal combat states.

use crate::core::state::GameState;

pub struct HeuristicEvaluator;

impl HeuristicEvaluator {
    /// Evaluates the current state and returns a normalized score.
    /// Higher is better for the player.
    pub fn score(state: &GameState) -> f32 {
        let mut score = 0.0;
        
        // 1. Value of Life
        let p_hp = state.player.current_hp.max(0) as f32;
        let p_max = state.player.max_hp.max(1) as f32;
        score += (p_hp / p_max) * 100.0;
        
        // 2. Value of Enemies
        for enemy in &state.enemies {
            if !enemy.is_dead() {
                score -= enemy.hp as f32 * 0.5;
                
                let str = enemy.powers.get("Strength");
                if str > 0 { score -= str as f32 * 2.0; }
                let vuln = enemy.powers.get("Vulnerable");
                if vuln > 0 { score += 5.0; }
                let weak = enemy.powers.get("Weak");
                if weak > 0 { score += 5.0; }
            }
        }
        
        // 3. Player Block
        score += state.player.block as f32 * 0.5;
        
        // 4. Player Powers
        let p_str = state.player.powers.get("Strength");
        if p_str > 0 { score += p_str as f32 * 3.0; }
        
        let p_dex = state.player.powers.get("Dexterity");
        if p_dex > 0 { score += p_dex as f32 * 3.0; }
        
        let p_vuln = state.player.powers.get("Vulnerable");
        if p_vuln > 0 { score -= 10.0; }
        
        let p_weak = state.player.powers.get("Weak");
        if p_weak > 0 { score -= 5.0; }
        
        // 5. Game Over Checks
        if state.player.current_hp <= 0 {
            return -10000.0; // Infinite penalty for dying
        }
        
        let all_dead = state.enemies.iter().all(|e| e.is_dead());
        if all_dead {
            score += 1000.0;
        }
        
        score
    }
}
