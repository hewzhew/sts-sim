use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatSearchTraceSummary, RunControlAutoAppliedStepV1,
    RunControlAutoStopKind, RunControlSession,
};

use super::boundary_router;
use super::combat_search_orchestrator;
use super::owner_orchestrator::{orchestrate_owner_boundary, OwnerOrchestration};
use super::run_deadline::RunDeadline;
use super::{Args, BossRetryReport, BranchStatus};

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) boss_retry: Option<BossRetryReport>,
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
        match apply_owner_audit_auto_run(
            session,
            combat_search_orchestrator::primary_auto_step_options(run_args),
        ) {
            Ok(outcome) => {
                let stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
                auto_ops_used = auto_ops_used.saturating_add(
                    outcome
                        .auto_stop
                        .as_ref()
                        .map(|stop| stop.applied_operations)
                        .unwrap_or(0),
                );
                combat_search.extend(combat_search_orchestrator::combat_search_summaries(
                    &outcome,
                ));
                auto_steps.extend(outcome.auto_applied_steps.clone());
                let mut status = boundary_router::classify_auto_outcome(session, &outcome);
                if stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
                    && auto_ops_used < args.auto_ops
                    && !deadline.should_stop()
                {
                    continue;
                }
                let combat_gap = matches!(status, BranchStatus::CombatGap { .. });
                let boss_combat = combat_search_orchestrator::is_boss_combat(session);
                let combat_budget_capped = if boss_combat {
                    args.wall_capped_boss_budget
                } else {
                    args.wall_capped_search_budget
                };
                if combat_gap && combat_budget_capped {
                    if boss_combat {
                        return advance_result(
                            awaiting_auto_boundary(
                                "Combat",
                                format!(
                                    "outer wall budget would cap boss retry; effective search={}ms rescue={}ms boss={}ms",
                                    args.search_ms, args.rescue_search_ms, args.boss_search_ms
                                ),
                            ),
                            None,
                            auto_steps,
                            combat_search,
                        );
                    }
                    return advance_result(
                        BranchStatus::BudgetGap {
                            boundary: "Combat".to_string(),
                            reason: format!(
                                "outer wall budget capped combat search; effective search={}ms rescue={}ms boss={}ms",
                                args.search_ms, args.rescue_search_ms, args.boss_search_ms
                            ),
                        },
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                if combat_gap && boss_combat && args.checkpoint_before_boss_retry {
                    return advance_result(
                        awaiting_auto_boundary(
                            "Combat",
                            "checkpoint before boss retry after primary search gap".to_string(),
                        ),
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                if combat_gap {
                    match combat_search_orchestrator::run_after_primary_gap(
                        session,
                        deadline.cap_args(args, 1),
                    ) {
                        Ok(portfolio) => {
                            combat_search.extend(portfolio.combat_search);
                            auto_steps.extend(portfolio.auto_steps);
                            if portfolio.boss_retry.is_some() {
                                return advance_result(
                                    portfolio.status,
                                    portfolio.boss_retry,
                                    auto_steps,
                                    combat_search,
                                );
                            }
                            status = portfolio.status;
                        }
                        Err(err) => {
                            return advance_result(
                                BranchStatus::AdvanceFailed(err),
                                None,
                                auto_steps,
                                combat_search,
                            );
                        }
                    }
                }
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

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}

fn advance_result(
    status: BranchStatus,
    boss_retry: Option<BossRetryReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
) -> AdvanceResult {
    AdvanceResult {
        status,
        boss_retry,
        auto_steps,
        combat_search,
    }
}
