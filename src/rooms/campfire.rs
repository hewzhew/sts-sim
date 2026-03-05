//! Campfire (Rest Site) system for Slay the Spire simulator.
//!
//! This module implements the rest site mechanics:
//! - Rest: Heal 30% of max HP
//! - Smith: Upgrade a card in the deck
//! - Lift: Gain 1 Strength (requires Girya relic)
//! - Toke: Remove a card from deck (requires Peace Pipe relic)
//! - Dig: Obtain a relic (requires Shovel relic)
//! - Recall: Obtain the Ruby Key (The Heart route)

use serde::{Deserialize, Serialize};

use crate::loader::CardLibrary;
use crate::state::{CampfireRelicState, GameState};

// ============================================================================
// Campfire Options
// ============================================================================

/// Available actions at a campfire/rest site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampfireOption {
    /// Rest: Heal 30% of max HP (rounded down).
    Rest,
    /// Smith: Upgrade a card in the deck.
    Smith,
    /// Lift: Gain 1 Strength (requires Girya relic, max 3 uses).
    Lift,
    /// Toke: Remove a card from deck (requires Peace Pipe relic).
    Toke,
    /// Dig: Obtain a relic (requires Shovel relic, one-time use).
    Dig,
    /// Recall: Obtain the Ruby Key (only at Act 3 rest sites with key available).
    Recall,
}

impl CampfireOption {
    /// Get a human-readable name for this option.
    pub fn name(&self) -> &'static str {
        match self {
            CampfireOption::Rest => "Rest",
            CampfireOption::Smith => "Smith",
            CampfireOption::Lift => "Lift",
            CampfireOption::Toke => "Toke",
            CampfireOption::Dig => "Dig",
            CampfireOption::Recall => "Recall",
        }
    }
    
    /// Get a description of what this option does.
    pub fn description(&self) -> &'static str {
        match self {
            CampfireOption::Rest => "Heal for 30% of your max HP.",
            CampfireOption::Smith => "Upgrade a card.",
            CampfireOption::Lift => "Gain 1 Strength. (Girya)",
            CampfireOption::Toke => "Remove a card from your deck. (Peace Pipe)",
            CampfireOption::Dig => "Obtain a random relic. (Shovel)",
            CampfireOption::Recall => "Obtain the Ruby Key.",
        }
    }
}

// ============================================================================
// Campfire Result
// ============================================================================

/// Result of executing a campfire action.
#[derive(Debug, Clone)]
pub enum CampfireResult {
    /// Action completed successfully.
    Success,
    /// Healed by a specific amount.
    Healed { amount: i32 },
    /// Card upgraded successfully.
    Upgraded { card_name: String },
    /// Gained a buff.
    GainedBuff { buff: String, amount: i32 },
    /// Removed a card.
    RemovedCard { card_name: String },
    /// Obtained a relic.
    ObtainedRelic { relic_id: String },
    /// Action not available (missing relic, already used, etc.).
    NotAvailable { reason: String },
    /// Invalid card index for Smith/Toke.
    InvalidCardIndex,
    /// Card cannot be upgraded (already upgraded or not upgradeable).
    CannotUpgrade { reason: String },
}

// ============================================================================
// Campfire Logic
// ============================================================================

/// Healing constants.
pub struct RestHealing;

impl RestHealing {
    /// Base heal percentage (30% of max HP).
    pub const BASE_PERCENT: f32 = 0.30;
    
    /// Calculate heal amount for base resting.
    pub fn base_heal(max_hp: i32) -> i32 {
        ((max_hp as f32) * Self::BASE_PERCENT).floor() as i32
    }
    
    /// Calculate heal amount with Regal Pillow relic (+15 HP when resting).
    pub fn with_regal_pillow(max_hp: i32) -> i32 {
        Self::base_heal(max_hp) + 15
    }
}

/// Get the available campfire options for the current game state.
///
/// # Arguments
/// * `state` - The current game state.
///
/// # Returns
/// A vector of available `CampfireOption`s.
///
/// # Base Options
/// - `Rest` is always available.
/// - `Smith` is always available (but requires a valid card to upgrade).
///
/// # Relic-Based Options
/// - `Lift` requires Girya relic and < 3 uses.
/// - `Toke` requires Peace Pipe relic.
/// - `Dig` requires Shovel relic and not yet used.
/// - `Recall` requires Act 3 and Ruby Key not yet obtained (future feature).
pub fn get_available_options(state: &GameState) -> Vec<CampfireOption> {
    let mut options = Vec::with_capacity(4);
    
    // Rest is always available
    options.push(CampfireOption::Rest);
    
    // Smith is available if there are upgradeable cards
    // (We check this at action time, but include it in options always)
    options.push(CampfireOption::Smith);
    
    // Check for relic-based options
    let has_girya = state.relics.iter().any(|r| r.id == "Girya");
    let has_peace_pipe = state.relics.iter().any(|r| r.id == "PeacePipe");
    let has_shovel = state.relics.iter().any(|r| r.id == "Shovel");
    
    // Lift: Girya allows gaining Strength at rest sites (max 3 times per run)
    if has_girya && state.campfire_state.girya_uses < 3 {
        options.push(CampfireOption::Lift);
    }
    
    // Toke: Peace Pipe allows removing a card at rest sites
    if has_peace_pipe {
        options.push(CampfireOption::Toke);
    }
    
    // Dig: Shovel allows digging for a relic (one-time use)
    if has_shovel && !state.campfire_state.shovel_used {
        options.push(CampfireOption::Dig);
    }
    
    // Recall: For Heart route (Act 3 only, Ruby Key not obtained)
    // TODO: Implement Heart route tracking
    // if state.act == 3 && !state.ruby_key_obtained {
    //     options.push(CampfireOption::Recall);
    // }
    
    options
}

/// Check if a card can be upgraded.
pub fn can_upgrade_card(state: &GameState, card_index: usize, library: Option<&CardLibrary>) -> bool {
    if card_index >= state.draw_pile.len() {
        return false;
    }
    
    let card = &state.draw_pile[card_index];
    
    // Already upgraded
    if card.upgraded {
        return false;
    }
    
    // Check if the card definition allows upgrading
    if let Some(lib) = library {
        if let Ok(def) = lib.get(&card.definition_id) {
            // Status and Curse cards typically can't be upgraded
            // (Some special ones can, but we'll simplify for now)
            use crate::schema::CardType;
            if matches!(def.card_type, CardType::Status | CardType::Curse) {
                return false;
            }
        }
    }
    
    true
}

/// Get indices of all upgradeable cards in the deck.
pub fn get_upgradeable_cards(state: &GameState, library: Option<&CardLibrary>) -> Vec<usize> {
    (0..state.draw_pile.len())
        .filter(|&i| can_upgrade_card(state, i, library))
        .collect()
}

/// Execute a campfire action.
///
/// # Arguments
/// * `state` - The current game state (mutated).
/// * `option` - The campfire option to execute.
/// * `target_index` - For Smith/Toke, the index of the card to upgrade/remove.
/// * `library` - Optional card library for validation.
///
/// # Returns
/// A `CampfireResult` indicating success or failure.
pub fn execute_option(
    state: &mut GameState,
    option: CampfireOption,
    target_index: Option<usize>,
    library: Option<&CardLibrary>,
) -> CampfireResult {
    match option {
        CampfireOption::Rest => execute_rest(state),
        CampfireOption::Smith => execute_smith(state, target_index, library),
        CampfireOption::Lift => execute_lift(state),
        CampfireOption::Toke => execute_toke(state, target_index),
        CampfireOption::Dig => execute_dig(state),
        CampfireOption::Recall => execute_recall(state),
    }
}

/// Execute the Rest action: Heal 30% of max HP.
fn execute_rest(state: &mut GameState) -> CampfireResult {
    let has_regal_pillow = state.relics.iter().any(|r| r.id == "RegalPillow");
    
    let heal_amount = if has_regal_pillow {
        RestHealing::with_regal_pillow(state.player.max_hp)
    } else {
        RestHealing::base_heal(state.player.max_hp)
    };
    
    // Apply healing (cap at max HP)
    let old_hp = state.player.current_hp;
    state.player.current_hp = (state.player.current_hp + heal_amount).min(state.player.max_hp);
    let actual_heal = state.player.current_hp - old_hp;
    
    // Check for Dream Catcher relic: Add a card to deck when resting
    let has_dream_catcher = state.relics.iter().any(|r| r.id == "DreamCatcher");
    if has_dream_catcher {
        // TODO: Trigger card reward selection
        // For now, we just note that this should happen
    }
    
    // TODO: Trigger PlayerRest relic events
    
    CampfireResult::Healed { amount: actual_heal }
}

/// Execute the Smith action: Upgrade a card.
fn execute_smith(
    state: &mut GameState,
    target_index: Option<usize>,
    library: Option<&CardLibrary>,
) -> CampfireResult {
    let Some(index) = target_index else {
        return CampfireResult::InvalidCardIndex;
    };
    
    if index >= state.draw_pile.len() {
        return CampfireResult::InvalidCardIndex;
    }
    
    if !can_upgrade_card(state, index, library) {
        return CampfireResult::CannotUpgrade {
            reason: "Card is already upgraded or cannot be upgraded".to_string(),
        };
    }
    
    // Upgrade the card
    let card = &mut state.draw_pile[index];
    card.upgraded = true;
    let card_name = card.definition_id.clone();
    
    // Apply upgrade effects from card library
    // Note: In StS, upgraded cards may have different costs defined in the card definition.
    // For now, we just mark the card as upgraded - the cost difference is handled
    // when the card is played via the card definition's upgrade values.
    
    CampfireResult::Upgraded { card_name }
}

/// Execute the Lift action: Gain 1 Strength (Girya).
fn execute_lift(state: &mut GameState) -> CampfireResult {
    let has_girya = state.relics.iter().any(|r| r.id == "Girya");
    
    if !has_girya {
        return CampfireResult::NotAvailable {
            reason: "Girya relic not owned".to_string(),
        };
    }
    
    if state.campfire_state.girya_uses >= 3 {
        return CampfireResult::NotAvailable {
            reason: "Girya has been used 3 times already".to_string(),
        };
    }
    
    // Gain 1 permanent Strength
    state.player.apply_status("Strength", 1);
    state.campfire_state.girya_uses += 1;
    
    // Update Girya counter on the relic instance
    if let Some(girya) = state.relics.iter_mut().find(|r| r.id == "Girya") {
        girya.counter = state.campfire_state.girya_uses as i32;
    }
    
    CampfireResult::GainedBuff {
        buff: "Strength".to_string(),
        amount: 1,
    }
}

/// Execute the Toke action: Remove a card (Peace Pipe).
fn execute_toke(state: &mut GameState, target_index: Option<usize>) -> CampfireResult {
    let has_peace_pipe = state.relics.iter().any(|r| r.id == "PeacePipe");
    
    if !has_peace_pipe {
        return CampfireResult::NotAvailable {
            reason: "Peace Pipe relic not owned".to_string(),
        };
    }
    
    let Some(index) = target_index else {
        return CampfireResult::InvalidCardIndex;
    };
    
    if index >= state.draw_pile.len() {
        return CampfireResult::InvalidCardIndex;
    }
    
    // Remove the card
    let card = state.draw_pile.remove(index);
    let card_name = card.definition_id;
    
    CampfireResult::RemovedCard { card_name }
}

/// Execute the Dig action: Obtain a relic (Shovel).
fn execute_dig(state: &mut GameState) -> CampfireResult {
    let has_shovel = state.relics.iter().any(|r| r.id == "Shovel");
    
    if !has_shovel {
        return CampfireResult::NotAvailable {
            reason: "Shovel relic not owned".to_string(),
        };
    }
    
    if state.campfire_state.shovel_used {
        return CampfireResult::NotAvailable {
            reason: "Shovel has already been used this run".to_string(),
        };
    }
    
    // Mark shovel as used
    state.campfire_state.shovel_used = true;
    
    // TODO: Generate a random relic from the relic library
    // For now, return a placeholder
    let relic_id = "RandomRelic".to_string();
    
    // Add a placeholder relic
    use crate::items::relics::RelicInstance;
    state.relics.push(RelicInstance::new(&relic_id));
    crate::items::relics::on_relic_equip(state, &relic_id);
    
    CampfireResult::ObtainedRelic { relic_id }
}

/// Execute the Recall action: Obtain the Ruby Key.
fn execute_recall(state: &mut GameState) -> CampfireResult {
    // TODO: Implement Heart route tracking
    // For now, this is a placeholder
    if state.act != 3 {
        return CampfireResult::NotAvailable {
            reason: "Recall is only available in Act 3".to_string(),
        };
    }
    
    // TODO: Mark Ruby Key as obtained
    // state.ruby_key_obtained = true;
    
    CampfireResult::Success
}

// ============================================================================
// GameState Integration
// ============================================================================

impl GameState {
    /// Enter a campfire/rest site.
    pub fn enter_campfire(&mut self) {
        // TODO: Trigger EnterRest relic events
        // For example, Eternal Feather heals when entering rest sites
    }
    
    /// Execute a campfire option.
    ///
    /// # Arguments
    /// * `option` - The action to take.
    /// * `target_index` - For Smith/Toke, the card index.
    /// * `library` - Optional card library.
    pub fn execute_campfire_option(
        &mut self,
        option: CampfireOption,
        target_index: Option<usize>,
        library: Option<&CardLibrary>,
    ) -> CampfireResult {
        execute_option(self, option, target_index, library)
    }
    
    /// Get available campfire options.
    pub fn get_campfire_options(&self) -> Vec<CampfireOption> {
        get_available_options(self)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::CardInstance;
    
    #[test]
    fn test_rest_healing_calculation() {
        // 30% of 80 HP = 24
        assert_eq!(RestHealing::base_heal(80), 24);
        // 30% of 100 HP = 30
        assert_eq!(RestHealing::base_heal(100), 30);
        // With Regal Pillow: 24 + 15 = 39
        assert_eq!(RestHealing::with_regal_pillow(80), 39);
    }
    
    #[test]
    fn test_rest_action() {
        let mut state = GameState::new(42);
        state.player.max_hp = 80;
        state.player.current_hp = 50;
        
        let result = execute_rest(&mut state);
        
        // Should heal 24 HP (30% of 80)
        if let CampfireResult::Healed { amount } = result {
            assert_eq!(amount, 24);
        } else {
            panic!("Expected Healed result");
        }
        
        assert_eq!(state.player.current_hp, 74);
    }
    
    #[test]
    fn test_rest_capped_at_max_hp() {
        let mut state = GameState::new(42);
        state.player.max_hp = 80;
        state.player.current_hp = 70; // Only 10 HP missing
        
        let result = execute_rest(&mut state);
        
        // Would heal 24, but capped at 10 (to reach max)
        if let CampfireResult::Healed { amount } = result {
            assert_eq!(amount, 10);
        } else {
            panic!("Expected Healed result");
        }
        
        assert_eq!(state.player.current_hp, 80);
    }
    
    #[test]
    fn test_smith_upgrade() {
        let mut state = GameState::new(42);
        state.draw_pile.push(CardInstance::new_basic("Strike", 1));
        
        let result = execute_smith(&mut state, Some(0), None);
        
        assert!(matches!(result, CampfireResult::Upgraded { .. }));
        assert!(state.draw_pile[0].upgraded);
    }
    
    #[test]
    fn test_smith_already_upgraded() {
        let mut state = GameState::new(42);
        let mut card = CardInstance::new_basic("Strike", 1);
        card.upgraded = true;
        state.draw_pile.push(card);
        
        let result = execute_smith(&mut state, Some(0), None);
        
        assert!(matches!(result, CampfireResult::CannotUpgrade { .. }));
    }
    
    #[test]
    fn test_lift_requires_girya() {
        let mut state = GameState::new(42);
        
        // Without Girya
        let result = execute_lift(&mut state);
        assert!(matches!(result, CampfireResult::NotAvailable { .. }));
        
        // With Girya
        use crate::items::relics::RelicInstance;
        state.relics.push(RelicInstance::new("Girya"));
        
        let result = execute_lift(&mut state);
        assert!(matches!(result, CampfireResult::GainedBuff { .. }));
        assert_eq!(state.player.strength(), 1);
        assert_eq!(state.campfire_state.girya_uses, 1);
    }
    
    #[test]
    fn test_lift_max_uses() {
        let mut state = GameState::new(42);
        use crate::items::relics::RelicInstance;
        state.relics.push(RelicInstance::new("Girya"));
        
        // Use Lift 3 times
        for _ in 0..3 {
            let result = execute_lift(&mut state);
            assert!(matches!(result, CampfireResult::GainedBuff { .. }));
        }
        
        // 4th time should fail
        let result = execute_lift(&mut state);
        assert!(matches!(result, CampfireResult::NotAvailable { .. }));
        assert_eq!(state.player.strength(), 3);
    }
    
    #[test]
    fn test_available_options_basic() {
        let state = GameState::new(42);
        let options = get_available_options(&state);
        
        // Should always have Rest and Smith
        assert!(options.contains(&CampfireOption::Rest));
        assert!(options.contains(&CampfireOption::Smith));
        
        // Should NOT have relic-based options without relics
        assert!(!options.contains(&CampfireOption::Lift));
        assert!(!options.contains(&CampfireOption::Toke));
        assert!(!options.contains(&CampfireOption::Dig));
    }
    
    #[test]
    fn test_available_options_with_relics() {
        let mut state = GameState::new(42);
        use crate::items::relics::RelicInstance;
        
        state.relics.push(RelicInstance::new("Girya"));
        state.relics.push(RelicInstance::new("PeacePipe"));
        state.relics.push(RelicInstance::new("Shovel"));
        
        let options = get_available_options(&state);
        
        assert!(options.contains(&CampfireOption::Rest));
        assert!(options.contains(&CampfireOption::Smith));
        assert!(options.contains(&CampfireOption::Lift));
        assert!(options.contains(&CampfireOption::Toke));
        assert!(options.contains(&CampfireOption::Dig));
    }
    
    #[test]
    fn test_toke_removes_card() {
        let mut state = GameState::new(42);
        use crate::items::relics::RelicInstance;
        state.relics.push(RelicInstance::new("PeacePipe"));
        
        state.draw_pile.push(CardInstance::new_basic("Strike", 1));
        state.draw_pile.push(CardInstance::new_basic("Defend", 1));
        
        let result = execute_toke(&mut state, Some(0));
        
        assert!(matches!(result, CampfireResult::RemovedCard { .. }));
        assert_eq!(state.draw_pile.len(), 1);
        assert_eq!(state.draw_pile[0].definition_id, "Defend");
    }
}
