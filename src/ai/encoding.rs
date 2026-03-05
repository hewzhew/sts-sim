//! Observation Space Encoding for RL Training
//!
//! Encodes the GameState into a fixed-size flat `Vec<f32>` suitable for neural network input.
//! All values are normalized to approximately [0.0, 1.0] range.
//!
//! ## Encoding Schema (315 floats) - Phase 7.2 Semantic Encoding
//!
//! | Index Range | Size | Description                                    |
//! |-------------|------|------------------------------------------------|
//! | 0-19        | 20   | Global info (HP, energy, floor, screen type)   |
//! | 20-169      | 150  | Hand cards (10 slots × 15 features each)       |
//! | 170-219     | 50   | Enemies (5 slots × 10 features each)           |
//! | 220-254     | 35   | Map/meta info (paths, gold, deck stats)        |
//! | 255-314     | 60   | Reward cards (4 slots × 15 features each)      |

use crate::state::{GameState, GamePhase};
use crate::loader::CardLibrary;
use crate::ai::card_features::{CardFeatures, CARD_FEATURE_DIM, get_card_features, get_instance_features};

/// Total observation vector dimension (Phase 7.2 - Semantic Encoding)
pub const OBS_DIM: usize = 315;

// Index ranges
const GLOBAL_START: usize = 0;
const GLOBAL_SIZE: usize = 20;

const HAND_START: usize = 20;
const HAND_SLOTS: usize = 10;
// CARD_FEATURE_DIM = 15

const ENEMY_START: usize = 170; // 20 + 10*15 = 170
const ENEMY_SLOTS: usize = 5;
const ENEMY_FEATURES: usize = 10;

const META_START: usize = 220; // 170 + 5*10 = 220
const META_SIZE: usize = 35;

const REWARD_CARDS_START: usize = 255; // 220 + 35 = 255
const REWARD_CARD_SLOTS: usize = 4;
// CARD_FEATURE_DIM = 15, so 4*15 = 60

/// Encode the complete game state into a fixed-size float vector.
///
/// This version uses semantic card features for better generalization.
/// Requires a CardLibrary to look up card definitions.
///
/// # Returns
/// A `Vec<f32>` of length `OBS_DIM` (315) with all values normalized.
pub fn encode_state_with_library(state: &GameState, library: &CardLibrary) -> Vec<f32> {
    let mut obs = vec![0.0f32; OBS_DIM];
    
    encode_global_info(state, &mut obs);
    encode_hand_cards_semantic(state, library, &mut obs);
    encode_enemies(state, &mut obs);
    encode_meta_info(state, &mut obs);
    encode_reward_cards(state, library, &mut obs);
    
    obs
}

/// Encode the game state without library (fallback - uses heuristics for card types).
/// This is kept for backwards compatibility.
pub fn encode_state(state: &GameState) -> Vec<f32> {
    let mut obs = vec![0.0f32; OBS_DIM];
    
    encode_global_info(state, &mut obs);
    encode_hand_cards_heuristic(state, &mut obs);
    encode_enemies(state, &mut obs);
    encode_meta_info(state, &mut obs);
    // Reward cards left empty without library
    
    obs
}

/// Encode global game information (indices 0-19)
///
/// Layout:
/// - [0]: HP ratio (current_hp / max_hp)
/// - [1]: Energy ratio (energy / max_energy)
/// - [2]: Block normalized (block / 100)
/// - [3]: Floor normalized (floor / 50)
/// - [4]: Act normalized (act / 3)
/// - [5]: Gold normalized (gold / 1000)
/// - [6]: Turn normalized (turn / 20)
/// - [7-12]: Screen type one-hot (Map, Combat, Shop, Rest, Event, Reward)
/// - [13]: Strength normalized (strength / 20)
/// - [14]: Dexterity normalized (dex / 10)
/// - [15]: Weak status (0 or 1)
/// - [16]: Frail status (0 or 1)
/// - [17]: Vulnerable status (0 or 1)
/// - [18]: Cards played this turn (/ 10)
/// - [19]: Rewards pending flag
#[inline]
fn encode_global_info(state: &GameState, obs: &mut [f32]) {
    // Player vitals
    obs[0] = state.player.current_hp as f32 / state.player.max_hp.max(1) as f32;
    obs[1] = state.player.energy as f32 / state.player.max_energy.max(1) as f32;
    obs[2] = (state.player.block as f32 / 100.0).min(1.0);
    
    // Progress
    obs[3] = state.floor as f32 / 50.0;
    obs[4] = state.act as f32 / 3.0;
    obs[5] = (state.gold as f32 / 1000.0).min(1.0);
    obs[6] = (state.turn as f32 / 20.0).min(1.0);
    
    // Screen type one-hot encoding [7-12]
    let screen_idx = match state.screen {
        GamePhase::Map => 7,
        GamePhase::Combat => 8,
        GamePhase::Shop => 9,
        GamePhase::Rest => 10,
        GamePhase::Event | GamePhase::CardSelect => 11,
        GamePhase::Reward => 12,
        GamePhase::GameOver => 12, // Map to Reward slot for GameOver
    };
    obs[screen_idx] = 1.0;
    
    // Player buffs/debuffs
    obs[13] = (state.player.strength() as f32 / 20.0).clamp(-1.0, 1.0);
    obs[14] = (state.player.dexterity() as f32 / 10.0).clamp(-1.0, 1.0);
    obs[15] = if state.player.is_weak() { 1.0 } else { 0.0 };
    obs[16] = if state.player.has_status("Frail") { 1.0 } else { 0.0 };
    obs[17] = if state.player.has_status("Vulnerable") { 1.0 } else { 0.0 };
    obs[18] = (state.cards_played_this_turn as f32 / 10.0).min(1.0);
    obs[19] = if state.rewards_pending { 1.0 } else { 0.0 };
}

/// Encode hand cards using semantic features (indices 20-169, 10 slots × 15 features)
///
/// Per-slot layout (15 features from CardFeatures):
/// - [0]: Present (1.0 if card exists, 0.0 if empty)
/// - [1]: Cost normalized (cost / 3.0)
/// - [2]: Is Attack (0/1)
/// - [3]: Is Skill (0/1)
/// - [4]: Is Power (0/1)
/// - [5]: Is Status/Curse (0/1)
/// - [6]: Base Damage (normalized / 50)
/// - [7]: Base Block (normalized / 50)
/// - [8]: Magic Number (normalized / 10)
/// - [9]: Is Upgraded (0/1)
/// - [10]: Has Exhaust (0/1)
/// - [11]: Has Ethereal (0/1)
/// - [12]: Targets All Enemies (0/1)
/// - [13]: Rarity (0.1-0.9 normalized)
/// - [14]: Is Playable (enough energy)
#[inline]
fn encode_hand_cards_semantic(state: &GameState, library: &CardLibrary, obs: &mut [f32]) {
    for (slot, card) in state.hand.iter().enumerate().take(HAND_SLOTS) {
        let base = HAND_START + slot * CARD_FEATURE_DIM;
        
        let features = get_instance_features(
            library,
            &card.definition_id,
            card.upgraded,
            card.current_cost,
            state.player.energy,
        );
        
        features.write_to_slice(obs, base);
    }
    // Empty slots remain 0.0 (default)
}

/// Encode hand cards using heuristics (fallback when no library available)
#[inline]
fn encode_hand_cards_heuristic(state: &GameState, obs: &mut [f32]) {
    for (slot, card) in state.hand.iter().enumerate().take(HAND_SLOTS) {
        let base = HAND_START + slot * CARD_FEATURE_DIM;
        
        // Present flag
        obs[base] = 1.0;
        
        // Cost normalized
        obs[base + 1] = (card.current_cost as f32 / 3.0).min(1.0);
        
        // Type heuristics based on card ID patterns
        let id = &card.definition_id;
        let is_attack = id.contains("Strike") || id.contains("Bash") || 
                       id.contains("Pommel") || id.contains("Cleave") ||
                       id.contains("Sword") || id.contains("Heavy") ||
                       id.contains("Reckless") || id.contains("Twin") ||
                       id.contains("Headbutt") || id.contains("Iron") ||
                       id.contains("Clothesline") || id.contains("Anger") ||
                       id.contains("Rampage") || id.contains("Uppercut");
        let is_skill = id.contains("Defend") || id.contains("Shrug") ||
                      id.contains("Armaments") || id.contains("True") ||
                      id.contains("Flex") || id.contains("Havoc") ||
                      id.contains("Impervious") || id.contains("Battle") ||
                      id.contains("Rage") || id.contains("Second");
        let is_power = id.contains("Demon") || id.contains("Inflame") ||
                      id.contains("Limit") || id.contains("Metallicize") ||
                      id.contains("Barricade") || id.contains("Berserk") ||
                      id.contains("Juggernaut") || id.contains("Combust");
        let is_curse = id.contains("Curse") || id.contains("Wound") ||
                      id.contains("Dazed") || id.contains("Burn") ||
                      id.contains("Slimed") || id.contains("Parasite") ||
                      id.contains("Normality") || id.contains("Pain");
        
        obs[base + 2] = if is_attack { 1.0 } else { 0.0 };
        obs[base + 3] = if is_skill { 1.0 } else { 0.0 };
        obs[base + 4] = if is_power { 1.0 } else { 0.0 };
        obs[base + 5] = if is_curse { 1.0 } else { 0.0 };
        
        // Estimate damage/block from common card patterns
        let (est_dmg, est_blk) = estimate_card_values(id);
        obs[base + 6] = (est_dmg as f32 / 50.0).min(1.0);
        obs[base + 7] = (est_blk as f32 / 50.0).min(1.0);
        
        // obs[base + 8] magic number - unknown without library
        obs[base + 9] = if card.upgraded { 1.0 } else { 0.0 };
        // obs[base + 10..12] - keywords unknown without library
        // obs[base + 13] - rarity unknown
        
        // Playable flag (enough energy)
        obs[base + 14] = if card.current_cost <= state.player.energy { 1.0 } else { 0.0 };
    }
}

/// Estimate damage/block values for common cards (heuristic fallback)
#[inline]
fn estimate_card_values(id: &str) -> (i32, i32) {
    // Common starter/basic cards
    if id.contains("Strike") { return (6, 0); }
    if id.contains("Defend") { return (0, 5); }
    if id == "Bash" { return (8, 0); }
    
    // Ironclad attacks
    if id.contains("Anger") { return (6, 0); }
    if id.contains("Cleave") { return (8, 0); }
    if id.contains("Clothesline") { return (12, 0); }
    if id.contains("Headbutt") { return (9, 0); }
    if id.contains("Heavy") { return (14, 0); }
    if id.contains("Iron Wave") { return (5, 5); }
    if id.contains("Perfected") { return (6, 0); }
    if id.contains("Pommel") { return (9, 0); }
    if id.contains("Sword") { return (12, 0); }
    if id.contains("Twin") { return (5, 0); } // x2
    if id.contains("Uppercut") { return (13, 0); }
    
    // Ironclad skills
    if id.contains("Shrug") { return (0, 8); }
    if id.contains("True Grit") { return (0, 7); }
    if id.contains("Impervious") { return (0, 30); }
    if id.contains("Entrench") { return (0, 0); } // doubles block
    
    // Default - unknown card
    (0, 0)
}

/// Encode enemy information (indices 170-219, 5 slots × 10 features)
///
/// Per-slot layout (10 features):
/// - [0]: Present (1.0 if enemy exists and alive)
/// - [1]: HP ratio (hp / max_hp)
/// - [2]: Block normalized (block / 100)
/// - [3-7]: Intent type one-hot (Attack, Defend, Buff, Debuff, Special/Unknown)
/// - [8]: Intent damage normalized (damage / 50)
/// - [9]: Vulnerable stacks normalized (vuln / 5)
#[inline]
fn encode_enemies(state: &GameState, obs: &mut [f32]) {
    for (slot, enemy) in state.enemies.iter().enumerate().take(ENEMY_SLOTS) {
        let base = ENEMY_START + slot * ENEMY_FEATURES;
        
        if enemy.is_dead() {
            // Dead enemy: all zeros (already default)
            continue;
        }
        
        // Present flag
        obs[base] = 1.0;
        
        // HP ratio
        obs[base + 1] = enemy.hp as f32 / enemy.max_hp.max(1) as f32;
        
        // Block normalized
        obs[base + 2] = (enemy.block as f32 / 100.0).min(1.0);
        
        // Intent encoding
        let (intent_type_id, intent_damage) = enemy.get_intent_info();
        
        // Intent type one-hot [3-7]: Attack, Defend, Buff, Debuff, Special
        let intent_idx = match intent_type_id {
            1 => 3,  // Attack
            2 => 4,  // Defend
            3 => 5,  // Buff
            4 => 6,  // Debuff
            _ => 7,  // Special/Unknown (0 or 5+)
        };
        obs[base + intent_idx] = 1.0;
        
        // Intent damage normalized
        obs[base + 8] = (intent_damage / 50.0).min(1.0);
        
        // Vulnerable stacks normalized
        obs[base + 9] = (enemy.get_buff("Vulnerable") as f32 / 5.0).min(1.0);
    }
}

/// Encode map and meta information (indices 220-254)
///
/// Layout:
/// - [220-222]: Valid path choices (up to 3 available paths)
/// - [223]: Has relic slots (relics.len() < 10)
/// - [224]: Draw pile size normalized (/ 30)
/// - [225]: Discard pile size normalized (/ 30)
/// - [226]: Exhaust pile size normalized (/ 10)
/// - [227]: Total deck size normalized (/ 50)
/// - [228]: Attack cards ratio (estimated)
/// - [229]: Skill cards ratio (estimated)
/// - [230-232]: Reserved
/// - [233]: Current map node index normalized (/ 50)
/// - [234]: Rewards pending flag
/// - [235-239]: Hand size indicator (1-5 cards present flags)
/// - [240]: Relic count (/ 20)
/// - [241-254]: Reserved
#[inline]
fn encode_meta_info(state: &GameState, obs: &mut [f32]) {
    // Valid path choices for map navigation
    if state.screen == GamePhase::Map {
        if let Some(ref _map) = state.map {
            let valid_moves = crate::engine::get_valid_moves(state);
            for (i, &_node_idx) in valid_moves.iter().enumerate().take(3) {
                obs[META_START + i] = 1.0;
            }
        }
    }
    
    // Has relic room (simplified: can still collect relics)
    obs[META_START + 3] = if state.relics.len() < 10 { 1.0 } else { 0.0 };
    
    // Pile sizes
    obs[META_START + 4] = (state.draw_pile.len() as f32 / 30.0).min(1.0);
    obs[META_START + 5] = (state.discard_pile.len() as f32 / 30.0).min(1.0);
    obs[META_START + 6] = (state.exhaust_pile.len() as f32 / 10.0).min(1.0);
    
    // Total deck size (all piles + hand)
    let total_deck = state.draw_pile.len() + state.discard_pile.len() 
                   + state.exhaust_pile.len() + state.hand.len();
    obs[META_START + 7] = (total_deck as f32 / 50.0).min(1.0);
    
    // Deck composition ratios (estimated by ID patterns)
    let (attack_count, skill_count, total) = estimate_deck_composition(state);
    if total > 0 {
        obs[META_START + 8] = attack_count as f32 / total as f32;
        obs[META_START + 9] = skill_count as f32 / total as f32;
    }
    // obs[META_START + 10..12] reserved
    
    // Map node
    obs[META_START + 13] = state.current_map_node.unwrap_or(0) as f32 / 50.0;
    
    // Rewards pending
    obs[META_START + 14] = if state.rewards_pending { 1.0 } else { 0.0 };
    
    // Hand size indicators (how many cards in hand, up to 10)
    let hand_size = state.hand.len().min(10);
    for i in 0..hand_size.min(5) {  // Only 5 indicator slots available
        obs[META_START + 15 + i] = 1.0;
    }
    
    // Relic count
    obs[META_START + 20] = (state.relics.len() as f32 / 20.0).min(1.0);
    
    // obs[META_START + 21..34] reserved
}

/// Encode reward card choices (indices 255-314, 4 slots × 15 features)
///
/// This encodes the card choices available when the player receives card rewards.
/// Uses the same semantic encoding as hand cards.
#[inline]
fn encode_reward_cards(state: &GameState, library: &CardLibrary, obs: &mut [f32]) {
    use crate::rewards::RewardType;
    
    // Find card rewards in pending rewards
    for reward in &state.current_rewards {
        if let RewardType::Card { cards } = reward {
            // Encode up to REWARD_CARD_SLOTS card choices
            for (slot, card_choice) in cards.iter().enumerate().take(REWARD_CARD_SLOTS) {
                let base = REWARD_CARDS_START + slot * CARD_FEATURE_DIM;
                
                let features = get_card_features(
                    library,
                    &card_choice.id,
                    false, // Reward cards are not upgraded by default
                );
                
                features.write_to_slice(obs, base);
            }
            break; // Only encode the first card reward
        }
    }
    // Empty slots remain 0.0 (default)
}

/// Estimate deck composition by card ID patterns (since we don't have card_type).
#[inline]
fn estimate_deck_composition(state: &GameState) -> (usize, usize, usize) {
    let mut attacks = 0;
    let mut skills = 0;
    
    // Check ID patterns for classification
    let check_card = |id: &str| -> (bool, bool) {
        let is_attack = id.contains("Strike") || id.contains("Bash") || 
                       id.contains("Pommel") || id.contains("Cleave") ||
                       id.contains("Sword") || id.contains("Heavy") ||
                       id.contains("Reckless") || id.contains("Twin") ||
                       id.contains("Headbutt") || id.contains("Iron") ||
                       id.contains("Perfected") || id.contains("Clothesline") ||
                       id.contains("Anger") || id.contains("Rampage");
        let is_skill = id.contains("Defend") || id.contains("Shrug") ||
                      id.contains("Armaments") || id.contains("True") ||
                      id.contains("Flex") || id.contains("Havoc") ||
                      id.contains("Impervious") || id.contains("Ghostly") ||
                      id.contains("Battle") || id.contains("Rage");
        (is_attack, is_skill)
    };
    
    // Count from all piles
    for card in state.draw_pile.iter()
        .chain(state.discard_pile.iter())
        .chain(state.hand.iter())
    {
        let (is_atk, is_skl) = check_card(&card.definition_id);
        if is_atk { attacks += 1; }
        if is_skl { skills += 1; }
    }
    
    let total = state.draw_pile.len() + state.discard_pile.len() + state.hand.len();
    (attacks, skills, total)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_state_dimension() {
        let state = GameState::new(42);
        let obs = encode_state(&state);
        assert_eq!(obs.len(), OBS_DIM);
    }
    
    #[test]
    fn test_encode_state_normalized() {
        let state = GameState::new(42);
        let obs = encode_state(&state);
        
        // All values should be in reasonable range
        for (i, &val) in obs.iter().enumerate() {
            assert!(val >= -1.0 && val <= 1.5, 
                "obs[{}] = {} out of range [-1, 1.5]", i, val);
        }
    }
    
    #[test]
    fn test_screen_type_onehot() {
        let mut state = GameState::new(42);
        
        // Map screen
        state.screen = GamePhase::Map;
        let obs = encode_state(&state);
        assert_eq!(obs[7], 1.0, "Map should be at index 7");
        assert_eq!(obs[8], 0.0, "Combat should be 0");
        
        // Combat screen
        state.screen = GamePhase::Combat;
        let obs = encode_state(&state);
        assert_eq!(obs[7], 0.0, "Map should be 0");
        assert_eq!(obs[8], 1.0, "Combat should be at index 8");
    }
    
    #[test]
    fn test_obs_dim_calculation() {
        // Verify OBS_DIM matches our layout
        let expected = GLOBAL_SIZE                      // 20
            + HAND_SLOTS * CARD_FEATURE_DIM             // 10 * 15 = 150
            + ENEMY_SLOTS * ENEMY_FEATURES             // 5 * 10 = 50
            + META_SIZE                                 // 35
            + REWARD_CARD_SLOTS * CARD_FEATURE_DIM;    // 4 * 15 = 60
        assert_eq!(expected, 315);
        assert_eq!(OBS_DIM, 315);
    }
    
    #[test]
    fn test_index_ranges_non_overlapping() {
        // Verify index ranges don't overlap
        assert_eq!(GLOBAL_START, 0);
        assert_eq!(HAND_START, GLOBAL_START + GLOBAL_SIZE); // 20
        assert_eq!(ENEMY_START, HAND_START + HAND_SLOTS * CARD_FEATURE_DIM); // 170
        assert_eq!(META_START, ENEMY_START + ENEMY_SLOTS * ENEMY_FEATURES); // 220
        assert_eq!(REWARD_CARDS_START, META_START + META_SIZE); // 255
        assert_eq!(REWARD_CARDS_START + REWARD_CARD_SLOTS * CARD_FEATURE_DIM, OBS_DIM); // 315
    }
    
    #[test]
    fn test_encode_with_library() {
        if let Ok(library) = CardLibrary::load("data/cards") {
            let state = GameState::new(42);
            let obs = encode_state_with_library(&state, &library);
            assert_eq!(obs.len(), OBS_DIM);
        }
    }
}
