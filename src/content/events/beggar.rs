use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

const GOLD_COST: i32 = 75;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Donate 75g → purge a card, OR leave
            if run_state.gold >= GOLD_COST {
                vec![
                    EventChoiceMeta::new(format!(
                        "[Donate] Lose {} Gold. Remove a card.",
                        GOLD_COST
                    )),
                    EventChoiceMeta::new("[Leave]"),
                ]
            } else {
                vec![
                    EventChoiceMeta::disabled(
                        format!("[Donate] {} Gold.", GOLD_COST),
                        "Not enough Gold",
                    ),
                    EventChoiceMeta::new("[Leave]"),
                ]
            }
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => match choice_idx {
            0 => {
                run_state.gold -= GOLD_COST;
                event_state.current_screen = 1;
                run_state.event_state = Some(event_state);
                *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: 1,
                    max_choices: 1,
                    reason: RunPendingChoiceReason::Purge,
                    return_state: Box::new(EngineState::EventRoom),
                });
                return;
            }
            _ => {
                event_state.completed = true;
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
