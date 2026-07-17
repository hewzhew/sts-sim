use super::*;

fn agenda_config() -> CombatPlannerAgendaConfig {
    CombatPlannerAgendaConfig {
        generator: config(),
        generation_work_per_item: 1,
    }
}

#[test]
fn unused_transition_reservation_is_released_after_each_agenda_item() {
    let stepper = TinyTurnStepper::plain();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());

    session.advance(&stepper, CombatPlannerAgendaQuantum::deterministic(1, 1, 4));

    let committed = session.committed_budget_for_test();
    assert_eq!(committed.generation_work, 1);
    assert_eq!(committed.engine_steps, 0);
}

#[test]
fn verifies_terminal_witness_and_builds_one_turn_exact_horizon() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());

    let report = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(100, 100, 100),
    );

    assert_eq!(report.status, CombatPlannerAgendaStatus::EvidenceComplete);
    assert_eq!(session.prospects().len(), 2);
    let terminal = session
        .prospects()
        .iter()
        .find(|prospect| prospect.option().boundary() == CompleteTurnOptionBoundary::TerminalWin)
        .unwrap();
    assert!(matches!(
        terminal.continuation(),
        ContinuationEvidence::VerifiedBoundary(BoundaryWitnessEvidence {
            boundary: CompleteTurnOptionBoundary::TerminalWin,
            ..
        })
    ));
    let next_turn = session
        .prospects()
        .iter()
        .find(|prospect| prospect.option().boundary() == CompleteTurnOptionBoundary::NextPlayerTurn)
        .unwrap();
    let ContinuationEvidence::ExactHorizon(horizon) = next_turn.continuation() else {
        panic!("next-turn prospect should carry an exact one-turn horizon");
    };
    assert_eq!(horizon.turn_boundaries, 1);
    assert_eq!(horizon.complete_options.len(), 2);
    assert_eq!(report.after.boundary_witness_replays, 1);
}

#[test]
fn engine_budget_interruption_retains_verified_terminal_while_continuation_waits() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());

    let interrupted = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(10, 10, 5),
    );
    assert_eq!(
        interrupted.status,
        CombatPlannerAgendaStatus::Partial(CombatPlannerAgendaInterruption::EngineStepBudget)
    );
    let terminal = session
        .prospects()
        .iter()
        .find(|prospect| prospect.option().boundary() == CompleteTurnOptionBoundary::TerminalWin)
        .unwrap();
    assert!(matches!(
        terminal.continuation(),
        ContinuationEvidence::VerifiedBoundary(_)
    ));
    let next_turn = session
        .prospects()
        .iter()
        .find(|prospect| prospect.option().boundary() == CompleteTurnOptionBoundary::NextPlayerTurn)
        .unwrap();
    assert_eq!(
        next_turn.continuation(),
        &ContinuationEvidence::Interrupted(ContinuationInterruption::EngineStepBudget)
    );

    let resumed = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(10, 10, 1),
    );
    assert_eq!(
        resumed.status,
        CombatPlannerAgendaStatus::Partial(CombatPlannerAgendaInterruption::EngineStepBudget)
    );
    assert!(matches!(
        session
            .prospects()
            .iter()
            .find(|prospect| {
                prospect.option().boundary() == CompleteTurnOptionBoundary::TerminalWin
            })
            .unwrap()
            .continuation(),
        ContinuationEvidence::VerifiedBoundary(_)
    ));
    assert_eq!(resumed.after.boundary_witness_replays, 1);

    let completed = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(100, 100, 100),
    );
    assert_eq!(
        completed.status,
        CombatPlannerAgendaStatus::EvidenceComplete
    );
}

#[test]
fn interleaves_root_discovery_with_one_continuation_quantum() {
    let stepper = TinyTurnStepper::plain();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());

    let report = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(6, 100, 100),
    );

    assert_eq!(
        report.status,
        CombatPlannerAgendaStatus::Partial(CombatPlannerAgendaInterruption::AgendaItemBudget)
    );
    assert_eq!(report.after.continuation_generation_work, 1);
    assert!(!session.prospects().is_empty());
    assert!(session.prospects().iter().all(|prospect| matches!(
        prospect.continuation(),
        ContinuationEvidence::Interrupted(ContinuationInterruption::GenerationWorkBudget)
    )));

    let completed = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(100, 100, 100),
    );
    assert_eq!(
        completed.status,
        CombatPlannerAgendaStatus::EvidenceComplete
    );
}

#[test]
fn split_quanta_retain_the_same_evidence_without_replaying_generation() {
    let split_stepper = TinyTurnStepper::lethal();
    let mut split = CombatPlannerAgendaSession::new(root(), agenda_config());
    split.advance(
        &split_stepper,
        CombatPlannerAgendaQuantum::deterministic(2, 2, 4),
    );
    let split_report = split.advance(
        &split_stepper,
        CombatPlannerAgendaQuantum::deterministic(98, 98, 96),
    );

    let one_shot_stepper = TinyTurnStepper::lethal();
    let mut one_shot = CombatPlannerAgendaSession::new(root(), agenda_config());
    let one_shot_report = one_shot.advance(
        &one_shot_stepper,
        CombatPlannerAgendaQuantum::deterministic(100, 100, 100),
    );

    let evidence = |session: &CombatPlannerAgendaSession| {
        session
            .prospects()
            .iter()
            .map(|prospect| {
                (
                    prospect.option().boundary(),
                    prospect.option().exact_successor_hash().to_owned(),
                    prospect.continuation().clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(split_report.status, one_shot_report.status);
    assert_eq!(split_report.after, one_shot_report.after);
    assert_eq!(evidence(&split), evidence(&one_shot));
    assert_eq!(split_stepper.call_count(&PLAY), 3);
    assert_eq!(split_stepper.call_count(&ClientInput::EndTurn), 2);
}
