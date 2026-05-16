use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

/// NoteForYourself event.
/// Java: playerPref stores a card across runs. Default: Iron Wave.
///   [Take] Obtain the stored card → GridSelect 1 card to remove (store for next run)
///   [Ignore] Do nothing
///
/// Since cross-run persistence is not supported, the obtained card is always Iron Wave.
/// The removal step is still important: player removes 1 card from deck (affects current run).
///
/// Screen 0: [Proceed]
/// Screen 1: [Take Card] / [Ignore]

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let def = crate::content::cards::get_card_definition(run_state.note_for_yourself_card);
            let upgrade_suffix = if run_state.note_for_yourself_upgrades > 0 {
                "+"
            } else {
                ""
            };
            vec![
                EventChoiceMeta::new(format!(
                    "[Take Card] Obtain {}{}. Remove a card.",
                    def.name, upgrade_suffix
                )),
                EventChoiceMeta::new("[Ignore]"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Take: obtain the profile note card, then pick 1 card to save.
                    // Java manually calls relic onObtainCard, adds to masterDeck, then opens
                    // CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards()).
                    run_state.add_card_to_deck_without_interception_from(
                        run_state.note_for_yourself_card,
                        run_state.note_for_yourself_upgrades,
                        DomainEventSource::Event(EventId::NoteForYourself),
                    );
                    event_state.current_screen = 2;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::PurgeNonBottled,
                        min_choices: 1,
                        max_choices: 1,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                _ => {
                    event_state.current_screen = 2;
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
    use crate::engine::run_loop::tick_run;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::ClientInput;
    use crate::state::selection::{
        DomainEvent, SelectionResolution, SelectionScope, SelectionTargetRef,
    };

    #[test]
    fn take_uses_profile_note_card_and_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.note_for_yourself_card = CardId::Bash;
        rs.note_for_yourself_upgrades = 1;
        rs.event_state = Some(EventState {
            id: EventId::NoteForYourself,
            current_screen: 1,
            completed: false,
            combat_pending: false,
            internal_state: 0,
            extra_data: Vec::new(),
        });

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let obtained = rs.master_deck.last().unwrap();
        assert_eq!(obtained.id, CardId::Bash);
        assert_eq!(obtained.upgrades, 1);
        assert!(matches!(
            engine_state,
            EngineState::RunPendingChoice(RunPendingChoiceState {
                reason: RunPendingChoiceReason::PurgeNonBottled,
                ..
            })
        ));
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::NoteForYourself),
            } if card.id == CardId::Bash && card.upgrades == 1
        )));
    }

    #[test]
    fn take_manual_obtain_is_not_blocked_by_omamori() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.note_for_yourself_card = CardId::Regret;
        rs.relics.push(RelicState::new(RelicId::Omamori));
        rs.event_state = Some(EventState {
            id: EventId::NoteForYourself,
            current_screen: 1,
            completed: false,
            combat_pending: false,
            internal_state: 0,
            extra_data: Vec::new(),
        });

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        assert!(rs.master_deck.iter().any(|card| card.id == CardId::Regret));
        let omamori = rs.relics.iter().find(|r| r.id == RelicId::Omamori).unwrap();
        assert_eq!(omamori.counter, 2);
    }

    #[test]
    fn selected_saved_card_updates_note_profile_before_removal() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::ShrugItOff, 12),
        ];
        rs.master_deck[1].upgrades = 1;
        rs.event_state = Some(EventState {
            id: EventId::NoteForYourself,
            current_screen: 2,
            completed: false,
            combat_pending: false,
            internal_state: 0,
            extra_data: Vec::new(),
        });
        let mut engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            reason: RunPendingChoiceReason::PurgeNonBottled,
            min_choices: 1,
            max_choices: 1,
            return_state: Box::new(EngineState::EventRoom),
        });
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut rs,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(12)],
            })),
        ));

        assert_eq!(rs.note_for_yourself_card, CardId::ShrugItOff);
        assert_eq!(rs.note_for_yourself_upgrades, 1);
        assert!(!rs.master_deck.iter().any(|card| card.uuid == 12));
    }
}
