use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const GOLD_COST: i32 = 85;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let mut choices = vec![];
            if run_state.gold >= GOLD_COST {
                choices.push(EventChoiceMeta::new(format!(
                    "[Pay] Lose {} Gold. Obtain a random Relic.",
                    GOLD_COST
                )));
            } else {
                choices.push(EventChoiceMeta::disabled(
                    format!("[Pay] Lose {} Gold. Obtain a random Relic.", GOLD_COST),
                    "Not enough Gold",
                ));
            }
            choices.push(EventChoiceMeta::new(
                "[Rob] Obtain a random Relic. Become Cursed - Shame.",
            ));
            choices.push(EventChoiceMeta::new("[Leave]"));
            choices
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
                    // Pay gold for relic
                    if run_state.gold >= GOLD_COST {
                        run_state.change_gold_with_source(
                            -GOLD_COST,
                            DomainEventSource::Event(EventId::Addict),
                        );
                        let relic_id = run_state.random_relic();
                        if let Some(next_state) = run_state.obtain_relic_with_source(
                            relic_id,
                            EngineState::EventRoom,
                            DomainEventSource::Event(EventId::Addict),
                        ) {
                            *_engine_state = next_state;
                        }
                    }
                    event_state.current_screen = 1;
                }
                1 => {
                    // Rob: relic + Shame curse
                    let relic_id = run_state.random_relic();
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::Addict),
                    ) {
                        *_engine_state = next_state;
                    }
                    run_state.add_card_to_deck(CardId::Shame);
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
