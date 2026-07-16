use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunProgressStepV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::combat_search_lane_runner::{combat_search_summaries, CombatSearchLaneAttempt};

#[derive(Default)]
pub(super) struct CombatSearchPortfolioOutput {
    pub(super) progress_steps: Vec<RunProgressStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

impl CombatSearchPortfolioOutput {
    pub(super) fn collect_attempt(
        &mut self,
        attempt: &CombatSearchLaneAttempt,
    ) -> Result<(), String> {
        self.combat_search.extend(combat_search_summaries(attempt));
        if attempt.selected {
            if let Some(diagnostic) = attempt.accepted_high_loss_diagnostic.as_ref() {
                self.accepted_high_loss_diagnostics.push(diagnostic.clone());
            }
            if let Some(outcome) = attempt.outcome.as_ref() {
                if outcome
                    .progress_steps
                    .iter()
                    .any(|step| matches!(step, RunProgressStepV1::Stop(_)))
                {
                    return Err(format!(
                        "selected combat lane {} returned a stop as committed progress",
                        attempt.label
                    ));
                }
                self.progress_steps.extend(outcome.progress_steps.clone());
            }
        }
        Ok(())
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
