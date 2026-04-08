use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let heal_cost = 35;
            let mut choices = Vec::new();

            if run_state.gold >= heal_cost {
                choices.push(EventChoiceMeta::new(format!(
                    "[Heal] Lose {} Gold. Heal 25% of your Max HP.",
                    heal_cost
                )));
            } else {
                choices.push(EventChoiceMeta::disabled(
                    format!("[Heal] Lose {} Gold. Heal 25% of your Max HP.", heal_cost),
                    "Not enough Gold.",
                ));
            }

            let purify_cost = if run_state.ascension_level >= 15 {
                75
            } else {
                50
            };
            if run_state.gold >= purify_cost {
                choices.push(EventChoiceMeta::new(format!(
                    "[Purify] Lose {} Gold. Remove a card from your deck.",
                    purify_cost
                )));
            } else {
                choices.push(EventChoiceMeta::disabled(
                    format!(
                        "[Purify] Lose {} Gold. Remove a card from your deck.",
                        purify_cost
                    ),
                    "Not enough Gold.",
                ));
            }

            choices.push(EventChoiceMeta::new("[Leave]"));
            choices
        }
        _ => vec![EventChoiceMeta::new("[Leave]")], // After any choice, only Leave is displayed.
    }
}

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Heal
                    run_state.gold -= 35;
                    let heal = (run_state.max_hp as f32 * 0.25).round() as i32;
                    run_state.current_hp = (run_state.current_hp + heal).min(run_state.max_hp);
                    event_state.current_screen = 1;
                    event_state.completed = true;
                }
                1 => {
                    // Purify
                    let purify_cost = if run_state.ascension_level >= 15 {
                        75
                    } else {
                        50
                    };
                    run_state.gold -= purify_cost;
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        min_choices: 1,
                        max_choices: 1,
                        reason: RunPendingChoiceReason::Purge,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    event_state.current_screen = 1;
                    event_state.completed = true;
                }
                2 => {
                    // Leave
                    event_state.current_screen = 1;
                    event_state.completed = true;
                }
                _ => {}
            }
        }
        _ => {
            // Screen 1 is the exit screen. Clicking leaves.
        }
    }

    run_state.event_state = Some(event_state);
}
