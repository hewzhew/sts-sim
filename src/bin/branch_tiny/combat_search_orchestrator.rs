use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlAutoAppliedStepV1, RunControlAutoStopKind, RunControlCommandOutcome,
    RunControlSession, RunControlTraceAnnotationV1,
};

use super::boundary_router;
use super::combat_search_lanes::{
    CombatSearchLane, CombatSearchLaneCommitPolicy, CombatSearchRequest, CombatSearchStakes,
};
use super::combat_search_report::{
    combat_portfolio_attempt_report, combat_portfolio_report, CombatSearchLaneReportInput,
};
use super::{
    Args, BranchStatus, CombatSearchLaneReport, CombatSearchPortfolioReport, TerminalOutcome,
};

struct CombatSearchLaneAttempt {
    outcome: Option<RunControlCommandOutcome>,
    status: BranchStatus,
    label: &'static str,
    max_nodes: usize,
    wall_ms: u64,
    potion_policy: Option<sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy>,
    max_potions_used: Option<u32>,
    action_keys: Vec<String>,
    committed: bool,
    auto_stop_kind: Option<RunControlAutoStopKind>,
    applied_operations: usize,
}

#[derive(Clone)]
pub(super) enum CombatSearchOutcome {
    AcceptedLine,
    CombatGap,
    BudgetLimited,
    OperationBudgetExhausted,
    Terminal,
    Failed,
}

pub(super) struct CombatSearchPortfolioResult {
    pub(super) status: BranchStatus,
    pub(super) outcome: CombatSearchOutcome,
    pub(super) report: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) applied_operations: usize,
}

pub(super) fn combat_search_summaries(
    outcome: &RunControlCommandOutcome,
) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
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

fn run_lane_attempt(
    session: &mut RunControlSession,
    request: &CombatSearchRequest,
    lane: CombatSearchLane,
) -> Result<CombatSearchLaneAttempt, String> {
    let before_curses = master_deck_curse_count(session);
    let mut trial = session.clone();
    let options = lane.options(request, session);
    let max_nodes = options.search.max_nodes.unwrap_or_default();
    let wall_ms = options.search.wall_ms.unwrap_or_default();
    let potion_policy = options.search.potion_policy;
    let max_potions_used = options.search.max_potions_used;
    let outcome = match apply_owner_audit_auto_run(&mut trial, options) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(CombatSearchLaneAttempt {
                outcome: None,
                status: BranchStatus::AdvanceFailed(err),
                label: lane.label(),
                max_nodes,
                wall_ms,
                potion_policy,
                max_potions_used,
                action_keys: Vec::new(),
                committed: false,
                auto_stop_kind: None,
                applied_operations: 0,
            });
        }
    };
    let mut status = lane_status(&trial, &outcome);
    if lane.rejects_new_curses()
        && !matches!(status, BranchStatus::CombatGap { .. })
        && master_deck_curse_count(&trial) > before_curses
    {
        let gained_curses = master_deck_curse_count(&trial).saturating_sub(before_curses);
        status = BranchStatus::CombatGap {
            boundary: "Combat".to_string(),
            reason: format!(
                "{} rejected dirty win: gained {gained_curses} curse card(s)",
                lane.label()
            ),
        };
    }
    let auto_stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
    let applied_operations = outcome
        .auto_stop
        .as_ref()
        .map(|stop| stop.applied_operations)
        .unwrap_or(0);
    let action_keys = complete_search_action_keys(&outcome);
    let committed = lane_commits(lane.commit_policy(), &status, auto_stop_kind);
    if committed {
        *session = trial;
    }
    Ok(CombatSearchLaneAttempt {
        outcome: Some(outcome),
        status,
        label: lane.label(),
        max_nodes,
        wall_ms,
        potion_policy,
        max_potions_used,
        action_keys,
        committed,
        auto_stop_kind,
        applied_operations,
    })
}

fn lane_commits(
    policy: CombatSearchLaneCommitPolicy,
    status: &BranchStatus,
    stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    lane_accepted(status)
        || matches!(
            policy,
            CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk
        ) && primary_operation_budget_exhausted(status, stop_kind)
}

fn lane_accepted(status: &BranchStatus) -> bool {
    !matches!(
        status,
        BranchStatus::CombatGap { .. }
            | BranchStatus::OperationBudgetExhausted { .. }
            | BranchStatus::BudgetGap { .. }
            | BranchStatus::ApplyFailed(_)
            | BranchStatus::AdvanceFailed(_)
            | BranchStatus::Terminal(TerminalOutcome::Defeat)
    )
}

fn lane_status(session: &RunControlSession, outcome: &RunControlCommandOutcome) -> BranchStatus {
    if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else {
        boundary_router::classify_auto_outcome(session, outcome)
    }
}

fn master_deck_curse_count(session: &RunControlSession) -> usize {
    session
        .run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .count()
}

fn lane_attempt_report(attempt: &CombatSearchLaneAttempt) -> CombatSearchLaneReport {
    combat_portfolio_attempt_report(CombatSearchLaneReportInput {
        label: attempt.label,
        status: attempt.status.clone(),
        max_nodes: attempt.max_nodes,
        wall_ms: attempt.wall_ms,
        potion_policy: attempt.potion_policy,
        max_potions_used: attempt.max_potions_used,
        action_keys: attempt.action_keys.clone(),
    })
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

fn primary_operation_budget_exhausted(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    primary_stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
        || matches!(status, BranchStatus::OperationBudgetExhausted { .. })
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}

fn complete_search_action_keys(outcome: &RunControlCommandOutcome) -> Vec<String> {
    outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source, actions, ..
            } if matches!(
                source,
                CombatAutomationTrajectorySource::SearchCombat
                    | CombatAutomationTrajectorySource::CompleteLineSolver
                    | CombatAutomationTrajectorySource::LineLabTurnPoolRescue
            ) =>
            {
                Some(
                    actions
                        .iter()
                        .map(|action| action.action_key.clone())
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        })
        .unwrap_or_default()
}
