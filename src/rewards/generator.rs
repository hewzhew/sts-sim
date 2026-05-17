use crate::content::cards::CardId;
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
            items.push(RewardItem::Gold { amount });
        } else {
            let amount = if is_elite {
                run_state.rng_pool.treasure_rng.random_range(25, 35)
            } else {
                run_state.rng_pool.treasure_rng.random_range(10, 20)
            };
            items.push(RewardItem::Gold { amount });
        }
    }

    // 2. Generate Potions
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

    if is_elite {
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
    use super::{
        adjusted_card_reward_choice_count, reward_card_candidate_pool_for_run,
        select_reward_card_candidate,
    };
    use crate::content::cards::{CardId, CardRarity};
    use crate::content::relics::{RelicId, RelicState};
    use crate::rewards::state::RewardItem;
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

    #[test]
    fn boss_combat_rewards_do_not_include_normal_relic() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();

        let rewards = super::generate_combat_rewards(&mut run_state, false, true);

        assert!(
            !rewards
                .items
                .iter()
                .any(|item| matches!(item, RewardItem::Relic { .. })),
            "Java MonsterRoomBoss uses the boss chest for boss relics; ordinary combat rewards do not include a normal relic"
        );
    }

    #[test]
    fn normal_reward_pool_remains_current_class_only() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();

        let pool = reward_card_candidate_pool_for_run(&run_state, CardRarity::Common);

        assert!(pool.contains(&CardId::PommelStrike));
        assert!(!pool.contains(&CardId::QuickSlash));
        assert!(!pool.contains(&CardId::BeamCell));
        assert!(!pool.contains(&CardId::BowlingBash));
    }

    #[test]
    fn prismatic_reward_pool_uses_any_color_cards_sorted_by_java_id() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::PrismaticShard));

        let common = reward_card_candidate_pool_for_run(&run_state, CardRarity::Common);
        assert!(common.contains(&CardId::PommelStrike));
        assert!(common.contains(&CardId::QuickSlash));
        assert!(common.contains(&CardId::BeamCell));
        assert!(common.contains(&CardId::BowlingBash));
        assert!(common.windows(2).all(|pair| {
            crate::content::cards::java_id(pair[0]) <= crate::content::cards::java_id(pair[1])
        }));

        let uncommon = reward_card_candidate_pool_for_run(&run_state, CardRarity::Uncommon);
        assert!(uncommon.contains(&CardId::BandageUp));
        assert!(uncommon.contains(&CardId::Tantrum));

        let rare = reward_card_candidate_pool_for_run(&run_state, CardRarity::Rare);
        assert!(rare.contains(&CardId::HandOfGreed));
        assert!(rare.contains(&CardId::Feed));
    }

    #[test]
    fn prismatic_reward_selection_consumes_card_rng_shuffle_seed_before_pick() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::PrismaticShard));
        let before = run_state.rng_pool.card_rng.counter;

        let selected = select_reward_card_candidate(&mut run_state, CardRarity::Common)
            .expect("common any-color reward pool should not be empty");

        assert_eq!(
            run_state.rng_pool.card_rng.counter,
            before + 2,
            "Java CardLibrary.getAnyColorCard(rarity) consumes cardRng.randomLong() for CardGroup.shuffle, then cardRng.random() for sorted getRandomCard"
        );
        assert!(
            reward_card_candidate_pool_for_run(&run_state, CardRarity::Common).contains(&selected)
        );
    }
}

fn has_prismatic_shard(run_state: &RunState) -> bool {
    run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::PrismaticShard)
}

fn any_color_reward_pool_sorted(rarity: CardRarity) -> Vec<CardId> {
    use crate::content::cards::{
        get_card_definition, java_id, CardType, COLORLESS_RARE_POOL, COLORLESS_UNCOMMON_POOL,
        DEFECT_COMMON_POOL, DEFECT_RARE_POOL, DEFECT_UNCOMMON_POOL, IRONCLAD_COMMON_POOL,
        IRONCLAD_RARE_POOL, IRONCLAD_UNCOMMON_POOL, SILENT_COMMON_POOL, SILENT_RARE_POOL,
        SILENT_UNCOMMON_POOL, WATCHER_COMMON_POOL, WATCHER_RARE_POOL, WATCHER_UNCOMMON_POOL,
    };

    let mut pool = [
        IRONCLAD_COMMON_POOL,
        IRONCLAD_UNCOMMON_POOL,
        IRONCLAD_RARE_POOL,
        SILENT_COMMON_POOL,
        SILENT_UNCOMMON_POOL,
        SILENT_RARE_POOL,
        DEFECT_COMMON_POOL,
        DEFECT_UNCOMMON_POOL,
        DEFECT_RARE_POOL,
        WATCHER_COMMON_POOL,
        WATCHER_UNCOMMON_POOL,
        WATCHER_RARE_POOL,
        COLORLESS_UNCOMMON_POOL,
        COLORLESS_RARE_POOL,
    ]
    .into_iter()
    .flatten()
    .copied()
    .filter(|id| {
        let def = get_card_definition(*id);
        def.rarity == rarity
            && def.card_type != CardType::Curse
            && def.card_type != CardType::Status
    })
    .collect::<Vec<_>>();
    pool.sort_by_key(|id| java_id(*id));
    pool
}

fn reward_card_candidate_pool_for_run(run_state: &RunState, rarity: CardRarity) -> Vec<CardId> {
    if has_prismatic_shard(run_state) {
        any_color_reward_pool_sorted(rarity)
    } else {
        crate::engine::campfire_handler::nonempty_card_pool_for_class(
            run_state.player_class,
            rarity,
        )
        .to_vec()
    }
}

fn select_reward_card_candidate(run_state: &mut RunState, rarity: CardRarity) -> Option<CardId> {
    let pool = reward_card_candidate_pool_for_run(run_state, rarity);
    if pool.is_empty() {
        return None;
    }

    if has_prismatic_shard(run_state) {
        // Java CardLibrary.getAnyColorCard(rarity) first calls
        // CardGroup.shuffle(AbstractDungeon.cardRng). getRandomCard(true, rarity)
        // then rebuilds a rarity list, sorts by AbstractCard.cardID, and selects
        // with AbstractDungeon.cardRng. Because the pool is already filtered to
        // one rarity, the shuffle only affects RNG consumption.
        let _shuffle_seed = run_state.rng_pool.card_rng.random_long();
    }

    let idx = run_state.rng_pool.card_rng.random(pool.len() as i32 - 1) as usize;
    Some(pool[idx])
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

        let pool = reward_card_candidate_pool_for_run(run_state, rarity);
        if !pool.is_empty() {
            let mut contains_dupe = true;
            let mut candidate = pool[0];
            while contains_dupe {
                contains_dupe = false;
                let Some(next_candidate) = select_reward_card_candidate(run_state, rarity) else {
                    break;
                };
                candidate = next_candidate;
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

    for reward_card in &mut cards {
        reward_card.upgrades =
            run_state.preview_obtain_card_upgrades(reward_card.id, reward_card.upgrades);
    }

    cards
}
