use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlAutoStopKind,
};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::combat_search_lane_commit::primary_operation_budget_exhausted;
use super::combat_search_portfolio_output::CombatSearchPortfolioOutput;
use super::combat_search_report::CombatSearchPortfolioReport;
use super::BranchStatus;

#[derive(Clone)]
enum CombatSearchOutcome {
    AcceptedLine,
    CombatGap,
    BudgetLimited,
    OperationBudgetExhausted,
    Terminal,
    Failed,
}

pub(super) struct CombatSearchPortfolioResult {
    pub(super) status: BranchStatus,
    outcome: CombatSearchOutcome,
    pub(super) report: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
    pub(super) applied_operations: usize,
}

impl CombatSearchPortfolioResult {
    pub(super) fn should_continue_operation_budget_chunk(
        &self,
        auto_ops_used: usize,
        auto_ops_limit: usize,
        deadline_reached: bool,
    ) -> bool {
        matches!(self.outcome, CombatSearchOutcome::OperationBudgetExhausted)
            && auto_ops_used < auto_ops_limit
            && !deadline_reached
    }
}

pub(super) fn combat_search_result(
    status: BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
    report: Option<CombatSearchPortfolioReport>,
    output: CombatSearchPortfolioOutput,
) -> CombatSearchPortfolioResult {
    let outcome = combat_search_outcome(&status, primary_stop_kind);
    CombatSearchPortfolioResult {
        status,
        outcome,
        report,
        auto_steps: output.auto_steps,
        combat_search: output.combat_search,
        accepted_high_loss_diagnostics: output.accepted_high_loss_diagnostics,
        applied_operations: output.applied_operations,
    }
}

fn combat_search_outcome(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> CombatSearchOutcome {
    match status {
        BranchStatus::Terminal(_) => CombatSearchOutcome::Terminal,
        _ if primary_operation_budget_exhausted(status, primary_stop_kind) => {
            CombatSearchOutcome::OperationBudgetExhausted
        }
        BranchStatus::CombatGap { .. } => CombatSearchOutcome::CombatGap,
        BranchStatus::BudgetGap { .. } => CombatSearchOutcome::BudgetLimited,
        BranchStatus::ApplyFailed(_) | BranchStatus::AdvanceFailed(_) => {
            CombatSearchOutcome::Failed
        }
        _ => CombatSearchOutcome::AcceptedLine,
    }
}
