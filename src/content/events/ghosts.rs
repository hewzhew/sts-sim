use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventOption,
    EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => {
            let hp_loss = ((run_state.max_hp as f32) * 0.5).ceil() as i32;
            let count = if run_state.ascension_level >= 15 {
                3
            } else {
                5
            };
            vec![
                EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Accept] Lose {} Max HP. Obtain {} Apparitions.",
                        hp_loss, count
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![
                            EventEffect::LoseMaxHp(hp_loss),
                            EventEffect::ObtainCard {
                                count: count as usize,
                                kind: EventCardKind::Specific(CardId::Apparition),
                            },
                        ],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new("[Refuse]"),
                    EventOptionSemantics {
                        action: EventActionKind::Decline,
                        effects: vec![],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
            ]
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
        )],
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Accept: lose 50% max HP, gain Apparitions
                    let mut hp_loss = ((run_state.max_hp as f32) * 0.5).ceil() as i32;
                    if hp_loss >= run_state.max_hp {
                        hp_loss = run_state.max_hp - 1;
                    }
                    run_state.max_hp -= hp_loss;
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                    let count = if run_state.ascension_level >= 15 {
                        3
                    } else {
                        5
                    };
                    for _ in 0..count {
                        run_state.add_card_to_deck(CardId::Apparition);
                    }
                    event_state.current_screen = 1;
                }
                _ => {
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
