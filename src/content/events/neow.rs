use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
/// Neow Event — the starting blessing event.
///
/// Java: NeowEvent.java + NeowReward.java
///
/// Two modes:
/// - **miniBlessing** (bossCount == 0): 2 fixed choices — NeowsLament or +10% MaxHP
/// - **blessing** (bossCount > 0): 4 categories randomly selected:
///   Cat 0: 3 cards / rare card / remove card / upgrade card / transform card / random colorless
///   Cat 1: 3 potions / common relic / +10% HP / NeowsLament / +100 gold
///   Cat 2: drawback + rare relic / remove 2 / 3 rare cards / +250 gold / transform 2 / +20% HP / random colorless 2
///   Cat 3: swap starter relic for boss relic
///
/// screens:
///   0 = initial dialog (click to advance to choices)
///   1 = choices displayed
///   2 = completed
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

/// Neow reward types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeowRewardType {
    ThreeEnemyKill,       // NeowsLament relic
    TenPercentHpBonus,    // +10% maxHP
    ThreeCards,           // Choose from 3 cards
    OneRandomRareCard,    // Obtain 1 random rare
    RemoveCard,           // Remove 1 card
    UpgradeCard,          // Upgrade 1 card
    TransformCard,        // Transform 1 card
    RandomColorless,      // Choose from 3 colorless
    ThreeSmallPotions,    // Obtain 3 random potions
    RandomCommonRelic,    // Obtain random common relic
    HundredGold,          // +100 gold
    RandomColorless2,     // Choose from 3 rare colorless (with drawback)
    RemoveTwo,            // Remove 2 cards (with drawback)
    OneRareRelic,         // Obtain random rare relic (with drawback)
    ThreeRareCards,       // Choose from 3 rare cards (with drawback)
    TwoFiftyGold,         // +250 gold (with drawback)
    TransformTwoCards,    // Transform 2 cards (with drawback)
    TwentyPercentHpBonus, // +20% maxHP (with drawback)
    BossRelic,            // Swap starter relic for boss relic
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeowDrawback {
    None,
    TenPercentHpLoss,
    NoGold,
    Curse,
    PercentDamage,
}

/// Stored in event_state.extra_data as encoded i32s:
/// [0] = number of choices
/// [1..N] = reward type for each choice (as i32)
/// [N+1..2N] = drawback type for each choice (as i32)

fn encode_reward(r: NeowRewardType) -> i32 {
    match r {
        NeowRewardType::ThreeEnemyKill => 0,
        NeowRewardType::TenPercentHpBonus => 1,
        NeowRewardType::ThreeCards => 2,
        NeowRewardType::OneRandomRareCard => 3,
        NeowRewardType::RemoveCard => 4,
        NeowRewardType::UpgradeCard => 5,
        NeowRewardType::TransformCard => 6,
        NeowRewardType::RandomColorless => 7,
        NeowRewardType::ThreeSmallPotions => 8,
        NeowRewardType::RandomCommonRelic => 9,
        NeowRewardType::HundredGold => 10,
        NeowRewardType::RandomColorless2 => 11,
        NeowRewardType::RemoveTwo => 12,
        NeowRewardType::OneRareRelic => 13,
        NeowRewardType::ThreeRareCards => 14,
        NeowRewardType::TwoFiftyGold => 15,
        NeowRewardType::TransformTwoCards => 16,
        NeowRewardType::TwentyPercentHpBonus => 17,
        NeowRewardType::BossRelic => 18,
    }
}

fn decode_reward(v: i32) -> NeowRewardType {
    match v {
        0 => NeowRewardType::ThreeEnemyKill,
        1 => NeowRewardType::TenPercentHpBonus,
        2 => NeowRewardType::ThreeCards,
        3 => NeowRewardType::OneRandomRareCard,
        4 => NeowRewardType::RemoveCard,
        5 => NeowRewardType::UpgradeCard,
        6 => NeowRewardType::TransformCard,
        7 => NeowRewardType::RandomColorless,
        8 => NeowRewardType::ThreeSmallPotions,
        9 => NeowRewardType::RandomCommonRelic,
        10 => NeowRewardType::HundredGold,
        11 => NeowRewardType::RandomColorless2,
        12 => NeowRewardType::RemoveTwo,
        13 => NeowRewardType::OneRareRelic,
        14 => NeowRewardType::ThreeRareCards,
        15 => NeowRewardType::TwoFiftyGold,
        16 => NeowRewardType::TransformTwoCards,
        17 => NeowRewardType::TwentyPercentHpBonus,
        18 => NeowRewardType::BossRelic,
        _ => NeowRewardType::TenPercentHpBonus,
    }
}

fn decode_drawback(v: i32) -> NeowDrawback {
    match v {
        0 => NeowDrawback::None,
        1 => NeowDrawback::TenPercentHpLoss,
        2 => NeowDrawback::NoGold,
        3 => NeowDrawback::Curse,
        4 => NeowDrawback::PercentDamage,
        _ => NeowDrawback::None,
    }
}

fn encode_drawback(d: NeowDrawback) -> i32 {
    match d {
        NeowDrawback::None => 0,
        NeowDrawback::TenPercentHpLoss => 1,
        NeowDrawback::NoGold => 2,
        NeowDrawback::Curse => 3,
        NeowDrawback::PercentDamage => 4,
    }
}

fn reward_label(r: NeowRewardType, hp_bonus: i32) -> String {
    match r {
        NeowRewardType::ThreeEnemyKill => {
            "Enemies in your next three combats have 1 HP.".to_string()
        }
        NeowRewardType::TenPercentHpBonus => format!("Max HP +{}", hp_bonus),
        NeowRewardType::ThreeCards => "Choose a card to obtain.".to_string(),
        NeowRewardType::OneRandomRareCard => "Obtain a random rare card.".to_string(),
        NeowRewardType::RemoveCard => "Remove a card.".to_string(),
        NeowRewardType::UpgradeCard => "Upgrade a card.".to_string(),
        NeowRewardType::TransformCard => "Transform a card.".to_string(),
        NeowRewardType::RandomColorless => "Choose a colorless card to obtain.".to_string(),
        NeowRewardType::ThreeSmallPotions => "Obtain 3 random potions.".to_string(),
        NeowRewardType::RandomCommonRelic => "Obtain a random common relic.".to_string(),
        NeowRewardType::HundredGold => "Obtain 100 Gold.".to_string(),
        NeowRewardType::RandomColorless2 => "Choose a rare colorless card to obtain.".to_string(),
        NeowRewardType::RemoveTwo => "Remove 2 cards.".to_string(),
        NeowRewardType::OneRareRelic => "Obtain a random rare relic.".to_string(),
        NeowRewardType::ThreeRareCards => "Choose a rare card to obtain.".to_string(),
        NeowRewardType::TwoFiftyGold => "Obtain 250 Gold.".to_string(),
        NeowRewardType::TransformTwoCards => "Transform 2 cards.".to_string(),
        NeowRewardType::TwentyPercentHpBonus => format!("Max HP +{}", hp_bonus * 2),
        NeowRewardType::BossRelic => {
            "Obtain a random Boss Relic. Lose your starter Relic.".to_string()
        }
    }
}

fn drawback_label(d: NeowDrawback, hp_bonus: i32, dmg: i32) -> String {
    match d {
        NeowDrawback::None => String::new(),
        NeowDrawback::TenPercentHpLoss => format!("Lose {} Max HP. ", hp_bonus),
        NeowDrawback::NoGold => "Lose all Gold. ".to_string(),
        NeowDrawback::Curse => "Obtain a Curse. ".to_string(),
        NeowDrawback::PercentDamage => format!("Take {} damage. ", dmg),
    }
}

/// Set up Neow event state with generated reward choices.
/// Called when entering the Neow room. Uses neowRng seeded from Settings.seed.
pub fn setup_neow_choices(run_state: &mut RunState) {
    let _hp_bonus = (run_state.max_hp as f32 * 0.1) as i32;
    let boss_count = 1; // For simulator assume standard run with prior boss kills

    let mut extra = Vec::new();

    if boss_count == 0 {
        // miniBlessing: 2 fixed choices
        extra.push(2); // count
        extra.push(encode_reward(NeowRewardType::ThreeEnemyKill));
        extra.push(encode_reward(NeowRewardType::TenPercentHpBonus));
        extra.push(encode_drawback(NeowDrawback::None));
        extra.push(encode_drawback(NeowDrawback::None));
    } else {
        // blessing: 4 categories, each pick one reward
        let mut neow_rng = crate::rng::StsRng::new(run_state.seed);

        // Category 0: small bonuses
        let cat0_options = [
            NeowRewardType::ThreeCards,
            NeowRewardType::OneRandomRareCard,
            NeowRewardType::RemoveCard,
            NeowRewardType::UpgradeCard,
            NeowRewardType::TransformCard,
            NeowRewardType::RandomColorless,
        ];
        let r0 = cat0_options[neow_rng.random_range(0, cat0_options.len() as i32 - 1) as usize];

        // Category 1: medium bonuses
        let cat1_options = [
            NeowRewardType::ThreeSmallPotions,
            NeowRewardType::RandomCommonRelic,
            NeowRewardType::TenPercentHpBonus,
            NeowRewardType::ThreeEnemyKill,
            NeowRewardType::HundredGold,
        ];
        let r1 = cat1_options[neow_rng.random_range(0, cat1_options.len() as i32 - 1) as usize];

        // Category 2: big bonuses with drawback
        // Roll drawback first (Java: getRewardDrawbackOptions)
        let drawback_options = [
            NeowDrawback::TenPercentHpLoss,
            NeowDrawback::NoGold,
            NeowDrawback::Curse,
            NeowDrawback::PercentDamage,
        ];
        let drawback =
            drawback_options[neow_rng.random_range(0, drawback_options.len() as i32 - 1) as usize];

        // Build cat2 options with conditional filtering based on drawback
        let mut cat2_options = vec![NeowRewardType::RandomColorless2];
        if drawback != NeowDrawback::Curse {
            cat2_options.push(NeowRewardType::RemoveTwo);
        }
        cat2_options.push(NeowRewardType::OneRareRelic);
        cat2_options.push(NeowRewardType::ThreeRareCards);
        if drawback != NeowDrawback::NoGold {
            cat2_options.push(NeowRewardType::TwoFiftyGold);
        }
        cat2_options.push(NeowRewardType::TransformTwoCards);
        if drawback != NeowDrawback::TenPercentHpLoss {
            cat2_options.push(NeowRewardType::TwentyPercentHpBonus);
        }
        let r2 = cat2_options[neow_rng.random_range(0, cat2_options.len() as i32 - 1) as usize];

        // Category 3: boss relic (fixed)
        let r3 = NeowRewardType::BossRelic;

        extra.push(4); // count
        extra.push(encode_reward(r0));
        extra.push(encode_reward(r1));
        extra.push(encode_reward(r2));
        extra.push(encode_reward(r3));
        extra.push(encode_drawback(NeowDrawback::None));
        extra.push(encode_drawback(NeowDrawback::None));
        extra.push(encode_drawback(drawback));
        extra.push(encode_drawback(NeowDrawback::None)); // Boss relic has no drawback enum (it IS the drawback)
    }

    run_state.event_state = Some(EventState {
        id: crate::state::events::EventId::Neow,
        current_screen: 0,
        internal_state: 0,
        completed: false,
        combat_pending: false,
        extra_data: extra,
    });
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    let hp_bonus = (run_state.max_hp as f32 * 0.1) as i32;
    let percent_dmg = run_state.current_hp / 10 * 3;

    match event_state.current_screen {
        0 => {
            // Initial dialog — single "proceed" button
            vec![EventChoiceMeta::new("[Proceed]")]
        }
        1 => {
            // Display reward choices
            let count = event_state.extra_data[0] as usize;
            let mut choices = Vec::new();
            for i in 0..count {
                let reward = decode_reward(event_state.extra_data[1 + i]);
                let drawback = decode_drawback(event_state.extra_data[1 + count + i]);
                let db_label = drawback_label(drawback, hp_bonus, percent_dmg);
                let rw_label = reward_label(reward, hp_bonus);
                choices.push(EventChoiceMeta::new(format!("{}{}", db_label, rw_label)));
            }
            choices
        }
        _ => {
            vec![EventChoiceMeta::new("[Leave]")]
        }
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Advance to choices screen
            event_state.current_screen = 1;
        }
        1 => {
            let count = event_state.extra_data[0] as usize;
            if choice_idx < count {
                let reward = decode_reward(event_state.extra_data[1 + choice_idx]);
                let drawback = decode_drawback(event_state.extra_data[1 + count + choice_idx]);

                // Apply drawback first (Java: NeowReward.activate())
                apply_drawback(run_state, drawback);

                // Apply reward
                apply_reward(engine_state, run_state, reward, &mut event_state);
            }
            event_state.current_screen = 2;
            event_state.completed = true;
        }
        _ => {
            // Leave — already completed
        }
    }

    run_state.event_state = Some(event_state);
}

fn apply_drawback(run_state: &mut RunState, drawback: NeowDrawback) {
    let hp_bonus = (run_state.max_hp as f32 * 0.1) as i32;
    match drawback {
        NeowDrawback::None => {}
        NeowDrawback::TenPercentHpLoss => {
            run_state.lose_max_hp_with_source(
                hp_bonus,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowDrawback::NoGold => {
            run_state.set_gold_with_source(
                0,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowDrawback::Curse => {
            // Add a random curse to deck
            // Java: AbstractDungeon.getCardWithoutRng(CardRarity.CURSE)
            run_state.add_card_to_deck(crate::content::cards::CardId::Regret);
        }
        NeowDrawback::PercentDamage => {
            let dmg = run_state.current_hp / 10 * 3;
            run_state.set_current_hp_with_source(
                (run_state.current_hp - dmg).max(1),
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
    }
}

fn apply_reward(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    reward: NeowRewardType,
    _event_state: &mut EventState,
) {
    let hp_bonus = (run_state.max_hp as f32 * 0.1) as i32;

    match reward {
        NeowRewardType::ThreeEnemyKill => {
            // Obtain NeowsLament relic
            let relic_id = crate::content::relics::RelicId::NeowsLament;
            if let Some(next_state) = run_state.obtain_relic_with_source(
                relic_id,
                crate::state::core::EngineState::EventRoom,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            ) {
                *engine_state = next_state;
            }
        }
        NeowRewardType::TenPercentHpBonus => {
            run_state.gain_max_hp_with_source(
                hp_bonus,
                hp_bonus,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowRewardType::TwentyPercentHpBonus => {
            run_state.gain_max_hp_with_source(
                hp_bonus * 2,
                hp_bonus * 2,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowRewardType::HundredGold => {
            run_state.change_gold_with_source(
                100,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowRewardType::TwoFiftyGold => {
            run_state.change_gold_with_source(
                250,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
        }
        NeowRewardType::RandomCommonRelic => {
            if let Some(relic_id) = run_state.common_relic_pool.pop() {
                if let Some(next_state) = run_state.obtain_relic_with_source(
                    relic_id,
                    crate::state::core::EngineState::EventRoom,
                    DomainEventSource::Event(crate::state::events::EventId::Neow),
                ) {
                    *engine_state = next_state;
                }
            }
        }
        NeowRewardType::OneRareRelic => {
            if let Some(relic_id) = run_state.rare_relic_pool.pop() {
                if let Some(next_state) = run_state.obtain_relic_with_source(
                    relic_id,
                    crate::state::core::EngineState::EventRoom,
                    DomainEventSource::Event(crate::state::events::EventId::Neow),
                ) {
                    *engine_state = next_state;
                }
            }
        }
        NeowRewardType::BossRelic => {
            // Remove starter relic (first relic)
            let _ = run_state.remove_relic_at_with_source(
                0,
                DomainEventSource::Event(crate::state::events::EventId::Neow),
            );
            // Obtain random boss relic
            if let Some(relic_id) = run_state.boss_relic_pool.pop() {
                // Trigger effects like Pandora's Box transforming the deck
                if let Some(next_state) = run_state.obtain_relic_with_source(
                    relic_id,
                    crate::state::core::EngineState::EventRoom,
                    DomainEventSource::Event(crate::state::events::EventId::Neow),
                ) {
                    *engine_state = next_state;
                }
            }
        }
        NeowRewardType::OneRandomRareCard => {
            // Get a random rare card from the player's class pool and add to deck
            let pool = crate::engine::campfire_handler::nonempty_card_pool_for_class(
                run_state.player_class,
                crate::content::cards::CardRarity::Rare,
            );
            if !pool.is_empty() {
                let idx = run_state
                    .rng_pool
                    .card_rng
                    .random_range(0, (pool.len() - 1) as i32) as usize;
                run_state.add_card_to_deck(pool[idx]);
            }
        }
        NeowRewardType::RemoveCard => {
            // Trigger RunPendingChoice for card removal
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 1,
                max_choices: 1,
                reason: RunPendingChoiceReason::Purge,
                return_state: Box::new(EngineState::EventRoom),
            });
        }
        NeowRewardType::RemoveTwo => {
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 2,
                max_choices: 2,
                reason: RunPendingChoiceReason::Purge,
                return_state: Box::new(EngineState::EventRoom),
            });
        }
        NeowRewardType::UpgradeCard => {
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 1,
                max_choices: 1,
                reason: RunPendingChoiceReason::Upgrade,
                return_state: Box::new(EngineState::EventRoom),
            });
        }
        NeowRewardType::TransformCard | NeowRewardType::TransformTwoCards => {
            let count = if reward == NeowRewardType::TransformTwoCards {
                2
            } else {
                1
            };
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: count,
                max_choices: count,
                reason: RunPendingChoiceReason::Transform,
                return_state: Box::new(EngineState::EventRoom),
            });
        }
        NeowRewardType::ThreeSmallPotions => {
            // Add 3 random potions to empty potion slots
            let pc = run_state.potion_class();
            for _ in 0..3 {
                let potion_id = crate::content::potions::random_potion(
                    &mut run_state.rng_pool.potion_rng,
                    pc,
                    false,
                );
                let _ = run_state.obtain_potion_with_source(
                    crate::content::potions::Potion::new(potion_id, 0),
                    DomainEventSource::Event(crate::state::events::EventId::Neow),
                );
            }
        }
        NeowRewardType::ThreeCards => {
            // Generate 3 card choices from player's class pool (mixed rarity)
            let cards = generate_neow_class_cards(run_state, false);
            let mut reward_state = crate::rewards::state::RewardState::new();
            reward_state
                .items
                .push(crate::rewards::state::RewardItem::Card { cards });
            *engine_state = EngineState::RewardScreen(reward_state);
        }
        NeowRewardType::ThreeRareCards => {
            // Generate 3 rare card choices from player's class pool
            let cards = generate_neow_class_cards(run_state, true);
            let mut reward_state = crate::rewards::state::RewardState::new();
            reward_state
                .items
                .push(crate::rewards::state::RewardItem::Card { cards });
            *engine_state = EngineState::RewardScreen(reward_state);
        }
        NeowRewardType::RandomColorless => {
            // 3 colorless cards (uncommon or rare)
            let cards = generate_neow_colorless_cards(run_state, false);
            let mut reward_state = crate::rewards::state::RewardState::new();
            reward_state
                .items
                .push(crate::rewards::state::RewardItem::Card { cards });
            *engine_state = EngineState::RewardScreen(reward_state);
        }
        NeowRewardType::RandomColorless2 => {
            // 3 rare colorless cards
            let cards = generate_neow_colorless_cards(run_state, true);
            let mut reward_state = crate::rewards::state::RewardState::new();
            reward_state
                .items
                .push(crate::rewards::state::RewardItem::Card { cards });
            *engine_state = EngineState::RewardScreen(reward_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_reward, NeowRewardType};
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn boss_relic_reward_emits_relic_loss_and_gain() {
        let mut run_state = RunState::new(11, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::Neow));
        run_state.boss_relic_pool = vec![RelicId::BlackStar];
        let mut engine_state = EngineState::EventRoom;
        let mut event_state = EventState::new(EventId::Neow);

        apply_reward(
            &mut engine_state,
            &mut run_state,
            NeowRewardType::BossRelic,
            &mut event_state,
        );

        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicLost {
                relic_id: RelicId::BurningBlood,
                source: DomainEventSource::Event(EventId::Neow),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::BlackStar,
                source: DomainEventSource::Event(EventId::Neow),
            }
        )));
        assert_eq!(run_state.relics[0].id, RelicId::BlackStar);
    }
}

/// Generate 3 cards from the player's class card pool.
/// If `rare_only` is true, all 3 cards come from the Rare pool.
/// Otherwise, uses the same rarity roll as DreamCatcher (standard card reward rarity distribution).
fn generate_neow_class_cards(
    run_state: &mut RunState,
    rare_only: bool,
) -> Vec<crate::rewards::state::RewardCard> {
    use crate::content::cards::CardRarity;
    let mut cards = Vec::new();
    for _ in 0..3 {
        let rarity = if rare_only {
            CardRarity::Rare
        } else {
            let roll = run_state.rng_pool.card_rng.random_range(0, 99);
            if roll < 3 {
                CardRarity::Rare
            } else if roll < 40 {
                CardRarity::Uncommon
            } else {
                CardRarity::Common
            }
        };
        let pool = crate::engine::campfire_handler::nonempty_card_pool_for_class(
            run_state.player_class,
            rarity,
        );
        if !pool.is_empty() {
            let idx = run_state
                .rng_pool
                .card_rng
                .random_range(0, (pool.len() - 1) as i32) as usize;
            cards.push(crate::rewards::state::RewardCard::new(pool[idx], 0));
        }
    }
    cards
}

/// Colorless card pools (matching Java's AbstractDungeon.getColorlessCardFromPool)
const COLORLESS_UNCOMMON_POOL: &[crate::content::cards::CardId] = &[
    crate::content::cards::CardId::BandageUp,
    crate::content::cards::CardId::Blind,
    crate::content::cards::CardId::DarkShackles,
    crate::content::cards::CardId::DeepBreath,
    crate::content::cards::CardId::Discovery,
    crate::content::cards::CardId::DramaticEntrance,
    crate::content::cards::CardId::Enlightenment,
    crate::content::cards::CardId::Finesse,
    crate::content::cards::CardId::FlashOfSteel,
    crate::content::cards::CardId::Forethought,
    crate::content::cards::CardId::GoodInstincts,
    crate::content::cards::CardId::Impatience,
    crate::content::cards::CardId::JackOfAllTrades,
    crate::content::cards::CardId::MindBlast,
    crate::content::cards::CardId::Panacea,
    crate::content::cards::CardId::PanicButton,
    crate::content::cards::CardId::Purity,
    crate::content::cards::CardId::SwiftStrike,
    crate::content::cards::CardId::Trip,
];

const COLORLESS_RARE_POOL: &[crate::content::cards::CardId] = &[
    crate::content::cards::CardId::Apotheosis,
    crate::content::cards::CardId::Chrysalis,
    crate::content::cards::CardId::HandOfGreed,
    crate::content::cards::CardId::Magnetism,
    crate::content::cards::CardId::MasterOfStrategy,
    crate::content::cards::CardId::Mayhem,
    crate::content::cards::CardId::Metamorphosis,
    crate::content::cards::CardId::Panache,
    crate::content::cards::CardId::SadisticNature,
    crate::content::cards::CardId::SecretTechnique,
    crate::content::cards::CardId::SecretWeapon,
    crate::content::cards::CardId::TheBomb,
    crate::content::cards::CardId::ThinkingAhead,
    crate::content::cards::CardId::Transmutation,
    crate::content::cards::CardId::Violence,
];

/// Generate 3 colorless cards for Neow rewards.
/// If `rare_only` is true, picks from rare colorless pool only.
/// Otherwise picks from uncommon + rare with standard rarity weighting.
fn generate_neow_colorless_cards(
    run_state: &mut RunState,
    rare_only: bool,
) -> Vec<crate::rewards::state::RewardCard> {
    let mut cards = Vec::new();
    for _ in 0..3 {
        let pool = if rare_only {
            COLORLESS_RARE_POOL
        } else {
            let roll = run_state.rng_pool.card_rng.random_range(0, 99);
            if roll < 30 {
                COLORLESS_RARE_POOL
            } else {
                COLORLESS_UNCOMMON_POOL
            }
        };
        if !pool.is_empty() {
            let idx = run_state
                .rng_pool
                .card_rng
                .random_range(0, (pool.len() - 1) as i32) as usize;
            cards.push(crate::rewards::state::RewardCard::new(pool[idx], 0));
        }
    }
    cards
}
