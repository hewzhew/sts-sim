// Java: GremlinWheelGame (shrines) — "Wheel of Change"
// Spin wheel → result 0-5:
//   0 = Gain gold (100/200/300 by act)
//   1 = Obtain random relic
//   2 = Heal to full HP
//   3 = Obtain Decay curse
//   4 = Remove a card (grid-select purge)
//   5 = Lose HP (10% max, 15% at A15+)
// Screen 0: [Spin] → spin result stored in internal_state
// Screen 1: [Leave] after result applied

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Spin] Spin the wheel!")],
        1 => {
            // Show result description
            let desc = match event_state.internal_state {
                0 => {
                    let gold = match run_state.act_num {
                        1 => 100,
                        2 => 200,
                        _ => 300,
                    };
                    format!("You gained {} Gold!", gold)
                }
                1 => "You obtained a random relic!".to_string(),
                2 => "You healed to full HP!".to_string(),
                3 => "A dark curse falls upon you... Gained Decay.".to_string(),
                4 => "The spirits consume a card...".to_string(),
                _ => {
                    let pct = if run_state.ascension_level >= 15 {
                        15
                    } else {
                        10
                    };
                    format!("The wheel damages you! Lost {}% max HP.", pct)
                }
            };
            vec![EventChoiceMeta::new(format!("{} [Leave]", desc))]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    if event_state.completed {
        run_state.event_state = Some(event_state);
        return;
    }

    match event_state.current_screen {
        0 => {
            // Spin: roll 0-5 using miscRng (Java: miscRng.random(0, 5))
            let result = run_state.rng_pool.misc_rng.random_range(0, 5) as i32;

            // Apply result immediately
            match result {
                0 => {
                    // Gold
                    let gold = match run_state.act_num {
                        1 => 100,
                        2 => 200,
                        _ => 300,
                    };
                    run_state.gold += gold;
                }
                1 => {
                    // Random relic — Java: addRelicToRewards(r) + combatRewardScreen.open()
                    let relic = run_state.random_relic();
                    let mut rewards = crate::rewards::state::RewardState::new();
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Relic { relic_id: relic });
                    event_state.internal_state = result;
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RewardScreen(rewards);
                    return;
                }
                2 => {
                    // Heal to full
                    run_state.current_hp = run_state.max_hp;
                }
                3 => {
                    // Obtain Decay curse
                    run_state.add_card_to_deck(crate::content::cards::CardId::Decay);
                }
                4 => {
                    // Remove a card (Java: grid-select purge)
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        min_choices: 1,
                        max_choices: 1,
                        reason: RunPendingChoiceReason::Purge,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                }
                _ => {
                    // Lose HP
                    let pct = if run_state.ascension_level >= 15 {
                        0.15f32
                    } else {
                        0.10f32
                    };
                    let damage = (run_state.max_hp as f32 * pct) as i32;
                    run_state.current_hp = (run_state.current_hp - damage).max(1);
                }
            }

            event_state.internal_state = result;
            event_state.current_screen = 1;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
