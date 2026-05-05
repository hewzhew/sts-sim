use crate::content::cards::CardId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Simplicity: Purge a card; Basics: Upgrade all Strikes/Defends
            let has_purgeable = run_state.master_deck.iter().any(|c| {
                c.id != CardId::AscendersBane
                    && c.id != CardId::CurseOfTheBell
                    && c.id != CardId::Necronomicurse
            });
            vec![
                if has_purgeable {
                    EventChoiceMeta::new("[Simplicity] Remove a card from your deck.")
                } else {
                    EventChoiceMeta::disabled("[Simplicity] No removable cards.", "No cards")
                },
                EventChoiceMeta::new("[Basics] Upgrade all Strikes and Defends."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        // After purge returns
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Purge a card: transition to RunPendingChoice::Purge
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
                1 => {
                    // Upgrade all Strikes and Defends
                    for card in run_state.master_deck.iter_mut() {
                        if crate::content::cards::is_starter_basic(card.id) {
                            card.upgrades += 1;
                        }
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    event_state.completed = true;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
