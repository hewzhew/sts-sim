use sts_simulator::eval::run_control::{
    build_decision_surface, capture_planner_boundary_yield_v1, BoundedRunDriveStopV1,
    BoundedRunDriver, BoundedRunStepContextV1, BoundedRunStepControlV1, CombatSearchTraceSummary,
    PlannerBoundaryCaptureSegmentV1, PlannerBoundaryYieldKindV1, RunControlAutoStepOptions,
    RunControlAutoStopKind, RunControlAutoStopV1, RunControlRouteAutomationMode, RunControlSession,
    RunProgressJournalV1, RunProgressStepV1,
};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::combat_search_orchestrator;
use super::combat_search_report::CombatSearchSessionReport;
use super::combat_search_session_result::CombatSearchSessionResult;
use super::owner_orchestrator::{orchestrate_owner_boundary, OwnerOrchestration};
use super::run_cutpoint_recorder::RunCutpointRecorder;
use super::run_deadline::RunDeadline;
use super::{Args, BranchStatus, TerminalOutcome};

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) combat_portfolio: Option<CombatSearchSessionReport>,
    pub(super) progress_journal: RunProgressJournalV1,
    pub(super) planner_capture: PlannerBoundaryCaptureSegmentV1,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

#[derive(Default)]
struct AdvanceLog {
    combat_search: Vec<CombatSearchTraceSummary>,
    accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

pub(super) fn advance_to_owner_or_gap(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
) -> AdvanceResult {
    advance_to_owner_or_gap_impl(session, args, deadline, None)
}

pub(super) fn advance_to_owner_or_gap_with_cutpoints(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
    cutpoints: &mut RunCutpointRecorder<'_>,
) -> AdvanceResult {
    advance_to_owner_or_gap_impl(session, args, deadline, Some(cutpoints))
}

fn advance_to_owner_or_gap_impl(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
    mut cutpoints: Option<&mut RunCutpointRecorder<'_>>,
) -> AdvanceResult {
    if args.auto_ops == 0 {
        let planner_capture = match capture_planner_boundary_yield_v1(
            session,
            PlannerBoundaryYieldKindV1::ProgressBudgetExhausted,
        ) {
            Ok(capture) => capture,
            Err(error) => return advance_failed(error),
        };
        let mut result = advance_result(
            BranchStatus::OperationBudgetExhausted {
                boundary: build_decision_surface(session).view.header.title,
                reason: "progress-step budget exhausted".to_string(),
            },
            None,
            RunProgressJournalV1::default(),
            Vec::new(),
            Vec::new(),
        );
        result.planner_capture = planner_capture;
        return result;
    }
    let driver = match BoundedRunDriver::new(args.auto_ops, deadline.remaining_ms()) {
        Ok(driver) => driver,
        Err(error) => return advance_failed(error),
    };
    let mut log = AdvanceLog::default();
    let drive = driver.drive_with(session, |session, context| {
        execute_one_owner_audit_step(session, args, deadline, context, &mut cutpoints, &mut log)
    });
    let drive = match drive {
        Ok(drive) => drive,
        Err(error) => {
            let mut result = log.finish(
                BranchStatus::AdvanceFailed(error.message),
                None,
                error.journal,
            );
            result.planner_capture = error.planner_capture;
            return result;
        }
    };
    let applied_progress_steps = drive.applied_progress_steps();
    let planner_capture = drive.planner_capture;
    let journal = drive.journal;
    match drive.stop {
        BoundedRunDriveStopV1::Step(mut result) => {
            result.progress_journal = journal;
            result.planner_capture = planner_capture;
            result
        }
        BoundedRunDriveStopV1::ProgressBudgetExhausted => {
            let stop = RunControlAutoStopV1 {
                kind: RunControlAutoStopKind::ProgressBudgetExhausted,
                reason: format!(
                    "progress-step budget exhausted at {}",
                    applied_progress_steps
                ),
                applied_operations: applied_progress_steps,
            };
            let status = super::boundary_router::classify_boundary(session, &stop);
            let mut result = log.finish(status, None, journal);
            result.planner_capture = planner_capture;
            result
        }
        BoundedRunDriveStopV1::WallDeadlineReached => {
            let stop = RunControlAutoStopV1 {
                kind: RunControlAutoStopKind::WallDeadlineReached,
                reason: "bounded run wall deadline reached".to_string(),
                applied_operations: applied_progress_steps,
            };
            let status = super::boundary_router::classify_boundary(session, &stop);
            let mut result = log.finish(status, None, journal);
            result.planner_capture = planner_capture;
            result
        }
        BoundedRunDriveStopV1::RunCompleted { victory } => {
            let mut result = log.finish(
                BranchStatus::Terminal(if victory {
                    TerminalOutcome::Victory
                } else {
                    TerminalOutcome::Defeat
                }),
                None,
                journal,
            );
            result.planner_capture = planner_capture;
            result
        }
    }
}

fn advance_failed(message: String) -> AdvanceResult {
    advance_result(
        BranchStatus::AdvanceFailed(message),
        None,
        RunProgressJournalV1::default(),
        Vec::new(),
        Vec::new(),
    )
}

fn execute_one_owner_audit_step(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
    context: BoundedRunStepContextV1,
    cutpoints: &mut Option<&mut RunCutpointRecorder<'_>>,
    log: &mut AdvanceLog,
) -> Result<BoundedRunStepControlV1<AdvanceResult>, String> {
    if session.active_combat.is_none() {
        return execute_one_noncombat_step(session, log);
    }
    let run_args = deadline.cap_args(args, 1);
    if let Some(recorder) = cutpoints.as_deref_mut() {
        recorder
            .before_combat_search(session, &context)
            .map_err(|error| format!("cutpoint persistence failed: {error}"))?;
    }
    let search = match combat_search_orchestrator::run_combat_search_session_step(session, run_args)
    {
        Ok(search) => search,
        Err(error) => {
            if let Some(recorder) = cutpoints.as_deref_mut() {
                recorder.retain_on_error().map_err(|cutpoint_error| {
                    format!(
                        "combat search failed: {error}; cutpoint persistence failed: {cutpoint_error}"
                    )
                })?;
            }
            return Err(format!("combat search failed: {error}"));
        }
    };
    absorb_search_output(log, &search);
    let progress_steps = search.progress_steps.clone();
    if let Some(recorder) = cutpoints.as_deref_mut() {
        if let Err(error) = recorder.after_combat_search(&search.status) {
            return Ok(BoundedRunStepControlV1::Stop {
                progress_steps,
                output: log.finish(
                    BranchStatus::AdvanceFailed(format!("cutpoint persistence failed: {error}")),
                    None,
                    RunProgressJournalV1::default(),
                ),
            });
        }
    }
    let applied = progress_steps.len();
    let status = search.status;
    if search.report.is_some() || applied == 0 {
        return Ok(BoundedRunStepControlV1::Stop {
            progress_steps,
            output: log.finish(status, search.report, RunProgressJournalV1::default()),
        });
    }
    Ok(BoundedRunStepControlV1::Continue { progress_steps })
}

fn execute_one_noncombat_step(
    session: &mut RunControlSession,
    log: &mut AdvanceLog,
) -> Result<BoundedRunStepControlV1<AdvanceResult>, String> {
    let outcome = session.apply_progress_step(RunControlAutoStepOptions {
        route: RunControlRouteAutomationMode::Planner,
        ..RunControlAutoStepOptions::default()
    })?;
    match outcome.progress_steps.as_slice() {
        [RunProgressStepV1::Decision(_)
        | RunProgressStepV1::ForcedTransition(_)
        | RunProgressStepV1::CombatResolution(_)] => {
            return Ok(BoundedRunStepControlV1::Continue {
                progress_steps: outcome.progress_steps.clone(),
            });
        }
        [RunProgressStepV1::Stop(_)] => {}
        _ => {
            return Err(
                "owner-audit atomic progress produced neither one mutation nor one stop"
                    .to_string(),
            )
        }
    }
    let status = super::boundary_router::classify_auto_outcome(session, &outcome);
    let BranchStatus::Running { owner, .. } = status else {
        return Ok(BoundedRunStepControlV1::Stop {
            progress_steps: Vec::new(),
            output: log.finish(status, None, RunProgressJournalV1::default()),
        });
    };
    match orchestrate_owner_boundary(session, owner) {
        OwnerOrchestration::StopAtCandidates => Ok(BoundedRunStepControlV1::Stop {
            progress_steps: Vec::new(),
            output: log.finish(status, None, RunProgressJournalV1::default()),
        }),
        OwnerOrchestration::Stop(status) => Ok(BoundedRunStepControlV1::Stop {
            progress_steps: Vec::new(),
            output: log.finish(status, None, RunProgressJournalV1::default()),
        }),
        OwnerOrchestration::AppliedRoutine(step) => Ok(BoundedRunStepControlV1::Continue {
            progress_steps: vec![step],
        }),
    }
}

fn absorb_search_output(log: &mut AdvanceLog, search: &CombatSearchSessionResult) {
    log.combat_search
        .extend(search.combat_search.iter().cloned());
    log.accepted_high_loss_diagnostics
        .extend(search.accepted_high_loss_diagnostics.iter().cloned());
}

impl AdvanceLog {
    fn finish(
        &mut self,
        status: BranchStatus,
        combat_portfolio: Option<CombatSearchSessionReport>,
        progress_journal: RunProgressJournalV1,
    ) -> AdvanceResult {
        advance_result(
            status,
            combat_portfolio,
            progress_journal,
            std::mem::take(&mut self.combat_search),
            std::mem::take(&mut self.accepted_high_loss_diagnostics),
        )
    }
}

fn advance_result(
    status: BranchStatus,
    combat_portfolio: Option<CombatSearchSessionReport>,
    progress_journal: RunProgressJournalV1,
    combat_search: Vec<CombatSearchTraceSummary>,
    accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
) -> AdvanceResult {
    AdvanceResult {
        status,
        combat_portfolio,
        progress_journal,
        planner_capture: PlannerBoundaryCaptureSegmentV1::default(),
        combat_search,
        accepted_high_loss_diagnostics,
    }
}
