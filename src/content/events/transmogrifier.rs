use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    vec![
        EventChoiceMeta::new("[Pray] Transform a card."),
        EventChoiceMeta::new("[Leave]"),
    ]
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Transform a card
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::TransformNonBottled,
                        min_choices: 1,
                        max_choices: 1,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                _ => {
                    // Leave
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
    use super::handle_choice;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::engine::run_loop::tick_run;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason};
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{
        DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
        SelectionTargetRef,
    };

    fn deck_card(id: CardId, uuid: u32, upgrades: u8) -> CombatCard {
        let mut card = CombatCard::new(id, uuid);
        card.upgrades = upgrades;
        card
    }

    #[test]
    fn transform_selection_excludes_bottled_and_unpurgeable_cards_like_java() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![
            deck_card(CardId::Strike, 11, 0),
            deck_card(CardId::Defend, 12, 0),
            deck_card(CardId::AscendersBane, 13, 0),
        ];
        let mut bottle = RelicState::new(RelicId::BottledFlame);
        bottle.amount = 11;
        rs.relics.push(bottle);
        rs.event_state = Some(EventState::new(EventId::Transmorgrifier));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let EngineState::RunPendingChoice(choice) = engine_state else {
            panic!("Transmogrifier should open deck transform selection");
        };
        assert_eq!(choice.reason, RunPendingChoiceReason::TransformNonBottled);
        let request = choice.selection_request(&rs);
        assert_eq!(request.reason, SelectionReason::Transform);
        assert_eq!(
            request.targets,
            vec![SelectionTargetRef::CardUuid(12)],
            "Java opens CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())"
        );
    }

    #[test]
    fn transformed_card_uses_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![deck_card(CardId::Strike, 11, 0)];
        rs.event_state = Some(EventState::new(EventId::Transmorgrifier));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut rs,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(11)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::EventRoom));
        assert_eq!(rs.master_deck.len(), 1);
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardTransformed {
                before,
                source: DomainEventSource::Event(EventId::Transmorgrifier),
                ..
            } if before.id == CardId::Strike
        )));
    }
}
