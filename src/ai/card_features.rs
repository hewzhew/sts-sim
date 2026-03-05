//! Card Feature Extraction for RL Observation Space
//!
//! This module provides semantic card embedding by extracting meaningful
//! features from card definitions. This allows the AI to understand cards
//! by their properties (damage, block, cost, effects) rather than just IDs.

use crate::loader::CardLibrary;
use crate::schema::{CardDefinition, CardCommand, CardType, CardRarity, TargetType};

/// Feature vector dimension for a single card
pub const CARD_FEATURE_DIM: usize = 15;

/// Extracted features from a card definition for RL observation.
#[derive(Debug, Clone, Default)]
pub struct CardFeatures {
    /// Whether a card is present (1.0) or slot is empty (0.0)
    pub presence: f32,
    /// Normalized cost (cost / 3.0, clamped to 1.0)
    pub cost_normalized: f32,
    /// Card type flags
    pub is_attack: f32,
    pub is_skill: f32,
    pub is_power: f32,
    pub is_status_curse: f32,
    /// Damage and block values (normalized)
    pub base_damage: f32,      // damage / 50.0
    pub base_block: f32,       // block / 50.0
    pub magic_number: f32,     // magic / 10.0
    /// Card properties
    pub is_upgraded: f32,
    pub has_exhaust: f32,
    pub has_ethereal: f32,
    pub targets_all: f32,
    /// Rarity (Common=0.25, Uncommon=0.5, Rare=0.75, Basic=0.1)
    pub rarity: f32,
    /// Is card playable (enough energy)
    pub is_playable: f32,
}

impl CardFeatures {
    /// Create an empty feature vector (for empty slots)
    pub fn empty() -> Self {
        Self::default()
    }
    
    /// Convert to a fixed-size float array
    pub fn to_array(&self) -> [f32; CARD_FEATURE_DIM] {
        [
            self.presence,           // 0
            self.cost_normalized,    // 1
            self.is_attack,          // 2
            self.is_skill,           // 3
            self.is_power,           // 4
            self.is_status_curse,    // 5
            self.base_damage,        // 6
            self.base_block,         // 7
            self.magic_number,       // 8
            self.is_upgraded,        // 9
            self.has_exhaust,        // 10
            self.has_ethereal,       // 11
            self.targets_all,        // 12
            self.rarity,             // 13
            self.is_playable,        // 14
        ]
    }
    
    /// Write features to a slice at the given offset
    pub fn write_to_slice(&self, slice: &mut [f32], offset: usize) {
        let arr = self.to_array();
        slice[offset..offset + CARD_FEATURE_DIM].copy_from_slice(&arr);
    }
}

/// Extract features from a CardDefinition.
/// 
/// This function parses the card's logic commands to extract:
/// - Base damage (from DealDamage/DealDamageAll commands)
/// - Base block (from GainBlock commands)
/// - Magic number (status stacks, draw count, etc.)
/// - Keywords (Exhaust, Ethereal)
/// - Target type (single vs all enemies)
pub fn extract_card_features(def: &CardDefinition, upgraded: bool) -> CardFeatures {
    let mut features = CardFeatures {
        presence: 1.0,
        ..Default::default()
    };
    
    // Cost (X-cost cards have cost -1, treat as 0 for normalization)
    let cost = def.cost.max(0);
    features.cost_normalized = (cost as f32 / 3.0).min(1.0);
    
    // Card type
    match def.card_type {
        CardType::Attack => features.is_attack = 1.0,
        CardType::Skill => features.is_skill = 1.0,
        CardType::Power => features.is_power = 1.0,
        CardType::Status | CardType::Curse => features.is_status_curse = 1.0,
    }
    
    // Rarity
    features.rarity = match def.rarity {
        Some(CardRarity::Basic) => 0.1,
        Some(CardRarity::Common) => 0.25,
        Some(CardRarity::Uncommon) => 0.5,
        Some(CardRarity::Rare) => 0.75,
        Some(CardRarity::Special) => 0.9,
        Some(CardRarity::Curse) => 0.0, // Curse rarity
        None => 0.25, // Default to common
    };
    
    // Target type
    features.targets_all = match def.logic.target_type {
        TargetType::AllEnemies => 1.0,
        _ => 0.0,
    };
    
    // Upgraded flag
    features.is_upgraded = if upgraded { 1.0 } else { 0.0 };
    
    // Parse commands for damage, block, magic number, and keywords
    let mut total_damage = 0i32;
    let mut total_block = 0i32;
    let mut magic_number = 0i32;
    
    for parsed_cmd in &def.logic.commands {
        if let Some(cmd) = parsed_cmd.as_known() {
            match cmd {
                CardCommand::DealDamage { base, upgrade, times, .. } => {
                    let dmg = if upgraded { *upgrade } else { *base };
                    let hits = times.unwrap_or(1);
                    total_damage += dmg * hits;
                    features.targets_all = 0.0; // Single target
                }
                CardCommand::DealDamageAll { base, upgrade, times } => {
                    let dmg = if upgraded { *upgrade } else { *base };
                    let hits = times.unwrap_or(1);
                    total_damage += dmg * hits;
                    features.targets_all = 1.0; // All enemies
                }
                CardCommand::GainBlock { base, upgrade } => {
                    total_block += if upgraded { *upgrade } else { *base };
                }
                CardCommand::ApplyStatus { base, upgrade, .. } |
                CardCommand::ApplyStatusAll { base, upgrade, .. } => {
                    magic_number += if upgraded { *upgrade } else { *base };
                }
                CardCommand::DrawCards { base, upgrade } => {
                    magic_number += if upgraded { *upgrade } else { *base };
                }
                CardCommand::GainEnergy { base, upgrade } => {
                    magic_number += if upgraded { *upgrade } else { *base };
                }
                CardCommand::GainBuff { base, upgrade, .. } => {
                    magic_number += if upgraded { *upgrade } else { *base };
                }
                CardCommand::ExhaustSelf { base_only, upgrade_only } => {
                    // Check if exhaust applies to this upgrade state
                    if (!*base_only || !upgraded) && (!*upgrade_only || upgraded) {
                        features.has_exhaust = 1.0;
                    }
                }
                CardCommand::Ethereal { base_only, upgrade_only } => {
                    if (!*base_only || !upgraded) && (!*upgrade_only || upgraded) {
                        features.has_ethereal = 1.0;
                    }
                }
                _ => {}
            }
        }
    }
    
    // Normalize values
    features.base_damage = (total_damage as f32 / 50.0).min(1.0);
    features.base_block = (total_block as f32 / 50.0).min(1.0);
    features.magic_number = (magic_number as f32 / 10.0).min(1.0);
    
    features
}

/// Extract features for a card by ID from the library.
/// Returns empty features if card not found.
pub fn get_card_features(
    library: &CardLibrary,
    card_id: &str,
    upgraded: bool,
) -> CardFeatures {
    match library.get(card_id) {
        Ok(def) => extract_card_features(def, upgraded),
        Err(_) => CardFeatures::empty(),
    }
}

/// Extract features for a card instance (uses current upgrade state).
pub fn get_instance_features(
    library: &CardLibrary,
    card_id: &str,
    upgraded: bool,
    current_cost: i32,
    player_energy: i32,
) -> CardFeatures {
    let mut features = get_card_features(library, card_id, upgraded);
    
    // Override cost with current cost (may be modified by effects)
    features.cost_normalized = (current_cost.max(0) as f32 / 3.0).min(1.0);
    
    // Set playability based on energy
    features.is_playable = if current_cost <= player_energy { 1.0 } else { 0.0 };
    
    features
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_features() {
        let features = CardFeatures::empty();
        let arr = features.to_array();
        assert_eq!(arr[0], 0.0); // presence
        assert_eq!(arr.len(), CARD_FEATURE_DIM);
    }
    
    #[test]
    fn test_feature_array_length() {
        let features = CardFeatures {
            presence: 1.0,
            cost_normalized: 0.33,
            is_attack: 1.0,
            base_damage: 0.24,
            ..Default::default()
        };
        let arr = features.to_array();
        assert_eq!(arr.len(), CARD_FEATURE_DIM);
        assert_eq!(arr[0], 1.0);
        assert_eq!(arr[2], 1.0); // is_attack
    }
    
    #[test]
    fn test_load_and_extract_features() {
        // This test requires the cards.json file
        if let Ok(library) = CardLibrary::load("data/cards") {
            // Test Strike
            let strike = get_card_features(&library, "Strike_Ironclad", false);
            assert_eq!(strike.presence, 1.0);
            assert_eq!(strike.is_attack, 1.0);
            assert!(strike.base_damage > 0.0, "Strike should have damage");
            
            // Test Defend
            let defend = get_card_features(&library, "Defend_Ironclad", false);
            assert_eq!(defend.is_skill, 1.0);
            assert!(defend.base_block > 0.0, "Defend should have block");
            
            // Test Bash (upgraded vs not)
            let bash = get_card_features(&library, "Bash", false);
            let bash_plus = get_card_features(&library, "Bash", true);
            assert_eq!(bash.is_attack, 1.0);
            assert!(bash_plus.base_damage >= bash.base_damage, "Upgraded should have >= damage");
        }
    }
}
