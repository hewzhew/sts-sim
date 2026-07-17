use super::*;

fn agenda_config() -> CombatPlannerAgendaConfig {
    CombatPlannerAgendaConfig {
        generator: config(),
        generation_work_per_item: 1,
    }
}

#[test]
fn verifies_terminal_witness_without_promoting_unavailable_continuations() {
    let stepper = TinyTurnStepper::lethal();
    let mut session = CombatPlannerAgendaSession::new(root(), agenda_config());

    let report = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(100, 100, 100),
    );

    assert_eq!(
        report.status,
        CombatPlannerAgendaStatus::ImmediateEvidenceComplete
    );
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
    assert_eq!(
        next_turn.continuation(),
        &ContinuationEvidence::Unavailable(ContinuationUnavailable::FutureTurnPlanningNotStarted)
    );
    assert_eq!(report.after.boundary_witness_replays, 1);
}

#[test]
fn engine_budget_interruption_retains_terminal_verification_work() {
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
    assert_eq!(
        terminal.continuation(),
        &ContinuationEvidence::Interrupted(ContinuationInterruption::EngineStepBudget)
    );

    let resumed = session.advance(
        &stepper,
        CombatPlannerAgendaQuantum::deterministic(10, 10, 1),
    );
    assert_eq!(
        resumed.status,
        CombatPlannerAgendaStatus::ImmediateEvidenceComplete
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
    assert_eq!(split_stepper.call_count(&PLAY), 2);
    assert_eq!(split_stepper.call_count(&ClientInput::EndTurn), 1);
}
