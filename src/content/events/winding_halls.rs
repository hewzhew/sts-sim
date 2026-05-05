use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventOption,
    EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => vec![EventOption::new(
            EventChoiceMeta::new("[Proceed]"),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        1 => {
            let hp_loss_pct = if run_state.ascension_level >= 15 {
                0.18
            } else {
                0.125
            };
            let hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
            let heal_pct = if run_state.ascension_level >= 15 {
                0.20
            } else {
                0.25
            };
            let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
            let max_hp_loss = (run_state.max_hp as f32 * 0.05).round() as i32;
            vec![
                EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Embrace] Lose {} HP. Obtain 2 Madness.",
                        hp_loss
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![
                            EventEffect::LoseHp(hp_loss),
                            EventEffect::ObtainCard {
                                count: 2,
                                kind: EventCardKind::Specific(CardId::Madness),
                            },
                        ],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Retrace] Heal {} HP. Become Cursed - Writhe.",
                        heal_amt
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![
                            EventEffect::Heal(heal_amt),
                            EventEffect::ObtainCurse {
                                count: 1,
                                kind: EventCardKind::Specific(CardId::Writhe),
                            },
                        ],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new(format!("[Accept] Lose {} Max HP.", max_hp_loss)),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![EventEffect::LoseMaxHp(max_hp_loss)],
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
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Embrace Madness: take damage (DEFAULT type) + 2 Madness
                    let hp_loss_pct = if run_state.ascension_level >= 15 {
                        0.18
                    } else {
                        0.125
                    };
                    let mut hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
                    // Tungsten Rod reduces DEFAULT damage by 1
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
                    {
                        hp_loss = (hp_loss - 1).max(0);
                    }
                    run_state.current_hp = (run_state.current_hp - hp_loss).max(0);
                    run_state.add_card_to_deck(CardId::Madness);
                    run_state.add_card_to_deck(CardId::Madness);
                    event_state.current_screen = 2;
                }
                1 => {
                    // Retrace: heal + Writhe
                    let heal_pct = if run_state.ascension_level >= 15 {
                        0.20
                    } else {
                        0.25
                    };
                    let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
                    run_state.current_hp = (run_state.current_hp + heal_amt).min(run_state.max_hp);
                    run_state.add_card_to_deck(CardId::Writhe);
                    event_state.current_screen = 2;
                }
                _ => {
                    // Accept: lose Max HP
                    let max_hp_loss = (run_state.max_hp as f32 * 0.05).round() as i32;
                    run_state.max_hp = (run_state.max_hp - max_hp_loss).max(1);
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
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
