use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    vec![
        EventChoiceMeta::new("[Pray] Duplicate a card in your deck."),
        EventChoiceMeta::new("[Leave]"),
    ]
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Duplicate a card
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::Duplicate,
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
    use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason, RunResult};
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
    fn duplicate_selection_uses_full_master_deck_like_java() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck = vec![
            deck_card(CardId::AscendersBane, 11, 0),
            deck_card(CardId::Strike, 12, 0),
        ];
        let mut bottle = RelicState::new(RelicId::BottledFlame);
        bottle.amount = 12;
        rs.relics.push(bottle);
        rs.event_state = Some(EventState::new(EventId::Duplicator));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let EngineState::RunPendingChoice(choice) = engine_state else {
            panic!("Duplicator should open deck duplicate selection");
        };
        assert_eq!(choice.reason, RunPendingChoiceReason::Duplicate);
        let request = choice.selection_request(&rs);
        assert_eq!(request.reason, SelectionReason::Duplicate);
        assert_eq!(
            request.targets,
            vec![
                SelectionTargetRef::CardUuid(11),
                SelectionTargetRef::CardUuid(12),
            ],
            "Java Duplicator opens the full masterDeck, not getPurgeableCards or non-bottled cards"
        );
    }

    #[test]
    fn duplicate_selection_obtains_stat_equivalent_copy_with_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        let mut original = deck_card(CardId::RitualDagger, 101, 2);
        original.misc_value = 17;
        original.base_damage_override = Some(23);
        original.cost_modifier = -1;
        original.cost_for_turn = Some(0);
        original.free_to_play_once = true;
        original.base_damage_mut = 99;
        original.base_block_mut = 88;
        original.base_magic_num_mut = 77;
        rs.master_deck = vec![original];
        rs.event_state = Some(EventState::new(EventId::Duplicator));

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut rs, 0);

        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut rs,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(!matches!(
            engine_state,
            EngineState::GameOver(RunResult::Defeat)
        ));
        assert_eq!(rs.master_deck.len(), 2);
        let copied = rs
            .master_deck
            .iter()
            .find(|card| card.uuid != 101)
            .expect("Duplicator should add a copied card");
        assert_eq!(copied.id, CardId::RitualDagger);
        assert_eq!(copied.upgrades, 2);
        assert_eq!(copied.misc_value, 17);
        assert_eq!(copied.base_damage_override, Some(23));
        assert_eq!(copied.cost_modifier, -1);
        assert_eq!(copied.cost_for_turn, Some(0));
        assert!(copied.free_to_play_once);
        assert_eq!(copied.base_damage_mut, 0);
        assert_eq!(copied.base_block_mut, 0);
        assert_eq!(copied.base_magic_num_mut, 0);
        assert_ne!(copied.uuid, 101);
        assert!(rs.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::Duplicator),
            } if card.id == CardId::RitualDagger && card.upgrades == 2
        )));
    }
}
