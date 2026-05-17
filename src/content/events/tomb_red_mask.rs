use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
                    EventChoiceMeta::new(format!(
                        "[Pay] Lose all ({}) Gold. Obtain Red Mask.",
                        run_state.gold
                    )),
                    EventChoiceMeta::new("[Leave]"),
                ]
            }
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();
    let has_mask = run_state.relics.iter().any(|r| r.id == RelicId::RedMask);

    match event_state.current_screen {
        0 => {
            if has_mask {
                match choice_idx {
                    0 => {
                        // Don the Mask: +222 gold
                        run_state.change_gold_with_source(
                            222,
                            DomainEventSource::Event(EventId::TombRedMask),
                        );
                        event_state.current_screen = 1;
                    }
                    _ => {
                        event_state.completed = true;
                    }
                }
            } else {
                match choice_idx {
                    0 => {}
                    1 => {
                        // Pay all gold, get Red Mask
                        run_state.set_gold_with_source(
                            0,
                            DomainEventSource::Event(EventId::TombRedMask),
                        );
                        if let Some(next_state) = run_state.obtain_relic_with_source(
                            RelicId::RedMask,
                            EngineState::EventRoom,
                            DomainEventSource::Event(EventId::TombRedMask),
                        ) {
                            *engine_state = next_state;
                        }
                        event_state.current_screen = 1;
                    }
                    _ => {
                        event_state.completed = true;
                    }
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::{get_choices, handle_choice};
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn paying_without_mask_loses_all_gold_and_obtains_red_mask_with_event_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 123;
        run_state.event_state = Some(EventState::new(EventId::TombRedMask));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.gold, 0);
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::RedMask));
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::GoldChanged {
                    delta: -123,
                    new_total: 0,
                    source: DomainEventSource::Event(EventId::TombRedMask)
                }
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::RelicObtained {
                    relic_id: RelicId::RedMask,
                    source: DomainEventSource::Event(EventId::TombRedMask)
                }
            )
        }));
    }

    #[test]
    fn wearing_existing_mask_gains_222_gold_with_event_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 10;
        run_state.relics.push(RelicState::new(RelicId::RedMask));
        run_state.event_state = Some(EventState::new(EventId::TombRedMask));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 232);
        assert!(run_state.take_emitted_events().iter().any(|event| {
            matches!(
                event,
                DomainEvent::GoldChanged {
                    delta: 222,
                    new_total: 232,
                    source: DomainEventSource::Event(EventId::TombRedMask)
                }
            )
        }));
    }

    #[test]
    fn choices_preserve_java_button_indices_when_mask_is_missing() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 17;
        let event_state = EventState::new(EventId::TombRedMask);

        let choices = get_choices(&run_state, &event_state);

        assert_eq!(choices.len(), 3);
        assert!(choices[0].disabled);
        assert!(!choices[1].disabled);
        assert!(!choices[2].disabled);
    }

    #[test]
    fn disabled_don_mask_without_red_mask_does_not_advance_or_grant_gold() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 17;
        run_state.event_state = Some(EventState::new(EventId::TombRedMask));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 17);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }
}
