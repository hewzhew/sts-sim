use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

// internal_state packing:
// bits[0..3]  = numRewards (how many times player has looted, 0-3)
// bits[4..11] = encounterChance (0-100)
// bits[12..13] = reward0 type (0=Gold, 1=Nothing, 2=Relic)
// bits[14..15] = reward1 type
// bits[16..17] = reward2 type

const GOLD_REWARD: i32 = 30;
const CHANCE_RAMP: i32 = 25;

fn num_rewards(s: i32) -> i32 {
    s & 0xF
}
fn encounter_chance(s: i32) -> i32 {
    (s >> 4) & 0xFF
}
fn reward_type(s: i32, idx: i32) -> i32 {
    (s >> (12 + idx * 2)) & 0x3
}

fn set_num_rewards(s: &mut i32, n: i32) {
    *s = (*s & !0xF) | (n & 0xF);
}
fn set_encounter_chance(s: &mut i32, c: i32) {
    *s = (*s & !(0xFF << 4)) | ((c & 0xFF) << 4);
}
fn set_reward_types(s: &mut i32, r0: i32, r1: i32, r2: i32) {
    *s = (*s & !(0x3F << 12)) | ((r0 & 0x3) << 12) | ((r1 & 0x3) << 14) | ((r2 & 0x3) << 16);
}

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let chance = encounter_chance(event_state.internal_state);
            vec![
                EventChoiceMeta::new(format!("[Search] {}% chance of a fight.", chance)),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        // Combat triggered
        1 => vec![EventChoiceMeta::new("[Fight!]")],
        // Post-loot or post-combat
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Search: check encounter chance
                    let chance = encounter_chance(event_state.internal_state);
                    let roll = run_state.rng_pool.misc_rng.random_range(0, 99);
                    if roll < chance {
                        // Combat!
                        // Java: addGoldToRewards(25-35) + remaining rewards via addGoldToRewards/addRelicToRewards
                        let mut rewards = crate::rewards::state::RewardState::new();
                        // Pre-combat gold reward (Java: miscRng.random(25, 35))
                        let combat_gold = run_state.rng_pool.misc_rng.random_range(25, 35);
                        rewards.items.push(crate::rewards::state::RewardItem::Gold {
                            amount: combat_gold,
                        });
                        // Remaining unclaimed search rewards
                        let n = num_rewards(event_state.internal_state) as usize;
                        for i in n..3 {
                            let rt = reward_type(event_state.internal_state, i as i32);
                            match rt {
                                0 => rewards.items.push(crate::rewards::state::RewardItem::Gold {
                                    amount: GOLD_REWARD,
                                }),
                                2 => {
                                    let relic_id = run_state.random_relic();
                                    rewards
                                        .items
                                        .push(crate::rewards::state::RewardItem::Relic {
                                            relic_id,
                                        });
                                }
                                _ => {}
                            }
                        }
                        event_state.current_screen = 2;
                        event_state.completed = true;
                        run_state.event_state = Some(event_state);
                        *engine_state =
                            EngineState::EventCombat(crate::state::core::EventCombatState {
                                rewards,
                                reward_allowed: true,
                                no_cards_in_rewards: false,
                                post_combat_return:
                                    crate::state::core::PostCombatReturn::MapNavigation,
                                encounter_key: "Dead Adventurer",
                            });
                        return;
                    }
                    // Safe loot
                    let n = num_rewards(event_state.internal_state);
                    let rt = reward_type(event_state.internal_state, n);
                    apply_reward(run_state, rt);

                    let new_n = n + 1;
                    set_num_rewards(&mut event_state.internal_state, new_n);
                    let new_chance = encounter_chance(event_state.internal_state) + CHANCE_RAMP;
                    set_encounter_chance(&mut event_state.internal_state, new_chance);

                    if new_n >= 3 {
                        // All 3 rewards claimed, done
                        event_state.current_screen = 2;
                    }
                    // else stay on screen 0
                }
                _ => {
                    // Leave
                    event_state.current_screen = 2;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

fn apply_reward(run_state: &mut RunState, reward_type: i32) {
    match reward_type {
        0 => {
            run_state.change_gold_with_source(
                GOLD_REWARD,
                DomainEventSource::Event(EventId::DeadAdventurer),
            );
        } // Gold
        2 => {
            // Relic
            let relic_id = run_state.random_relic();
            let _ = run_state.obtain_relic_with_source(
                relic_id,
                EngineState::EventRoom,
                DomainEventSource::Event(EventId::DeadAdventurer),
            );
        }
        _ => {} // Nothing
    }
}

/// Initialize DeadAdventurer internal_state with shuffled rewards + starting encounter chance
pub fn init_dead_adventurer_state(run_state: &mut RunState) -> i32 {
    let base_chance = if run_state.ascension_level >= 15 {
        35
    } else {
        25
    };
    // Shuffle reward types: [Gold(0), Nothing(1), Relic(2)]
    // Java: Collections.shuffle(rewards, new Random(miscRng.randomLong()))
    let mut rewards = [0i32, 1, 2];
    crate::runtime::rng::shuffle_with_random_long(&mut rewards, &mut run_state.rng_pool.misc_rng);
    let mut s = 0i32;
    set_num_rewards(&mut s, 0);
    set_encounter_chance(&mut s, base_chance);
    set_reward_types(&mut s, rewards[0], rewards[1], rewards[2]);
    s
}
