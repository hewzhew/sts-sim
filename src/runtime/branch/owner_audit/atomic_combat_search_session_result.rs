use sts_simulator::eval::run_control::{AtomicCombatSearchTraceSummaryV2, RunProgressStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::atomic_combat_search_report::AtomicCombatSearchSessionReportV2;
use super::atomic_combat_search_session_output::AtomicCombatSearchSessionOutputV2;
use super::BranchStatus;

pub(super) struct AtomicCombatSearchSessionResultV2 {
    pub(super) status: BranchStatus,
    pub(super) report: Option<AtomicCombatSearchSessionReportV2>,
    pub(super) progress_steps: Vec<RunProgressStepV1>,
    pub(super) atomic_combat_search_attempts: Vec<AtomicCombatSearchTraceSummaryV2>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

pub(super) fn atomic_combat_search_result(
    status: BranchStatus,
    report: Option<AtomicCombatSearchSessionReportV2>,
    output: AtomicCombatSearchSessionOutputV2,
) -> AtomicCombatSearchSessionResultV2 {
    AtomicCombatSearchSessionResultV2 {
        status,
        report,
        progress_steps: output.progress_steps,
        atomic_combat_search_attempts: output.atomic_combat_search_attempts,
        accepted_high_loss_diagnostics: output.accepted_high_loss_diagnostics,
    }
}
