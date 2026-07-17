use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunProgressStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::combat_search_report::CombatSearchSessionReport;
use super::combat_search_session_output::CombatSearchSessionOutput;
use super::BranchStatus;

pub(super) struct CombatSearchSessionResult {
    pub(super) status: BranchStatus,
    pub(super) report: Option<CombatSearchSessionReport>,
    pub(super) progress_steps: Vec<RunProgressStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

pub(super) fn combat_search_result(
    status: BranchStatus,
    report: Option<CombatSearchSessionReport>,
    output: CombatSearchSessionOutput,
) -> CombatSearchSessionResult {
    CombatSearchSessionResult {
        status,
        report,
        progress_steps: output.progress_steps,
        combat_search: output.combat_search,
        accepted_high_loss_diagnostics: output.accepted_high_loss_diagnostics,
    }
}
