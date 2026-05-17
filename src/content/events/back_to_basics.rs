use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Simplicity: Purge a card; Basics: Upgrade all Strikes/Defends
            vec![
                EventChoiceMeta::new("[Simplicity] Remove a card from your deck."),
                EventChoiceMeta::new("[Basics] Upgrade all Strikes and Defends."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        // After purge returns
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Purge a card: transition to RunPendingChoice::Purge
                    event_state.current_screen = 1;
                    if crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state) {
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 1,
                            max_choices: 1,
                            reason: RunPendingChoiceReason::PurgeNonBottled,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    }
                }
                1 => {
                    // Upgrade all Strikes and Defends
                    let source = DomainEventSource::Event(EventId::BackTotheBasics);
                    let upgrade_uuids: Vec<u32> = run_state
                        .master_deck
                        .iter()
                        .filter(|card| {
                            crate::content::cards::is_starter_basic(card.id)
                                && crate::state::core::master_deck_card_can_upgrade(card)
                        })
                        .map(|card| card.uuid)
                        .collect();
                    for uuid in upgrade_uuids {
                        run_state.upgrade_card_with_source(uuid, source);
                    }
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
    use super::handle_choice;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn card(id: CardId, uuid: u32, upgrades: u8) -> CombatCard {
        let mut card = CombatCard::new(id, uuid);
        card.upgrades = upgrades;
        card
    }

    #[test]
    fn basics_upgrades_only_upgradeable_starter_strikes_and_defends() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.master_deck = vec![
            card(CardId::Strike, 11, 0),
            card(CardId::Defend, 12, 1),
            card(CardId::Bash, 13, 0),
            card(CardId::AscendersBane, 14, 0),
        ];
        run_state.event_state = Some(EventState::new(EventId::BackTotheBasics));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(
            run_state
                .master_deck
                .iter()
                .map(|card| (card.id, card.upgrades))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Strike, 1),
                (CardId::Defend, 1),
                (CardId::Bash, 0),
                (CardId::AscendersBane, 0),
            ]
        );
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardUpgraded {
                before,
                after,
                source: DomainEventSource::Event(EventId::BackTotheBasics),
            } if before.uuid == 11 && before.upgrades == 0 && after.upgrades == 1
        )));
    }

    #[test]
    fn simplicity_without_purgeable_cards_advances_without_pending_like_java() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.master_deck = vec![card(CardId::AscendersBane, 11, 0)];
        run_state.event_state = Some(EventState::new(EventId::BackTotheBasics));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        let choices = super::get_choices(&run_state, run_state.event_state.as_ref().unwrap());
        assert!(
            !choices[0].disabled,
            "Java BackToBasics always exposes the Simplicity button"
        );

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }
}
