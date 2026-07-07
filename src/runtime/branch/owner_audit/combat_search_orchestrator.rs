use sts_simulator::eval::run_control::RunControlSession;

use super::combat_search_lane_commit::primary_operation_budget_exhausted;
use super::combat_search_lane_runner::{lane_attempt_report, run_lane_attempt};
use super::combat_search_lanes::{CombatSearchLane, CombatSearchRequest, CombatSearchStakes};
use super::combat_search_portfolio_output::CombatSearchPortfolioOutput;
use super::combat_search_portfolio_result::{combat_search_result, CombatSearchPortfolioResult};
use super::combat_search_report::{combat_portfolio_report, CombatSearchLaneReport};
use super::{Args, BranchStatus};

pub(super) fn run_combat_portfolio_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchPortfolioResult, String> {
    let request = CombatSearchRequest::from_session(session, args);
    let mut output = CombatSearchPortfolioOutput::default();
    let mut attempts = Vec::new();

    let primary = run_lane_attempt(session, &request, CombatSearchLane::primary())
        .map_err(|err| format!("primary failed: {err}"))?;
    output.collect_attempt(&primary);
    let mut status = primary.status.clone();
    let primary_stop_kind = primary.auto_stop_kind;
    if primary_operation_budget_exhausted(&status, primary_stop_kind)
        && !matches!(status, BranchStatus::Terminal(_))
    {
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            None,
            output,
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
            output,
        ));
    }

    let post_primary_lanes = request.portfolio_after_primary();
    if post_primary_lanes.is_empty() {
        let report = portfolio_report(&request, status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            output,
        ));
    }

    if request.combat_budget_capped() {
        status = combat_budget_capped_status(&request);
        let report = portfolio_report(&request, status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            output,
        ));
    }
    if request.stakes() == CombatSearchStakes::Boss && args.checkpoint_before_combat_portfolio {
        status = awaiting_auto_boundary(
            "Combat",
            "checkpoint before combat portfolio after primary search gap".to_string(),
        );
        let report = portfolio_report(&request, status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            output,
        ));
    }

    for lane in post_primary_lanes {
        let attempt = run_lane_attempt(session, &request, lane)
            .map_err(|err| format!("{} failed: {err}", lane.label()))?;
        output.collect_attempt(&attempt);
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
        output,
    ))
}

fn combat_budget_capped_status(request: &CombatSearchRequest) -> BranchStatus {
    if request.stakes() == CombatSearchStakes::Boss {
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
) -> Option<super::combat_search_report::CombatSearchPortfolioReport> {
    request
        .should_report()
        .then(|| combat_portfolio_report(request.args, status, attempts))
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}
