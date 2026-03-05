//! Integration tests for individual card effects.
//! 
//! Each test:
//! 1. Loads the card definition from cards.json
//! 2. Sets up a minimal GameState scenario
//! 3. Plays the card via `play_card` (full pipeline: energy, modifiers, commands)
//! 4. Asserts the expected game state changes

pub mod ironclad;
pub mod colorless;

use crate::loader::CardLibrary;
use crate::state::GameState;
use crate::schema::CardInstance;
use crate::engine::play_card;

use once_cell::sync::Lazy;

/// Shared card library loaded once for all tests.
static CARD_LIBRARY: Lazy<CardLibrary> = Lazy::new(|| {
    CardLibrary::load("data/cards")
        .expect("Failed to load data/cards for tests")
});

/// Helper: Play a card by its definition ID.
/// 
/// # Arguments
/// * `state` - The game state to modify
/// * `card_id` - Definition ID (e.g., "Bash", "Strike_Ironclad")
/// * `upgraded` - Whether the card is upgraded
/// * `target_idx` - Optional enemy target index
/// 
/// # Returns
/// Vec of CommandResults from playing the card.
/// 
/// # Panics
/// Panics if the card is not found in the library or has unimplemented commands.
pub fn play_card_by_id(
    state: &mut GameState,
    card_id: &str,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<crate::engine::CommandResult> {
    let library = &*CARD_LIBRARY;
    let def = library.get(card_id)
        .unwrap_or_else(|_| panic!("Card '{}' not found in library", card_id));
    
    let card_instance = if upgraded {
        CardInstance::new_upgraded(card_id.to_string(), def.cost)
    } else {
        CardInstance::new(card_id.to_string(), def.cost)
    };
    
    play_card(state, library, &card_instance, target_idx)
        .unwrap_or_else(|e| panic!("Failed to play card '{}': {}", card_id, e))
}

/// Helper: Create a test state with specified player energy and enemy HP.
pub fn test_state(seed: u64, energy: i32, enemy_hp: i32) -> GameState {
    let mut state = GameState::new(seed);
    state.player.energy = energy;
    state.enemies.push(crate::enemy::MonsterState::new_simple("Test Dummy", enemy_hp));
    state
}

/// Helper: Create a test state with multiple enemies.
pub fn test_state_multi(seed: u64, energy: i32, enemy_hps: &[i32]) -> GameState {
    let mut state = GameState::new(seed);
    state.player.energy = energy;
    for (i, &hp) in enemy_hps.iter().enumerate() {
        state.enemies.push(
            crate::enemy::MonsterState::new_simple(&format!("Enemy_{}", i), hp)
        );
    }
    state
}

/// Helper: Add cards to the draw pile for testing draw effects.
pub fn add_draw_pile(state: &mut GameState, card_ids: &[&str]) {
    for &id in card_ids {
        state.draw_pile.push(CardInstance::new_basic(id, 1));
    }
}

/// Helper: Add cards to the hand for testing discard/exhaust effects.
pub fn add_hand(state: &mut GameState, card_ids: &[&str]) {
    for &id in card_ids {
        state.hand.push(CardInstance::new_basic(id, 1));
    }
}

/// Helper: Add cards to the discard pile.
pub fn add_discard(state: &mut GameState, card_ids: &[&str]) {
    for &id in card_ids {
        state.discard_pile.push(CardInstance::new_basic(id, 1));
    }
}
