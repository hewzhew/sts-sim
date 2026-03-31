use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }
    
    let gold_amt = if run_state.ascension_level >= 15 { 50 } else { 100 };
    vec![
        EventChoiceMeta::new(format!("[Pray] Gain {} Gold.", gold_amt)),
        EventChoiceMeta::new("[Desecrate] Gain 275 Gold. Become Cursed - Regret."),
        EventChoiceMeta::new("[Leave]"),
    ]
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    if let EngineState::EventRoom = engine_state {
        let (completed, current_screen) = if let Some(es) = &run_state.event_state {
            (es.completed, es.current_screen)
        } else {
            return;
        };

        if completed {
            return;
        }

        if current_screen == 0 {
            match choice_idx {
                0 => { // Pray: +gold (100 or 50 at A15)
                    let gold_amt = if run_state.ascension_level >= 15 { 50 } else { 100 };
                    run_state.gold += gold_amt;
                    if let Some(es) = &mut run_state.event_state {
                        es.current_screen = 1; // Transition to leave screen
                    }
                },
                1 => { // Desecrate: +275 Gold, +Regret (via add_card_to_deck for Omamori check)
                    run_state.gold += 275;
                    run_state.add_card_to_deck(CardId::Regret);

                    if let Some(es) = &mut run_state.event_state {
                        es.current_screen = 1;
                    }
                },
                _ => { // Leave
                    if let Some(es) = &mut run_state.event_state {
                        es.completed = true;
                    }
                }
            }
        } else {
            if let Some(es) = &mut run_state.event_state {
                es.completed = true;
            }
        }
    }
}
