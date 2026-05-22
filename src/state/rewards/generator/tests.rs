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
fn boss_combat_card_reward_uses_java_boss_rare_override() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.relics.clear();

    let rewards = super::generate_combat_rewards(&mut run_state, false, true);
    let cards = rewards
        .items
        .iter()
        .find_map(|item| match item {
            RewardItem::Card { cards } => Some(cards),
            _ => None,
        })
        .expect("boss combat should append a card reward");

    assert!(
        cards.iter().all(|card| {
            crate::content::cards::get_card_definition(card.id).rarity == CardRarity::Rare
        }),
        "Java MonsterRoomBoss.getCardRarity always returns RARE for boss combat rewards"
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
    assert!(reward_card_candidate_pool_for_run(&run_state, CardRarity::Common).contains(&selected));
}
