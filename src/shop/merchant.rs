use crate::content::cards::{colorless_pool_for_rarity, get_card_definition, java_id};
use crate::content::cards::{CardId, CardRarity, CardType};
use crate::runtime::rng::RngPool;

pub fn roll_shop_card_rarity(rng_pool: &mut RngPool, blizz_randomizer: i32) -> CardRarity {
    let roll = rng_pool.card_rng.random_range(0, 99) + blizz_randomizer;
    if roll < 9 {
        CardRarity::Rare
    } else if roll < 46 {
        CardRarity::Uncommon
    } else {
        CardRarity::Common
    }
}

pub fn random_shop_colored_card_of_type(
    rng_pool: &mut RngPool,
    player_class: &str,
    blizz_randomizer: i32,
    card_type: CardType,
) -> CardId {
    use crate::content::cards::CardRarity;

    let rarity_fallbacks = |rarity: CardRarity| match rarity {
        CardRarity::Rare => [CardRarity::Rare, CardRarity::Uncommon, CardRarity::Common],
        CardRarity::Uncommon => [CardRarity::Uncommon, CardRarity::Common, CardRarity::Rare],
        CardRarity::Common => [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare],
        _ => [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare],
    };

    loop {
        let rarity = roll_shop_card_rarity(rng_pool, blizz_randomizer);

        let mut typed_pool = Vec::new();
        for candidate_rarity in rarity_fallbacks(rarity) {
            typed_pool = crate::engine::campfire_handler::card_pool_for_class(
                player_class,
                candidate_rarity,
            )
            .iter()
            .copied()
            .filter(|&id| get_card_definition(id).card_type == card_type)
            .collect();
            if !typed_pool.is_empty() {
                break;
            }
        }
        if typed_pool.is_empty() {
            return match (player_class, card_type) {
                ("Silent", CardType::Attack) => {
                    if rng_pool.card_rng.random_boolean() {
                        CardId::StrikeG
                    } else {
                        CardId::Neutralize
                    }
                }
                ("Silent", CardType::Skill) => {
                    if rng_pool.card_rng.random_boolean() {
                        CardId::DefendG
                    } else {
                        CardId::Survivor
                    }
                }
                ("Silent", CardType::Power) => CardId::Footwork,
                _ => CardId::Strike,
            };
        }

        typed_pool.sort_by_key(|&id| java_id(id));

        let idx = rng_pool.card_rng.random(typed_pool.len() as i32 - 1) as usize;
        return typed_pool[idx];
    }
}

pub fn random_shop_colorless_card(rng_pool: &mut RngPool, rarity: CardRarity) -> CardId {
    let pool = colorless_pool_for_rarity(rarity);
    let mut typed_pool = pool.to_vec();
    typed_pool.sort_by_key(|&id| java_id(id));
    let idx = rng_pool.card_rng.random(typed_pool.len() as i32 - 1) as usize;
    typed_pool[idx]
}

/// Equivalent to Java's `Merchant` class constructor.
/// Generates 5 colored cards (2 Attack, 2 Skill, 1 Power) and 2 colorless cards (1 Uncommon, 1 Rare).
pub fn generate_cards(
    rng_pool: &mut RngPool,
    player_class: &str,
    blizz_randomizer: i32,
) -> (Vec<CardId>, Vec<CardId>) {
    let c1_atk = random_shop_colored_card_of_type(
        rng_pool,
        player_class,
        blizz_randomizer,
        CardType::Attack,
    );
    let mut c2_atk = random_shop_colored_card_of_type(
        rng_pool,
        player_class,
        blizz_randomizer,
        CardType::Attack,
    );
    let mut attack_attempts = 0;
    while c2_atk == c1_atk && attack_attempts < 12 {
        c2_atk = random_shop_colored_card_of_type(
            rng_pool,
            player_class,
            blizz_randomizer,
            CardType::Attack,
        );
        attack_attempts += 1;
    }

    let c3_skl =
        random_shop_colored_card_of_type(rng_pool, player_class, blizz_randomizer, CardType::Skill);
    let mut c4_skl =
        random_shop_colored_card_of_type(rng_pool, player_class, blizz_randomizer, CardType::Skill);
    let mut skill_attempts = 0;
    while c4_skl == c3_skl && skill_attempts < 12 {
        c4_skl = random_shop_colored_card_of_type(
            rng_pool,
            player_class,
            blizz_randomizer,
            CardType::Skill,
        );
        skill_attempts += 1;
    }

    let c5_pwr =
        random_shop_colored_card_of_type(rng_pool, player_class, blizz_randomizer, CardType::Power);

    let colored_cards = vec![c1_atk, c2_atk, c3_skl, c4_skl, c5_pwr];

    let c6_clr_unc = random_shop_colorless_card(rng_pool, CardRarity::Uncommon);
    let c7_clr_rar = random_shop_colorless_card(rng_pool, CardRarity::Rare);

    let colorless_cards = vec![c6_clr_unc, c7_clr_rar];

    (colored_cards, colorless_cards)
}
