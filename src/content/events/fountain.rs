use crate::content::cards::{CardId, CardType, get_card_definition};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let has_removable_curses = run_state.master_deck.iter().any(|c| {
                let def = get_card_definition(c.id);
                def.card_type == CardType::Curse
                    && c.id != CardId::AscendersBane
                    && c.id != CardId::CurseOfTheBell
                    && c.id != CardId::Necronomicurse
            });
            if has_removable_curses {
                vec![
                    EventChoiceMeta::new("[Drink] Remove all removable Curses."),
                    EventChoiceMeta::new("[Leave]"),
                ]
            } else {
                vec![
                    EventChoiceMeta::disabled("[Drink] No removable Curses.", "No curses"),
                    EventChoiceMeta::new("[Leave]"),
                ]
            }
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
                    // Remove all removable curses
                    run_state.master_deck.retain(|c| {
                        let def = get_card_definition(c.id);
                        def.card_type != CardType::Curse
                            || c.id == CardId::AscendersBane
                            || c.id == CardId::CurseOfTheBell
                            || c.id == CardId::Necronomicurse
                    });
                    event_state.current_screen = 1;
                },
                _ => {
                    event_state.current_screen = 1;
                },
            }
        },
        _ => { event_state.completed = true; }
    }

    run_state.event_state = Some(event_state);
}
