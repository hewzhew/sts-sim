use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let curse_chance = if run_state.ascension_level >= 15 { 100 } else { 50 };
            vec![
                EventChoiceMeta::new(format!("[Open] {}% chance of Writhe. Obtain a random Relic.", curse_chance)),
                EventChoiceMeta::new("[Leave]"),
            ]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Open: relic + possible Writhe curse
                    // Java: always calls miscRng.randomBoolean(), then overrides at A15
                    let mut gets_curse = run_state.rng_pool.misc_rng.random_boolean();
                    if run_state.ascension_level >= 15 {
                        gets_curse = true;
                    }
                    if gets_curse {
                        run_state.add_card_to_deck(CardId::Writhe);
                    }
                    let relic_id = run_state.random_relic();
                    run_state.relics.push(crate::content::relics::RelicState::new(relic_id));
                    event_state.current_screen = 1;
                },
                _ => {
                    event_state.completed = true;
                },
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
