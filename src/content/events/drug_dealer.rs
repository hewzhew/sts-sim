// Java: DrugDealer (city) — "Drug Dealer"
// Screen 0:
//   [0] Obtain J.A.X. card
//   [1] Transform 2 cards (requires ≥2 purgeable) — grid-select
//   [2] Obtain MutagenicStrength relic (Circlet if already owned)
// Screen 1: [Leave]

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let purgeable_count = run_state
        .master_deck
        .iter()
        .filter(|c| {
            // Java: getPurgeableCards() excludes non-purgeable curses
            c.id != crate::content::cards::CardId::AscendersBane
                && c.id != crate::content::cards::CardId::CurseOfTheBell
                && c.id != crate::content::cards::CardId::Necronomicurse
        })
        .count();

    let mut choices = vec![EventChoiceMeta::new("[Ingest Mutagens] Obtain J.A.X.")];

    if purgeable_count >= 2 {
        choices.push(EventChoiceMeta::new(
            "[Become a Test Subject] Transform 2 cards.",
        ));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Become a Test Subject] Transform 2 cards.",
            "Not enough purgeable cards",
        ));
    }

    choices.push(EventChoiceMeta::new(
        "[Inject Mutagens] Obtain Mutagenic Strength relic.",
    ));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    if event_state.completed {
        run_state.event_state = Some(event_state);
        return;
    }

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Obtain J.A.X.
                    run_state.add_card_to_deck(crate::content::cards::CardId::JAX);
                    event_state.current_screen = 1;
                }
                1 => {
                    // Transform 2 cards (Java: gridSelectScreen.open(getPurgeableCards(), 2, ...))
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        min_choices: 2,
                        max_choices: 2,
                        reason: RunPendingChoiceReason::Transform,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    event_state.current_screen = 1;
                }
                2 => {
                    // Obtain MutagenicStrength relic
                    let relic_id = if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::MutagenicStrength)
                    {
                        crate::content::relics::RelicId::Circlet
                    } else {
                        crate::content::relics::RelicId::MutagenicStrength
                    };
                    run_state
                        .relics
                        .push(crate::content::relics::RelicState::new(relic_id));
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
