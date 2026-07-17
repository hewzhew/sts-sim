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
fn budget_incumbent_preserves_the_exact_interrupted_prospect() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());
    session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(10, 10, 5),
    );

    let CombatPlannerDecisionResult::Selected(decision) = decide_combat_option(&session) else {
        panic!("an interrupted continuation must remain visible on the bounded incumbent");
    };
    assert!(decision.unresolved_gaps.iter().any(|gap| matches!(
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
fn selects_a_typed_budget_incumbent_for_different_nonwinning_exact_states() {
    let session = finish_agenda(&TinyTurnStepper::plain());

    let CombatPlannerDecisionResult::Selected(decision) = decide_combat_option(&session) else {
        panic!("a complete non-losing option must remain executable at the budget boundary");
    };
    assert!(matches!(
        decision.basis,
        CombatPlannerDecisionBasis::BudgetBoundedIncumbent {
            evaluator: CombatPlannerIncumbentEvaluator::ObservedResourceParetoV1,
            exact_winning_horizon: None,
            considered_prospects: 2,
        }
    ));
    assert_eq!(decision.selected_option.actions().len(), 2);
    assert_eq!(decision.nondominated_alternatives.len(), 1);
    assert_eq!(
        decision.unresolved_gaps,
        vec![CombatPlannerDecisionGap::IncomparableExactProspects]
    );
}

#[test]
fn partial_root_generation_selects_only_a_complete_option_and_retains_the_gap() {
    let stepper = TinyTurnStepper::plain();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());
    session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(6, 6, 24),
    );

    let CombatPlannerDecisionResult::Selected(decision) = decide_combat_option(&session) else {
        panic!("a discovered complete option should be usable without exhausting the root");
    };
    assert!(decision
        .unresolved_gaps
        .iter()
        .any(|gap| matches!(gap, CombatPlannerDecisionGap::RetainedAgendaWork { .. })));
    assert!(!decision.selected_option.actions().is_empty());
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
