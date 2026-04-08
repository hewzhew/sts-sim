use crate::content::cards::{
    colorless_pool_for_rarity, get_card_definition, ironclad_pool_for_rarity, java_id,
};
use crate::content::cards::{CardId, CardRarity, CardType};
use crate::rng::RngPool;

/// Equivalent to Java's `Merchant` class constructor.
/// Generates 5 colored cards (2 Attack, 2 Skill, 1 Power) and 2 colorless cards (1 Uncommon, 1 Rare).
pub fn generate_cards(rng_pool: &mut RngPool, blizz_randomizer: i32) -> (Vec<CardId>, Vec<CardId>) {
    let get_colored_card = |rng_pool: &mut RngPool, card_type: CardType| -> CardId {
        loop {
            // Rarity roll: Java AbstractDungeon.rollRarity uses cardRng + cardBlizzRandomizer
            let roll = rng_pool.card_rng.random_range(0, 99) + blizz_randomizer;
            let mut rarity = if roll < 9 {
                CardRarity::Rare
            } else if roll < 46 {
                // 9 + 37
                CardRarity::Uncommon
            } else {
                CardRarity::Common
            };

            let mut pool = ironclad_pool_for_rarity(rarity);
            let mut typed_pool: Vec<CardId> = pool
                .iter()
                .copied()
                .filter(|&id| get_card_definition(id).card_type == card_type)
                .collect();

            // If pool is empty, emulate getCardFromPool fallback
            if typed_pool.is_empty() {
                if card_type == CardType::Power {
                    rarity = if rarity == CardRarity::Common {
                        CardRarity::Uncommon
                    } else if rarity == CardRarity::Uncommon {
                        CardRarity::Rare
                    } else {
                        rarity
                    };
                    pool = ironclad_pool_for_rarity(rarity);
                    typed_pool = pool
                        .iter()
                        .copied()
                        .filter(|&id| get_card_definition(id).card_type == card_type)
                        .collect();
                }
            }

            // Emulate Collections.sort(tmp) mapping to Java cardID strings
            typed_pool.sort_by_key(|&id| java_id(id));

            let idx = rng_pool.card_rng.random(typed_pool.len() as i32 - 1) as usize;
            let c = typed_pool[idx];

            return c;
        }
    };

    let c1_atk = get_colored_card(rng_pool, CardType::Attack);
    let mut c2_atk = get_colored_card(rng_pool, CardType::Attack);
    while c2_atk == c1_atk {
        c2_atk = get_colored_card(rng_pool, CardType::Attack);
    }

    let c3_skl = get_colored_card(rng_pool, CardType::Skill);
    let mut c4_skl = get_colored_card(rng_pool, CardType::Skill);
    while c4_skl == c3_skl {
        c4_skl = get_colored_card(rng_pool, CardType::Skill);
    }

    let c5_pwr = get_colored_card(rng_pool, CardType::Power);

    let colored_cards = vec![c1_atk, c2_atk, c3_skl, c4_skl, c5_pwr];

    let get_colorless = |rng_pool: &mut RngPool, rarity: CardRarity| -> CardId {
        let pool = colorless_pool_for_rarity(rarity);
        let mut typed_pool = pool.to_vec();
        typed_pool.sort_by_key(|&id| java_id(id));
        let idx = rng_pool.card_rng.random(typed_pool.len() as i32 - 1) as usize;
        typed_pool[idx]
    };

    let c6_clr_unc = get_colorless(rng_pool, CardRarity::Uncommon);
    let c7_clr_rar = get_colorless(rng_pool, CardRarity::Rare);

    let colorless_cards = vec![c6_clr_unc, c7_clr_rar];

    (colored_cards, colorless_cards)
}
