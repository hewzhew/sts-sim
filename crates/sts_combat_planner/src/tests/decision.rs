use super::*;

fn agenda_config() -> CombatPlannerAgendaConfig {
    CombatPlannerAgendaConfig {
        generator: config(),
        generation_work_per_item: 1,
    }
}

fn finish_agenda(stepper: &TinyTurnStepper) -> CombatPlannerAgendaSession {
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());
    let report = session.advance(
        stepper,
        CombatPlannerAgendaQuantum::deterministic(200, 200, 200),
    );
    assert_eq!(report.status, CombatPlannerAgendaStatus::EvidenceComplete);
    session
}

fn outcome_example(enemy_hp: i32, energy: u8, victory: bool) -> CombatOutcomeTrainingExampleV1 {
    let mut position = root().position().clone();
    position.combat.turn.turn_count = 2;
    position.combat.turn.energy = energy;
    position.combat.entities.monsters[0].current_hp = enemy_hp;
    CombatOutcomeTrainingExampleV1::from_position(
        &position,
        victory,
        if victory { 50 } else { 0 },
        80,
        CombatOutcomeLabelProvenanceV1::ExactScenarioReplay,
        "tiny-exact-continuation-v1",
    )
}

fn calibrated_outcome_model() -> CombatOutcomeModelV1 {
    let training = (0..24)
        .flat_map(|index| {
            [
                outcome_example(16 + index % 8, (index % 2) as u8, true),
                outcome_example(36 + index % 8, (index % 2) as u8, false),
            ]
        })
        .collect::<Vec<_>>();
    let calibration = (0..8)
        .flat_map(|index| {
            [
                outcome_example(18 + index % 4, (index % 2) as u8, true),
                outcome_example(38 + index % 4, (index % 2) as u8, false),
            ]
        })
        .collect::<Vec<_>>();
    let config = CombatOutcomeModelTrainingConfigV1 {
        epochs: 4_000,
        learning_rate: 0.2,
        ..CombatOutcomeModelTrainingConfigV1::default()
    };
    CombatOutcomeModelV1::fit(
        "exact-outcome-test-v1",
        "tiny-exact-continuation-v1",
        &training,
        &calibration,
        config,
    )
    .unwrap()
}

#[test]
fn defers_instead_of_ranking_partial_evidence() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());
    session.advance(&stepper, CombatPlannerAgendaQuantum::deterministic(1, 1, 4));

    let CombatPlannerDecisionResult::Deferred(deferral) = decide_combat_option(&session) else {
        panic!("partial generation must not select an incumbent");
    };
    assert!(deferral
        .gaps
        .iter()
        .any(|gap| matches!(gap, CombatPlannerDecisionGap::RetainedAgendaWork { .. })));
}

#[test]
fn selects_a_unique_verified_terminal_win_without_a_scalar_score() {
    let session = finish_agenda(&TinyTurnStepper::lethal());

    let CombatPlannerDecisionResult::Selected(decision) = decide_combat_option(&session) else {
        panic!("the only verified immediate win should be selected");
    };
    assert_eq!(
        decision.basis,
        CombatPlannerDecisionBasis::VerifiedTerminalWin
    );
    assert_eq!(
        decision.selected_option.boundary(),
        CompleteTurnOptionBoundary::TerminalWin
    );
    assert!(decision.nondominated_alternatives.is_empty());
}

#[test]
fn selects_the_only_option_with_an_exact_win_in_the_next_turn() {
    let session = finish_agenda(&TinyTurnStepper::lethal_after_current_turn());

    let CombatPlannerDecisionResult::Selected(decision) = decide_combat_option(&session) else {
        panic!("the unique exact one-turn winning continuation should be selected");
    };
    assert_eq!(
        decision.basis,
        CombatPlannerDecisionBasis::PreferredExactWinningHorizon { turn_boundaries: 1 }
    );
    assert_eq!(decision.selected_option.actions().len(), 1);
    assert_eq!(
        decision.selected_option.actions()[0].input,
        ClientInput::EndTurn
    );
}

#[test]
fn deferral_preserves_the_exact_interrupted_prospect() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());
    session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(10, 10, 6),
    );

    let CombatPlannerDecisionResult::Deferred(deferral) = decide_combat_option(&session) else {
        panic!("an interrupted continuation must remain unselectable");
    };
    assert!(deferral.gaps.iter().any(|gap| matches!(
        gap,
        CombatPlannerDecisionGap::ProspectEvidence {
            gap: ProspectEvidenceGap::Interrupted(ContinuationInterruption::EngineStepBudget),
            ..
        }
    )));
}

#[test]
fn defers_a_forced_terminal_loss_without_policy_authority() {
    let session = finish_agenda(&TinyTurnStepper::losing());

    let CombatPlannerDecisionResult::Deferred(deferral) = decide_combat_option(&session) else {
        panic!("mechanical uniqueness must not authorize choosing a terminal loss");
    };
    assert!(matches!(
        deferral.gaps.as_slice(),
        [CombatPlannerDecisionGap::UnresolvedBoundaryPreference {
            boundary: CompleteTurnOptionBoundary::TerminalLoss,
            ..
        }]
    ));
}

#[test]
fn leaves_different_nonwinning_exact_states_incomparable() {
    let session = finish_agenda(&TinyTurnStepper::plain());

    let CombatPlannerDecisionResult::Deferred(deferral) = decide_combat_option(&session) else {
        panic!("no hidden scalar should rank different nonwinning states");
    };
    assert_eq!(
        deferral.gaps,
        vec![CombatPlannerDecisionGap::IncomparableExactProspects]
    );
    assert_eq!(deferral.nondominated_prospects.len(), 2);
}

#[test]
fn calibrated_outcome_model_selects_only_when_success_intervals_separate() {
    let session = finish_agenda(&TinyTurnStepper::damaging(20));

    let result = decide_combat_option_with_outcome_model(&session, &calibrated_outcome_model());
    let CombatPlannerDecisionResult::Selected(decision) = result else {
        panic!("calibrated separated outcome estimates should select: {result:?}");
    };
    assert!(matches!(
        decision.basis,
        CombatPlannerDecisionBasis::PreferredCalibratedOutcome { .. }
    ));
    assert_eq!(decision.selected_option.actions().len(), 2);
    assert!(matches!(
        decision.selected_option.actions()[0].input,
        ClientInput::PlayCard { .. }
    ));
}

#[test]
fn split_and_one_shot_agendas_select_the_same_option_and_basis() {
    let split_stepper = TinyTurnStepper::lethal_after_current_turn();
    let mut split = CombatPlannerAgendaSession::new(root(), agenda_config());
    split.advance(
        &split_stepper,
        CombatPlannerAgendaQuantum::deterministic(8, 8, 8),
    );
    split.advance(
        &split_stepper,
        CombatPlannerAgendaQuantum::deterministic(192, 192, 192),
    );

    let one_shot = finish_agenda(&TinyTurnStepper::lethal_after_current_turn());
    assert_eq!(
        decide_combat_option(&split),
        decide_combat_option(&one_shot)
    );
}
