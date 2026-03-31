use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

fn has_upgradable_cards(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        c.id == crate::content::cards::CardId::SearingBlow 
            || (c.upgrades == 0 
                && def.card_type != crate::content::cards::CardType::Status
                && def.card_type != crate::content::cards::CardType::Curse)
    })
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let mut choices = Vec::new();

    if has_upgradable_cards(run_state) {
        choices.push(EventChoiceMeta::new("[Pray] Upgrade a card."));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Pray] Upgrade a card.",
            "No upgradable cards.",
        ));
    }

    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    if has_upgradable_cards(run_state) {
                        event_state.current_screen = 1;
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            reason: RunPendingChoiceReason::Upgrade,
                            min_choices: 1,
                            max_choices: 1,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    }
                },
                _ => {
                    // Leave
                    event_state.current_screen = 1;
                }
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
