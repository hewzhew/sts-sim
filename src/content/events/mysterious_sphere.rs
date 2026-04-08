use crate::state::core::{EngineState, EventCombatState, PostCombatReturn};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![
                EventChoiceMeta::new("[Open] Fight the guardians for a rare Relic!"),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        1 => {
            // Confirm fight
            vec![EventChoiceMeta::new("[Fight]")]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Open — advance to confirm screen
                    event_state.current_screen = 1;
                }
                _ => {
                    // Leave
                    event_state.completed = true;
                }
            }
        }
        1 => {
            // Fight! Set up rewards and enter event combat.
            // Java: miscRng.random(45, 55) for gold, returnRandomScreenlessRelic(RARE) for relic
            let gold = run_state.rng_pool.misc_rng.random_range(45, 55);
            let mut rewards = crate::rewards::state::RewardState::new();
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Gold { amount: gold });

            let relic_id =
                run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Relic { relic_id });

            event_state.completed = true;
            run_state.event_state = Some(event_state);

            // Transition to event combat
            *engine_state = EngineState::EventCombat(EventCombatState {
                rewards,
                reward_allowed: true,
                no_cards_in_rewards: false,
                post_combat_return: PostCombatReturn::MapNavigation,
                encounter_key: "2 Orb Walkers",
            });
            return;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
