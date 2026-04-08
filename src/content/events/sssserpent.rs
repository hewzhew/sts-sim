use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

fn gold_reward(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        150
    } else {
        175
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let gold = gold_reward(run_state);
            vec![
                EventChoiceMeta::new(format!(
                    "[Agree] Gain {} Gold. Become Cursed - Doubt.",
                    gold
                )),
                EventChoiceMeta::new("[Disagree] Leave."),
            ]
        }
        1 => {
            // AGREE screen: confirm
            vec![EventChoiceMeta::new("[Confirm]")]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Agree: advance to confirm screen
                    event_state.current_screen = 1;
                }
                _ => {
                    // Disagree: leave
                    event_state.current_screen = 99;
                }
            }
        }
        1 => {
            // Confirm: gain gold + receive curse
            let gold = gold_reward(run_state);
            run_state.gold += gold;
            run_state.add_card_to_deck(CardId::Doubt);
            event_state.current_screen = 99;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
