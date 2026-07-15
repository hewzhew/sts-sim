use sts_simulator::eval::run_control::RunControlSession;

use super::combat_search_incumbent::CombatSearchIncumbent;
use super::combat_search_lane_commit::primary_operation_budget_exhausted;
use super::combat_search_lane_runner::{
    lane_attempt_report, run_lane_attempt, CombatSearchLaneAttempt,
};
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
    let mut lane_reports = Vec::new();

    let mut primary = run_lane_attempt(session, &request, CombatSearchLane::primary())
        .map_err(|err| format!("primary failed: {err}"))?;
    if primary.applicable {
        primary.commit_into(session)?;
    }
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
        lane_reports.push(lane_attempt_report(&primary));
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
        let report = portfolio_report(&request, status.clone(), lane_reports);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            output,
        ));
    }

    if request.combat_budget_capped() {
        status = combat_budget_capped_status(&request);
        let report = portfolio_report(&request, status.clone(), lane_reports);
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
        let report = portfolio_report(&request, status.clone(), lane_reports);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            output,
        ));
    }

    let arbitration = arbitrate_post_primary(session, status, post_primary_lanes, |root, lane| {
        run_lane_attempt(root, &request, lane)
    })?;
    status = arbitration.status;
    for attempt in arbitration.attempts {
        output.collect_attempt(&attempt);
        if request.should_report() {
            lane_reports.push(lane_attempt_report(&attempt));
        }
    }

    let report = portfolio_report(&request, status.clone(), lane_reports);
    Ok(combat_search_result(
        status,
        primary_stop_kind,
        report,
        output,
    ))
}

struct PostPrimaryArbitration {
    status: BranchStatus,
    attempts: Vec<CombatSearchLaneAttempt>,
}

fn arbitrate_post_primary<I, F>(
    session: &mut RunControlSession,
    fallback_status: BranchStatus,
    lanes: I,
    mut run_attempt: F,
) -> Result<PostPrimaryArbitration, String>
where
    I: IntoIterator<Item = CombatSearchLane>,
    F: FnMut(&RunControlSession, CombatSearchLane) -> Result<CombatSearchLaneAttempt, String>,
{
    let root = session.clone();
    let mut incumbent = CombatSearchIncumbent::new();
    let mut attempts: Vec<CombatSearchLaneAttempt> = Vec::new();

    for lane in lanes {
        let mut attempt =
            run_attempt(&root, lane).map_err(|err| format!("{} failed: {err}", lane.label()))?;
        if attempt.applicable {
            let candidate = attempt.candidate_facts.ok_or_else(|| {
                format!(
                    "lane {} produced an applicable trial without facts",
                    lane.label()
                )
            })?;
            let previous_index = incumbent.selected_index();
            let decision = incumbent.offer(attempts.len(), candidate);
            attempt.incumbent_reason = decision.reason;
            if decision.replaced {
                if let Some(previous_index) = previous_index {
                    attempts[previous_index].incumbent_reason = "replaced_by_better_candidate";
                }
            }
        }
        attempts.push(attempt);
    }

    let status = if let Some(selected_index) = incumbent.selected_index() {
        let selected_status = attempts[selected_index].status.clone();
        attempts[selected_index].commit_into(session)?;
        selected_status
    } else {
        attempts
            .last()
            .map(|attempt| attempt.status.clone())
            .unwrap_or(fallback_status)
    };

    Ok(PostPrimaryArbitration { status, attempts })
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

#[cfg(test)]
mod tests {
    use super::super::combat_search_incumbent::{
        CombatSearchCandidateFacts, CombatSearchCandidateTier,
    };
    use super::super::combat_search_lane_runner::CombatSearchLaneAttempt;
    use super::super::combat_search_lanes::CombatSearchLaneKind;
    use super::*;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    fn candidate(
        tier: CombatSearchCandidateTier,
        run_hp: i32,
        potions_used: u32,
    ) -> CombatSearchCandidateFacts {
        CombatSearchCandidateFacts {
            terminal_run_victory: false,
            tier,
            combat_final_hp: run_hp,
            run_hp,
            potions_used,
            potions_discarded: 0,
            turns: 5,
            action_count: 20,
        }
    }

    #[test]
    fn post_primary_attempts_share_root_and_commit_only_global_incumbent() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.current_hp = 70;
        let lanes = [
            CombatSearchLane::new(CombatSearchLaneKind::DiagnosticRescue),
            CombatSearchLane::new(CombatSearchLaneKind::PrimaryImmediateEscalation),
            CombatSearchLane::new(CombatSearchLaneKind::HallwayQualityPotionRescue),
            CombatSearchLane::new(CombatSearchLaneKind::HallwaySurvivalFallback),
        ];
        let candidates = [
            candidate(
                CombatSearchCandidateTier::ReserveCompliantCompleteWin,
                38,
                2,
            ),
            candidate(
                CombatSearchCandidateTier::ReserveCompliantCompleteWin,
                48,
                2,
            ),
            candidate(CombatSearchCandidateTier::RelaxedCompleteWin, 60, 0),
            candidate(
                CombatSearchCandidateTier::ReserveCompliantCompleteWin,
                60,
                3,
            ),
        ];
        let mut calls = 0usize;

        let result = arbitrate_post_primary(
            &mut session,
            BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: "primary gap".to_string(),
            },
            lanes,
            |root, lane| {
                assert_eq!(root.run_state.current_hp, 70);
                let index = calls;
                calls += 1;
                Ok(CombatSearchLaneAttempt::synthetic_for_test(
                    root,
                    lane.label(),
                    candidates[index],
                ))
            },
        )
        .expect("portfolio arbitration");

        assert_eq!(calls, 4);
        assert_eq!(result.attempts.len(), 4);
        assert_eq!(session.run_state.current_hp, 48);
        assert_eq!(
            result
                .attempts
                .iter()
                .filter(|attempt| attempt.selected)
                .count(),
            1
        );
        assert!(result.attempts[1].selected);
        assert_eq!(result.attempts[2].incumbent_reason, "lower_candidate_tier");
        assert_eq!(
            result.attempts[3].incumbent_reason,
            "incomparable_resource_trade"
        );
    }
}
