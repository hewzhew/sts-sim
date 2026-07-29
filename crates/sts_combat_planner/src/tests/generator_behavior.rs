use super::*;

#[test]
fn policy_guided_generator_emits_preferred_and_complete_sibling_options() {
    let stepper = TinyTurnStepper::lethal();
    let mut session =
        TurnOptionGeneratorSession::with_policy(root(), config(), Arc::new(PreferPlayPolicy));

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(4, 8));

    assert_eq!(report.newly_completed_options, 2);
    assert_eq!(session.completed_options()[0].actions()[0].input, PLAY);
    assert!(session
        .completed_options()
        .iter()
        .any(|option| option.actions()[0].input == ClientInput::EndTurn));
    assert_eq!(session.retained_work_items(), 0);
    assert!(session.is_finished());
}

#[test]
fn shared_guide_publishes_and_services_the_best_partial_expansion() {
    let stepper = TinyTurnStepper::plain();
    let mut generator =
        TurnOptionGeneratorSession::with_policy(root(), config(), Arc::new(SharedGuidePolicy));

    assert_eq!(
        generator
            .best_retained_guide_promise_snapshot(SHARED_TEST_GUIDE)
            .expect("root guide promise")
            .rank,
        CombatStateGuideRank::new(vec![0])
    );

    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));
    generator.prefer_lane(TurnOptionGeneratorPreferredLane::Guide(SHARED_TEST_GUIDE));
    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));

    assert_eq!(
        generator
            .best_retained_guide_promise_snapshot(SHARED_TEST_GUIDE)
            .expect("partial-state guide promise")
            .rank,
        CombatStateGuideRank::new(vec![1]),
        "the resumable parent must publish its best retained partial state, not its stale root rank"
    );

    let guided_before = generator.guided_work_pops();
    generator.prefer_lane(TurnOptionGeneratorPreferredLane::Guide(SHARED_TEST_GUIDE));
    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));
    assert_eq!(generator.guided_work_pops(), guided_before + 1);
}

#[test]
fn atomic_siblings_share_one_resumable_cursor_and_one_service_emits_one_edge() {
    let stepper = TinyTurnStepper::plain();
    let decision_root = root();
    let mut session = TurnOptionGeneratorSession::with_policy(
        decision_root.clone(),
        config(),
        Arc::new(PreferPlayPolicy),
    );

    let discovery = session.advance(&stepper, CombatPlanningQuantum::deterministic(1, 8));
    assert_eq!(discovery.after_diagnostics.applied_action_transitions, 0);
    assert_eq!(session.retained_work_items(), 1);
    assert_eq!(
        session.live_work_counts_at_exact_position(decision_root.position()),
        (0, 2, 0),
        "one cursor owns both concrete atomic siblings"
    );

    let first_edge = session.advance(&stepper, CombatPlanningQuantum::deterministic(1, 8));
    assert_eq!(first_edge.after_diagnostics.applied_action_transitions, 1);
    assert_eq!(
        first_edge
            .after_diagnostics
            .applied_action_transitions
            .saturating_sub(first_edge.before_diagnostics.applied_action_transitions),
        1,
        "one cursor service must not turn into eager all-successor expansion"
    );
}

#[test]
fn generator_publishes_a_reached_turn_boundary_without_rescheduling_it() {
    let stepper = TinyTurnStepper::plain();
    let mut position = root().position().clone();
    position.combat.turn.energy = 0;
    let root = CombatDecisionRoot::new(position).unwrap();
    let mut session = TurnOptionGeneratorSession::new(root, config());

    // One work item expands the root and one executes EndTurn. The resulting
    // next-player-turn state is already stable and must be published without
    // requiring a third agenda pop merely to recognize the boundary.
    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(2, 8));

    assert_eq!(report.after.generation_work, 2);
    assert_eq!(report.newly_completed_options, 1);
    assert_eq!(
        session.completed_options()[0].actions()[0].input,
        ClientInput::EndTurn
    );
}

#[test]
fn only_complete_turn_options_are_public() {
    let stepper = TinyTurnStepper::plain();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let first = session.advance(&stepper, CombatPlanningQuantum::deterministic(2, 4));
    assert!(matches!(
        first.status,
        TurnOptionGenerationStatus::Partial(GenerationInterruption::GenerationWorkBudget)
    ));
    assert!(session.completed_options().is_empty());

    let finished = finish(&mut session, &stepper);
    assert_eq!(finished.status, TurnOptionGenerationStatus::Complete);
    assert_eq!(session.completed_options().len(), 2);
    assert!(session.completed_options().iter().all(|option| {
        option.boundary() == CompleteTurnOptionBoundary::NextPlayerTurn
            && matches!(
                option.actions().last().map(|action| &action.input),
                Some(ClientInput::EndTurn)
            )
    }));
}

#[test]
fn split_quantum_matches_one_shot_without_replaying_transitions() {
    let split_stepper = TinyTurnStepper::plain();
    let mut split = TurnOptionGeneratorSession::new(root(), config());
    split.advance(&split_stepper, CombatPlanningQuantum::deterministic(2, 4));
    finish(&mut split, &split_stepper);

    let one_shot_stepper = TinyTurnStepper::plain();
    let mut one_shot = TurnOptionGeneratorSession::new(root(), config());
    finish(&mut one_shot, &one_shot_stepper);

    let split_options = split
        .completed_options()
        .iter()
        .map(|option| {
            (
                option.actions().to_vec(),
                option.exact_successor_hash().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let one_shot_options = one_shot
        .completed_options()
        .iter()
        .map(|option| {
            (
                option.actions().to_vec(),
                option.exact_successor_hash().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(split_options, one_shot_options);
    assert_eq!(split_stepper.call_count(&PLAY), 1);
    assert_eq!(split_stepper.call_count(&ClientInput::EndTurn), 2);
}

#[test]
fn generation_diagnostics_count_exact_successor_merges_without_changing_options() {
    let stepper = TinyTurnStepper::with_duplicate_play_surface();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let report = finish(&mut session, &stepper);

    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    assert_eq!(session.completed_options().len(), 2);
    assert_eq!(report.after_diagnostics.duplicate_exact_successors, 1);
    assert_eq!(report.after_diagnostics.applied_action_transitions, 4);
    assert_eq!(report.after_diagnostics.unique_successor_states, 3);
    assert_eq!(report.after_diagnostics.completed_turn_options, 2);
}

#[test]
fn engine_transition_waits_for_a_full_reservation() {
    let stepper = TinyTurnStepper::plain();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let blocked = session.advance(&stepper, CombatPlanningQuantum::deterministic(10, 3));
    assert_eq!(
        blocked.status,
        TurnOptionGenerationStatus::Partial(GenerationInterruption::EngineStepBudget)
    );
    assert!(stepper.calls.lock().unwrap().is_empty());

    session.advance(&stepper, CombatPlanningQuantum::deterministic(0, 1));
    assert_eq!(stepper.call_count(&PLAY), 1);
}

#[test]
fn ordered_structured_selections_survive_complete_option_generation() {
    let stepper = TinyTurnStepper::with_selection();
    let mut session = TurnOptionGeneratorSession::new(root(), config());
    let report = finish(&mut session, &stepper);

    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let submitted_orders = session
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(submitted_orders.contains(&vec![11, 22]));
    assert!(submitted_orders.contains(&vec![22, 11]));
}

#[test]
fn singleton_selection_member_policy_reorders_without_removing_siblings() {
    let stepper = TinyTurnStepper::with_single_selection();
    let mut uniform = TurnOptionGeneratorSession::new(root(), config());
    finish(&mut uniform, &stepper);
    let uniform_members = uniform
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(uniform_members, vec![vec![11], vec![22]]);

    let mut guided = TurnOptionGeneratorSession::with_policy(
        root(),
        config(),
        Arc::new(PreferSelection22Policy),
    );
    finish(&mut guided, &stepper);
    let guided_members = guided
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(guided_members, vec![vec![22], vec![11]]);
}

#[test]
fn real_engine_preserves_targeted_potion_inside_an_exact_option() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::JawWorm, 1);
    let target = monster.id;
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::new(
        root.clone(),
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            ..TurnOptionGeneratorConfig::default()
        },
    );

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(1_000, 8_192));
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let option = session
        .completed_options()
        .iter()
        .find(|option| {
            option.actions().iter().any(|action| {
                action.input
                    == ClientInput::UsePotion {
                        potion_index: 0,
                        target: Some(target),
                    }
            })
        })
        .expect("targeted Fire Potion should survive option generation");

    let replay = replay_turn_option(
        &root,
        option,
        &stepper,
        ReplayLimits::deterministic(option.engine_steps()),
    )
    .unwrap();
    assert_eq!(replay.position, *option.exact_successor());
    assert!(option.exact_successor().combat.entities.potions[0].is_none());
    assert!(
        option.exact_successor().combat.entities.monsters[0].current_hp
            < root.position().combat.entities.monsters[0].current_hp
    );
}

#[test]
fn explicit_zero_potion_generator_phase_removes_potion_expenditure_inputs() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::JawWorm, 1);
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::new(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            allow_potion_expenditure: false,
            ..TurnOptionGeneratorConfig::default()
        },
    );

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(1_000, 8_192));
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    assert!(
        session
            .completed_options()
            .iter()
            .all(|option| option.actions().iter().all(|action| !matches!(
                action.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            ))),
        "a zero-potion phase must not generate potion use or discard lines"
    );
}

#[test]
fn finite_potion_generator_limit_prunes_over_budget_prefixes() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::TheGuardian, 1);
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![
        Some(Potion::new(PotionId::EnergyPotion, 7)),
        Some(Potion::new(PotionId::EnergyPotion, 8)),
    ];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::with_policy_and_potion_limit(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            ..TurnOptionGeneratorConfig::default()
        },
        Arc::new(PreferPlayPolicy),
        Some(1),
    );

    let report = session.advance(
        &stepper,
        CombatPlanningQuantum::deterministic(4_000, 32_768),
    );
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let potion_expenditures = session
        .completed_options()
        .iter()
        .map(|option| {
            option
                .actions()
                .iter()
                .filter(|action| {
                    matches!(
                        action.input,
                        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
                    )
                })
                .count()
        })
        .collect::<Vec<_>>();
    assert!(
        potion_expenditures.iter().any(|count| *count == 1),
        "the finite allowance must retain legal one-potion turn lines"
    );
    assert!(
        potion_expenditures.iter().all(|count| *count <= 1),
        "the generator must not spend work completing over-budget turn lines"
    );
}
