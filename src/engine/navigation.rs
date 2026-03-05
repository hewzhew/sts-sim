//! Map navigation — room transitions, rewards, screen management.
//!
//! Corresponds to Java's `AbstractDungeon.setCurrMapNode` and room transition logic.

use crate::loader::CardLibrary;
use crate::schema::{CardInstance, CardColor};
use crate::state::{GameState, GamePhase};
use crate::items::relics::{GameEvent, trigger_relics, apply_relic_results};
use crate::map::RoomType;
use crate::dungeon::{get_random_encounter, advance_act, spawn_encounter};
pub use crate::dungeon::ActTransitionResult;
use crate::shop::generate_shop;
use crate::events::{EventSelector, ActiveEventState};
use super::events::build_event_pool_context;
use rand::Rng;

/// Result of proceeding to a map node.
#[derive(Debug, Clone)]
pub enum NodeResult {
    /// Successfully moved to a combat node - enemies spawned.
    Combat { encounter_id: String, is_elite: bool, is_boss: bool },
    /// Successfully moved to a shop node.
    Shop,
    /// Successfully moved to a rest site (campfire).
    Rest,
    /// Successfully moved to an event node.
    Event { event_id: String },
    /// Successfully moved to a treasure room.
    Treasure,
    /// Invalid move - not a valid child of current node.
    InvalidMove { reason: String },
    /// No map loaded.
    NoMap,
    /// Game is over.
    GameOver,
}

/// Navigate to a map node and initialize the appropriate screen.
/// 
/// This is the main "glue" function that transitions the game state
/// based on what type of room the player enters.
/// 
/// # Arguments
/// * `state` - The current game state.
/// * `node_index` - The index of the target node in the map.
/// * `card_library` - Card library for shop generation.
/// 
/// # Returns
/// A `NodeResult` indicating what happened.
pub fn proceed_to_node(
    state: &mut GameState,
    node_index: usize,
    card_library: &CardLibrary,
) -> NodeResult {
    // Check if game is over
    if state.screen == GamePhase::GameOver {
        return NodeResult::GameOver;
    }
    
    // Check if we can even move (must be in Map phase)
    if state.screen != GamePhase::Map {
        return NodeResult::InvalidMove {
            reason: format!("Cannot move while in {:?} phase", state.screen),
        };
    }
    
    // Get the map
    let map = match &state.map {
        Some(m) => m.clone(),
        None => return NodeResult::NoMap,
    };
    
    // Validate the move
    if node_index >= map.nodes.len() {
        return NodeResult::InvalidMove {
            reason: format!("Node index {} out of bounds", node_index),
        };
    }
    
    let target_node = &map.nodes[node_index];
    
    // Check if this is a valid move from current position
    let is_valid_move = match state.current_map_node {
        None => {
            // Starting the run - must be floor 0
            target_node.y == 0
        }
        Some(current_idx) => {
            // Must be a child of current node
            if current_idx >= map.nodes.len() {
                false
            } else {
                map.nodes[current_idx].children.contains(&node_index)
            }
        }
    };
    
    if !is_valid_move {
        return NodeResult::InvalidMove {
            reason: format!("Node {} is not reachable from current position", node_index),
        };
    }
    
    // Update position
    state.current_map_node = Some(node_index);
    state.floor = target_node.y as u8;
    state.floor_num += 1;
    
    // Handle the room type
    match target_node.room_type {
        RoomType::Monster | RoomType::MonsterElite | RoomType::Boss => {
            // Spawn encounter using combat_count for proper pool selection
            let encounter = get_random_encounter(
                &mut state.encounter_rng,
                state.act,
                target_node.room_type,
                state.combat_count,
            );
            
            match encounter {
                Some(enc) => {
                    state.screen = GamePhase::Combat;
                    // Note: Actual enemy spawning would require MonsterLibrary
                    // The caller should use enc.encounter_id to spawn enemies
                    NodeResult::Combat {
                        encounter_id: enc.encounter_id,
                        is_elite: enc.is_elite,
                        is_boss: enc.is_boss,
                    }
                }
                None => {
                    // No encounter found - shouldn't happen but handle gracefully
                    NodeResult::InvalidMove {
                        reason: "No encounter available for this room".to_string(),
                    }
                }
            }
        }
        
        RoomType::Shop => {
            // Generate shop inventory
            // TODO: Get player color from character state, default to Red (Ironclad)
            let player_color = CardColor::Red;
            let shop_state = generate_shop(
                player_color,
                card_library,
                None, // relic_library - optional for now
                &mut state.rng,
            );
            state.shop_state = Some(shop_state);
            state.screen = GamePhase::Shop;
            NodeResult::Shop
        }
        
        RoomType::Rest => {
            // Campfire - options will be determined by get_available_options
            state.screen = GamePhase::Rest;
            NodeResult::Rest
        }
        
        RoomType::Treasure => {
            // Treasure room - for now just transition, rewards handled separately
            state.screen = GamePhase::Reward;
            NodeResult::Treasure
        }
        
        RoomType::Event => {
            // Use EventSelector to pick a random event
            let ctx = build_event_pool_context(state);
            
            if let Some(event_id) = EventSelector::select_event(&ctx, &mut state.rng) {
                // Initialize event state
                let event_seed = state.rng.random::<u64>();
                let event_state = ActiveEventState::new(event_id, event_seed);
                state.event_state = Some(event_state);
                state.screen = GamePhase::Event;
                
                // Mark event as seen
                state.seen_events.push(event_id.to_string());
                
                NodeResult::Event {
                    event_id: event_id.to_string(),
                }
            } else {
                // Fallback: no event available, give small heal and return to map
                let heal = 5;
                state.player.current_hp = (state.player.current_hp + heal).min(state.player.max_hp);
                state.screen = GamePhase::Map;
                NodeResult::Event {
                    event_id: "fallback_heal".to_string(),
                }
            }
        }
    }
}

/// Get valid node indices the player can move to.
pub fn get_valid_moves(state: &GameState) -> Vec<usize> {
    let map = match &state.map {
        Some(m) => m,
        None => return Vec::new(),
    };
    
    match state.current_map_node {
        None => {
            // Starting positions (floor 0)
            map.get_starting_positions()
        }
        Some(current_idx) => {
            if current_idx >= map.nodes.len() {
                Vec::new()
            } else {
                map.nodes[current_idx].children.clone()
            }
        }
    }
}

/// Transition from Reward screen back to Map (after collecting rewards).
/// 
/// For boss rewards, use `finish_boss_rewards` instead to handle Act transition.
pub fn finish_rewards(state: &mut GameState) {
    if state.screen == GamePhase::Reward {
        state.rewards_pending = false;
        state.screen = GamePhase::Map;
    }
}

/// Transition from Boss Reward screen and advance to next Act.
/// 
/// This should be called after the player has collected boss rewards
/// (relics from boss chest, etc.). It will:
/// 1. Clear the reward screen
/// 2. Advance to the next Act (or trigger Victory if Act 3)
/// 3. Generate a new map for the new Act
/// 
/// # Returns
/// The result of the act transition.
pub fn finish_boss_rewards(state: &mut GameState) -> ActTransitionResult {
    if state.screen != GamePhase::Reward {
        return ActTransitionResult::CannotAdvance {
            reason: "Not on reward screen".to_string(),
        };
    }
    
    if !state.boss_defeated {
        return ActTransitionResult::CannotAdvance {
            reason: "Boss has not been defeated".to_string(),
        };
    }
    
    state.rewards_pending = false;
    
    // Advance to next act (handles map generation, healing, etc.)
    advance_act(state)
}

/// Transition from Shop back to Map (when leaving the shop).
pub fn leave_shop(state: &mut GameState) {
    if state.screen == GamePhase::Shop {
        state.shop_state = None;
        state.screen = GamePhase::Map;
    }
}

/// Transition from Rest site back to Map (after resting/smithing).
pub fn leave_rest(state: &mut GameState) {
    if state.screen == GamePhase::Rest {
        state.screen = GamePhase::Map;
    }
}

/// Handle combat victory - transition to Reward screen.
/// 
/// This function handles the post-combat state:
/// 1. Increments combat_count for encounter pool tracking
/// 2. Sets up the reward screen
/// 3. For boss fights, marks boss as defeated for Act transition
pub fn on_combat_victory(state: &mut GameState, is_boss: bool) {
    // Increment combat count for weak/strong pool tracking
    state.combat_count = state.combat_count.saturating_add(1);
    
    // Mark boss as defeated if this was a boss fight
    if is_boss {
        state.boss_defeated = true;
    }
    
    state.rewards_pending = true;
    state.screen = GamePhase::Reward;
}


/// Handle player death - transition to GameOver.
pub fn on_player_death(state: &mut GameState) {
    state.screen = GamePhase::GameOver;
}

