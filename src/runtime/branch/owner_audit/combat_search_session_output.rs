use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunProgressStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;

#[derive(Default)]
pub(super) struct CombatSearchSessionOutput {
    pub(super) progress_steps: Vec<RunProgressStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_output_starts_without_high_loss_diagnostics() {
        assert!(CombatSearchSessionOutput::default()
            .accepted_high_loss_diagnostics
            .is_empty());
    }
}
