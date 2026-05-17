// Java: DrugDealer (city) — "Drug Dealer"
// Screen 0:
//   [0] Obtain J.A.X. card
//   [1] Transform 2 cards (requires ≥2 purgeable) — grid-select
//   [2] Obtain MutagenicStrength relic (Circlet if already owned)
// Screen 1: [Leave]

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
                    super::obtain_event_card(
                        run_state,
                        EventId::DrugDealer,
                        crate::content::cards::CardId::JAX,
                    );
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
                    let _ = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::DrugDealer),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::selection::DomainEvent;

    fn drug_dealer_run() -> RunState {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.event_state = Some(EventState {
            id: EventId::DrugDealer,
            current_screen: 0,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn ingest_mutagens_obtains_jax_with_event_source() {
        let mut run_state = drug_dealer_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::JAX));
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::DrugDealer),
            } if card.id == CardId::JAX
        )));
    }

    #[test]
    fn inject_mutagens_obtains_relic_with_event_source() {
        let mut run_state = drug_dealer_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 2);

        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::MutagenicStrength));
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::MutagenicStrength,
                source: DomainEventSource::Event(EventId::DrugDealer),
            }
        )));
    }

    #[test]
    fn inject_mutagens_grants_circlet_through_obtain_pipeline_when_already_owned() {
        let mut run_state = drug_dealer_run();
        run_state
            .relics
            .push(RelicState::new(RelicId::MutagenicStrength));
        run_state.relics.push(RelicState::new(RelicId::Circlet));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 2);

        let circlet = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Circlet)
            .expect("existing Circlet should remain");
        assert_eq!(circlet.counter, 2);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::Circlet,
                source: DomainEventSource::Event(EventId::DrugDealer),
            }
        )));
    }
}
