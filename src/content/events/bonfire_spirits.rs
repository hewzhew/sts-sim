// Java: Bonfire (shrines) — "Bonfire Elementals" / "Bonfire Spirits"
// This event is a duplicate/variant of bonfire_elementals. Both correspond
// to Java's single Bonfire.java (ID: "Bonfire Elementals").
//
// Screen 0: [Approach] → Screen 1
// Screen 1: [Offer] → grid-select (Purge)
// During grid-select resolution, reward based on offered card's rarity is
// applied before ordinary master-deck removal, matching Java update().
// Screen 2 exists only for direct reward helper tests.
// Screen 3: [Leave]

use crate::content::relics::RelicId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventOption, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_options(_run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    crate::content::events::bonfire_options(
        event_state,
        "[Approach]",
        "[Offer] Select a card to offer.",
    )
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            if crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state) {
                // Transition to RunPendingChoice::Purge to select a card.
                // The Purge handler stores the removed card's rarity in
                // event_state.internal_state before removal.
                event_state.current_screen = 2;
                run_state.event_state = Some(event_state);
                *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: 1,
                    max_choices: 1,
                    reason: RunPendingChoiceReason::PurgeNonBottled,
                    source: DomainEventSource::Event(EventId::BonfireSpirits),
                    return_state: Box::new(EngineState::EventRoom),
                });
                return;
            } else {
                event_state.current_screen = 3;
            }
        }
        2 => {
            // Post-purge: apply rarity-based reward from internal_state
            // (set by Purge handler: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare)
            let rarity = event_state.internal_state;
            apply_offer_reward(engine_state, run_state, rarity);
            event_state.current_screen = 3;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

pub fn apply_offer_reward(engine_state: &mut EngineState, run_state: &mut RunState, rarity: i32) {
    let source = DomainEventSource::Event(EventId::BonfireSpirits);
    match rarity {
        0 => {
            // Curse -> SpiritPoop relic (Circlet if already owned)
            let relic_id = if run_state.relics.iter().any(|r| r.id == RelicId::SpiritPoop) {
                RelicId::Circlet
            } else {
                RelicId::SpiritPoop
            };
            if let Some(next_state) =
                run_state.obtain_relic_with_source(relic_id, EngineState::EventRoom, source)
            {
                *engine_state = next_state;
            }
        }
        1 => {
            // Basic -> nothing
        }
        2 | 3 => {
            // Common / Special -> heal 5
            run_state.heal_with_source(5, source);
        }
        4 => {
            // Uncommon -> heal to full
            run_state.heal_with_source(run_state.max_hp, source);
        }
        5 => {
            // Rare -> Java increaseMaxHp(10, false), then heal(maxHealth).
            run_state.gain_max_hp_with_source(10, 10, source);
            run_state.heal_with_source(run_state.max_hp, source);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{get_options, handle_choice};
    use crate::content::cards::CardId;
    use crate::content::relics::RelicId;
    use crate::engine::run_loop::tick_run;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason};
    use crate::state::events::{
        EventActionKind, EventCardKind, EventEffect, EventId, EventOptionConstraint,
        EventOptionTransition, EventSelectionKind, EventState,
    };
    use crate::state::run::RunState;
    use crate::state::selection::{
        DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
        SelectionTargetRef,
    };

    fn bonfire_run(rarity_state: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 2;
        event_state.internal_state = rarity_state;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    fn deck_card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    #[test]
    fn structured_options_expose_bonfire_spirits_offer_selection_boundary() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![deck_card(CardId::Strike, 101)];
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);

        let options = get_options(&run_state, run_state.event_state.as_ref().unwrap());

        assert_eq!(options.len(), 1);
        assert_eq!(options[0].semantics.action, EventActionKind::DeckOperation);
        assert!(options[0]
            .semantics
            .effects
            .contains(&EventEffect::RemoveCard {
                count: 1,
                target_uuid: None,
                kind: EventCardKind::Unknown,
            }));
        assert!(options[0]
            .semantics
            .constraints
            .contains(&EventOptionConstraint::RequiresNonBottledPurgeableCard));
        assert_eq!(
            options[0].semantics.transition,
            EventOptionTransition::OpenSelection(EventSelectionKind::OfferCard)
        );
    }

    #[test]
    fn structured_options_expose_bonfire_spirits_intro_and_complete_boundaries() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let mut intro = EventState::new(EventId::BonfireSpirits);
        intro.current_screen = 0;
        let intro_options = get_options(&run_state, &intro);
        assert_eq!(intro_options[0].semantics.action, EventActionKind::Continue);
        assert_eq!(
            intro_options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        let mut complete = EventState::new(EventId::BonfireSpirits);
        complete.current_screen = 3;
        let complete_options = get_options(&run_state, &complete);
        assert_eq!(complete_options[0].semantics.action, EventActionKind::Leave);
        assert!(complete_options[0].semantics.terminal);
    }

    #[test]
    fn offer_without_purgeable_card_advances_without_pending_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![deck_card(CardId::AscendersBane, 11)];
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 3);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }

    #[test]
    fn offer_selection_excludes_bottled_and_unpurgeable_cards_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![
            deck_card(CardId::Strike, 101),
            deck_card(CardId::Defend, 102),
            deck_card(CardId::AscendersBane, 103),
        ];
        let mut bottle = crate::content::relics::RelicState::new(RelicId::BottledFlame);
        bottle.amount = 101;
        run_state.relics.push(bottle);
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::RunPendingChoice(choice) = engine_state else {
            panic!("Bonfire should open deck purge selection");
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
    fn offer_removes_selected_card_with_spirits_event_source_and_applies_post_selection_reward() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![deck_card(CardId::Strike, 101)];
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
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
        assert_eq!(
            run_state.event_state.as_ref().unwrap().current_screen,
            3,
            "Java Bonfire applies the selected-card callback during the grid-select update path"
        );
        assert_eq!(run_state.event_state.as_ref().unwrap().internal_state, 1);
        assert!(run_state.master_deck.is_empty());
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardRemoved {
                card,
                source: DomainEventSource::Event(EventId::BonfireSpirits),
            } if card.id == CardId::Strike && card.uuid == 101
        )));
    }

    #[test]
    fn common_offer_heals_with_spirits_event_source() {
        let mut run_state = bonfire_run(2);
        run_state.current_hp = 10;
        run_state.max_hp = 80;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 15);
        assert!(run_state.take_emitted_events().iter().any(|event| {
            matches!(
                event,
                DomainEvent::HpChanged {
                    delta: 5,
                    current_hp: 15,
                    max_hp: 80,
                    source: DomainEventSource::Event(EventId::BonfireSpirits)
                }
            )
        }));
    }

    #[test]
    fn curse_offer_obtains_spirit_poop_with_spirits_event_source() {
        let mut run_state = bonfire_run(0);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SpiritPoop));
        assert!(run_state.take_emitted_events().iter().any(|event| {
            matches!(
                event,
                DomainEvent::RelicObtained {
                    relic_id: RelicId::SpiritPoop,
                    source: DomainEventSource::Event(EventId::BonfireSpirits)
                }
            )
        }));
    }
}
