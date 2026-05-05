use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventOption, EventOptionConstraint,
    EventOptionSemantics, EventOptionTransition, EventState,
};
use crate::state::run::RunState;

const COST_1: i32 = 20;
const COST_2: i32 = 30;
const COST_3: i32 = 40;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => {
            let mut choices = vec![];
            if run_state.gold >= COST_1 {
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!("[1 Potion] Lose {} Gold.", COST_1)),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_1),
                            EventEffect::ObtainPotion { count: 1 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_1),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled(
                        format!("[1 Potion] {} Gold.", COST_1),
                        "Not enough Gold",
                    ),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_1),
                            EventEffect::ObtainPotion { count: 1 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_1),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            if run_state.gold >= COST_2 {
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!("[2 Potions] Lose {} Gold.", COST_2)),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_2),
                            EventEffect::ObtainPotion { count: 2 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_2),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled(
                        format!("[2 Potions] {} Gold.", COST_2),
                        "Not enough Gold",
                    ),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_2),
                            EventEffect::ObtainPotion { count: 2 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_2),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            if run_state.gold >= COST_3 {
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!("[3 Potions] Lose {} Gold.", COST_3)),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_3),
                            EventEffect::ObtainPotion { count: 3 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_3),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
                choices.push(EventOption::new(
                    EventChoiceMeta::disabled(
                        format!("[3 Potions] {} Gold.", COST_3),
                        "Not enough Gold",
                    ),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![
                            EventEffect::LoseGold(COST_3),
                            EventEffect::ObtainPotion { count: 3 },
                        ],
                        constraints: vec![
                            EventOptionConstraint::RequiresGold(COST_3),
                            EventOptionConstraint::RequiresPotionSlotValue,
                        ],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            }
            if run_state.ascension_level >= 15 {
                let dmg = ((run_state.max_hp as f32 * 0.05).ceil()) as i32;
                choices.push(EventOption::new(
                    EventChoiceMeta::new(format!("[Leave] Lose {} HP.", dmg)),
                    EventOptionSemantics {
                        action: EventActionKind::Leave,
                        effects: vec![EventEffect::LoseHp(dmg)],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ));
            } else {
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
            }
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
                    run_state.gold -= COST_1;
                    let pid = run_state.random_potion();
                    let p = crate::content::potions::Potion::new(pid, 30001);
                    run_state.obtain_potion(p);
                    event_state.current_screen = 1;
                }
                1 => {
                    run_state.gold -= COST_2;
                    for i in 0..2u32 {
                        let pid = run_state.random_potion();
                        let p = crate::content::potions::Potion::new(pid, 30010 + i);
                        run_state.obtain_potion(p);
                    }
                    event_state.current_screen = 1;
                }
                2 => {
                    run_state.gold -= COST_3;
                    for i in 0..3u32 {
                        let pid = run_state.random_potion();
                        let p = crate::content::potions::Potion::new(pid, 30020 + i);
                        run_state.obtain_potion(p);
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    // Leave (A15: take HP loss)
                    if run_state.ascension_level >= 15 {
                        let dmg = ((run_state.max_hp as f32 * 0.05).ceil()) as i32;
                        run_state.current_hp = (run_state.current_hp - dmg).max(0);
                    }
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
    use super::*;

    #[test]
    fn three_potion_option_exposes_trade_semantics() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.gold = 50;
        let state = EventState::new(crate::state::events::EventId::WomanInBlue);
        let options = get_options(&rs, &state);
        assert!(matches!(
            options[2].semantics.effects.as_slice(),
            [
                EventEffect::LoseGold(40),
                EventEffect::ObtainPotion { count: 3 }
            ]
        ));
    }
}
