use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlAutoStopKind, RunControlCommandOutcome, RunControlSession,
    RunControlTraceAnnotationV1,
};

use super::combat_search_lanes::{
    CombatSearchLane, CombatSearchLaneCommitPolicy, CombatSearchRequest,
};
use super::combat_search_report::{
    combat_portfolio_attempt_report, CombatSearchLaneReport, CombatSearchLaneReportInput,
};
use super::{boundary_router, BranchStatus, TerminalOutcome};

pub(super) struct CombatSearchLaneAttempt {
    pub(super) outcome: Option<RunControlCommandOutcome>,
    pub(super) status: BranchStatus,
    pub(super) label: &'static str,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy:
        Option<sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy>,
    pub(super) max_potions_used: Option<u32>,
    pub(super) action_keys: Vec<String>,
    pub(super) committed: bool,
    pub(super) auto_stop_kind: Option<RunControlAutoStopKind>,
    pub(super) applied_operations: usize,
}

pub(super) fn run_lane_attempt(
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

pub(super) fn combat_search_summaries(
    outcome: &RunControlCommandOutcome,
) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
}

pub(super) fn lane_attempt_report(attempt: &CombatSearchLaneAttempt) -> CombatSearchLaneReport {
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

pub(super) fn primary_operation_budget_exhausted(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    primary_stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
        || matches!(status, BranchStatus::OperationBudgetExhausted { .. })
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
                    | CombatAutomationTrajectorySource::TurnPoolRescue
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
