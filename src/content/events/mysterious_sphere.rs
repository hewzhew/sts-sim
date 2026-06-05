use crate::content::monsters::factory::EncounterId;
use crate::state::core::{CombatStartRequest, EngineState, PostCombatReturn};
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventOption, EventOptionSemantics,
    EventOptionTransition, EventRelicKind, EventState,
};
use crate::state::run::RunState;

fn fight_gold_range(run_state: &RunState) -> (i32, i32) {
    if run_state.is_daily_run {
        (0, 49)
    } else {
        (45, 55)
    }
}

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => vec![
            EventOption::new(
                EventChoiceMeta::new("[Open] Fight the guardians for a rare Relic!"),
                EventOptionSemantics {
                    action: EventActionKind::Fight,
                    transition: EventOptionTransition::AdvanceScreen,
                    ..Default::default()
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("[Leave]"),
                EventOptionSemantics {
                    action: EventActionKind::Leave,
                    transition: EventOptionTransition::AdvanceScreen,
                    ..Default::default()
                },
            ),
        ],
        1 => {
            // Confirm fight
            let (min, max) = fight_gold_range(run_state);
            vec![EventOption::new(
                EventChoiceMeta::new("[Fight]"),
                EventOptionSemantics {
                    action: EventActionKind::Fight,
                    effects: vec![
                        EventEffect::GainGoldRange { min, max },
                        EventEffect::ObtainRelic {
                            count: 1,
                            kind: EventRelicKind::RandomRareRelic,
                        },
                        EventEffect::StartCombat,
                    ],
                    transition: EventOptionTransition::StartCombat,
                    terminal: true,
                    ..Default::default()
                },
            )]
        }
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
                    // Open — advance to confirm screen
                    event_state.current_screen = 1;
                }
                _ => {
                    // Java first moves to END text, then opens the map on the
                    // next click.
                    event_state.current_screen = 2;
                }
            }
        }
        1 => {
            // Fight! Set up rewards and enter event combat.
            // Java: daily uses miscRng.random(50), ordinary runs use miscRng.random(45, 55).
            let gold = if run_state.is_daily_run {
                run_state.rng_pool.misc_rng.random(50)
            } else {
                run_state.rng_pool.misc_rng.random_range(45, 55)
            };
            let mut rewards = crate::rewards::state::RewardState::new();
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Gold { amount: gold });

            let relic_id =
                run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Relic { relic_id });

            event_state.completed = true;
            run_state.event_state = Some(event_state);

            // Transition to event combat
            *engine_state = EngineState::CombatStart(CombatStartRequest::event(
                EncounterId::TwoOrbWalkers,
                rewards,
                true,
                false,
                false,
                PostCombatReturn::MapNavigation,
            ));
            return;
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
    use crate::content::relics::RelicId;
    use crate::state::core::CombatContext;
    use crate::state::events::{
        EventActionKind, EventEffect, EventId, EventOptionTransition, EventRelicKind,
    };

    #[test]
    fn structured_options_split_open_leave_and_confirm_fight() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::MysteriousSphere));

        let options = crate::engine::event_handler::try_get_structured_event_options_for_state(
            &run_state,
            run_state.event_state.as_ref().unwrap(),
        )
        .expect("Mysterious Sphere should expose structured event options");

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].semantics.action, EventActionKind::Fight);
        assert_eq!(
            options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );
        assert_eq!(options[1].semantics.action, EventActionKind::Leave);
        assert_eq!(
            options[1].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        let mut confirm = EventState::new(EventId::MysteriousSphere);
        confirm.current_screen = 1;
        let confirm_options =
            crate::engine::event_handler::try_get_structured_event_options_for_state(
                &run_state, &confirm,
            )
            .expect("Mysterious Sphere confirm screen should expose structured event options");

        assert_eq!(confirm_options[0].semantics.action, EventActionKind::Fight);
        assert!(confirm_options[0]
            .semantics
            .effects
            .contains(&EventEffect::GainGoldRange { min: 45, max: 55 }));
        assert!(confirm_options[0]
            .semantics
            .effects
            .contains(&EventEffect::ObtainRelic {
                count: 1,
                kind: EventRelicKind::RandomRareRelic,
            }));
        assert!(confirm_options[0]
            .semantics
            .effects
            .contains(&EventEffect::StartCombat));
        assert_eq!(
            confirm_options[0].semantics.transition,
            EventOptionTransition::StartCombat
        );
    }

    #[test]
    fn leave_path_preserves_java_end_screen_before_map() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let event_state = run_state.event_state.as_ref().unwrap();
        assert_eq!(event_state.current_screen, 2);
        assert!(!event_state.completed);
        assert!(matches!(engine_state, EngineState::EventRoom));

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state.event_state.as_ref().unwrap().completed);
    }

    #[test]
    fn fight_path_generates_java_event_rewards_before_event_combat() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("confirmed Mysterious Sphere fight should request CombatStart");
        };
        assert_eq!(request.encounter_id, EncounterId::TwoOrbWalkers);
        let CombatContext::Event(combat) = request.context else {
            panic!("confirmed Mysterious Sphere fight should carry event combat context");
        };
        assert!(combat.reward_allowed);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Gold { amount } if (45..=55).contains(amount))));
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Relic { .. })));
    }

    #[test]
    fn daily_fight_reward_uses_java_daily_gold_roll() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.is_daily_run = true;
        let mut expected_rng = run_state.rng_pool.misc_rng.clone();
        let expected_gold = expected_rng.random(50);
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("confirmed Mysterious Sphere fight should request CombatStart");
        };
        let CombatContext::Event(combat) = request.context else {
            panic!("confirmed Mysterious Sphere fight should carry event combat context");
        };
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Gold { amount } if *amount == expected_gold
        )));
    }

    #[test]
    fn fight_reward_uses_rare_screenless_relic_pool() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.rare_relic_pool = vec![RelicId::Mango];
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("confirmed Mysterious Sphere fight should request CombatStart");
        };
        let CombatContext::Event(combat) = request.context else {
            panic!("confirmed Mysterious Sphere fight should carry event combat context");
        };
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Relic {
                relic_id: RelicId::Mango
            }
        )));
    }
}
