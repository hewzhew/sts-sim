use crate::content::monsters::factory::EncounterId;
use crate::content::relics::RelicId;
use crate::state::core::{CombatStartRequest, EngineState, PostCombatReturn};
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventId, EventOption, EventOptionSemantics,
    EventOptionTransition, EventRelicKind, EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => {
            let reward_relic = if run_state.relics.iter().any(|r| r.id == RelicId::RedMask) {
                RelicId::Circlet
            } else {
                RelicId::RedMask
            };
            let gold_effect = if run_state.is_daily_run {
                EventEffect::GainGoldRange { min: 0, max: 30 }
            } else {
                EventEffect::GainGoldRange { min: 25, max: 35 }
            };
            vec![
                EventOption::new(
                    EventChoiceMeta::new(format!("[Pay] Lose all ({}) Gold.", run_state.gold)),
                    EventOptionSemantics {
                        action: EventActionKind::Trade,
                        effects: vec![EventEffect::LoseGold(run_state.gold)],
                        transition: EventOptionTransition::AdvanceScreen,
                        ..Default::default()
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new("[Fight] Engage the bandits!"),
                    EventOptionSemantics {
                        action: EventActionKind::Fight,
                        effects: vec![
                            EventEffect::StartCombat,
                            gold_effect,
                            EventEffect::ObtainRelic {
                                count: 1,
                                kind: EventRelicKind::Specific(reward_relic),
                            },
                        ],
                        transition: EventOptionTransition::StartCombat,
                        ..Default::default()
                    },
                ),
            ]
        }
        1 | 2 => vec![EventOption::new(
            EventChoiceMeta::new("[Continue]"),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                transition: EventOptionTransition::AdvanceScreen,
                ..Default::default()
            },
        )],
        3 => vec![EventOption::new(
            EventChoiceMeta::new("[Continue]"),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                transition: EventOptionTransition::Complete,
                terminal: true,
                ..Default::default()
            },
        )],
        _ => vec![EventOption::new(
            EventChoiceMeta::new("[Leave]"),
            EventOptionSemantics {
                action: EventActionKind::Leave,
                transition: EventOptionTransition::Complete,
                terminal: true,
                ..Default::default()
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
        0 => {
            match choice_idx {
                0 => {
                    // Pay all gold
                    run_state
                        .set_gold_with_source(0, DomainEventSource::Event(EventId::MaskedBandits));
                    event_state.current_screen = 1;
                }
                _ => {
                    // Fight bandits
                    let gold = if run_state.is_daily_run {
                        run_state.rng_pool.misc_rng.random(30)
                    } else {
                        run_state.rng_pool.misc_rng.random_range(25, 35)
                    };
                    let mut rewards = crate::rewards::state::RewardState::new();
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Gold { amount: gold });

                    if run_state.relics.iter().any(|r| r.id == RelicId::RedMask) {
                        rewards
                            .items
                            .push(crate::rewards::state::RewardItem::Relic {
                                relic_id: RelicId::Circlet,
                            });
                    } else {
                        rewards
                            .items
                            .push(crate::rewards::state::RewardItem::Relic {
                                relic_id: RelicId::RedMask,
                            });
                    }

                    event_state.completed = true;
                    run_state.event_state = Some(event_state);

                    *engine_state = EngineState::CombatStart(CombatStartRequest::event(
                        EncounterId::MaskedBandits,
                        rewards,
                        true,
                        false,
                        false,
                        PostCombatReturn::MapNavigation,
                    ));
                    return;
                }
            }
        }
        1 => {
            event_state.current_screen = 2;
        }
        2 => {
            event_state.current_screen = 3;
        }
        3 => {
            event_state.completed = true;
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
    use crate::content::relics::RelicState;
    use crate::state::core::CombatContext;
    use crate::state::events::{
        EventActionKind, EventEffect, EventOptionTransition, EventRelicKind,
    };
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn structured_options_expose_pay_and_fight_semantics() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 123;
        let event_state = EventState::new(EventId::MaskedBandits);

        let options = get_options(&run_state, &event_state);

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].semantics.action, EventActionKind::Trade);
        assert!(options[0]
            .semantics
            .effects
            .contains(&EventEffect::LoseGold(123)));
        assert_eq!(
            options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        assert_eq!(options[1].semantics.action, EventActionKind::Fight);
        assert!(options[1]
            .semantics
            .effects
            .contains(&EventEffect::StartCombat));
        assert!(options[1]
            .semantics
            .effects
            .contains(&EventEffect::GainGoldRange { min: 25, max: 35 }));
        assert!(options[1]
            .semantics
            .effects
            .contains(&EventEffect::ObtainRelic {
                count: 1,
                kind: EventRelicKind::Specific(RelicId::RedMask),
            }));
        assert_eq!(
            options[1].semantics.transition,
            EventOptionTransition::StartCombat
        );
    }

    #[test]
    fn structured_fight_option_exposes_circlet_when_red_mask_is_already_owned() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::RedMask));
        let event_state = EventState::new(EventId::MaskedBandits);

        let options = get_options(&run_state, &event_state);

        assert!(options[1]
            .semantics
            .effects
            .contains(&EventEffect::ObtainRelic {
                count: 1,
                kind: EventRelicKind::Specific(RelicId::Circlet),
            }));
    }

    #[test]
    fn structured_paid_dialog_continues_until_java_map_open_boundary() {
        let run_state = RunState::new(1, 0, false, "Ironclad");

        let mut screen1 = EventState::new(EventId::MaskedBandits);
        screen1.current_screen = 1;
        let screen1_options = get_options(&run_state, &screen1);
        assert_eq!(
            screen1_options[0].semantics.action,
            EventActionKind::Continue
        );
        assert_eq!(
            screen1_options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        let mut screen3 = EventState::new(EventId::MaskedBandits);
        screen3.current_screen = 3;
        let screen3_options = get_options(&run_state, &screen3);
        assert_eq!(
            screen3_options[0].semantics.action,
            EventActionKind::Continue
        );
        assert_eq!(
            screen3_options[0].semantics.transition,
            EventOptionTransition::Complete
        );
        assert!(screen3_options[0].semantics.terminal);
    }

    #[test]
    fn pay_path_opens_map_after_java_dialog_sequence_without_extra_leave_click() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 123;
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.gold, 0);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: -123,
                new_total: 0,
                source: DomainEventSource::Event(EventId::MaskedBandits),
            }
        )));
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 2);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 3);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert!(run_state.event_state.as_ref().unwrap().completed);
    }

    #[test]
    fn fight_uses_java_event_encounter_and_event_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("fight should request CombatStart");
        };
        assert_eq!(request.encounter_id, EncounterId::MaskedBandits);
        let CombatContext::Event(combat) = request.context else {
            panic!("fight should carry event combat context");
        };
        assert!(combat.reward_allowed);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Gold { amount } if (25..=35).contains(amount))));
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Relic {
                relic_id: RelicId::RedMask
            }
        )));
    }

    #[test]
    fn daily_fight_reward_uses_java_daily_gold_roll() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.is_daily_run = true;
        let mut expected_rng = run_state.rng_pool.misc_rng.clone();
        let expected_gold = expected_rng.random(30);
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("fight should request CombatStart");
        };
        let CombatContext::Event(combat) = request.context else {
            panic!("fight should carry event combat context");
        };
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Gold { amount } if *amount == expected_gold
        )));
    }

    #[test]
    fn fight_reward_gives_circlet_when_red_mask_is_already_owned() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::RedMask));
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("fight should request CombatStart");
        };
        let CombatContext::Event(combat) = request.context else {
            panic!("fight should carry event combat context");
        };
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Relic {
                relic_id: RelicId::Circlet
            }
        )));
    }
}
