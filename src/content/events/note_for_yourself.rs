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
        DomainEvent, SelectionReason, SelectionResolution, SelectionScope, SelectionTargetRef,
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
    fn take_manual_obtain_runs_relic_hooks_before_card_obtained_event() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.note_for_yourself_card = CardId::Bash;
        rs.relics.push(RelicState::new(RelicId::CeramicFish));
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

        let events = rs.take_emitted_events();
        let fish_gold_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::GoldChanged {
                        delta: 9,
                        source: DomainEventSource::Event(EventId::NoteForYourself),
                        ..
                    }
                )
            })
            .expect("NoteForYourself should manually run relic onObtainCard");
        let obtained_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::CardObtained {
                        card,
                        source: DomainEventSource::Event(EventId::NoteForYourself),
                    } if card.id == CardId::Bash
                )
            })
            .expect("NoteForYourself should add the stored note card to the master deck");

        assert!(
            fish_gold_pos < obtained_pos,
            "Java NoteForYourself manually calls relic onObtainCard before masterDeck.addToTop"
        );
    }

    #[test]
    fn take_manual_obtain_applies_egg_upgrade_to_note_card() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.note_for_yourself_card = CardId::Strike;
        rs.relics.push(RelicState::new(RelicId::MoltenEgg));
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

        let obtained = rs
            .master_deck
            .last()
            .expect("NoteForYourself should add the note card to the top of master deck");
        assert_eq!(obtained.id, CardId::Strike);
        assert_eq!(
            obtained.upgrades, 1,
            "Java NoteForYourself calls relic onObtainCard before addToTop, so Molten Egg upgrades the stored Attack"
        );
    }

    #[test]
    fn take_selection_excludes_bottled_and_unpurgeable_cards_after_obtaining_note_card() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::AscendersBane, 103),
        ];
        let mut bottle = RelicState::new(RelicId::BottledFlame);
        bottle.amount = 101;
        rs.relics.push(bottle);
        rs.note_for_yourself_card = CardId::Bash;
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

        let obtained_uuid = rs
            .master_deck
            .last()
            .expect("Note card should be added before selection opens")
            .uuid;
        let EngineState::RunPendingChoice(choice) = engine_state else {
            panic!("Taking the note card should open deck purge selection");
        };
        assert_eq!(choice.reason, RunPendingChoiceReason::PurgeNonBottled);
        let request = choice.selection_request(&rs);
        assert_eq!(request.reason, SelectionReason::Purge);
        assert_eq!(
            request.targets,
            vec![
                SelectionTargetRef::CardUuid(102),
                SelectionTargetRef::CardUuid(obtained_uuid),
            ],
            "Java adds the note card, then opens CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())"
        );
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
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardRemoved {
                card,
                source: DomainEventSource::Event(EventId::NoteForYourself),
            } if card.id == CardId::ShrugItOff && card.uuid == 12
        )));
    }
}
