use crate::content::cards::CardId;
use crate::content::cards::CardRarity;
use crate::state::rewards::RewardCard;
use crate::state::rewards::RewardItem;
use crate::state::rewards::RewardState;
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
    generate_combat_rewards_from_existing(run_state, is_elite, is_boss, Vec::new(), true)
}

pub fn generate_combat_rewards_from_existing(
    run_state: &mut RunState,
    is_elite: bool,
    is_boss: bool,
    existing_items: Vec<RewardItem>,
    include_card_rewards: bool,
) -> RewardState {
    generate_combat_rewards_from_existing_with_escape_gate(
        run_state,
        is_elite,
        is_boss,
        existing_items,
        include_card_rewards,
        true,
    )
}

pub fn generate_combat_rewards_from_existing_with_escape_gate(
    run_state: &mut RunState,
    is_elite: bool,
    is_boss: bool,
    existing_items: Vec<RewardItem>,
    include_card_rewards: bool,
    normal_monster_rewards_allowed: bool,
) -> RewardState {
    let mut items = generate_room_rewards_before_screen(
        run_state,
        is_elite,
        is_boss,
        existing_items,
        normal_monster_rewards_allowed,
    );

    if include_card_rewards {
        items.extend(generate_card_reward_items(
            run_state, is_elite, is_boss, true,
        ));
    }

    RewardState {
        items,
        skippable: !is_boss,
        screen_context: crate::state::rewards::RewardScreenContext::Standard,
        pending_card_choice: None,
    }
}

/// Java room rewards before `CombatRewardScreen.setupItemReward()`.
///
/// Existing rewards are those already inserted into `currRoom.rewards` during
/// combat, such as thief stolen gold. Java then appends/merges normal room
/// gold, elite relic/key rewards, and the potion roll before the reward screen
/// copies the list and optionally appends card rewards.
pub fn generate_room_rewards_before_screen(
    run_state: &mut RunState,
    is_elite: bool,
    is_boss: bool,
    mut items: Vec<RewardItem>,
    normal_monster_rewards_allowed: bool,
) -> Vec<RewardItem> {
    let has_ectoplasm = run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::Ectoplasm);

    // 1. Generate Gold
    if !has_ectoplasm {
        if is_boss {
            let amount = if run_state.is_daily_run {
                100
            } else {
                let mut amount = 100 + run_state.rng_pool.misc_rng.random_range(-5, 5);
                if run_state.ascension_level >= 13 {
                    amount = (amount as f32 * 0.75).round() as i32;
                }
                amount
            };
            add_gold_reward_like_java(&mut items, amount);
        } else if !is_elite && !normal_monster_rewards_allowed {
            // Java skips ordinary MonsterRoom gold when every monster escaped.
        } else {
            let amount = if run_state.is_daily_run {
                if is_elite {
                    30
                } else {
                    15
                }
            } else if is_elite {
                run_state.rng_pool.treasure_rng.random_range(25, 35)
            } else {
                run_state.rng_pool.treasure_rng.random_range(10, 20)
            };
            add_gold_reward_like_java(&mut items, amount);
        }
    }

    if is_elite {
        // Java: MonsterRoomElite.dropReward() runs before addPotionToRewards()
        // and before CombatRewardScreen.setupItemReward() appends card rewards.
        let relic_id = run_state.random_relic();
        items.push(RewardItem::Relic { relic_id });

        // Black Star: second relic reward from elites.
        if run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::BlackStar)
        {
            let relic_id2 = run_state.random_noncampfire_relic_reward();
            items.push(RewardItem::Relic {
                relic_id: relic_id2,
            });
        }

        // Java MonsterRoomElite.addEmeraldKey() appends the key immediately
        // after elite relic rewards, before potion and card rewards are added.
        if run_state.is_final_act_available && !run_state.keys[2] {
            if let Some(node) = run_state.map.get_current_node() {
                if node.has_emerald_key {
                    items.push(RewardItem::EmeraldKey);
                }
            }
        }
    }

    // 2. Generate Potions
    add_potion_reward_like_java_with_room_gate(
        run_state,
        &mut items,
        is_elite || is_boss || normal_monster_rewards_allowed,
    );

    items
}

pub fn add_gold_reward_like_java(items: &mut Vec<RewardItem>, amount: i32) {
    for item in items.iter_mut() {
        if let RewardItem::Gold { amount: existing } = item {
            *existing += amount;
            return;
        }
    }
    items.push(RewardItem::Gold { amount });
}

/// Java `AbstractRoom.addPotionToRewards()` for room types whose base potion
/// chance is eligible before relic and reward-size overrides.
pub fn add_potion_reward_like_java(run_state: &mut RunState, items: &mut Vec<RewardItem>) {
    add_potion_reward_like_java_with_room_gate(run_state, items, true);
}

pub fn add_potion_reward_like_java_with_room_gate(
    run_state: &mut RunState,
    items: &mut Vec<RewardItem>,
    base_chance_allowed: bool,
) {
    let mut chance = if base_chance_allowed {
        40 + run_state.potion_drop_chance_mod
    } else {
        0
    };
    if run_state
        .relics
        .iter()
        .any(|r| r.id == crate::content::relics::RelicId::WhiteBeastStatue)
    {
        chance = 100;
    }
    if items.len() >= 4 {
        chance = 0;
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

/// Java `CombatRewardScreen.setupItemReward()`: appends card rewards after the
/// room's existing rewards have already been copied into the reward screen.
pub fn generate_card_reward_items(
    run_state: &mut RunState,
    is_elite: bool,
    is_boss: bool,
    prayer_wheel_allowed: bool,
) -> Vec<RewardItem> {
    let mut items = Vec::new();
    let num_cards_eff = adjusted_card_reward_choice_count(run_state, 3);

    let cards = generate_card_reward(run_state, num_cards_eff, is_elite);
    if !cards.is_empty() {
        items.push(RewardItem::Card { cards });
    }

    if prayer_wheel_allowed
        && !is_boss
        && !is_elite
        && run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::PrayerWheel)
    {
        let cards = generate_card_reward(run_state, num_cards_eff, is_elite);
        if !cards.is_empty() {
            items.push(RewardItem::Card { cards });
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::{
        adjusted_card_reward_choice_count, reward_card_candidate_pool_for_run,
        select_reward_card_candidate,
    };
    use crate::content::cards::{CardId, CardRarity};
    use crate::content::relics::{RelicId, RelicState, RelicTier};
    use crate::runtime::rng::StsRng;
    use crate::state::rewards::RewardItem;
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
    fn elite_rewards_follow_java_drop_potion_card_order() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::WhiteBeastStatue));
        run_state.relics.push(RelicState::new(RelicId::BlackStar));

        let rewards = super::generate_combat_rewards(&mut run_state, true, false);

        assert!(matches!(rewards.items[0], RewardItem::Gold { .. }));
        assert!(matches!(rewards.items[1], RewardItem::Relic { .. }));
        assert!(matches!(rewards.items[2], RewardItem::Relic { .. }));
        assert!(matches!(rewards.items[3], RewardItem::Potion { .. }));
        assert!(matches!(rewards.items[4], RewardItem::Card { .. }));
    }

    #[test]
    fn black_star_second_elite_relic_skips_campfire_relics_like_java() {
        fn tier_from_relic_rng(rng: &mut StsRng) -> RelicTier {
            match rng.random_range(0, 99) {
                0..=49 => RelicTier::Common,
                50..=82 => RelicTier::Uncommon,
                _ => RelicTier::Rare,
            }
        }

        let seed = (1..10_000)
            .find(|seed| {
                let mut rng = StsRng::new(*seed);
                tier_from_relic_rng(&mut rng) == RelicTier::Common
                    && tier_from_relic_rng(&mut rng) == RelicTier::Rare
            })
            .expect("test seed with Common then Rare relic tier rolls");

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::BlackStar));
        run_state.rng_pool.relic_rng = StsRng::new(seed);
        run_state.common_relic_pool = vec![RelicId::Anchor];
        run_state.uncommon_relic_pool = vec![RelicId::Sundial];
        run_state.rare_relic_pool = vec![
            RelicId::PeacePipe,
            RelicId::Shovel,
            RelicId::Girya,
            RelicId::Mango,
        ];

        let rewards = super::generate_combat_rewards(&mut run_state, true, false);
        let relics = rewards
            .items
            .iter()
            .filter_map(|item| match item {
                RewardItem::Relic { relic_id } => Some(*relic_id),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            relics,
            vec![RelicId::Anchor, RelicId::Mango],
            "Java Black Star uses returnRandomNonCampfireRelic for the second elite relic"
        );
        assert!(
            run_state.rare_relic_pool.is_empty(),
            "Java consumes skipped Peace Pipe/Shovel/Girya candidates while searching the same tier"
        );
    }

    #[test]
    fn existing_combat_rewards_precede_standard_room_rewards_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();

        let rewards = super::generate_combat_rewards_from_existing(
            &mut run_state,
            false,
            false,
            vec![RewardItem::StolenGold { amount: 40 }],
            true,
        );

        assert!(matches!(
            rewards.items[0],
            RewardItem::StolenGold { amount: 40 }
        ));
        assert!(
            matches!(rewards.items[1], RewardItem::Gold { .. }),
            "Java addStolenGoldToRewards happens during combat before normal room gold is appended"
        );
    }

    #[test]
    fn existing_gold_reward_is_incremented_by_standard_room_gold_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();

        let rewards = super::generate_combat_rewards_from_existing(
            &mut run_state,
            false,
            false,
            vec![RewardItem::Gold { amount: 5 }],
            false,
        );

        let gold_rewards = rewards
            .items
            .iter()
            .filter_map(|item| match item {
                RewardItem::Gold { amount } => Some(*amount),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(gold_rewards.len(), 1);
        assert!(
            gold_rewards[0] > 5,
            "Java AbstractRoom.addGoldToRewards increments an existing GOLD reward item instead of appending a second one"
        );
    }

    #[test]
    fn daily_normal_combat_gold_is_fixed_and_does_not_consume_treasure_rng() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.is_daily_run = true;
        let treasure_before = run_state.rng_pool.treasure_rng.counter;

        let rewards = super::generate_combat_rewards_from_existing(
            &mut run_state,
            false,
            false,
            Vec::new(),
            false,
        );

        assert!(matches!(rewards.items[0], RewardItem::Gold { amount: 15 }));
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, treasure_before,
            "Java Daily normal combat uses fixed 15 gold and does not consume treasureRng"
        );
    }

    #[test]
    fn daily_elite_combat_gold_is_fixed_and_does_not_consume_treasure_rng() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.common_relic_pool = vec![RelicId::Anchor];
        let treasure_before = run_state.rng_pool.treasure_rng.counter;
        run_state.is_daily_run = true;

        let rewards = super::generate_combat_rewards_from_existing(
            &mut run_state,
            true,
            false,
            Vec::new(),
            false,
        );

        assert!(matches!(rewards.items[0], RewardItem::Gold { amount: 30 }));
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, treasure_before,
            "Java Daily elite combat uses fixed 30 gold and does not consume treasureRng"
        );
    }

    #[test]
    fn daily_boss_combat_gold_is_fixed_and_does_not_consume_misc_rng() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.is_daily_run = true;
        let misc_before = run_state.rng_pool.misc_rng.counter;

        let rewards = super::generate_combat_rewards_from_existing(
            &mut run_state,
            false,
            true,
            Vec::new(),
            false,
        );

        assert!(matches!(rewards.items[0], RewardItem::Gold { amount: 100 }));
        assert_eq!(
            run_state.rng_pool.misc_rng.counter, misc_before,
            "Java Daily boss combat uses fixed 100 gold and does not consume miscRng"
        );
    }

    #[test]
    fn elite_emerald_key_counts_toward_java_potion_reward_size_gate() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::WhiteBeastStatue));
        run_state.relics.push(RelicState::new(RelicId::BlackStar));
        run_state.map.current_y = 0;
        run_state.map.current_x = 0;
        run_state.map.graph[0][0].has_emerald_key = true;
        let potion_rng_before = run_state.rng_pool.potion_rng.counter;

        let rewards = super::generate_combat_rewards(&mut run_state, true, false);

        assert_eq!(
            run_state.rng_pool.potion_rng.counter,
            potion_rng_before + 1,
            "Java addPotionToRewards still consumes potionRng when rewards.size() >= 4 forces chance to 0"
        );
        assert_eq!(
            run_state.potion_drop_chance_mod, 10,
            "a blocked potion roll still follows the Java miss path and increases blizzardPotionMod"
        );
        assert!(
            !rewards
                .items
                .iter()
                .any(|item| matches!(item, RewardItem::Potion { .. })),
            "Gold + two Black Star relic rewards + Emerald Key reach Java rewards.size() >= 4 before potion generation"
        );
        assert!(matches!(rewards.items[0], RewardItem::Gold { .. }));
        assert!(matches!(rewards.items[1], RewardItem::Relic { .. }));
        assert!(matches!(rewards.items[2], RewardItem::Relic { .. }));
        assert!(matches!(rewards.items[3], RewardItem::EmeraldKey));
        assert!(matches!(rewards.items[4], RewardItem::Card { .. }));
    }

    #[test]
    fn white_beast_overrides_all_escaped_monster_room_potion_chance_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::WhiteBeastStatue));
        let treasure_before = run_state.rng_pool.treasure_rng.counter;
        let potion_before = run_state.rng_pool.potion_rng.counter;

        let rewards = super::generate_combat_rewards_from_existing_with_escape_gate(
            &mut run_state,
            false,
            false,
            Vec::new(),
            false,
            false,
        );

        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, treasure_before,
            "all-escaped ordinary MonsterRoom skips Java normal gold generation"
        );
        assert!(
            run_state.rng_pool.potion_rng.counter > potion_before,
            "Java addPotionToRewards still consumes potionRng even when the base MonsterRoom chance was 0"
        );
        assert_eq!(
            run_state.potion_drop_chance_mod, -10,
            "White Beast Statue overrides the room base chance after the escaped-monster gate"
        );
        assert_eq!(rewards.items.len(), 1);
        assert!(matches!(rewards.items[0], RewardItem::Potion { .. }));
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
