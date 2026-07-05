use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlAutoStopKind,
    RunControlSession,
};

use super::combat_search_lane_runner::{
    combat_search_summaries, lane_attempt_report, primary_operation_budget_exhausted,
    run_lane_attempt, CombatSearchLaneAttempt,
};
use super::combat_search_lanes::{CombatSearchLane, CombatSearchRequest, CombatSearchStakes};
use super::combat_search_report::{
    combat_portfolio_report, CombatSearchLaneReport, CombatSearchPortfolioReport,
};
use super::{Args, BranchStatus};

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

pub(super) fn run_combat_portfolio_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchPortfolioResult, String> {
    let request = CombatSearchRequest::from_session(session, args);
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    let mut attempts = Vec::new();
    let mut applied_operations = 0usize;

    let primary = run_lane_attempt(session, &request, CombatSearchLane::primary())
        .map_err(|err| format!("primary failed: {err}"))?;
    collect_lane_output(
        &primary,
        &mut auto_steps,
        &mut combat_search,
        &mut applied_operations,
    );
    let mut status = primary.status.clone();
    let primary_stop_kind = primary.auto_stop_kind;
    if primary_operation_budget_exhausted(&status, primary_stop_kind)
        && !matches!(status, BranchStatus::Terminal(_))
    {
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            None,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }
    let saw_primary_combat_gap = matches!(status, BranchStatus::CombatGap { .. });
    if request.should_report() && saw_primary_combat_gap {
        attempts.push(lane_attempt_report(&primary));
    }
    if !saw_primary_combat_gap {
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            None,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }

    if request.combat_budget_capped() {
        status = combat_budget_capped_status(&request);
        let report = portfolio_report(&request, status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }
    if request.stakes == CombatSearchStakes::Boss && args.checkpoint_before_combat_portfolio {
        status = awaiting_auto_boundary(
            "Combat",
            "checkpoint before combat portfolio after primary search gap".to_string(),
        );
        let report = portfolio_report(&request, status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }

    for lane in request.portfolio_after_primary(session) {
        let attempt = run_lane_attempt(session, &request, lane)
            .map_err(|err| format!("{} failed: {err}", lane.label()))?;
        collect_lane_output(
            &attempt,
            &mut auto_steps,
            &mut combat_search,
            &mut applied_operations,
        );
        if request.should_report() {
            attempts.push(lane_attempt_report(&attempt));
        }
        status = attempt.status;
        if !matches!(status, BranchStatus::CombatGap { .. }) {
            break;
        }
    }

    let report = portfolio_report(&request, status.clone(), attempts);
    Ok(combat_search_result(
        status,
        primary_stop_kind,
        report,
        auto_steps,
        combat_search,
        applied_operations,
    ))
}

fn collect_lane_output(
    attempt: &CombatSearchLaneAttempt,
    auto_steps: &mut Vec<RunControlAutoAppliedStepV1>,
    combat_search: &mut Vec<CombatSearchTraceSummary>,
    applied_operations: &mut usize,
) {
    let Some(outcome) = attempt.outcome.as_ref() else {
        return;
    };
    *applied_operations = applied_operations.saturating_add(attempt.applied_operations);
    combat_search.extend(combat_search_summaries(outcome));
    if attempt.committed {
        auto_steps.extend(outcome.auto_applied_steps.clone());
    }
}

fn combat_budget_capped_status(request: &CombatSearchRequest) -> BranchStatus {
    if request.stakes == CombatSearchStakes::Boss {
        awaiting_auto_boundary(
            "Combat",
            format!(
                "outer wall budget would cap combat portfolio; effective search={}ms rescue={}ms boss={}ms",
                request.args.search_ms, request.args.rescue_search_ms, request.args.boss_search_ms
            ),
        )
    } else {
        BranchStatus::BudgetGap {
            boundary: "Combat".to_string(),
            reason: format!(
                "outer wall budget capped combat search; effective search={}ms rescue={}ms boss={}ms",
                request.args.search_ms, request.args.rescue_search_ms, request.args.boss_search_ms
            ),
        }
    }
}

fn portfolio_report(
    request: &CombatSearchRequest,
    status: BranchStatus,
    attempts: Vec<CombatSearchLaneReport>,
) -> Option<CombatSearchPortfolioReport> {
    request
        .should_report()
        .then(|| combat_portfolio_report(request.args, status, attempts))
}

fn combat_search_result(
    status: BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
    report: Option<CombatSearchPortfolioReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
    applied_operations: usize,
) -> CombatSearchPortfolioResult {
    let outcome = combat_search_outcome(&status, primary_stop_kind);
    CombatSearchPortfolioResult {
        status,
        outcome,
        report,
        auto_steps,
        combat_search,
        applied_operations,
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

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}
