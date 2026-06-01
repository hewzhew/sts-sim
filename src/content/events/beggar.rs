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
                if run_state.gold >= GOLD_COST {
                    run_state.change_gold_with_source(
                        -GOLD_COST,
                        DomainEventSource::Event(EventId::Beggar),
                    );
                    event_state.current_screen = 1;
                }
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

    fn beggar_run(screen: usize) -> RunState {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.gold = 100;
        let mut event_state = EventState::new(EventId::Beggar);
        event_state.current_screen = screen;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    fn deck_card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
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

    #[test]
    fn paid_continue_selection_excludes_bottled_and_unpurgeable_cards_like_java() {
        let mut run_state = beggar_run(1);
        run_state.master_deck = vec![
            deck_card(CardId::Strike, 101),
            deck_card(CardId::Defend, 102),
            deck_card(CardId::AscendersBane, 103),
        ];
        let mut bottle = RelicState::new(RelicId::BottledFlame);
        bottle.amount = 101;
        run_state.relics.push(bottle);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::RunPendingChoice(choice) = engine_state else {
            panic!("Beggar should open deck purge selection");
        };
        assert_eq!(choice.reason, RunPendingChoiceReason::PurgeNonBottled);
        let request = choice.selection_request(&run_state);
        assert_eq!(request.reason, SelectionReason::Purge);
        assert_eq!(
            request.targets,
            vec![SelectionTargetRef::CardUuid(102)],
            "Java opens CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())"
        );
    }

    #[test]
    fn paid_continue_removes_selected_card_with_event_source() {
        let mut run_state = beggar_run(1);
        run_state.master_deck = vec![deck_card(CardId::Strike, 101)];
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.master_deck.is_empty());
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardRemoved {
                card,
                source: DomainEventSource::Event(EventId::Beggar),
            } if card.id == CardId::Strike && card.uuid == 101
        )));
    }

    #[test]
    fn disabled_donate_does_not_pay_or_advance() {
        let mut run_state = beggar_run(0);
        run_state.gold = 50;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.gold, 50);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }
}
