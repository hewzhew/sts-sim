use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let hp_loss = ((run_state.max_hp as f32) * 0.5).ceil() as i32;
            let count = if run_state.ascension_level >= 15 { 3 } else { 5 };
            vec![
                EventChoiceMeta::new(format!("[Accept] Lose {} Max HP. Obtain {} Apparitions.", hp_loss, count)),
                EventChoiceMeta::new("[Refuse]"),
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
                    // Accept: lose 50% max HP, gain Apparitions
                    let mut hp_loss = ((run_state.max_hp as f32) * 0.5).ceil() as i32;
                    if hp_loss >= run_state.max_hp {
                        hp_loss = run_state.max_hp - 1;
                    }
                    run_state.max_hp -= hp_loss;
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                    let count = if run_state.ascension_level >= 15 { 3 } else { 5 };
                    for _ in 0..count {
                        run_state.add_card_to_deck(CardId::Apparition);
                    }
                    event_state.current_screen = 1;
                },
                _ => {
                    event_state.current_screen = 1;
                },
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
