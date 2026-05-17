use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

fn is_fountain_removable_curse(
    card: &crate::runtime::combat::CombatCard,
    run_state: &RunState,
) -> bool {
    let def = get_card_definition(card.id);
    def.card_type == CardType::Curse
        && card.id != CardId::AscendersBane
        && card.id != CardId::CurseOfTheBell
        && card.id != CardId::Necronomicurse
        && !crate::state::core::master_deck_card_is_bottled(card, &run_state.relics)
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let has_removable_curses = run_state
                .master_deck
                .iter()
                .any(|card| is_fountain_removable_curse(card, run_state));
            if has_removable_curses {
                vec![
                    EventChoiceMeta::new("[Drink] Remove all removable Curses."),
                    EventChoiceMeta::new("[Leave]"),
                ]
            } else {
                vec![
                    EventChoiceMeta::disabled("[Drink] No removable Curses.", "No curses"),
                    EventChoiceMeta::new("[Leave]"),
                ]
            }
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
                    // Remove all removable curses
                    let curses_to_remove: Vec<u32> = run_state
                        .master_deck
                        .iter()
                        .filter(|card| is_fountain_removable_curse(card, run_state))
                        .map(|card| card.uuid)
                        .collect();

                    let source = DomainEventSource::Event(EventId::FountainOfCurseCleansing);
                    for uuid in curses_to_remove {
                        run_state.remove_card_from_deck_with_source(uuid, source);
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    event_state.current_screen = 1;
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
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn fountain_removes_only_non_bottled_removable_curses_with_event_source() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.master_deck = vec![
            CombatCard::new(CardId::Injury, 11),
            CombatCard::new(CardId::Pain, 12),
            CombatCard::new(CardId::AscendersBane, 13),
            CombatCard::new(CardId::CurseOfTheBell, 14),
            CombatCard::new(CardId::Necronomicurse, 15),
            CombatCard::new(CardId::Strike, 16),
        ];
        let mut bottle = RelicState::new(RelicId::BottledFlame);
        bottle.amount = 12;
        run_state.relics.push(bottle);
        run_state.event_state = Some(EventState::new(EventId::FountainOfCurseCleansing));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(
            run_state
                .master_deck
                .iter()
                .map(|card| (card.id, card.uuid))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Pain, 12),
                (CardId::AscendersBane, 13),
                (CardId::CurseOfTheBell, 14),
                (CardId::Necronomicurse, 15),
                (CardId::Strike, 16),
            ]
        );
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardRemoved {
                card,
                source: DomainEventSource::Event(EventId::FountainOfCurseCleansing),
            } if card.id == CardId::Injury && card.uuid == 11
        )));
    }

    #[test]
    fn fountain_drink_is_disabled_when_only_bottled_or_special_curses_exist() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.master_deck = vec![
            CombatCard::new(CardId::Pain, 21),
            CombatCard::new(CardId::AscendersBane, 22),
        ];
        let mut bottle = RelicState::new(RelicId::BottledLightning);
        bottle.amount = 21;
        run_state.relics.push(bottle);
        let event_state = EventState::new(EventId::FountainOfCurseCleansing);

        let choices = get_choices(&run_state, &event_state);

        assert!(choices[0].disabled);
    }
}
