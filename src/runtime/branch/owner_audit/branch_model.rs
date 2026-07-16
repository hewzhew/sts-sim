use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, PlannerBoundaryCaptureSegmentV1, RunControlSession,
    RunProgressJournalV1,
};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::branch_path::BranchPathStep;
use super::branch_policy_lane::BranchPolicyLane;
use super::branch_trajectory::BranchTrajectoryState;
use super::combat_search_report::CombatSearchPortfolioReport;
pub(super) use sts_simulator::runtime::branch::{
    BoundarySite, BranchStatus, Owner, TerminalOutcome,
};

#[derive(Clone)]
pub(super) struct Branch {
    pub(super) id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) path: Vec<BranchPathStep>,
    pub(super) session: RunControlSession,
    pub(super) status: BranchStatus,
    pub(super) policy_lane: BranchPolicyLane,
    pub(super) combat_portfolio: Option<CombatSearchPortfolioReport>,
    pub(super) recent_progress_journal: RunProgressJournalV1,
    pub(super) recent_planner_capture: PlannerBoundaryCaptureSegmentV1,
    pub(super) trajectory: BranchTrajectoryState,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) combat_search_history: Vec<CombatSearchTraceSummary>,
    pub(super) comparison_search_start: Option<usize>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}

impl Branch {
    pub(super) fn bind_trajectory_run(
        &mut self,
        run_id: &str,
        generation: usize,
    ) -> Result<(), String> {
        self.trajectory.bind_run(
            run_id,
            self.id,
            self.policy_lane.trajectory_lane(),
            generation,
            &self.status,
            &self.recent_progress_journal,
            &self.recent_planner_capture,
        )
    }

    pub(super) fn capture_recent_trajectory(&mut self, generation: usize) -> Result<(), String> {
        self.trajectory.append_recent(
            self.id,
            self.policy_lane.trajectory_lane(),
            generation,
            &self.status,
            &self.recent_progress_journal,
            &self.recent_planner_capture,
        )
    }
}
