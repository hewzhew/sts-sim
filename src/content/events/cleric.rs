use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventId, EventOption,
    EventOptionConstraint, EventOptionSemantics, EventOptionTransition, EventSelectionKind,
    EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => {
            let heal_cost = 35;
            let mut choices = Vec::new();
            let heal = (run_state.max_hp as f32 * 0.25) as i32;

            if run_state.gold >= heal_cost {
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Heal] Lose {} Gold. Heal 25% of your Max HP.",
                        heal_cost
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![EventEffect::LoseGold(heal_cost), EventEffect::Heal(heal)],
                        constraints: vec![EventOptionConstraint::RequiresGold(heal_cost)],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled(
                        format!("[Heal] Lose {} Gold. Heal 25% of your Max HP.", heal_cost),
                        "Not enough Gold.",
                    ),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![EventEffect::LoseGold(heal_cost), EventEffect::Heal(heal)],
                        constraints: vec![EventOptionConstraint::RequiresGold(heal_cost)],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }

            let purify_cost = if run_state.ascension_level >= 15 {
                75
            } else {
                50
            };
            let has_removable =
                crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state);
            if run_state.gold >= purify_cost {
                let (effects, constraints, transition) = if has_removable {
                    (
                        vec![
                            EventEffect::LoseGold(purify_cost),
                            EventEffect::RemoveCard {
                                count: 1,
                                target_uuid: None,
                                kind: EventCardKind::Unknown,
                            },
                        ],
                        vec![EventOptionConstraint::RequiresGold(purify_cost)],
                        EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard),
                    )
                } else {
                    (
                        vec![],
                        vec![EventOptionConstraint::RequiresGold(purify_cost)],
                        EventOptionTransition::AdvanceScreen,
                    )
                };
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Purify] Lose {} Gold. Remove a card from your deck.",
                        purify_cost
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects,
                        constraints,
                        transition,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled(
                        format!(
                            "[Purify] Lose {} Gold. Remove a card from your deck.",
                            purify_cost
                        ),
                        "Not enough Gold.",
                    ),
                    EventOptionSemantics {
                        action: EventActionKind::DeckOperation,
                        effects: vec![
                            EventEffect::LoseGold(purify_cost),
                            EventEffect::RemoveCard {
                                count: 1,
                                target_uuid: None,
                                kind: EventCardKind::Unknown,
                            },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(purify_cost),
                            EventOptionConstraint::RequiresRemovableCard,
                        ],
                        transition: EventOptionTransition::OpenSelection(
                            EventSelectionKind::RemoveCard,
                        ),
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }

            choices.push(EventOption::new(
                EventChoiceMeta::new("[Leave]"),
                EventOptionSemantics {
                    action: EventActionKind::Leave,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ));
            choices
        }
        _ => vec![EventOption::new(
            EventChoiceMeta::new("[Leave]"),
            EventOptionSemantics {
                action: EventActionKind::Leave,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::Complete,
                repeatable: false,
                terminal: true,
            },
        )], // After any choice, only Leave is displayed.
    }
}

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Heal
                    if run_state.gold >= 35 {
                        run_state.change_gold_with_source(
                            -35,
                            DomainEventSource::Event(EventId::Cleric),
                        );
                        let heal = (run_state.max_hp as f32 * 0.25) as i32;
                        run_state.heal_with_source(heal, DomainEventSource::Event(EventId::Cleric));
                        event_state.current_screen = 1;
                        event_state.completed = true;
                    }
                }
                1 => {
                    // Purify
                    let purify_cost = if run_state.ascension_level >= 15 {
                        75
                    } else {
                        50
                    };
                    if run_state.gold >= purify_cost {
                        if !crate::state::core::has_non_bottled_purgeable_master_deck_card(
                            run_state,
                        ) {
                            event_state.current_screen = 1;
                            event_state.completed = true;
                        } else {
                            run_state.change_gold_with_source(
                                -purify_cost,
                                DomainEventSource::Event(EventId::Cleric),
                            );
                            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                                min_choices: 1,
                                max_choices: 1,
                                reason: RunPendingChoiceReason::PurgeNonBottled,
                                return_state: Box::new(EngineState::EventRoom),
                            });
                            event_state.current_screen = 1;
                            event_state.completed = true;
                            run_state.event_state = Some(event_state);
                            return;
                        }
                    }
                }
                2 => {
                    // Leave
                    event_state.current_screen = 1;
                    event_state.completed = true;
                }
                _ => {}
            }
        }
        _ => {
            // Screen 1 is the exit screen. Clicking leaves.
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::selection::DomainEvent;

    #[test]
    fn purify_option_exposes_remove_selection_semantics() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.gold = 100;
        let state = EventState::new(crate::state::events::EventId::Cleric);
        let options = get_options(&rs, &state);
        assert!(matches!(
            options[1].semantics.transition,
            EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard)
        ));
    }

    #[test]
    fn purify_without_removable_card_is_enabled_but_advances_without_payment_like_java() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.gold = 100;
        rs.master_deck = vec![CombatCard::new(CardId::AscendersBane, 11)];
        rs.event_state = Some(EventState::new(EventId::Cleric));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        let options = get_options(&rs, rs.event_state.as_ref().unwrap());
        assert!(!options[1].ui.disabled);
        assert!(matches!(
            options[1].semantics.transition,
            EventOptionTransition::AdvanceScreen
        ));
        assert!(options[1].semantics.effects.is_empty());

        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(rs.gold, 100);
        assert!(rs.event_state.as_ref().unwrap().completed);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(rs.take_emitted_events().is_empty());
    }

    #[test]
    fn disabled_heal_does_not_pay_or_advance() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.gold = 34;
        rs.current_hp = 1;
        rs.max_hp = 80;
        rs.event_state = Some(EventState::new(EventId::Cleric));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 0);

        assert_eq!(rs.gold, 34);
        assert_eq!(rs.current_hp, 1);
        assert_eq!(rs.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(rs.take_emitted_events().is_empty());
    }

    #[test]
    fn disabled_purify_does_not_pay_or_open_selection() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.gold = 49;
        rs.event_state = Some(EventState::new(EventId::Cleric));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(rs.gold, 49);
        assert_eq!(rs.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(rs.take_emitted_events().is_empty());
    }

    #[test]
    fn heal_amount_uses_java_float_cast_not_rounding() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.gold = 35;
        rs.current_hp = 1;
        rs.max_hp = 82;
        rs.event_state = Some(EventState::new(EventId::Cleric));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 0);

        assert_eq!(rs.current_hp, 21);
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: -35,
                source: DomainEventSource::Event(EventId::Cleric),
                ..
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 20,
                current_hp: 21,
                max_hp: 82,
                source: DomainEventSource::Event(EventId::Cleric),
            }
        )));
    }

    #[test]
    fn heal_cost_is_paid_even_when_mark_of_the_bloom_blocks_heal() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.gold = 35;
        rs.current_hp = 1;
        rs.max_hp = 80;
        rs.relics.push(RelicState::new(RelicId::MarkOfTheBloom));
        rs.event_state = Some(EventState::new(EventId::Cleric));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 0);

        assert_eq!(rs.gold, 0);
        assert_eq!(rs.current_hp, 1);
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: -35,
                source: DomainEventSource::Event(EventId::Cleric),
                ..
            }
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }
}
