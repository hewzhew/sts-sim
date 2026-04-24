use crate::content::cards::{
    get_card_definition, CardId, CardRarity, COLORLESS_RARE_POOL, COLORLESS_UNCOMMON_POOL,
};
use crate::content::potions::{self, PotionId};
use crate::content::relics::{RelicId, RelicTier};
use crate::shop::merchant::{random_shop_colored_card_of_type, random_shop_colorless_card};
use crate::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

fn has_relic(run_state: &RunState, relic_id: RelicId) -> bool {
    run_state.relics.iter().any(|relic| relic.id == relic_id)
}

fn current_shop_price_multiplier(run_state: &RunState) -> f32 {
    let mut multiplier = 1.0;
    if has_relic(run_state, RelicId::Courier) {
        multiplier *= 0.8;
    }
    if has_relic(run_state, RelicId::MembershipCard) {
        multiplier *= 0.5;
    }
    multiplier
}

fn base_purge_cost(run_state: &RunState) -> i32 {
    75 + run_state.shop_purge_count * 25
}

fn apply_shop_discount(
    shop: &mut ShopState,
    multiplier: f32,
    affect_purge: bool,
    smiling_mask_active: bool,
    purge_base_cost: i32,
) {
    for card in &mut shop.cards {
        card.price = (card.price as f32 * multiplier).round() as i32;
    }
    for relic in &mut shop.relics {
        relic.price = (relic.price as f32 * multiplier).round() as i32;
    }
    for potion in &mut shop.potions {
        potion.price = (potion.price as f32 * multiplier).round() as i32;
    }
    if smiling_mask_active {
        shop.purge_cost = 50;
    } else if affect_purge {
        shop.purge_cost = (purge_base_cost as f32 * multiplier).round() as i32;
    }
}

fn base_relic_price_for_tier(tier: RelicTier) -> i32 {
    match tier {
        RelicTier::Common => 150,
        RelicTier::Uncommon => 250,
        RelicTier::Rare => 300,
        RelicTier::Shop => 150,
        _ => 150,
    }
}

fn roll_shop_relic_tier(run_state: &mut RunState) -> RelicTier {
    let roll = run_state.rng_pool.merchant_rng.random_range(0, 99);
    if roll < 48 {
        RelicTier::Common
    } else if roll < 82 {
        RelicTier::Uncommon
    } else {
        RelicTier::Rare
    }
}

fn reprice_relic_price(run_state: &mut RunState, base_price: i32) -> i32 {
    let jittered = (base_price as f32
        * run_state
            .rng_pool
            .merchant_rng
            .random_f32_min_max(0.95, 1.05))
    .round() as i32;
    (jittered as f32 * current_shop_price_multiplier(run_state)).round() as i32
}

fn reprice_potion_price(run_state: &mut RunState, potion_id: PotionId) -> i32 {
    let base_price = potions::get_potion_price(potion_id) as f32;
    let jittered = (base_price
        * run_state
            .rng_pool
            .merchant_rng
            .random_f32_min_max(0.95, 1.05))
    .round() as i32;
    (jittered as f32 * current_shop_price_multiplier(run_state)).round() as i32
}

fn reprice_card_price(run_state: &mut RunState, card_id: CardId) -> i32 {
    let def = get_card_definition(card_id);
    let mut price = match def.rarity {
        CardRarity::Common => 50.0,
        CardRarity::Uncommon => 75.0,
        CardRarity::Rare => 150.0,
        _ => 50.0,
    } * run_state.rng_pool.merchant_rng.random_f32_min_max(0.9, 1.1);
    if COLORLESS_UNCOMMON_POOL.contains(&card_id) || COLORLESS_RARE_POOL.contains(&card_id) {
        price *= 1.2;
    }
    price *= current_shop_price_multiplier(run_state);
    price.trunc() as i32
}

fn replace_shop_relic_slot(run_state: &mut RunState, shop: &mut ShopState, idx: usize) {
    let replacement = loop {
        let tier = roll_shop_relic_tier(run_state);
        let relic_id = run_state.random_relic_by_tier(tier);
        if matches!(
            relic_id,
            RelicId::OldCoin | RelicId::SmilingMask | RelicId::MawBank | RelicId::Courier
        ) {
            continue;
        }
        break ShopRelic {
            relic_id,
            price: reprice_relic_price(run_state, base_relic_price_for_tier(tier)),
            can_buy: true,
            blocked_reason: None,
        };
    };

    if idx < shop.relics.len() {
        shop.relics[idx] = replacement;
    } else {
        shop.relics.push(replacement);
    }
}

fn replace_shop_potion_slot(run_state: &mut RunState, shop: &mut ShopState, idx: usize) {
    let potion_id = run_state.random_potion();
    let replacement = ShopPotion {
        potion_id,
        price: reprice_potion_price(run_state, potion_id),
        can_buy: true,
        blocked_reason: None,
    };
    if idx < shop.potions.len() {
        shop.potions[idx] = replacement;
    } else {
        shop.potions.push(replacement);
    }
}

fn replace_shop_card_slot(
    run_state: &mut RunState,
    shop: &mut ShopState,
    idx: usize,
    purchased_card_id: CardId,
) {
    let replacement_id = if COLORLESS_UNCOMMON_POOL.contains(&purchased_card_id)
        || COLORLESS_RARE_POOL.contains(&purchased_card_id)
    {
        let rarity = if run_state.rng_pool.merchant_rng.random_boolean_chance(0.3) {
            CardRarity::Rare
        } else {
            CardRarity::Uncommon
        };
        random_shop_colorless_card(&mut run_state.rng_pool, rarity)
    } else {
        let card_type = get_card_definition(purchased_card_id).card_type;
        random_shop_colored_card_of_type(
            &mut run_state.rng_pool,
            run_state.player_class,
            run_state.card_blizz_randomizer,
            card_type,
        )
    };
    let replacement = ShopCard {
        card_id: replacement_id,
        price: reprice_card_price(run_state, replacement_id),
        can_buy: true,
        blocked_reason: None,
    };
    if idx < shop.cards.len() {
        shop.cards[idx] = replacement;
    } else {
        shop.cards.push(replacement);
    }
}

pub fn handle(
    run_state: &mut RunState,
    shop: &mut ShopState,
    input: Option<ClientInput>,
) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::BuyCard(idx) => {
                if idx < shop.cards.len() && run_state.gold >= shop.cards[idx].price {
                    let purchased = shop.cards.remove(idx);
                    run_state.change_gold_with_source(-purchased.price, DomainEventSource::Shop);
                    run_state.add_card_to_deck(purchased.card_id);
                    if has_relic(run_state, RelicId::Courier) {
                        replace_shop_card_slot(run_state, shop, idx, purchased.card_id);
                    }
                }
            }
            ClientInput::BuyRelic(idx) => {
                if idx < shop.relics.len() && run_state.gold >= shop.relics[idx].price {
                    let purchased = shop.relics.remove(idx);
                    run_state.change_gold_with_source(-purchased.price, DomainEventSource::Shop);
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        purchased.relic_id,
                        EngineState::Shop(shop.clone()),
                        DomainEventSource::Shop,
                    ) {
                        return Some(next_state);
                    }

                    if purchased.relic_id == RelicId::MembershipCard {
                        apply_shop_discount(
                            shop,
                            0.5,
                            true,
                            has_relic(run_state, RelicId::SmilingMask),
                            base_purge_cost(run_state),
                        );
                    }
                    if purchased.relic_id == RelicId::SmilingMask {
                        shop.purge_cost = 50;
                    }
                    if purchased.relic_id == RelicId::Courier
                        || has_relic(run_state, RelicId::Courier)
                    {
                        replace_shop_relic_slot(run_state, shop, idx);
                    }
                }
            }
            ClientInput::BuyPotion(idx) => {
                if idx < shop.potions.len() && run_state.gold >= shop.potions[idx].price {
                    if has_relic(run_state, RelicId::Sozu) {
                    } else if let Some(empty_slot) = run_state.find_empty_potion_slot() {
                        let purchased = shop.potions.remove(idx);
                        run_state
                            .change_gold_with_source(-purchased.price, DomainEventSource::Shop);
                        run_state.obtain_potion_with_source(
                            crate::content::potions::Potion::new(
                                purchased.potion_id,
                                empty_slot as u32,
                            ),
                            DomainEventSource::Shop,
                        );
                        if has_relic(run_state, RelicId::Courier) {
                            replace_shop_potion_slot(run_state, shop, idx);
                        }
                    }
                }
            }
            ClientInput::PurgeCard(idx) => {
                if shop.purge_available
                    && run_state.gold >= shop.purge_cost
                    && idx < run_state.master_deck.len()
                {
                    let uuid = run_state.master_deck[idx].uuid;
                    run_state.change_gold_with_source(-shop.purge_cost, DomainEventSource::Shop);
                    shop.purge_available = false;
                    run_state.remove_card_from_deck(uuid);
                    run_state.shop_purge_count += 1;
                }
            }
            ClientInput::Proceed | ClientInput::Cancel => {
                return Some(EngineState::MapNavigation);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::content::cards::CardId;
    use crate::content::potions::PotionId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::shop::state::{ShopCard, ShopPotion, ShopRelic, ShopState};
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::run::RunState;

    #[test]
    fn spending_gold_in_shop_uses_up_maw_bank() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 200;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::MawBank));

        let mut shop = ShopState::new();
        shop.cards.push(ShopCard {
            card_id: CardId::Strike,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyCard(0)));
        assert!(next.is_none());
        let maw_bank = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::MawBank)
            .expect("MawBank should still be present");
        assert!(maw_bank.used_up);
        assert_eq!(maw_bank.counter, -2);
    }

    #[test]
    fn leaving_shop_without_spending_keeps_maw_bank_active() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 200;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::MawBank));

        let mut shop = ShopState::new();
        let next = handle(&mut run_state, &mut shop, Some(ClientInput::Proceed));
        assert_eq!(next, Some(EngineState::MapNavigation));
        let maw_bank = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::MawBank)
            .expect("MawBank should still be present");
        assert!(!maw_bank.used_up);
        assert_eq!(maw_bank.counter, -1);
    }

    #[test]
    fn membership_card_purchase_discounts_remaining_shop_inventory() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 500;
        run_state.shop_purge_count = 1;
        run_state.relics.clear();

        let mut shop = ShopState::new();
        shop.purge_cost = 100;
        shop.cards.push(ShopCard {
            card_id: CardId::Strike,
            price: 100,
            can_buy: true,
            blocked_reason: None,
        });
        shop.relics.push(ShopRelic {
            relic_id: RelicId::MembershipCard,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Anchor,
            price: 200,
            can_buy: true,
            blocked_reason: None,
        });
        shop.potions.push(ShopPotion {
            potion_id: PotionId::BlockPotion,
            price: 60,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyRelic(0)));
        assert!(next.is_none());
        assert_eq!(shop.cards[0].price, 50);
        assert_eq!(shop.relics[0].price, 100);
        assert_eq!(shop.potions[0].price, 30);
        assert_eq!(shop.purge_cost, 50);
    }

    #[test]
    fn smiling_mask_purchase_sets_purge_cost_to_50() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 500;
        run_state.relics.clear();

        let mut shop = ShopState::new();
        shop.purge_cost = 125;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::SmilingMask,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyRelic(0)));
        assert!(next.is_none());
        assert_eq!(shop.purge_cost, 50);
    }

    #[test]
    fn courier_keeps_relic_slot_filled_after_purchase() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 500;
        run_state.relics.clear();
        run_state.common_relic_pool = vec![RelicId::Anchor];
        run_state.uncommon_relic_pool = vec![RelicId::LetterOpener];
        run_state.rare_relic_pool = vec![RelicId::LizardTail];

        let mut shop = ShopState::new();
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Courier,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyRelic(0)));
        assert!(next.is_none());
        assert_eq!(shop.relics.len(), 1);
        assert_ne!(shop.relics[0].relic_id, RelicId::Courier);
    }

    #[test]
    fn courier_keeps_potion_slot_filled_after_purchase() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 500;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Courier));

        let mut shop = ShopState::new();
        shop.potions.push(ShopPotion {
            potion_id: PotionId::BlockPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyPotion(0)));
        assert!(next.is_none());
        assert_eq!(shop.potions.len(), 1);
        assert!(run_state.potions.iter().any(|p| p.is_some()));
    }

    #[test]
    fn courier_keeps_card_slot_filled_after_purchase() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 500;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Courier));
        let starting_deck_len = run_state.master_deck.len();

        let mut shop = ShopState::new();
        shop.cards.push(ShopCard {
            card_id: CardId::Strike,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let next = handle(&mut run_state, &mut shop, Some(ClientInput::BuyCard(0)));
        assert!(next.is_none());
        assert_eq!(run_state.master_deck.len(), starting_deck_len + 1);
        assert_eq!(shop.cards.len(), 1);
    }
}
