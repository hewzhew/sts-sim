//! Shop system for Slay the Spire simulator.
//!
//! This module implements the shop node mechanics:
//! - Shop inventory generation (cards, relics, potions)
//! - Pricing rules based on rarity
//! - Purchase transactions (gold management)
//! - Card removal service (purging)

use rand::Rng;
use rand_xoshiro::Xoshiro256StarStar;

use crate::loader::CardLibrary;
use crate::items::relics::{RelicLibrary, RelicTier};
use crate::schema::{CardColor, CardDefinition, CardInstance, CardRarity, CardType};
use crate::state::{GameState, ShopCard, ShopPotion, ShopRelic, ShopState};

// ============================================================================
// Pricing Constants (from global.txt)
// ============================================================================

/// Base card prices by rarity (before variance).
pub struct CardPricing;

impl CardPricing {
    /// Get base price for a card rarity.
    pub fn base_price(rarity: CardRarity) -> i32 {
        match rarity {
            CardRarity::Common => 50,
            CardRarity::Uncommon => 75,
            CardRarity::Rare => 150,
            CardRarity::Basic => 45, // Rarely sold, but possible
            CardRarity::Special => 100,
            CardRarity::Curse => 0, // Never sold normally
        }
    }
    
    /// Apply price variance (-5% to +5%).
    pub fn with_variance(base_price: i32, rng: &mut Xoshiro256StarStar) -> i32 {
        let variance: f32 = rng.random_range(-0.05..=0.05);
        ((base_price as f32) * (1.0 + variance)).round() as i32
    }
    
    /// Apply 50% sale discount.
    pub fn apply_sale(price: i32) -> i32 {
        price / 2
    }
}

/// Base relic prices by tier.
pub struct RelicPricing;

impl RelicPricing {
    /// Get base price for a relic tier.
    pub fn base_price(tier: RelicTier) -> i32 {
        match tier {
            RelicTier::Common => 150,
            RelicTier::Uncommon => 250,
            RelicTier::Rare => 300,
            RelicTier::Shop => 150, // Shop relics at Common price
            RelicTier::Boss => 300, // If somehow sold
            RelicTier::Event => 200,
            RelicTier::Starter => 100, // If somehow sold
        }
    }
    
    /// Apply price variance (-5% to +5%).
    pub fn with_variance(base_price: i32, rng: &mut Xoshiro256StarStar) -> i32 {
        let variance: f32 = rng.random_range(-0.05..=0.05);
        ((base_price as f32) * (1.0 + variance)).round() as i32
    }
}

/// Potion pricing.
pub struct PotionPricing;

impl PotionPricing {
    /// Base potion price (all potions cost the same in StS).
    pub const BASE_PRICE: i32 = 50;
    
    /// Apply price variance (-5% to +5%).
    pub fn with_variance(rng: &mut Xoshiro256StarStar) -> i32 {
        let variance: f32 = rng.random_range(-0.05..=0.05);
        ((Self::BASE_PRICE as f32) * (1.0 + variance)).round() as i32
    }
}

/// Card removal (purge) pricing.
pub struct PurgePricing;

impl PurgePricing {
    /// Starting purge cost.
    pub const STARTING_COST: i32 = 75;
    /// Cost increase per purge.
    pub const INCREASE_PER_PURGE: i32 = 25;
    
    /// Calculate purge cost based on number of times purged.
    pub fn cost(purge_count: i32) -> i32 {
        Self::STARTING_COST + purge_count * Self::INCREASE_PER_PURGE
    }
}

// ============================================================================
// Shop Generation
// ============================================================================

/// Generate a shop inventory for the given player class.
///
/// # Shop Inventory Rules (Standard StS):
/// - 5 colored cards:
///   - 1 Attack (Common/Uncommon/Rare weighted)
///   - 1 Skill (Common/Uncommon/Rare weighted)
///   - 1 Power (Uncommon/Rare only - powers are never common)
///   - 2 Random cards (any type)
/// - 2 Colorless cards (we skip these for now)
/// - 3 Relics: 1 Common/Shop, 1 Uncommon, 1 Rare
/// - 3 Potions
/// - 1 random card is on sale (50% off)
pub fn generate_shop(
    player_color: CardColor,
    card_library: &CardLibrary,
    relic_library: Option<&RelicLibrary>,
    rng: &mut Xoshiro256StarStar,
) -> ShopState {
    let mut shop = ShopState::new();
    
    // Generate colored cards
    generate_shop_cards(&mut shop, player_color, card_library, rng);
    
    // Generate relics
    generate_shop_relics(&mut shop, relic_library, rng);
    
    // Generate potions
    generate_shop_potions(&mut shop, rng);
    
    // Apply sale to one random card
    if !shop.cards.is_empty() {
        let sale_index = rng.random_range(0..shop.cards.len());
        shop.cards[sale_index].on_sale = true;
        shop.cards[sale_index].price = CardPricing::apply_sale(shop.cards[sale_index].price);
    }
    
    shop
}

/// Generate the card inventory for the shop.
fn generate_shop_cards(
    shop: &mut ShopState,
    player_color: CardColor,
    card_library: &CardLibrary,
    rng: &mut Xoshiro256StarStar,
) {
    // Collect cards by type for the player's color
    let attacks: Vec<&CardDefinition> = card_library
        .iter()
        .filter(|c| c.color == Some(player_color) && c.card_type == CardType::Attack)
        .filter(|c| matches!(c.rarity, Some(CardRarity::Common | CardRarity::Uncommon | CardRarity::Rare)))
        .collect();
    
    let skills: Vec<&CardDefinition> = card_library
        .iter()
        .filter(|c| c.color == Some(player_color) && c.card_type == CardType::Skill)
        .filter(|c| matches!(c.rarity, Some(CardRarity::Common | CardRarity::Uncommon | CardRarity::Rare)))
        .collect();
    
    // Powers are only Uncommon or Rare
    let powers: Vec<&CardDefinition> = card_library
        .iter()
        .filter(|c| c.color == Some(player_color) && c.card_type == CardType::Power)
        .filter(|c| matches!(c.rarity, Some(CardRarity::Uncommon | CardRarity::Rare)))
        .collect();
    
    let all_colored: Vec<&CardDefinition> = card_library
        .iter()
        .filter(|c| c.color == Some(player_color))
        .filter(|c| matches!(c.rarity, Some(CardRarity::Common | CardRarity::Uncommon | CardRarity::Rare)))
        .collect();
    
    // 1. One Attack
    if let Some(card) = pick_weighted_card(&attacks, rng) {
        add_card_to_shop(shop, card, rng);
    }
    
    // 2. One Skill
    if let Some(card) = pick_weighted_card(&skills, rng) {
        add_card_to_shop(shop, card, rng);
    }
    
    // 3. One Power (Uncommon/Rare only)
    if let Some(card) = pick_weighted_power(&powers, rng) {
        add_card_to_shop(shop, card, rng);
    }
    
    // 4-5. Two random colored cards
    for _ in 0..2 {
        if let Some(card) = pick_weighted_card(&all_colored, rng) {
            add_card_to_shop(shop, card, rng);
        }
    }
}

/// Pick a card with rarity weighting: Common 60%, Uncommon 30%, Rare 10%.
fn pick_weighted_card<'a>(
    cards: &[&'a CardDefinition],
    rng: &mut Xoshiro256StarStar,
) -> Option<&'a CardDefinition> {
    if cards.is_empty() {
        return None;
    }
    
    // Separate by rarity
    let commons: Vec<_> = cards.iter()
        .filter(|c| c.rarity == Some(CardRarity::Common))
        .copied()
        .collect();
    let uncommons: Vec<_> = cards.iter()
        .filter(|c| c.rarity == Some(CardRarity::Uncommon))
        .copied()
        .collect();
    let rares: Vec<_> = cards.iter()
        .filter(|c| c.rarity == Some(CardRarity::Rare))
        .copied()
        .collect();
    
    // Roll for rarity: 60% Common, 30% Uncommon, 10% Rare
    let roll: f32 = rng.random();
    let target_rarity = if roll < 0.60 && !commons.is_empty() {
        &commons
    } else if roll < 0.90 && !uncommons.is_empty() {
        &uncommons
    } else if !rares.is_empty() {
        &rares
    } else if !uncommons.is_empty() {
        &uncommons
    } else {
        &commons
    };
    
    if target_rarity.is_empty() {
        // Fallback to any card
        let idx = rng.random_range(0..cards.len());
        Some(cards[idx])
    } else {
        let idx = rng.random_range(0..target_rarity.len());
        Some(target_rarity[idx])
    }
}

/// Pick a power card (Uncommon/Rare only): 70% Uncommon, 30% Rare.
fn pick_weighted_power<'a>(
    powers: &[&'a CardDefinition],
    rng: &mut Xoshiro256StarStar,
) -> Option<&'a CardDefinition> {
    if powers.is_empty() {
        return None;
    }
    
    let uncommons: Vec<_> = powers.iter()
        .filter(|c| c.rarity == Some(CardRarity::Uncommon))
        .copied()
        .collect();
    let rares: Vec<_> = powers.iter()
        .filter(|c| c.rarity == Some(CardRarity::Rare))
        .copied()
        .collect();
    
    // 70% Uncommon, 30% Rare
    let roll: f32 = rng.random();
    let target = if roll < 0.70 && !uncommons.is_empty() {
        &uncommons
    } else if !rares.is_empty() {
        &rares
    } else {
        &uncommons
    };
    
    if target.is_empty() {
        let idx = rng.random_range(0..powers.len());
        Some(powers[idx])
    } else {
        let idx = rng.random_range(0..target.len());
        Some(target[idx])
    }
}

/// Add a card to the shop with appropriate pricing.
fn add_card_to_shop(
    shop: &mut ShopState,
    card: &CardDefinition,
    rng: &mut Xoshiro256StarStar,
) {
    let rarity = card.rarity.unwrap_or(CardRarity::Common);
    let base_price = CardPricing::base_price(rarity);
    let price = CardPricing::with_variance(base_price, rng);
    
    let card_instance = CardInstance::new(card.id.clone(), card.cost);
    
    shop.cards.push(ShopCard {
        card: card_instance,
        price,
        on_sale: false,
    });
}

/// Generate the relic inventory for the shop.
fn generate_shop_relics(
    shop: &mut ShopState,
    relic_library: Option<&RelicLibrary>,
    rng: &mut Xoshiro256StarStar,
) {
    let Some(library) = relic_library else {
        // No relic library available, use placeholder relics
        shop.relics.push(ShopRelic {
            relic_id: "ShopRelic_Common".to_string(),
            price: RelicPricing::with_variance(RelicPricing::base_price(RelicTier::Common), rng),
        });
        shop.relics.push(ShopRelic {
            relic_id: "ShopRelic_Uncommon".to_string(),
            price: RelicPricing::with_variance(RelicPricing::base_price(RelicTier::Uncommon), rng),
        });
        shop.relics.push(ShopRelic {
            relic_id: "ShopRelic_Rare".to_string(),
            price: RelicPricing::with_variance(RelicPricing::base_price(RelicTier::Rare), rng),
        });
        return;
    };
    
    // 1. One Common or Shop tier relic
    let common_shop_relics: Vec<_> = library
        .iter()
        .filter(|r| matches!(r.tier, RelicTier::Common | RelicTier::Shop))
        .collect();
    
    if !common_shop_relics.is_empty() {
        let idx = rng.random_range(0..common_shop_relics.len());
        let relic = common_shop_relics[idx];
        shop.relics.push(ShopRelic {
            relic_id: relic.id.clone(),
            price: RelicPricing::with_variance(RelicPricing::base_price(relic.tier), rng),
        });
    }
    
    // 2. One Uncommon relic
    let uncommon_relics: Vec<_> = library
        .iter()
        .filter(|r| r.tier == RelicTier::Uncommon)
        .collect();
    
    if !uncommon_relics.is_empty() {
        let idx = rng.random_range(0..uncommon_relics.len());
        let relic = uncommon_relics[idx];
        shop.relics.push(ShopRelic {
            relic_id: relic.id.clone(),
            price: RelicPricing::with_variance(RelicPricing::base_price(RelicTier::Uncommon), rng),
        });
    }
    
    // 3. One Rare relic
    let rare_relics: Vec<_> = library
        .iter()
        .filter(|r| r.tier == RelicTier::Rare)
        .collect();
    
    if !rare_relics.is_empty() {
        let idx = rng.random_range(0..rare_relics.len());
        let relic = rare_relics[idx];
        shop.relics.push(ShopRelic {
            relic_id: relic.id.clone(),
            price: RelicPricing::with_variance(RelicPricing::base_price(RelicTier::Rare), rng),
        });
    }
}

/// Generate the potion inventory for the shop.
fn generate_shop_potions(
    shop: &mut ShopState,
    rng: &mut Xoshiro256StarStar,
) {
    // Standard shop has 3 potions
    // For now, use placeholder potion IDs
    let potion_ids = [
        "FirePotion",
        "BlockPotion", 
        "SwiftPotion",
        "StrengthPotion",
        "WeakPotion",
        "FearPotion",
        "HealthPotion",
        "EnergyPotion",
        "DexterityPotion",
    ];
    
    for _ in 0..3 {
        let idx = rng.random_range(0..potion_ids.len());
        shop.potions.push(ShopPotion {
            potion_id: potion_ids[idx].to_string(),
            price: PotionPricing::with_variance(rng),
        });
    }
}

// ============================================================================
// Shop Transactions
// ============================================================================

/// Result of a shop transaction.
#[derive(Debug, Clone)]
pub enum ShopResult {
    /// Purchase successful.
    Success,
    /// Not enough gold.
    InsufficientGold { have: i32, need: i32 },
    /// Item not found in shop.
    ItemNotFound,
    /// Shop not active.
    NoActiveShop,
    /// Cannot purge (e.g., deck too small).
    CannotPurge { reason: String },
}

impl GameState {
    /// Buy a card from the shop.
    ///
    /// # Arguments
    /// * `shop_index` - Index of the card in `shop_state.cards`.
    ///
    /// # Returns
    /// * `ShopResult::Success` - Card purchased and added to deck.
    /// * `ShopResult::InsufficientGold` - Not enough gold.
    /// * `ShopResult::ItemNotFound` - Invalid index.
    /// * `ShopResult::NoActiveShop` - No shop is currently active.
    pub fn buy_card(&mut self, shop_index: usize) -> ShopResult {
        let Some(ref mut shop) = self.shop_state else {
            return ShopResult::NoActiveShop;
        };
        
        if shop_index >= shop.cards.len() {
            return ShopResult::ItemNotFound;
        }
        
        let price = shop.cards[shop_index].price;
        
        if self.gold < price {
            return ShopResult::InsufficientGold {
                have: self.gold,
                need: price,
            };
        }
        
        // Deduct gold and add card to deck
        self.gold -= price;
        let shop_card = shop.cards.remove(shop_index);
        self.draw_pile.push(shop_card.card);
        
        ShopResult::Success
    }
    
    /// Buy a relic from the shop.
    ///
    /// # Arguments
    /// * `shop_index` - Index of the relic in `shop_state.relics`.
    /// * `relic_library` - Optional relic library for OnPickup trigger.
    ///
    /// # Returns
    /// * `ShopResult::Success` - Relic purchased and added to inventory.
    /// * `ShopResult::InsufficientGold` - Not enough gold.
    /// * `ShopResult::ItemNotFound` - Invalid index.
    /// * `ShopResult::NoActiveShop` - No shop is currently active.
    pub fn buy_relic(&mut self, shop_index: usize, relic_library: Option<&RelicLibrary>) -> ShopResult {
        let price = {
            let Some(ref shop) = self.shop_state else {
                return ShopResult::NoActiveShop;
            };
            
            if shop_index >= shop.relics.len() {
                return ShopResult::ItemNotFound;
            }
            
            shop.relics[shop_index].price
        };
        
        if self.gold < price {
            return ShopResult::InsufficientGold {
                have: self.gold,
                need: price,
            };
        }
        
        // Get relic info before removing from shop
        let relic_id = {
            let shop = self.shop_state.as_mut().unwrap();
            let shop_relic = shop.relics.remove(shop_index);
            shop_relic.relic_id
        };
        
        // Deduct gold
        self.gold -= price;
        
        // Add relic to player's relics
        if let Some(library) = relic_library {
            if let Some(def) = library.get(&relic_id) {
                use crate::items::relics::RelicInstance;
                let instance = RelicInstance::new(&def.id);
                self.relics.push(instance);
                
                // TODO: Trigger OnPickup effect
                // This would require the relic trigger system
            }
        } else {
            // No library, add a placeholder relic
            use crate::items::relics::RelicInstance;
            self.relics.push(RelicInstance::new(&relic_id));
        }
        
        ShopResult::Success
    }
    
    /// Buy a potion from the shop.
    ///
    /// # Arguments
    /// * `shop_index` - Index of the potion in `shop_state.potions`.
    ///
    /// # Returns
    /// * `ShopResult::Success` - Potion purchased (TODO: add to inventory).
    /// * `ShopResult::InsufficientGold` - Not enough gold.
    /// * `ShopResult::ItemNotFound` - Invalid index.
    /// * `ShopResult::NoActiveShop` - No shop is currently active.
    pub fn buy_potion(&mut self, shop_index: usize) -> ShopResult {
        let Some(ref mut shop) = self.shop_state else {
            return ShopResult::NoActiveShop;
        };
        
        if shop_index >= shop.potions.len() {
            return ShopResult::ItemNotFound;
        }
        
        let price = shop.potions[shop_index].price;
        
        if self.gold < price {
            return ShopResult::InsufficientGold {
                have: self.gold,
                need: price,
            };
        }
        
        // Deduct gold and remove from shop
        self.gold -= price;
        let _potion = shop.potions.remove(shop_index);
        
        // TODO: Add potion to player's potion slots
        // For now, just remove it from the shop (potion is "consumed" on purchase for testing)
        
        ShopResult::Success
    }
    
    /// Purge (remove) a card from the master deck.
    ///
    /// # Arguments
    /// * `deck_index` - Index of the card in `draw_pile` (master deck when outside combat).
    ///
    /// # Returns
    /// * `ShopResult::Success` - Card removed, purge_count incremented.
    /// * `ShopResult::InsufficientGold` - Not enough gold for current purge cost.
    /// * `ShopResult::ItemNotFound` - Invalid card index.
    /// * `ShopResult::NoActiveShop` - No shop is currently active.
    /// * `ShopResult::CannotPurge` - Deck too small or other restriction.
    ///
    /// # Note
    /// Purge cost starts at 75 and increases by 25 for each purge.
    pub fn purge_card(&mut self, deck_index: usize) -> ShopResult {
        if self.shop_state.is_none() {
            return ShopResult::NoActiveShop;
        }
        
        // Minimum deck size check (optional, StS allows purging down to 0)
        // For safety, let's require at least 1 card remaining
        if self.draw_pile.len() <= 1 {
            return ShopResult::CannotPurge {
                reason: "Deck must have at least one card remaining".to_string(),
            };
        }
        
        if deck_index >= self.draw_pile.len() {
            return ShopResult::ItemNotFound;
        }
        
        let purge_cost = PurgePricing::cost(self.purge_count);
        
        if self.gold < purge_cost {
            return ShopResult::InsufficientGold {
                have: self.gold,
                need: purge_cost,
            };
        }
        
        // Deduct gold, remove card, increment purge count
        self.gold -= purge_cost;
        self.draw_pile.remove(deck_index);
        self.purge_count += 1;
        
        ShopResult::Success
    }
    
    /// Get the current purge cost.
    pub fn current_purge_cost(&self) -> i32 {
        PurgePricing::cost(self.purge_count)
    }
    
    /// Enter a shop node, generating a new shop.
    pub fn enter_shop(
        &mut self,
        player_color: CardColor,
        card_library: &CardLibrary,
        relic_library: Option<&RelicLibrary>,
    ) {
        self.shop_state = Some(generate_shop(
            player_color,
            card_library,
            relic_library,
            &mut self.rng,
        ));
        
        // TODO: Trigger EnterShop relic effects
    }
    
    /// Leave the shop, clearing the shop state.
    pub fn leave_shop(&mut self) {
        self.shop_state = None;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_card_pricing() {
        assert_eq!(CardPricing::base_price(CardRarity::Common), 50);
        assert_eq!(CardPricing::base_price(CardRarity::Uncommon), 75);
        assert_eq!(CardPricing::base_price(CardRarity::Rare), 150);
        
        // Sale should halve the price
        assert_eq!(CardPricing::apply_sale(100), 50);
        assert_eq!(CardPricing::apply_sale(75), 37); // Integer division
    }
    
    #[test]
    fn test_relic_pricing() {
        assert_eq!(RelicPricing::base_price(RelicTier::Common), 150);
        assert_eq!(RelicPricing::base_price(RelicTier::Uncommon), 250);
        assert_eq!(RelicPricing::base_price(RelicTier::Rare), 300);
    }
    
    #[test]
    fn test_purge_pricing() {
        assert_eq!(PurgePricing::cost(0), 75);
        assert_eq!(PurgePricing::cost(1), 100);
        assert_eq!(PurgePricing::cost(2), 125);
        assert_eq!(PurgePricing::cost(3), 150);
    }
    
    #[test]
    fn test_buy_card_insufficient_gold() {
        let mut state = GameState::new(42);
        state.gold = 10; // Very little gold
        state.shop_state = Some(ShopState {
            cards: vec![ShopCard {
                card: CardInstance::new_basic("TestCard", 1),
                price: 50,
                on_sale: false,
            }],
            relics: vec![],
            potions: vec![],
        });
        
        let result = state.buy_card(0);
        assert!(matches!(result, ShopResult::InsufficientGold { have: 10, need: 50 }));
    }
    
    #[test]
    fn test_buy_card_success() {
        let mut state = GameState::new(42);
        state.gold = 100;
        state.shop_state = Some(ShopState {
            cards: vec![ShopCard {
                card: CardInstance::new_basic("TestCard", 1),
                price: 50,
                on_sale: false,
            }],
            relics: vec![],
            potions: vec![],
        });
        
        let initial_deck_size = state.draw_pile.len();
        let result = state.buy_card(0);
        
        assert!(matches!(result, ShopResult::Success));
        assert_eq!(state.gold, 50); // 100 - 50
        assert_eq!(state.draw_pile.len(), initial_deck_size + 1);
        assert!(state.shop_state.as_ref().unwrap().cards.is_empty());
    }
    
    #[test]
    fn test_purge_card() {
        let mut state = GameState::new(42);
        state.gold = 200;
        state.shop_state = Some(ShopState::new());
        
        // Add some cards to the deck
        state.draw_pile.push(CardInstance::new_basic("Card1", 1));
        state.draw_pile.push(CardInstance::new_basic("Card2", 1));
        state.draw_pile.push(CardInstance::new_basic("Card3", 1));
        
        assert_eq!(state.current_purge_cost(), 75);
        
        // First purge
        let result = state.purge_card(0);
        assert!(matches!(result, ShopResult::Success));
        assert_eq!(state.gold, 125); // 200 - 75
        assert_eq!(state.draw_pile.len(), 2);
        assert_eq!(state.purge_count, 1);
        assert_eq!(state.current_purge_cost(), 100);
        
        // Second purge
        let result = state.purge_card(0);
        assert!(matches!(result, ShopResult::Success));
        assert_eq!(state.gold, 25); // 125 - 100
        assert_eq!(state.draw_pile.len(), 1);
        assert_eq!(state.purge_count, 2);
    }
    
    #[test]
    fn test_no_active_shop() {
        let mut state = GameState::new(42);
        state.gold = 1000;
        // No shop_state set
        
        assert!(matches!(state.buy_card(0), ShopResult::NoActiveShop));
        assert!(matches!(state.buy_relic(0, None), ShopResult::NoActiveShop));
        assert!(matches!(state.buy_potion(0), ShopResult::NoActiveShop));
        assert!(matches!(state.purge_card(0), ShopResult::NoActiveShop));
    }
}
