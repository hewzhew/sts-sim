use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let has_mask = run_state.relics.iter().any(|r| r.id == RelicId::RedMask);
            if has_mask {
                vec![
                    EventChoiceMeta::new("[Don the Mask] Gain 222 Gold."),
                    EventChoiceMeta::new("[Leave]"),
                ]
            } else {
                vec![
                    EventChoiceMeta::disabled("[Don the Mask] Requires Red Mask.", "No Red Mask"),
                    EventChoiceMeta::new(format!("[Pay] Lose all ({}) Gold. Obtain Red Mask.", run_state.gold)),
                    EventChoiceMeta::new("[Leave]"),
                ]
            }
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();
    let has_mask = run_state.relics.iter().any(|r| r.id == RelicId::RedMask);

    match event_state.current_screen {
        0 => {
            if has_mask {
                match choice_idx {
                    0 => {
                        // Don the Mask: +222 gold
                        run_state.gold += 222;
                        event_state.current_screen = 1;
                    },
                    _ => {
                        event_state.completed = true;
                    },
                }
            } else {
                match choice_idx {
                    1 => {
                        // Pay all gold, get Red Mask
                        run_state.gold = 0;
                        run_state.relics.push(RelicState::new(RelicId::RedMask));
                        event_state.current_screen = 1;
                    },
                    _ => {
                        event_state.completed = true;
                    },
                }
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
