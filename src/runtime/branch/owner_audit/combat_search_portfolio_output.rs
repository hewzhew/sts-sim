use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunControlAutoAppliedStepV1};

use super::combat_search_lane_runner::{combat_search_summaries, CombatSearchLaneAttempt};

#[derive(Default)]
pub(super) struct CombatSearchPortfolioOutput {
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) applied_operations: usize,
}

impl CombatSearchPortfolioOutput {
    pub(super) fn collect_attempt(&mut self, attempt: &CombatSearchLaneAttempt) {
        let Some(outcome) = attempt.outcome.as_ref() else {
            return;
        };
        self.applied_operations = self
            .applied_operations
            .saturating_add(attempt.applied_operations);
        self.combat_search.extend(combat_search_summaries(attempt));
        if attempt.committed {
            self.auto_steps.extend(outcome.auto_applied_steps.clone());
        }
    }
}
