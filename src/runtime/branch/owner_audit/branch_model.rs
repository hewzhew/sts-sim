use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, PlannerBoundaryCaptureSegmentV1, RunControlSession,
    RunProgressJournalV1,
};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::branch_path::BranchPathStep;
use super::branch_policy_lane::BranchPolicyLane;
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
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) combat_search_history: Vec<CombatSearchTraceSummary>,
    pub(super) comparison_search_start: Option<usize>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
}
