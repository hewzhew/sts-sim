use crate::content::cards::CardId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

/// NoteForYourself event.
/// Java: playerPref stores a card across runs. Default: Iron Wave.
///   [Take] Obtain the stored card → GridSelect 1 card to remove (store for next run)
///   [Ignore] Do nothing
///
/// Since cross-run persistence is not supported, the obtained card is always Iron Wave.
/// The removal step is still important: player removes 1 card from deck (affects current run).
///
/// Screen 0: [Proceed]
/// Screen 1: [Take Card] / [Ignore]

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => vec![
            EventChoiceMeta::new("[Take Card] Obtain Iron Wave. Remove a card."),
            EventChoiceMeta::new("[Ignore]"),
        ],
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Take: obtain Iron Wave, then pick 1 card to remove
                    // Java: masterDeck.addToTop(obtainCard) → gridSelect(getPurgeableCards, 1)
                    run_state.add_card_to_deck(CardId::IronWave);
                    event_state.current_screen = 2;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::Purge,
                        min_choices: 1,
                        max_choices: 1,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                _ => {
                    event_state.current_screen = 2;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
