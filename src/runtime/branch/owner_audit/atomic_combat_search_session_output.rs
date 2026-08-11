use sts_simulator::eval::run_control::{AtomicCombatSearchTraceSummaryV2, RunProgressStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;

#[derive(Default)]
pub(super) struct AtomicCombatSearchSessionOutputV2 {
    pub(super) progress_steps: Vec<RunProgressStepV1>,
    pub(super) atomic_combat_search_attempts: Vec<AtomicCombatSearchTraceSummaryV2>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_output_starts_without_high_loss_diagnostics() {
        assert!(AtomicCombatSearchSessionOutputV2::default()
            .accepted_high_loss_diagnostics
            .is_empty());
    }
}
