use crate::content::cards::{CardId, CardTag};
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventId, EventOption,
    EventOptionConstraint, EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

fn get_hp_loss(run_state: &RunState) -> i32 {
    let mut loss = (run_state.max_hp as f32 * 0.3).ceil() as i32;
    if loss >= run_state.max_hp {
        loss = run_state.max_hp - 1;
    }
    loss
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    if event_state.current_screen == 1 {
        return vec![EventOption::new(
            EventChoiceMeta::new("[Leave]"),
            EventOptionSemantics {
                action: EventActionKind::Leave,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::Complete,
                repeatable: false,
                terminal: true,
            },
        )];
    }

    let hp_loss = get_hp_loss(run_state);
    let mut choices = vec![EventOption::new(
        EventChoiceMeta::new(format!(
            "[Accept] Lose {} Max HP. Replace all Strikes with 5 Bites.",
            hp_loss
        )),
        EventOptionSemantics {
            action: EventActionKind::Accept,
            effects: vec![
                EventEffect::LoseMaxHp(hp_loss),
                EventEffect::ObtainCard {
                    count: 5,
                    kind: EventCardKind::Specific(CardId::Bite),
                },
            ],
            constraints: vec![],
            transition: EventOptionTransition::AdvanceScreen,
            repeatable: false,
            terminal: false,
        },
    )];

    let has_vial = run_state.relics.iter().any(|r| r.id == RelicId::BloodVial);
    if has_vial {
        choices.push(EventOption::new(
            EventChoiceMeta::new("[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites."),
            EventOptionSemantics {
                action: EventActionKind::Trade,
                effects: vec![
                    EventEffect::LoseRelic {
                        specific: Some(RelicId::BloodVial),
                        starter_only: false,
                    },
                    EventEffect::ObtainCard {
                        count: 5,
                        kind: EventCardKind::Specific(CardId::Bite),
                    },
                ],
                constraints: vec![EventOptionConstraint::RequiresRelic(RelicId::BloodVial)],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        ));
    } else {
        choices.push(EventOption::new(
            EventChoiceMeta::disabled(
                "[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites.",
                "Requires Blood Vial",
            ),
            EventOptionSemantics {
                action: EventActionKind::Trade,
                effects: vec![
                    EventEffect::LoseRelic {
                        specific: Some(RelicId::BloodVial),
                        starter_only: false,
                    },
                    EventEffect::ObtainCard {
                        count: 5,
                        kind: EventCardKind::Specific(CardId::Bite),
                    },
                ],
                constraints: vec![EventOptionConstraint::RequiresRelic(RelicId::BloodVial)],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        ));
    }

    choices.push(EventOption::new(
        EventChoiceMeta::new("[Refuse] Leave."),
        EventOptionSemantics {
            action: EventActionKind::Decline,
            effects: vec![],
            constraints: vec![],
            transition: EventOptionTransition::Complete,
            repeatable: false,
            terminal: true,
        },
    ));
    choices
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Accept: Max HP loss
                    let hp_loss = get_hp_loss(run_state);
                    let source = DomainEventSource::Event(EventId::Vampires);
                    run_state.lose_max_hp_with_source(hp_loss, source);
                    replace_attacks(run_state, source);
                    event_state.current_screen = 1;
                }
                1 => {
                    // Give Vial -> Requires BloodVial
                    let source = DomainEventSource::Event(EventId::Vampires);
                    if let Some(pos) = run_state
                        .relics
                        .iter()
                        .position(|r| r.id == RelicId::BloodVial)
                    {
                        run_state.remove_relic_at_with_source(pos, source);
                        replace_attacks(run_state, source);
                        event_state.current_screen = 1;
                    }
                }
                _ => {
                    // Refuse
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

fn replace_attacks(run_state: &mut RunState, source: DomainEventSource) {
    // Identify Strikes to remove
    let strikes_to_remove: Vec<u32> = run_state
        .master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.tags.contains(&CardTag::StarterStrike)
        })
        .map(|card| card.uuid)
        .collect();

    for uuid in strikes_to_remove {
        run_state.remove_card_from_deck_with_source(uuid, source);
    }

    // Add 5 Bites through the DeckManager pipeline
    for _ in 0..5 {
        super::obtain_event_card(run_state, EventId::Vampires, CardId::Bite);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;
    use crate::state::events::{
        EventActionKind, EventCardKind, EventEffect, EventId, EventOptionConstraint,
        EventOptionTransition, EventState,
    };
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn give_vial_option_exposes_constraint_and_bite_reward() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.event_state = Some(EventState::new(EventId::Vampires));
        let options = get_options(&rs, rs.event_state.as_ref().unwrap());
        let give_vial = &options[1];

        assert!(give_vial.ui.disabled);
        assert_eq!(give_vial.semantics.action, EventActionKind::Trade);
        assert_eq!(
            give_vial.semantics.constraints,
            vec![EventOptionConstraint::RequiresRelic(RelicId::BloodVial)]
        );
        assert!(give_vial
            .semantics
            .effects
            .contains(&EventEffect::ObtainCard {
                count: 5,
                kind: EventCardKind::Specific(CardId::Bite),
            }));
        assert_eq!(
            give_vial.semantics.transition,
            EventOptionTransition::AdvanceScreen
        );
    }

    #[test]
    fn accept_loses_max_hp_replaces_starter_strikes_with_event_sources() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.current_hp = 70;
        rs.max_hp = 80;
        rs.event_state = Some(EventState::new(EventId::Vampires));
        rs.emitted_events.clear();
        let starter_strikes = rs
            .master_deck
            .iter()
            .filter(|card| {
                crate::content::cards::get_card_definition(card.id)
                    .tags
                    .contains(&CardTag::StarterStrike)
            })
            .count();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 0);

        assert_eq!(rs.max_hp, 56);
        assert_eq!(rs.current_hp, 56);
        assert_eq!(
            rs.master_deck
                .iter()
                .filter(|card| card.id == CardId::Bite)
                .count(),
            5
        );
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: -24,
                current_hp: 56,
                max_hp: 56,
                source: DomainEventSource::Event(EventId::Vampires),
            }
        )));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    DomainEvent::CardRemoved {
                        source: DomainEventSource::Event(EventId::Vampires),
                        ..
                    }
                ))
                .count(),
            starter_strikes
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    DomainEvent::CardObtained {
                        card,
                        source: DomainEventSource::Event(EventId::Vampires),
                    } if card.id == CardId::Bite
                ))
                .count(),
            5
        );
    }

    #[test]
    fn give_vial_removes_relic_without_max_hp_loss_and_replaces_strikes() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.current_hp = 70;
        rs.max_hp = 80;
        rs.relics.push(RelicState::new(RelicId::BloodVial));
        rs.event_state = Some(EventState::new(EventId::Vampires));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(rs.max_hp, 80);
        assert_eq!(rs.current_hp, 70);
        assert!(!rs.relics.iter().any(|relic| relic.id == RelicId::BloodVial));
        assert_eq!(
            rs.master_deck
                .iter()
                .filter(|card| card.id == CardId::Bite)
                .count(),
            5
        );
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicLost {
                relic_id: RelicId::BloodVial,
                source: DomainEventSource::Event(EventId::Vampires),
            }
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event, DomainEvent::MaxHpChanged { .. })));
    }

    #[test]
    fn disabled_give_vial_does_not_replace_strikes_without_blood_vial() {
        let mut rs = RunState::new(1, 0, false, "Ironclad");
        rs.current_hp = 70;
        rs.max_hp = 80;
        rs.event_state = Some(EventState::new(EventId::Vampires));
        rs.emitted_events.clear();
        let before = rs
            .master_deck
            .iter()
            .map(|card| (card.id, card.uuid))
            .collect::<Vec<_>>();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 1);

        assert_eq!(
            rs.master_deck
                .iter()
                .map(|card| (card.id, card.uuid))
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(rs.event_state.as_ref().unwrap().current_screen, 0);
        assert!(rs.take_emitted_events().is_empty());
    }
}
