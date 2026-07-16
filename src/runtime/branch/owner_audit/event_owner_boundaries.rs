use sts_simulator::content::cards::CardId;
use sts_simulator::content::potions::{Potion, PotionId};
use sts_simulator::engine::event_handler::get_event_options;
use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlConfig, RunControlSession, RunDecisionAction,
};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::{ClientInput, EngineState, RunPendingChoiceReason};
use sts_simulator::state::events::{EventEffect, EventId, EventState};
use sts_simulator::state::selection::{DomainEventSource, SelectionScope};

use super::boundary_router::owner_for_current_boundary;
use super::owner_model::{OwnerDecision, OwnerRoutine};
use super::{owners, Owner};

fn event_session(event_id: EventId, screen: usize) -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut event = EventState::new(event_id);
    event.current_screen = screen;
    session.engine_state = EngineState::EventRoom;
    session.run_state.event_state = Some(event);
    session.run_state.emitted_events.clear();
    session
}

fn event_owner_action(session: &RunControlSession, event_id: EventId) -> RunDecisionAction {
    assert!(matches!(
        owner_for_current_boundary(session),
        Some(Owner::Event(actual)) if actual == event_id
    ));
    let surface = build_decision_surface(session);
    match owners::owner_decision(session, Owner::Event(event_id), &surface) {
        OwnerDecision::Routine(OwnerRoutine::Candidate { action, .. }) => action,
        _ => panic!("{event_id:?} owner must produce one executable routine action"),
    }
}

fn apply_event_owner(session: &mut RunControlSession, event_id: EventId) -> ClientInput {
    let action = event_owner_action(session, event_id);
    let RunDecisionAction::Input(input @ ClientInput::EventChoice(_)) = action else {
        panic!("{event_id:?} owner must select a typed event option");
    };
    session
        .apply_decision_action(RunDecisionAction::Input(input.clone()))
        .unwrap_or_else(|err| panic!("{event_id:?} owner action failed: {err}"));
    input
}

fn assert_run_choice_handoff(
    session: &mut RunControlSession,
    event_id: EventId,
    expected_reason: RunPendingChoiceReason,
) {
    let EngineState::RunPendingChoice(pending) = &session.engine_state else {
        panic!("{event_id:?} must enter a real RunPendingChoice");
    };
    assert_eq!(pending.reason, expected_reason);
    assert_eq!(pending.min_choices, 1);
    assert_eq!(pending.max_choices, 1);
    assert!(matches!(
        pending.source,
        DomainEventSource::Event(actual) if actual == event_id
    ));
    assert!(matches!(
        owner_for_current_boundary(session),
        Some(Owner::RunChoice)
    ));

    let surface = build_decision_surface(session);
    let OwnerDecision::Candidates(choices) =
        owners::owner_decision(session, Owner::RunChoice, &surface)
    else {
        panic!("{event_id:?} pending choice must be owned by RunChoice");
    };
    let [choice] = choices.as_slice() else {
        panic!("{event_id:?} RunChoice must produce exactly one committed candidate");
    };
    let RunDecisionAction::Input(ClientInput::SubmitSelection(resolution)) = &choice.action else {
        panic!("{event_id:?} RunChoice must submit a typed deck selection");
    };
    assert_eq!(resolution.scope, SelectionScope::Deck);
    assert_eq!(resolution.selected_card_uuids().len(), pending.max_choices);
    assert!(surface
        .visible_executable_inputs
        .contains(&ClientInput::SubmitSelection(resolution.clone())));

    session
        .apply_decision_action(choice.action.clone())
        .unwrap_or_else(|err| panic!("{event_id:?} RunChoice action was not legal: {err}"));
    assert!(matches!(session.engine_state, EngineState::EventRoom));
}

#[test]
fn deck_events_cross_real_pending_choice_and_typed_run_choice_boundaries() {
    let upgrade_cases = [
        (EventId::UpgradeShrine, RunPendingChoiceReason::Upgrade),
        (EventId::AccursedBlacksmith, RunPendingChoiceReason::Upgrade),
    ];
    for (event_id, reason) in upgrade_cases {
        let mut session = event_session(event_id, 0);
        session.run_state.master_deck = vec![CombatCard::new(CardId::Bash, 101)];

        apply_event_owner(&mut session, event_id);
        assert_run_choice_handoff(&mut session, event_id, reason);
    }

    let mut duplicator = event_session(EventId::Duplicator, 0);
    duplicator.run_state.master_deck = vec![CombatCard::new(CardId::Offering, 201)];
    apply_event_owner(&mut duplicator, EventId::Duplicator);
    assert_run_choice_handoff(
        &mut duplicator,
        EventId::Duplicator,
        RunPendingChoiceReason::Duplicate,
    );

    let mut note = event_session(EventId::NoteForYourself, 1);
    note.run_state.master_deck = vec![CombatCard::new(CardId::Strike, 301)];
    note.run_state.note_for_yourself_card = CardId::Offering;
    apply_event_owner(&mut note, EventId::NoteForYourself);
    assert_run_choice_handoff(
        &mut note,
        EventId::NoteForYourself,
        RunPendingChoiceReason::PurgeNonBottled,
    );

    let mut wheel = event_session(EventId::GremlinWheelGame, 0);
    let base_rng = wheel.run_state.rng_pool.misc_rng.clone();
    let purge_counter = (0..64)
        .find(|counter| {
            let mut candidate = base_rng.clone();
            candidate.advance_counter_to(*counter);
            candidate.random_range(0, 5) == 4
        })
        .expect("a nearby public misc RNG counter must model the wheel purge result");
    wheel
        .run_state
        .rng_pool
        .misc_rng
        .advance_counter_to(purge_counter);
    apply_event_owner(&mut wheel, EventId::GremlinWheelGame);
    assert_run_choice_handoff(
        &mut wheel,
        EventId::GremlinWheelGame,
        RunPendingChoiceReason::PurgeNonBottled,
    );
}

#[test]
fn event_rewards_remain_at_reward_owned_boundaries() {
    let mut lab = event_session(EventId::Lab, 0);
    for (index, slot) in lab.run_state.potions.iter_mut().enumerate() {
        *slot = Some(Potion::new(PotionId::FirePotion, 40_000 + index as u32));
    }
    apply_event_owner(&mut lab, EventId::Lab);
    assert!(matches!(lab.engine_state, EngineState::RewardScreen(_)));
    assert!(matches!(
        owner_for_current_boundary(&lab),
        Some(Owner::RewardTiny)
    ));

    let mut sensory = event_session(EventId::SensoryStone, 1);
    sensory.run_state.current_hp = sensory.run_state.max_hp;
    apply_event_owner(&mut sensory, EventId::SensoryStone);
    assert!(matches!(sensory.engine_state, EngineState::RewardScreen(_)));
    assert!(matches!(
        owner_for_current_boundary(&sensory),
        Some(Owner::RewardTiny)
    ));
}

#[test]
fn colosseum_first_combat_returns_to_event_owner_that_flees() {
    let mut session = event_session(EventId::Colosseum, 0);

    assert_eq!(
        apply_event_owner(&mut session, EventId::Colosseum),
        ClientInput::EventChoice(0)
    );
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        1
    );
    assert_eq!(
        apply_event_owner(&mut session, EventId::Colosseum),
        ClientInput::EventChoice(0)
    );
    assert!(matches!(
        session.engine_state,
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing
    ));
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        2
    );

    let active = session
        .active_combat
        .as_mut()
        .expect("mandatory first fight must create an active combat");
    for monster in &mut active.combat_state.entities.monsters {
        monster.current_hp = 0;
        monster.is_dying = true;
    }
    let finished = sts_simulator::engine::run_loop::tick_run_active_with_observer(
        &mut session.engine_state,
        &mut session.run_state,
        &mut session.active_combat,
        Some(ClientInput::EndTurn),
    );
    assert!(finished.finished_combat.is_some());
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert!(matches!(
        owner_for_current_boundary(&session),
        Some(Owner::Event(EventId::Colosseum))
    ));

    let action = event_owner_action(&session, EventId::Colosseum);
    assert!(matches!(
        action,
        RunDecisionAction::Input(ClientInput::EventChoice(0))
    ));
    session.apply_decision_action(action).unwrap();
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
    assert!(session.active_combat.is_none());
}

#[test]
fn secret_portal_owner_declines_without_entering_boss_combat() {
    let mut session = event_session(EventId::SecretPortal, 0);
    session.run_state.act_num = 3;

    assert_eq!(
        apply_event_owner(&mut session, EventId::SecretPortal),
        ClientInput::EventChoice(1)
    );
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        2
    );
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert!(session.active_combat.is_none());

    assert_eq!(
        apply_event_owner(&mut session, EventId::SecretPortal),
        ClientInput::EventChoice(0)
    );
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
    assert!(session.active_combat.is_none());
}

#[test]
fn knowing_skull_rechecks_escalating_cost_until_safe_leave() {
    let mut session = event_session(EventId::KnowingSkull, 0);
    session.run_state.current_hp = session.run_state.max_hp;
    session.run_state.gold = 0;

    apply_event_owner(&mut session, EventId::KnowingSkull);
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        1
    );

    let mut paid_costs = Vec::new();
    let mut left = false;
    for _ in 0..16 {
        let options = get_event_options(&session.run_state);
        let action = event_owner_action(&session, EventId::KnowingSkull);
        let RunDecisionAction::Input(ClientInput::EventChoice(index)) = action else {
            panic!("Knowing Skull owner must select a typed event option");
        };
        match index {
            1 => {
                let cost = options[index]
                    .semantics
                    .effects
                    .iter()
                    .find_map(|effect| match effect {
                        EventEffect::LoseHp(cost) => Some(*cost),
                        _ => None,
                    })
                    .expect("gold trade must expose its current HP cost");
                if let Some(previous) = paid_costs.last() {
                    assert_eq!(cost, previous + 1);
                }
                paid_costs.push(cost);
                session
                    .apply_decision_action(RunDecisionAction::Input(ClientInput::EventChoice(
                        index,
                    )))
                    .unwrap();
            }
            3 => {
                assert!(
                    !paid_costs.is_empty(),
                    "healthy owner should buy gold first"
                );
                let next_gold_cost = options[1]
                    .semantics
                    .effects
                    .iter()
                    .find_map(|effect| match effect {
                        EventEffect::LoseHp(cost) => Some(*cost),
                        _ => None,
                    })
                    .unwrap();
                assert!(session.run_state.current_hp > next_gold_cost);
                assert!(session.run_state.current_hp > 6);
                session
                    .apply_decision_action(RunDecisionAction::Input(ClientInput::EventChoice(
                        index,
                    )))
                    .unwrap();
                assert!(session.run_state.current_hp > 0);
                assert_eq!(
                    session
                        .run_state
                        .event_state
                        .as_ref()
                        .unwrap()
                        .current_screen,
                    2
                );
                left = true;
                break;
            }
            other => panic!("Knowing Skull owner selected unexpected option {other}"),
        }
    }
    assert!(
        left,
        "Knowing Skull owner must stop before exhausting its HP budget"
    );
}
