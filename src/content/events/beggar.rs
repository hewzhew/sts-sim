use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
        1 => vec![EventChoiceMeta::new("[Proceed] Remove a card.")],
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => match choice_idx {
            0 => {
                run_state
                    .change_gold_with_source(-GOLD_COST, DomainEventSource::Event(EventId::Beggar));
                event_state.current_screen = 1;
            }
            _ => {
                event_state.completed = true;
            }
        },
        1 => {
            event_state.current_screen = 2;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 1,
                max_choices: 1,
                reason: RunPendingChoiceReason::PurgeNonBottled,
                return_state: Box::new(EngineState::EventRoom),
            });
            return;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::state::core::{EngineState, RunPendingChoiceReason};
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn beggar_run(screen: usize) -> RunState {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.gold = 100;
        let mut event_state = EventState::new(EventId::Beggar);
        event_state.current_screen = screen;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn donate_pays_gold_before_opening_purge_prompt_like_java() {
        let mut run_state = beggar_run(0);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 25);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: -75,
                new_total: 25,
                source: DomainEventSource::Event(EventId::Beggar)
            }
        )));
    }

    #[test]
    fn paid_continue_opens_non_bottled_purge_selection() {
        let mut run_state = beggar_run(1);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 2);
        assert!(matches!(
            engine_state,
            EngineState::RunPendingChoice(ref pending)
                if pending.reason == RunPendingChoiceReason::PurgeNonBottled
        ));
    }
}
