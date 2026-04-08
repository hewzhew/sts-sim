use crate::content::cards::{get_card_definition, CardId, CardType};
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
                    // Remove all removable curses
                    let curses_to_remove: Vec<u32> = run_state
                        .master_deck
                        .iter()
                        .filter(|c| {
                            let def = get_card_definition(c.id);
                            def.card_type == CardType::Curse
                                && c.id != CardId::AscendersBane
                                && c.id != CardId::CurseOfTheBell
                                && c.id != CardId::Necronomicurse
                        })
                        .map(|c| c.uuid)
                        .collect();

                    for uuid in curses_to_remove {
                        run_state.remove_card_from_deck(uuid);
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    event_state.current_screen = 1;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
