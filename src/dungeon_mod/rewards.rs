//! Reward generation system for Slay the Spire simulator.
//!
//! This module implements the game's Pseudo-RNG reward system with:
//! - **Potion drops**: Dynamic 40% base chance, +/-10% per combat
//! - **Card rarity pity timer**: Rare chance increases with each Common rolled
//! - **Gold rewards**: Room-type-specific ranges
//! - **Relic drops**: Tier-weighted distribution
//!
//! ## Probability Tables (from global.txt)
//!
//! ### Card Rarity (Normal Rooms)
//! | Rarity   | Base % | With Offset |
//! |----------|--------|-------------|
//! | Rare     | 3%     | 3% + offset |
//! | Uncommon | 37%    | 37%         |
//! | Common   | 60%    | Remainder   |
//!
//! ### Card Rarity (Elite Rooms)
//! | Rarity   | % |
//! |----------|---|
//! | Rare     | 10% |
//! | Uncommon | 40% |
//! | Common   | 50% |
//!
//! ### Relic Tier (not from chests)
//! | Tier     | % |
//! |----------|---|
//! | Rare     | 17% |
//! | Uncommon | 33% |
//! | Common   | 50% |
//!
//! ### Potion Rarity
//! | Rarity   | % |
//! |----------|---|
//! | Rare     | 10% |
//! | Uncommon | 25% |
//! | Common   | 65% |

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::loader::CardLibrary;
use crate::map::RoomType;
use crate::schema::{CardColor, CardRarity};
use crate::state::GameState;

// ============================================================================
// Reward Types
// ============================================================================

/// A single reward from combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    /// Gold reward
    Gold { amount: i32 },
    /// Potion reward (placeholder ID for now)
    Potion { id: String, rarity: PotionRarity },
    /// Relic reward
    Relic { id: String, tier: RelicTier },
    /// Card selection (3-card draft)
    Card { cards: Vec<CardReward> },
}

/// A card in the reward selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardReward {
    pub id: String,
    pub rarity: CardRarity,
    pub upgraded: bool,
}

/// Potion rarity for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionRarity {
    Common,
    Uncommon,
    Rare,
}

/// Relic tier for reward generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelicTier {
    Common,
    Uncommon,
    Rare,
    Boss,
    Shop,
}

/// Collection of rewards from a combat.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BattleRewards {
    pub rewards: Vec<RewardType>,
}

impl BattleRewards {
    pub fn new() -> Self {
        Self { rewards: Vec::new() }
    }
    
    pub fn add(&mut self, reward: RewardType) {
        self.rewards.push(reward);
    }
    
    /// Get total gold in rewards.
    pub fn total_gold(&self) -> i32 {
        self.rewards.iter().filter_map(|r| {
            if let RewardType::Gold { amount } = r {
                Some(*amount)
            } else {
                None
            }
        }).sum()
    }
}

// ============================================================================
// Rarity Weights Configuration
// ============================================================================

/// Card rarity weights for different room types.
#[derive(Debug, Clone, Copy)]
pub struct CardRarityWeights {
    pub rare: i32,
    pub uncommon: i32,
    pub common: i32,
}

impl CardRarityWeights {
    /// Get weights for normal encounters (with pity timer offset).
    pub fn normal(rare_offset: i32) -> Self {
        // Base: Rare 3%, Uncommon 37%, Common 60%
        // Offset is applied to rare, common absorbs the difference
        let rare = (3 + rare_offset).max(0);
        let uncommon = 37;
        let common = 100 - rare - uncommon;
        Self { rare, uncommon, common }
    }
    
    /// Get weights for elite encounters (fixed rates).
    pub fn elite() -> Self {
        // Elite: Rare 10%, Uncommon 40%, Common 50%
        Self { rare: 10, uncommon: 40, common: 50 }
    }
    
    /// Get weights for shop (increased rare chance).
    pub fn shop() -> Self {
        // Shop: Rare 9%, Uncommon 37%, Common 54%
        Self { rare: 9, uncommon: 37, common: 54 }
    }
    
    /// Roll a rarity based on weights.
    pub fn roll(&self, rng: &mut impl Rng) -> CardRarity {
        let roll: i32 = rng.random_range(1..=100);
        
        if roll <= self.rare {
            CardRarity::Rare
        } else if roll <= self.rare + self.uncommon {
            CardRarity::Uncommon
        } else {
            CardRarity::Common
        }
    }
}

/// Relic tier weights (not from chests).
#[derive(Debug, Clone, Copy)]
pub struct RelicTierWeights {
    pub rare: i32,    // 17%
    pub uncommon: i32, // 33%
    pub common: i32,   // 50%
}

impl Default for RelicTierWeights {
    fn default() -> Self {
        Self { rare: 17, uncommon: 33, common: 50 }
    }
}

impl RelicTierWeights {
    /// Roll a relic tier based on weights.
    pub fn roll(&self, rng: &mut impl Rng) -> RelicTier {
        let roll: i32 = rng.random_range(1..=100);
        
        if roll <= self.rare {
            RelicTier::Rare
        } else if roll <= self.rare + self.uncommon {
            RelicTier::Uncommon
        } else {
            RelicTier::Common
        }
    }
}

/// Potion rarity weights.
#[derive(Debug, Clone, Copy)]
pub struct PotionRarityWeights {
    pub rare: i32,     // 10%
    pub uncommon: i32, // 25%
    pub common: i32,   // 65%
}

impl Default for PotionRarityWeights {
    fn default() -> Self {
        Self { rare: 10, uncommon: 25, common: 65 }
    }
}

impl PotionRarityWeights {
    /// Roll a potion rarity.
    pub fn roll(&self, rng: &mut impl Rng) -> PotionRarity {
        let roll: i32 = rng.random_range(1..=100);
        
        if roll <= self.rare {
            PotionRarity::Rare
        } else if roll <= self.rare + self.uncommon {
            PotionRarity::Uncommon
        } else {
            PotionRarity::Common
        }
    }
}

// ============================================================================
// Gold Ranges
// ============================================================================

/// Gold drop ranges by room type.
pub struct GoldRange;

impl GoldRange {
    /// Normal encounter: 10-20 gold.
    pub fn normal(rng: &mut impl Rng) -> i32 {
        rng.random_range(10..=20)
    }
    
    /// Elite encounter: 25-35 gold.
    pub fn elite(rng: &mut impl Rng) -> i32 {
        rng.random_range(25..=35)
    }
    
    /// Boss encounter: 95-105 gold.
    pub fn boss(rng: &mut impl Rng) -> i32 {
        rng.random_range(95..=105)
    }
    
    /// Get gold for a room type.
    pub fn for_room(room_type: RoomType, rng: &mut impl Rng) -> i32 {
        match room_type {
            RoomType::Monster => Self::normal(rng),
            RoomType::MonsterElite => Self::elite(rng),
            RoomType::Boss => Self::boss(rng),
            _ => 0, // Non-combat rooms don't drop gold this way
        }
    }
}

// ============================================================================
// Main Reward Generation
// ============================================================================

/// Generate rewards for completing a combat.
///
/// This function implements the game's Pseudo-RNG system:
/// - Potion drops use a dynamic 40% base that adjusts +/-10%
/// - Card rarity uses a "pity timer" that increases rare chance
/// - Gold is rolled from room-type-specific ranges
/// - Elites/Bosses guarantee a relic
pub fn generate_rewards(
    state: &mut GameState,
    room_type: RoomType,
    card_library: Option<&CardLibrary>,
    player_class: Option<CardColor>,
) -> BattleRewards {
    let mut rewards = BattleRewards::new();
    
    // 1. Gold (always)
    let gold = GoldRange::for_room(room_type, &mut state.rng);
    if gold > 0 {
        rewards.add(RewardType::Gold { amount: gold });
    }
    
    // 2. Potion (dynamic probability)
    if let Some(potion) = try_generate_potion(state) {
        rewards.add(potion);
    }
    
    // 3. Cards (with pity timer for rarity)
    let card_reward = generate_card_reward(state, room_type, card_library, player_class);
    rewards.add(card_reward);
    
    // 4. Relic (guaranteed for Elite/Boss)
    if matches!(room_type, RoomType::MonsterElite | RoomType::Boss) {
        let relic = generate_relic(state, room_type);
        rewards.add(relic);
    }
    
    // Mark that rewards are pending
    state.rewards_pending = true;
    
    rewards
}

/// Try to generate a potion using dynamic drop chance.
///
/// Base chance is 40%, modified by +/-10% based on previous drops.
fn try_generate_potion(state: &mut GameState) -> Option<RewardType> {
    let roll: i32 = state.rng.random_range(1..=100);
    
    if roll <= state.potion_drop_chance {
        // Potion dropped - decrease chance by 10%
        state.potion_drop_chance = (state.potion_drop_chance - 10).max(0);
        
        // Roll potion rarity
        let weights = PotionRarityWeights::default();
        let rarity = weights.roll(&mut state.rng);
        
        // Generate placeholder potion (we don't have PotionLibrary yet)
        let id = match rarity {
            PotionRarity::Common => "Potion_Common_Placeholder",
            PotionRarity::Uncommon => "Potion_Uncommon_Placeholder",
            PotionRarity::Rare => "Potion_Rare_Placeholder",
        }.to_string();
        
        Some(RewardType::Potion { id, rarity })
    } else {
        // No potion - increase chance by 10%
        state.potion_drop_chance = (state.potion_drop_chance + 10).min(100);
        None
    }
}

/// Generate a card reward with 3 cards using the pity timer system.
fn generate_card_reward(
    state: &mut GameState,
    room_type: RoomType,
    card_library: Option<&CardLibrary>,
    player_class: Option<CardColor>,
) -> RewardType {
    let mut cards = Vec::with_capacity(3);
    
    // Determine if we use elite rates or normal rates
    let is_elite = matches!(room_type, RoomType::MonsterElite);
    
    for _ in 0..3 {
        // Get weights (with pity timer for normal rooms)
        let weights = if is_elite {
            CardRarityWeights::elite()
        } else {
            CardRarityWeights::normal(state.rare_card_offset)
        };
        
        // Roll rarity
        let rarity = weights.roll(&mut state.rng);
        
        // Update pity timer (only for normal encounters)
        if !is_elite {
            match rarity {
                CardRarity::Common => {
                    // Each common rolled increases rare chance by 1%
                    state.rare_card_offset += 1;
                }
                CardRarity::Rare => {
                    // Rare rolled resets the offset to -5
                    state.rare_card_offset = -5;
                }
                _ => {} // Uncommon doesn't affect the offset
            }
        }
        
        // Try to get a card from the library, or use placeholder
        let card_id = if let Some(library) = card_library {
            get_random_card_of_rarity(library, rarity, player_class, &mut state.rng)
                .unwrap_or_else(|| format!("Card_{:?}_Placeholder", rarity))
        } else {
            format!("Card_{:?}_Placeholder", rarity)
        };
        
        cards.push(CardReward {
            id: card_id,
            rarity,
            upgraded: false, // May be upgraded below by Eggs
        });
    }
    
    // Egg relics: auto-upgrade cards in the reward preview (Java: onPreviewObtainCard)
    // FrozenEgg2 → Power, MoltenEgg2 → Attack, ToxicEgg2 → Skill
    if let Some(lib) = card_library {
        let has_frozen = state.relics.iter().any(|r| r.id == "FrozenEgg" || r.id == "Frozen Egg 2");
        let has_molten = state.relics.iter().any(|r| r.id == "MoltenEgg" || r.id == "Molten Egg 2");
        let has_toxic = state.relics.iter().any(|r| r.id == "ToxicEgg" || r.id == "Toxic Egg 2");
        
        if has_frozen || has_molten || has_toxic {
            for card in cards.iter_mut() {
                if card.upgraded { continue; }
                if let Ok(def) = lib.get(&card.id) {
                    let should_upgrade = match def.card_type {
                        crate::schema::CardType::Power => has_frozen,
                        crate::schema::CardType::Attack => has_molten,
                        crate::schema::CardType::Skill => has_toxic,
                        _ => false,
                    };
                    if should_upgrade {
                        card.upgraded = true;
                    }
                }
            }
        }
    }
    
    RewardType::Card { cards }
}

/// Generate a relic reward.
fn generate_relic(state: &mut GameState, room_type: RoomType) -> RewardType {
    let tier = if room_type == RoomType::Boss {
        // Boss always drops Boss relic
        RelicTier::Boss
    } else {
        // Elite uses standard tier weights
        RelicTierWeights::default().roll(&mut state.rng)
    };
    
    // Generate placeholder relic ID (we don't have full relic selection yet)
    let id = match tier {
        RelicTier::Common => "Relic_Common_Placeholder",
        RelicTier::Uncommon => "Relic_Uncommon_Placeholder",
        RelicTier::Rare => "Relic_Rare_Placeholder",
        RelicTier::Boss => "Relic_Boss_Placeholder",
        RelicTier::Shop => "Relic_Shop_Placeholder",
    }.to_string();
    
    RewardType::Relic { id, tier }
}

/// Get a random card of a specific rarity from the library.
fn get_random_card_of_rarity(
    library: &CardLibrary,
    rarity: CardRarity,
    player_class: Option<CardColor>,
    rng: &mut impl Rng,
) -> Option<String> {
    // Get cards of the requested rarity
    let mut candidates: Vec<&str> = library
        .cards_of_rarity(rarity)
        .into_iter()
        .filter(|card| {
            // Filter out Status and Curse cards - they should never appear in rewards
            if card.card_type == crate::schema::CardType::Status 
                || card.card_type == crate::schema::CardType::Curse {
                return false;
            }
            
            // Filter by player class if specified
            if let Some(class) = player_class {
                card.color == Some(class) || card.color == Some(CardColor::Colorless)
            } else {
                true
            }
        })
        .map(|card| card.id.as_str())
        .collect();
    
    if candidates.is_empty() {
        return None;
    }
    
    // Select random card
    let idx = rng.random_range(0..candidates.len());
    Some(candidates[idx].to_string())
}

/// Reset meta-scaling RNG at the start of a new Act.
///
/// Called when entering a new Act to reset:
/// - Potion drop chance to 40%
/// - Rare card offset to -5
pub fn reset_act_rng(state: &mut GameState) {
    state.potion_drop_chance = 40;
    state.rare_card_offset = -5;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_card_rarity_weights_normal() {
        // With offset 0: Rare 3%, Uncommon 37%, Common 60%
        let weights = CardRarityWeights::normal(0);
        assert_eq!(weights.rare, 3);
        assert_eq!(weights.uncommon, 37);
        assert_eq!(weights.common, 60);
        
        // With offset +5: Rare 8%, Uncommon 37%, Common 55%
        let weights = CardRarityWeights::normal(5);
        assert_eq!(weights.rare, 8);
        assert_eq!(weights.uncommon, 37);
        assert_eq!(weights.common, 55);
        
        // With offset -5 (start): Rare 0% (clamped), Uncommon 37%, Common 63%
        let weights = CardRarityWeights::normal(-5);
        assert_eq!(weights.rare, 0);
        assert_eq!(weights.uncommon, 37);
        assert_eq!(weights.common, 63);
    }
    
    #[test]
    fn test_card_rarity_weights_elite() {
        // Elite: Rare 10%, Uncommon 40%, Common 50%
        let weights = CardRarityWeights::elite();
        assert_eq!(weights.rare, 10);
        assert_eq!(weights.uncommon, 40);
        assert_eq!(weights.common, 50);
    }
    
    #[test]
    fn test_potion_drop_chance_update() {
        let mut state = GameState::new(42);
        
        // Initial state
        assert_eq!(state.potion_drop_chance, 40);
        
        // Simulate drop: should decrease to 30
        state.potion_drop_chance -= 10;
        assert_eq!(state.potion_drop_chance, 30);
        
        // Simulate no drop: should increase to 40
        state.potion_drop_chance += 10;
        assert_eq!(state.potion_drop_chance, 40);
    }
    
    #[test]
    fn test_rare_card_offset_pity() {
        let mut state = GameState::new(42);
        
        // Initial offset
        assert_eq!(state.rare_card_offset, -5);
        
        // After 5 commons, offset should be 0
        for _ in 0..5 {
            state.rare_card_offset += 1;
        }
        assert_eq!(state.rare_card_offset, 0);
        
        // After rolling a rare, offset resets to -5
        state.rare_card_offset = -5;
        assert_eq!(state.rare_card_offset, -5);
    }
    
    #[test]
    fn test_gold_ranges() {
        use rand::SeedableRng;
        use rand_xoshiro::Xoshiro256StarStar;
        
        let mut rng = Xoshiro256StarStar::seed_from_u64(42);
        
        // Test normal range
        for _ in 0..100 {
            let gold = GoldRange::normal(&mut rng);
            assert!(gold >= 10 && gold <= 20, "Normal gold {} out of range", gold);
        }
        
        // Test elite range
        for _ in 0..100 {
            let gold = GoldRange::elite(&mut rng);
            assert!(gold >= 25 && gold <= 35, "Elite gold {} out of range", gold);
        }
        
        // Test boss range
        for _ in 0..100 {
            let gold = GoldRange::boss(&mut rng);
            assert!(gold >= 95 && gold <= 105, "Boss gold {} out of range", gold);
        }
    }
    
    #[test]
    fn test_reset_act_rng() {
        let mut state = GameState::new(42);
        
        // Modify the values
        state.potion_drop_chance = 70;
        state.rare_card_offset = 10;
        
        // Reset
        reset_act_rng(&mut state);
        
        assert_eq!(state.potion_drop_chance, 40);
        assert_eq!(state.rare_card_offset, -5);
    }
    
    #[test]
    fn test_generate_rewards_structure() {
        let mut state = GameState::new(42);
        
        let rewards = generate_rewards(&mut state, RoomType::Monster, None, None);
        
        // Should have at least gold and cards
        assert!(rewards.rewards.len() >= 2, "Should have gold and cards");
        
        // Check gold exists
        let has_gold = rewards.rewards.iter().any(|r| matches!(r, RewardType::Gold { .. }));
        assert!(has_gold, "Should have gold reward");
        
        // Check cards exist
        let has_cards = rewards.rewards.iter().any(|r| matches!(r, RewardType::Card { .. }));
        assert!(has_cards, "Should have card reward");
    }
    
    #[test]
    fn test_elite_rewards_include_relic() {
        let mut state = GameState::new(42);
        
        let rewards = generate_rewards(&mut state, RoomType::MonsterElite, None, None);
        
        // Elite should have relic
        let has_relic = rewards.rewards.iter().any(|r| matches!(r, RewardType::Relic { .. }));
        assert!(has_relic, "Elite should drop a relic");
    }
    
    #[test]
    fn test_boss_rewards_include_boss_relic() {
        let mut state = GameState::new(42);
        
        let rewards = generate_rewards(&mut state, RoomType::Boss, None, None);
        
        // Boss should have Boss tier relic
        let has_boss_relic = rewards.rewards.iter().any(|r| {
            matches!(r, RewardType::Relic { tier: RelicTier::Boss, .. })
        });
        assert!(has_boss_relic, "Boss should drop a Boss relic");
    }
}
