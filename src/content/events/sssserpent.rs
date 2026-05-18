use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

fn gold_reward(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        150
    } else {
        175
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let gold = gold_reward(run_state);
            vec![
                EventChoiceMeta::new(format!(
                    "[Agree] Gain {} Gold. Become Cursed - Doubt.",
                    gold
                )),
                EventChoiceMeta::new("[Disagree] Leave."),
            ]
        }
        1 => {
            // AGREE screen: confirm
            vec![EventChoiceMeta::new("[Confirm]")]
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
                    // Agree: advance to confirm screen
                    event_state.current_screen = 1;
                }
                _ => {
                    // Disagree: leave
                    event_state.current_screen = 99;
                }
            }
        }
        1 => {
            // Confirm: gain gold + receive curse
            let gold = gold_reward(run_state);
            run_state.change_gold_with_source(gold, DomainEventSource::Event(EventId::Ssssserpent));
            super::obtain_event_card(run_state, EventId::Ssssserpent, CardId::Doubt);
            event_state.current_screen = 99;
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
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn serpent_run(ascension: u8) -> RunState {
        let mut run_state = RunState::new(1, ascension, false, "Ironclad");
        run_state.gold = 0;
        run_state.event_state = Some(EventState::new(EventId::Ssssserpent));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn agree_is_two_step_and_confirm_grants_java_gold_and_doubt() {
        let mut run_state = serpent_run(0);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 0);
        assert!(!run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Doubt));
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 175);
        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Doubt));
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: 175,
                source: DomainEventSource::Event(EventId::Ssssserpent),
                ..
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::Ssssserpent),
            } if card.id == CardId::Doubt
        )));
    }

    #[test]
    fn ascension_15_uses_java_lower_gold_reward() {
        let mut run_state = serpent_run(15);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 150);
    }

    #[test]
    fn omamori_blocks_doubt_but_not_gold() {
        let mut run_state = serpent_run(0);
        run_state.relics.push(RelicState::new(RelicId::Omamori));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 175);
        assert!(!run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Doubt));
        let omamori = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Omamori)
            .expect("Omamori should remain after blocking the curse");
        assert_eq!(omamori.counter, 1);
    }

    #[test]
    fn confirm_gold_resolves_before_delayed_doubt_obtain_like_java_effect_list() {
        let mut run_state = serpent_run(0);
        run_state.relics.push(RelicState::new(RelicId::CeramicFish));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 184);
        let labels = run_state
            .take_emitted_events()
            .into_iter()
            .filter_map(|event| match event {
                DomainEvent::GoldChanged {
                    delta: 175,
                    source: DomainEventSource::Event(EventId::Ssssserpent),
                    ..
                } => Some("event_gold"),
                DomainEvent::GoldChanged {
                    delta: 9,
                    source: DomainEventSource::Event(EventId::Ssssserpent),
                    ..
                } => Some("ceramic_fish_gold"),
                DomainEvent::CardObtained {
                    card,
                    source: DomainEventSource::Event(EventId::Ssssserpent),
                } if card.id == CardId::Doubt => Some("doubt_obtained"),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec!["event_gold", "ceramic_fish_gold", "doubt_obtained"],
            "Java queues ShowCardAndObtainEffect before RainingGoldEffect but gains gold immediately; actual card obtain resolves later"
        );
    }
}
