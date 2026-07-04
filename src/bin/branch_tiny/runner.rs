use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};

use super::combat_search_orchestrator;
use super::combat_search_orchestrator::CombatSearchOutcome;
use super::owner_orchestrator::{orchestrate_owner_boundary, OwnerOrchestration};
use super::run_deadline::RunDeadline;
use super::{Args, BranchStatus, CombatSearchPortfolioReport};

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) combat_portfolio: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

pub(super) fn advance_to_owner_or_gap(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
) -> AdvanceResult {
    let mut policy_steps = 0usize;
    let mut auto_ops_used = 0usize;
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    loop {
        let run_args = deadline.cap_args(args, 1);
        match combat_search_orchestrator::run_combat_portfolio_step(session, run_args) {
            Ok(portfolio) => {
                auto_ops_used = auto_ops_used.saturating_add(portfolio.applied_operations);
                combat_search.extend(portfolio.combat_search);
                auto_steps.extend(portfolio.auto_steps);
                if matches!(
                    portfolio.outcome,
                    CombatSearchOutcome::OperationBudgetExhausted
                ) && auto_ops_used < args.auto_ops
                    && !deadline.should_stop()
                {
                    continue;
                }
                if portfolio.report.is_some() {
                    return advance_result(
                        portfolio.status,
                        portfolio.report,
                        auto_steps,
                        combat_search,
                    );
                }
                let status = portfolio.status;
                if let BranchStatus::Terminal(result) = status {
                    return advance_result(
                        BranchStatus::Terminal(result),
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                let owner = match &status {
                    BranchStatus::Running { owner, .. } => *owner,
                    _ => return advance_result(status, None, auto_steps, combat_search),
                };
                match orchestrate_owner_boundary(session, owner, &mut policy_steps) {
                    OwnerOrchestration::StopAtCandidates => {
                        return advance_result(status, None, auto_steps, combat_search);
                    }
                    OwnerOrchestration::Stop(status) => {
                        return advance_result(status, None, auto_steps, combat_search);
                    }
                    OwnerOrchestration::AppliedRoutine(step) => {
                        auto_steps.push(step);
                    }
                }
            }
            Err(err) => {
                return advance_result(
                    BranchStatus::AdvanceFailed(err),
                    None,
                    auto_steps,
                    combat_search,
                )
            }
        }
    }
}

fn advance_result(
    status: BranchStatus,
    combat_portfolio: Option<CombatSearchPortfolioReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
) -> AdvanceResult {
    AdvanceResult {
        status,
        combat_portfolio,
        auto_steps,
        combat_search,
    }
}
