use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunControlAutoAppliedStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::combat_search_lane_runner::{combat_search_summaries, CombatSearchLaneAttempt};

#[derive(Default)]
pub(super) struct CombatSearchPortfolioOutput {
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
    pub(super) applied_operations: usize,
}

impl CombatSearchPortfolioOutput {
    pub(super) fn collect_attempt(&mut self, attempt: &CombatSearchLaneAttempt) {
        self.combat_search.extend(combat_search_summaries(attempt));
        if attempt.selected {
            if let Some(diagnostic) = attempt.accepted_high_loss_diagnostic.as_ref() {
                self.accepted_high_loss_diagnostics.push(diagnostic.clone());
            }
            if let Some(outcome) = attempt.outcome.as_ref() {
                self.auto_steps.extend(outcome.auto_applied_steps.clone());
                self.applied_operations = attempt.applied_operations;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_output_starts_without_high_loss_diagnostics() {
        assert!(CombatSearchPortfolioOutput::default()
            .accepted_high_loss_diagnostics
            .is_empty());
    }
}
