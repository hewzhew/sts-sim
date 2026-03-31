use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Java: 3rd option depends on floorNum % 50
            let desire_text = if run_state.floor_num % 50 <= 40 {
                "[Desire] Gain 999 Gold. Obtain 2 Normality."
            } else {
                "[Desire] Heal to full HP. Obtain Doubt."
            };
            vec![
                EventChoiceMeta::new("[Fight] Battle a boss for rewards."),
                EventChoiceMeta::new("[Remember] Upgrade all cards. Obtain Mark of the Bloom."),
                EventChoiceMeta::new(desire_text),
            ]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Fight: battle Act 1 boss
                    // Java: shuffle boss list with miscRng.randomLong()
                    let mut boss_indices = [0u8, 1, 2]; // Guardian, Hexaghost, SlimeBoss
                    crate::rng::shuffle_with_random_long(&mut boss_indices, &mut run_state.rng_pool.misc_rng);

                    // Java: addGoldToRewards(A13>=13 ? 25 : 50) + addRelicToRewards(RARE)
                    let mut rewards = crate::state::reward::RewardState::new();
                    let gold = if run_state.ascension_level >= 13 { 25 } else { 50 };
                    rewards.items.push(crate::state::reward::RewardItem::Gold { amount: gold });
                    let rare_relic = run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
                    rewards.items.push(crate::state::reward::RewardItem::Relic { relic_id: rare_relic });

                    event_state.current_screen = 1;
                    event_state.completed = true;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::EventCombat(crate::state::core::EventCombatState {
                        rewards,
                        reward_allowed: true,
                        no_cards_in_rewards: false,
                        post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
                        encounter_key: "Mind Bloom Boss",
                    });
                    return;
                },
                1 => {
                    // Remember: upgrade all upgradable cards + MarkOfTheBloom
                    // Java checks canUpgrade() — most cards: upgrades == 0, SearingBlow: always
                    for card in run_state.master_deck.iter_mut() {
                        let def = crate::content::cards::get_card_definition(card.id);
                        let can_upgrade = match def.rarity {
                            crate::content::cards::CardRarity::Curse => false,
                            _ => {
                                // SearingBlow can upgrade infinitely; others only once
                                card.id == crate::content::cards::CardId::SearingBlow || card.upgrades == 0
                            }
                        };
                        if can_upgrade {
                            card.upgrades += 1;
                        }
                    }
                    run_state.relics.push(RelicState::new(RelicId::MarkOfTheBloom));
                    event_state.current_screen = 1;
                },
                _ => {
                    // Desire: depends on floorNum % 50
                    if run_state.floor_num % 50 <= 40 {
                        // Normal path: 999 gold + 2 Normality
                        run_state.gold += 999;
                        run_state.add_card_to_deck(CardId::Normality);
                        run_state.add_card_to_deck(CardId::Normality);
                    } else {
                        // High floor path: heal to full + Doubt curse
                        run_state.current_hp = run_state.max_hp;
                        run_state.add_card_to_deck(CardId::Doubt);
                    }
                    event_state.current_screen = 1;
                },
            }
        },
        _ => { event_state.completed = true; }
    }

    run_state.event_state = Some(event_state);
}
