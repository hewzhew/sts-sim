use crate::ai::event_policy_v1::{
    build_event_decision_context_v1, plan_event_decision_v1, EventPolicyActionV1,
    EventPolicyClassV1, EventPolicyConfigV1,
};
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventId, EventOption, EventOptionSemantics,
    EventOptionTransition, EventState,
};
use crate::state::run::RunState;

#[test]
fn event_context_classifies_free_known_benefit() {
    let run = RunState::new(1, 0, false, "Ironclad");
    let context = build_event_decision_context_v1(
        &run,
        EventId::GoldenShrine,
        vec![
            option(
                "[Pray] Gain 100 Gold.",
                EventActionKind::Gain,
                vec![EventEffect::GainGold(100)],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Desecrate] Gain 275 Gold. Become Cursed - Regret.",
                EventActionKind::Gain,
                vec![
                    EventEffect::GainGold(275),
                    EventEffect::ObtainCurse {
                        count: 1,
                        kind: crate::state::events::EventCardKind::Specific(
                            crate::content::cards::CardId::Regret,
                        ),
                    },
                ],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Leave]",
                EventActionKind::Leave,
                vec![],
                EventOptionTransition::Complete,
            ),
        ],
    );

    assert_eq!(
        context.candidates[0].class,
        EventPolicyClassV1::FreeKnownBenefit
    );
}

#[test]
fn event_context_classifies_leave_option_without_deciding_policy() {
    let run = RunState::new(1, 0, false, "Ironclad");
    let context = build_event_decision_context_v1(
        &run,
        EventId::Addict,
        vec![
            option(
                "[Pay] Lose 85 Gold. Obtain a random Relic.",
                EventActionKind::Trade,
                vec![
                    EventEffect::LoseGold(85),
                    EventEffect::ObtainRelic {
                        count: 1,
                        kind: crate::state::events::EventRelicKind::RandomRelic,
                    },
                ],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Rob] Obtain a random Relic. Become Cursed - Shame.",
                EventActionKind::Trade,
                vec![
                    EventEffect::ObtainRelic {
                        count: 1,
                        kind: crate::state::events::EventRelicKind::RandomRelic,
                    },
                    EventEffect::ObtainCurse {
                        count: 1,
                        kind: crate::state::events::EventCardKind::Specific(
                            crate::content::cards::CardId::Shame,
                        ),
                    },
                ],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Leave]",
                EventActionKind::Leave,
                vec![],
                EventOptionTransition::Complete,
            ),
        ],
    );

    assert_eq!(context.candidates[2].class, EventPolicyClassV1::SafeExit);
}

#[test]
fn event_policy_does_not_treat_attack_as_safe_exit() {
    let run = RunState::new(1, 0, false, "Ironclad");
    let context = build_event_decision_context_v1(
        &run,
        EventId::WeMeetAgain,
        vec![
            option(
                "[Give Gold] Lose 124 Gold. Obtain a Relic.",
                EventActionKind::Trade,
                vec![
                    EventEffect::LoseGold(124),
                    EventEffect::ObtainRelic {
                        count: 1,
                        kind: crate::state::events::EventRelicKind::RandomRelic,
                    },
                ],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Give Card] Give Clothesline. Obtain a Relic.",
                EventActionKind::Trade,
                vec![
                    EventEffect::RemoveCard {
                        count: 1,
                        target_uuid: Some(42),
                        kind: crate::state::events::EventCardKind::Specific(
                            crate::content::cards::CardId::Clothesline,
                        ),
                    },
                    EventEffect::ObtainRelic {
                        count: 1,
                        kind: crate::state::events::EventRelicKind::RandomRelic,
                    },
                ],
                EventOptionTransition::AdvanceScreen,
            ),
            option(
                "[Attack]",
                EventActionKind::Decline,
                vec![],
                EventOptionTransition::AdvanceScreen,
            ),
        ],
    );

    let decision = plan_event_decision_v1(&context, &EventPolicyConfigV1::default());

    assert!(matches!(decision.action, EventPolicyActionV1::Stop { .. }));
}

#[test]
fn event_policy_stops_for_neow() {
    let run = RunState::new(1, 0, false, "Ironclad");
    let context = build_event_decision_context_v1(
        &run,
        EventId::Neow,
        vec![option(
            "Obtain a random rare card.",
            EventActionKind::Gain,
            vec![EventEffect::ObtainCard {
                count: 1,
                kind: crate::state::events::EventCardKind::RandomClassRare,
            }],
            EventOptionTransition::AdvanceScreen,
        )],
    );

    let decision = plan_event_decision_v1(&context, &EventPolicyConfigV1::default());

    assert!(matches!(decision.action, EventPolicyActionV1::Stop { .. }));
}

#[test]
fn event_policy_takes_max_hp_for_hp_when_health_buffer_is_safe() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.current_hp = 74;
    run.max_hp = 80;
    run.event_state = Some(EventState::new(EventId::ForgottenAltar));
    let options = crate::content::events::forgotten_altar::get_options(
        &run,
        run.event_state.as_ref().unwrap(),
    );
    let context = build_event_decision_context_v1(&run, EventId::ForgottenAltar, options);

    let decision = plan_event_decision_v1(&context, &EventPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        EventPolicyActionV1::Pick { index: 1, .. }
    ));
}

#[test]
fn event_policy_stops_max_hp_for_hp_when_health_buffer_is_low() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.current_hp = 24;
    run.max_hp = 80;
    run.event_state = Some(EventState::new(EventId::ForgottenAltar));
    let options = crate::content::events::forgotten_altar::get_options(
        &run,
        run.event_state.as_ref().unwrap(),
    );
    let context = build_event_decision_context_v1(&run, EventId::ForgottenAltar, options);

    let decision = plan_event_decision_v1(&context, &EventPolicyConfigV1::default());

    assert!(matches!(decision.action, EventPolicyActionV1::Stop { .. }));
}

fn option(
    label: &'static str,
    action: EventActionKind,
    effects: Vec<EventEffect>,
    transition: EventOptionTransition,
) -> EventOption {
    EventOption::new(
        EventChoiceMeta::new(label),
        EventOptionSemantics {
            action,
            effects,
            constraints: Vec::new(),
            transition,
            repeatable: false,
            terminal: transition == EventOptionTransition::Complete,
        },
    )
}
