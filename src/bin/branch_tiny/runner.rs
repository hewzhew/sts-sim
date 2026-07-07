use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};

use super::combat_search_orchestrator;
use super::combat_search_portfolio_result::CombatSearchPortfolioResult;
use super::combat_search_report::CombatSearchPortfolioReport;
use super::owner_orchestrator::{orchestrate_owner_boundary, OwnerOrchestration};
use super::run_deadline::RunDeadline;
use super::{Args, BranchStatus, Owner};

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) combat_portfolio: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

enum PortfolioTransition {
    ContinueAuto,
    Stop {
        status: BranchStatus,
        combat_portfolio: Option<CombatSearchPortfolioReport>,
    },
    OwnerBoundary {
        status: BranchStatus,
        owner: Owner,
    },
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
                let transition = absorb_portfolio_result(
                    portfolio,
                    args,
                    deadline,
                    &mut auto_ops_used,
                    &mut auto_steps,
                    &mut combat_search,
                );
                let (status, owner) = match transition {
                    PortfolioTransition::ContinueAuto => continue,
                    PortfolioTransition::Stop {
                        status,
                        combat_portfolio,
                    } => {
                        return advance_result(status, combat_portfolio, auto_steps, combat_search)
                    }
                    PortfolioTransition::OwnerBoundary { status, owner } => (status, owner),
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

fn absorb_portfolio_result(
    portfolio: CombatSearchPortfolioResult,
    args: Args,
    deadline: RunDeadline,
    auto_ops_used: &mut usize,
    auto_steps: &mut Vec<RunControlAutoAppliedStepV1>,
    combat_search: &mut Vec<CombatSearchTraceSummary>,
) -> PortfolioTransition {
    let next_auto_ops_used = auto_ops_used.saturating_add(portfolio.applied_operations);
    let continue_operation_budget_chunk = portfolio.should_continue_operation_budget_chunk(
        next_auto_ops_used,
        args.auto_ops,
        deadline.should_stop(),
    );
    *auto_ops_used = next_auto_ops_used;
    combat_search.extend(portfolio.combat_search);
    auto_steps.extend(portfolio.auto_steps);
    if continue_operation_budget_chunk {
        return PortfolioTransition::ContinueAuto;
    }
    let combat_portfolio = portfolio.report;
    let status = portfolio.status;
    if combat_portfolio.is_some() {
        return PortfolioTransition::Stop {
            status,
            combat_portfolio,
        };
    }
    let owner = match &status {
        BranchStatus::Running { owner, .. } => Some(*owner),
        _ => None,
    };
    match owner {
        Some(owner) => PortfolioTransition::OwnerBoundary { status, owner },
        None => PortfolioTransition::Stop {
            status,
            combat_portfolio: None,
        },
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
