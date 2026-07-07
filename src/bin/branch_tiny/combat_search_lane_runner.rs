use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatSearchTraceSummary, RunControlAutoStopKind,
    RunControlCommandOutcome, RunControlSession,
};

use super::combat_search_dirty_win::reject_dirty_win_status;
use super::combat_search_lane_commit::lane_commits;
use super::combat_search_lanes::{CombatSearchLane, CombatSearchRequest};
use super::combat_search_report::{
    combat_portfolio_attempt_report, CombatSearchLaneReport, CombatSearchLaneReportInput,
};
use super::combat_search_trace_actions::complete_search_action_keys;
use super::{boundary_router, BranchStatus};

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
    let status = reject_dirty_win_status(
        lane.rejects_new_curses(),
        lane.label(),
        lane_status(&trial, &outcome),
        before_curses,
        master_deck_curse_count(&trial),
    );
    let auto_stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
    let applied_operations = outcome
        .auto_stop
        .as_ref()
        .map(|stop| stop.applied_operations)
        .unwrap_or(0);
    let action_keys = complete_search_action_keys(&outcome.trace_annotations);
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
