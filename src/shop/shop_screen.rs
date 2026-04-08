use crate::content::cards::get_card_definition;
use crate::content::potions;
use crate::content::relics::{RelicId, RelicTier};
use crate::shop::merchant::generate_cards;
use crate::shop::state::{ShopCard, ShopConfig, ShopPotion, ShopRelic, ShopState};

/// Equivalent to Java's `ShopScreen.init(...)` + associated methods.
/// Consumes rng sequences and populates the ShopState, determining prices and discounts.
pub fn generate_shop<F>(
    rng_pool: &mut crate::rng::RngPool,
    config: &ShopConfig,
    mut get_relic: F,
) -> ShopState
where
    F: FnMut(RelicTier) -> RelicId,
{
    let mut shop = ShopState::new();

    // 1. Ask Merchant to generate the raw card slots from cardRng
    let (colored_cards, colorless_cards) = generate_cards(rng_pool, config.card_blizz_randomizer);

    // Process Colored Cards Jitter (merchantRng)
    for c in colored_cards {
        let rarity = get_card_definition(c).rarity;
        let base_price = match rarity {
            crate::content::cards::CardRarity::Common => 50.0,
            crate::content::cards::CardRarity::Uncommon => 75.0,
            crate::content::cards::CardRarity::Rare => 150.0,
            _ => 50.0,
        };
        let tmp_price = base_price * rng_pool.merchant_rng.random_f32_min_max(0.9, 1.1);
        let price = tmp_price.trunc() as i32;
        shop.cards.push(ShopCard { card_id: c, price });
    }

    // Process Colorless Cards Jitter (merchantRng)
    for c in colorless_cards {
        let rarity = get_card_definition(c).rarity;
        let base_price = match rarity {
            crate::content::cards::CardRarity::Common => 50.0,
            crate::content::cards::CardRarity::Uncommon => 75.0,
            crate::content::cards::CardRarity::Rare => 150.0,
            _ => 50.0,
        };
        let mut tmp_price = base_price * rng_pool.merchant_rng.random_f32_min_max(0.9, 1.1);
        tmp_price *= 1.2; // Colorless bump
        let price = tmp_price.trunc() as i32;
        shop.cards.push(ShopCard { card_id: c, price });
    }

    // Apply Sale tag via integer division on a random colored card (0-4)
    let sale_index = rng_pool.merchant_rng.random_range(0, 4) as usize;
    shop.cards[sale_index].price /= 2;

    // 2. Relics (3 slots via merchantRng)
    for i in 0..3 {
        let tier = if i != 2 {
            let roll = rng_pool.merchant_rng.random_range(0, 99);
            if roll < 48 {
                RelicTier::Common
            } else if roll < 82 {
                RelicTier::Uncommon
            } else {
                RelicTier::Rare
            }
        } else {
            RelicTier::Shop
        };

        let relic_id = get_relic(tier);
        let base_price: f32 = match tier {
            RelicTier::Common => 150.0,
            RelicTier::Uncommon => 250.0,
            RelicTier::Rare => 300.0,
            RelicTier::Shop => 150.0,
            _ => 150.0,
        };

        let jitter_mult = rng_pool.merchant_rng.random_f32_min_max(0.95, 1.05);
        let price = (base_price * jitter_mult).round() as i32;
        shop.relics.push(ShopRelic { relic_id, price });
    }

    // 3. Potions (3 slots via potionRng & merchantRng)
    let pc = config.potion_class;
    for _ in 0..3 {
        let potion_id = potions::random_potion(&mut rng_pool.potion_rng, pc, false);
        let base_price = potions::get_potion_price(potion_id) as f32;
        let jitter_mult = rng_pool.merchant_rng.random_f32_min_max(0.95, 1.05);
        let price = (base_price * jitter_mult).round() as i32;
        shop.potions.push(ShopPotion { potion_id, price });
    }

    // 4. Initial Purge Cost
    let actual_purge_cost = 75.0 + (config.previous_purge_count as f32 * 25.0);
    shop.purge_cost = actual_purge_cost as i32;

    // 5. Apply Discounts Sequentially (ShopScreen.init logic)
    let ascension_level = config.ascension_level;
    let has_courier = config.has_courier;
    let has_membership_card = config.has_membership_card;
    let has_smiling_mask = config.has_smiling_mask;

    let mut pass = |mult: f32, affect_purge: bool| {
        for c in shop.cards.iter_mut() {
            c.price = (c.price as f32 * mult).round() as i32;
        }
        for r in shop.relics.iter_mut() {
            r.price = (r.price as f32 * mult).round() as i32;
        }
        for p in shop.potions.iter_mut() {
            p.price = (p.price as f32 * mult).round() as i32;
        }
        if affect_purge {
            shop.purge_cost = (shop.purge_cost as f32 * mult).round() as i32;
        }
    };

    if ascension_level >= 16 {
        pass(1.1, false);
    }
    if has_courier {
        pass(0.8, true);
    }
    if has_membership_card {
        pass(0.5, true);
    }
    if has_smiling_mask {
        shop.purge_cost = 50;
    }

    shop
}
