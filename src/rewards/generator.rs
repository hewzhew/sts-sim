use crate::content::cards::CardRarity;
use crate::rewards::state::RewardCard;
use crate::rewards::state::RewardItem;
use crate::rewards::state::RewardState;
use crate::state::run::RunState;

pub fn adjusted_card_reward_choice_count(run_state: &RunState, base_count: usize) -> usize {
    let mut num_cards = base_count;
    if run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::BustedCrown)
    {
        num_cards = num_cards.saturating_sub(2).max(1);
    }
    if run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::QuestionCard)
    {
        num_cards += 1;
    }
    num_cards
}

/// Generates post-combat loot transitioning into the RewardState
pub fn generate_combat_rewards(
    run_state: &mut RunState,
    is_elite: bool,
    is_boss: bool,
) -> RewardState {
    let mut items = Vec::new();

    let has_golden_idol = run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::GoldenIdol);
    let has_prayer_wheel = run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::PrayerWheel);

    let has_ectoplasm = run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::Ectoplasm);

    // 1. Generate Gold
    if !has_ectoplasm {
        if is_boss {
            let mut amount = 100 + run_state.rng_pool.misc_rng.random_range(-5, 5);
            if run_state.ascension_level >= 13 {
                amount = (amount as f32 * 0.75).round() as i32;
            }
            if has_golden_idol {
                amount += (amount as f32 * 0.25).round() as i32;
            }
            items.push(RewardItem::Gold { amount });
        } else {
            let mut amount = if is_elite {
                run_state.rng_pool.treasure_rng.random_range(25, 35)
            } else {
                run_state.rng_pool.treasure_rng.random_range(10, 20)
            };
            if has_golden_idol {
                amount += (amount as f32 * 0.25).round() as i32;
            }
            items.push(RewardItem::Gold { amount });
        }
    }

    // 2. Generate Potions
    let has_sozu = run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::Sozu);
    if !has_sozu {
        let mut chance = 40 + run_state.potion_drop_chance_mod;
        if run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::WhiteBeastStatue)
        {
            chance = 100;
        }

        let roll = run_state.rng_pool.potion_rng.random_range(0, 99);
        if roll < chance {
            run_state.potion_drop_chance_mod -= 10;
            let potion_class = run_state.potion_class();
            let potion_id = crate::content::potions::random_potion(
                &mut run_state.rng_pool.potion_rng,
                potion_class,
                false,
            );
            items.push(RewardItem::Potion { potion_id });
        } else {
            run_state.potion_drop_chance_mod += 10;
        }
    }

    // 3. Generate Cards
    let num_cards_eff = adjusted_card_reward_choice_count(run_state, 3);

    items.push(RewardItem::Card {
        cards: generate_card_reward(run_state, num_cards_eff, is_elite),
    });
    if !is_boss && has_prayer_wheel {
        items.push(RewardItem::Card {
            cards: generate_card_reward(run_state, num_cards_eff, is_elite),
        });
    }

    if is_elite || is_boss {
        // Java: MonsterRoomElite.dropReward() → addRelicToRewards(returnRandomRelicTier())
        let relic_id = run_state.random_relic();
        items.push(RewardItem::Relic { relic_id });

        // BlackStar: second relic reward from elites
        if is_elite
            && run_state
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::BlackStar)
        {
            let relic_id2 = run_state.random_relic();
            items.push(RewardItem::Relic {
                relic_id: relic_id2,
            });
        }
    }

    RewardState {
        items,
        skippable: !is_boss,
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        pending_card_choice: None,
    }
}

#[cfg(test)]
mod tests {
    use super::adjusted_card_reward_choice_count;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::run::RunState;

    #[test]
    fn question_card_adds_one_choice() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::QuestionCard));
        assert_eq!(adjusted_card_reward_choice_count(&run_state, 3), 4);
    }

    #[test]
    fn busted_crown_reduces_choices_with_floor_of_one() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::BustedCrown));
        assert_eq!(adjusted_card_reward_choice_count(&run_state, 3), 1);
    }

    #[test]
    fn question_card_and_busted_crown_still_net_one_choice() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::BustedCrown));
        run_state
            .relics
            .push(RelicState::new(RelicId::QuestionCard));
        assert_eq!(adjusted_card_reward_choice_count(&run_state, 3), 2);
    }
}

/// Generates a set of card rewards based on current rarity chances.
/// Applies Room Context to base drop chances.
pub fn generate_card_reward(
    run_state: &mut RunState,
    num_cards: usize,
    is_elite: bool,
) -> Vec<RewardCard> {
    const BLIZZ_GROWTH: i32 = 1;
    const BLIZZ_MAX_OFFSET: i32 = -40;
    const BLIZZ_START_OFFSET: i32 = 5;

    // Room Context modifiers handling mapping
    let base_rare_chance = if is_elite { 10 } else { 3 };
    let base_uncommon_chance = if is_elite { 40 } else { 37 };

    let mut cards: Vec<RewardCard> = Vec::with_capacity(num_cards);
    for _ in 0..num_cards {
        let base_roll = run_state.rng_pool.card_rng.random_range(0, 99);
        let roll = base_roll + run_state.card_blizz_randomizer;

        let mut rare_chance = base_rare_chance;
        let uncommon_chance = base_uncommon_chance;

        // NlothsGift triples rare chance
        if run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::NlothsGift)
        {
            rare_chance *= 3;
        }

        let rarity = if roll < rare_chance {
            CardRarity::Rare
        } else if roll < rare_chance + uncommon_chance {
            CardRarity::Uncommon
        } else {
            CardRarity::Common
        };

        match rarity {
            CardRarity::Rare => {
                run_state.card_blizz_randomizer = BLIZZ_START_OFFSET;
            }
            CardRarity::Common => {
                run_state.card_blizz_randomizer -= BLIZZ_GROWTH;
                if run_state.card_blizz_randomizer < BLIZZ_MAX_OFFSET {
                    run_state.card_blizz_randomizer = BLIZZ_MAX_OFFSET;
                }
            }
            _ => {}
        }

        let pool = crate::engine::campfire_handler::nonempty_card_pool_for_class(
            run_state.player_class,
            rarity,
        );
        if !pool.is_empty() {
            let mut contains_dupe = true;
            let mut candidate = pool[0];
            while contains_dupe {
                contains_dupe = false;
                let idx = run_state
                    .rng_pool
                    .card_rng
                    .random_range(0, (pool.len() - 1) as i32) as usize;
                candidate = pool[idx];
                for c in &cards {
                    if c.id == candidate {
                        contains_dupe = true;
                        break;
                    }
                }
            }
            cards.push(RewardCard::new(candidate, 0));
        }
    }

    if run_state.card_upgraded_chance > 0.0 {
        for reward_card in &mut cards {
            let def = crate::content::cards::get_card_definition(reward_card.id);
            if def.rarity != CardRarity::Rare {
                if run_state
                    .rng_pool
                    .card_rng
                    .random_boolean_chance(run_state.card_upgraded_chance)
                {
                    reward_card.upgrades += 1;
                }
            }
        }
    }

    cards
}
