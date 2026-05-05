use crate::content::relics::RelicId;
use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventId, EventOption, EventOptionSemantics,
    EventOptionTransition, EventRelicKind, EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => vec![
            EventOption::new(
                EventChoiceMeta::new("[Read] Begin reading the tome."),
                EventOptionSemantics {
                    action: EventActionKind::Continue,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("[Leave]"),
                EventOptionSemantics {
                    action: EventActionKind::Leave,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::Complete,
                    repeatable: false,
                    terminal: true,
                },
            ),
        ],
        1 => vec![EventOption::new(
            EventChoiceMeta::new("[Continue] Take 1 damage."),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![EventEffect::LoseHp(1)],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        2 => vec![EventOption::new(
            EventChoiceMeta::new("[Continue] Take 2 damage."),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![EventEffect::LoseHp(2)],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        3 => vec![EventOption::new(
            EventChoiceMeta::new("[Continue] Take 3 damage."),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![EventEffect::LoseHp(3)],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        4 => {
            let final_dmg = if run_state.ascension_level >= 15 {
                15
            } else {
                10
            };
            vec![
                EventOption::new(
                    EventChoiceMeta::new(format!(
                        "[Take the Book] Take {} damage. Obtain a Book relic.",
                        final_dmg
                    )),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![
                            EventEffect::LoseHp(final_dmg),
                            EventEffect::ObtainRelic {
                                count: 1,
                                kind: EventRelicKind::RandomBook,
                            },
                        ],
                        constraints: vec![],
                        transition: EventOptionTransition::OpenReward,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new("[Stop Reading] Take 3 damage."),
                    EventOptionSemantics {
                        action: EventActionKind::Decline,
                        effects: vec![EventEffect::LoseHp(3)],
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

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => match choice_idx {
            0 => {
                event_state.current_screen = 1;
            }
            _ => {
                event_state.completed = true;
            }
        },
        1 => {
            run_state.change_hp_with_source(-1, DomainEventSource::Event(EventId::CursedTome));
            event_state.current_screen = 2;
        }
        2 => {
            run_state.change_hp_with_source(-2, DomainEventSource::Event(EventId::CursedTome));
            event_state.current_screen = 3;
        }
        3 => {
            run_state.change_hp_with_source(-3, DomainEventSource::Event(EventId::CursedTome));
            event_state.current_screen = 4;
        }
        4 => {
            match choice_idx {
                0 => {
                    // Take the book
                    let final_dmg = if run_state.ascension_level >= 15 {
                        15
                    } else {
                        10
                    };
                    run_state.change_hp_with_source(
                        -final_dmg,
                        DomainEventSource::Event(EventId::CursedTome),
                    );
                    // Random book relic (Java randomBook)
                    let book_relics = [
                        RelicId::Necronomicon,
                        RelicId::Enchiridion,
                        RelicId::NilrysCodex,
                    ];
                    let available: Vec<RelicId> = book_relics
                        .iter()
                        .copied()
                        .filter(|r| !run_state.relics.iter().any(|owned| owned.id == *r))
                        .collect();
                    let relic_id = if available.is_empty() {
                        RelicId::Circlet
                    } else {
                        let idx = run_state
                            .rng_pool
                            .misc_rng
                            .random_range(0, available.len() as i32 - 1)
                            as usize;
                        available[idx]
                    };
                    // Java: addRelicToRewards(r) + combatRewardScreen.open()
                    let mut rewards = RewardState::new();
                    rewards.items.push(RewardItem::Relic { relic_id });
                    event_state.current_screen = 5;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RewardScreen(rewards);
                    return;
                }
                _ => {
                    // Stop reading: 3 damage
                    run_state
                        .change_hp_with_source(-3, DomainEventSource::Event(EventId::CursedTome));
                    event_state.current_screen = 5;
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
    use crate::state::events::{
        EventActionKind, EventEffect, EventOptionTransition, EventRelicKind,
    };

    #[test]
    fn take_book_option_exposes_reward_transition() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.event_state = Some(EventState {
            id: EventId::CursedTome,
            current_screen: 4,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        let options = get_options(&rs, rs.event_state.as_ref().unwrap());
        let take_book = &options[0];

        assert_eq!(take_book.semantics.action, EventActionKind::Accept);
        assert!(take_book
            .semantics
            .effects
            .contains(&EventEffect::ObtainRelic {
                count: 1,
                kind: EventRelicKind::RandomBook,
            }));
        assert_eq!(
            take_book.semantics.transition,
            EventOptionTransition::OpenReward
        );
    }
}
