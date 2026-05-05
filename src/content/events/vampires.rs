use crate::content::cards::{CardId, CardTag};
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventOption,
    EventOptionConstraint, EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;

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
                    run_state.max_hp -= hp_loss;
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                    replace_attacks(run_state);
                    event_state.current_screen = 1;
                }
                1 => {
                    // Give Vial -> Requires BloodVial
                    if let Some(pos) = run_state
                        .relics
                        .iter()
                        .position(|r| r.id == RelicId::BloodVial)
                    {
                        run_state.relics.remove(pos);
                    }
                    replace_attacks(run_state);
                    event_state.current_screen = 1;
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

fn replace_attacks(run_state: &mut RunState) {
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
        run_state.remove_card_from_deck(uuid);
    }

    // Add 5 Bites through the DeckManager pipeline
    for _ in 0..5 {
        run_state.add_card_to_deck(CardId::Bite);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::events::{
        EventActionKind, EventCardKind, EventEffect, EventId, EventOptionConstraint,
        EventOptionTransition, EventState,
    };

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
}
