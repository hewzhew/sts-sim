// Java: Colosseum (city)
// Screen 0 (INTRO): [Proceed] → fight description
// Screen 1 (FIGHT): [Fight!] → combat with "Colosseum Slavers" (no rewards, rewardAllowed=false)
// After first combat → reopen() → Screen 2 (POST_COMBAT):
//   [Flee] → leave (openMap)
//   [Fight] → combat with "Colosseum Nobs" (rewards: RARE relic, UNCOMMON relic, 100g)
// Screen 3 (LEAVE): [Leave] → openMap

use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::{EngineState, EventCombatState, PostCombatReturn};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Introduction
            vec![EventChoiceMeta::new("[Proceed]")]
        }
        1 => {
            // Ready to fight
            vec![EventChoiceMeta::new("[Fight!]")]
        }
        2 => {
            // Post-first-combat: choose to fight Nobs or flee
            vec![
                EventChoiceMeta::new("[Flee] Leave the Colosseum."),
                EventChoiceMeta::new("[Fight] Challenge the Nobs for riches!"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Intro → ready to fight
            event_state.current_screen = 1;
        }
        1 => {
            // First fight: Colosseum Slavers (no rewards)
            // Java: rewardAllowed = false, enterCombatFromImage()
            // After combat, return to EventRoom → screen 2 (POST_COMBAT)
            event_state.current_screen = 2;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::EventCombat(EventCombatState {
                rewards: RewardState::new(),
                reward_allowed: false,
                no_cards_in_rewards: true,
                post_combat_return: PostCombatReturn::EventRoom,
                encounter_key: "Colosseum Slavers",
            });
            return;
        }
        2 => {
            match choice_idx {
                0 => {
                    // Flee — leave the Colosseum
                    event_state.completed = true;
                }
                _ => {
                    // Fight Nobs: set up rewards BEFORE combat (matches Java)
                    // Java: addRelicToRewards(RARE), addRelicToRewards(UNCOMMON), addGoldToRewards(100)
                    let mut rewards = RewardState::new();

                    let rare_relic =
                        run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
                    rewards.items.push(RewardItem::Relic {
                        relic_id: rare_relic,
                    });

                    let uncommon_relic = run_state
                        .random_screenless_relic(crate::content::relics::RelicTier::Uncommon);
                    rewards.items.push(RewardItem::Relic {
                        relic_id: uncommon_relic,
                    });

                    // Gold reward
                    rewards.items.push(RewardItem::Gold { amount: 100 });

                    event_state.current_screen = 3;
                    event_state.completed = true;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::EventCombat(EventCombatState {
                        rewards,
                        reward_allowed: true,
                        no_cards_in_rewards: false,
                        post_combat_return: PostCombatReturn::MapNavigation,
                        encounter_key: "Colosseum Nobs",
                    });
                    return;
                }
            }
        }
        _ => {
            // Post-second-combat leave
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
