use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};

use super::branch_path::BranchPathStep;
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
    pub(super) combat_portfolio: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}
