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
    pub(super) internal_no_win_rescue_enabled: bool,
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
    let profile_config = options.search.profile.map(|profile| profile.to_config());
    let max_nodes = options
        .search
        .max_nodes
        .or_else(|| profile_config.as_ref().map(|config| config.max_nodes))
        .unwrap_or_default();
    let wall_ms = options
        .search
        .wall_ms
        .or_else(|| {
            profile_config
                .as_ref()
                .and_then(|config| config.wall_time.map(|duration| duration.as_millis() as u64))
        })
        .unwrap_or_default();
    let potion_policy = options
        .search
        .potion_policy
        .or_else(|| profile_config.as_ref().map(|config| config.potion_policy));
    let max_potions_used = options.search.max_potions_used.or_else(|| {
        profile_config
            .as_ref()
            .and_then(|config| config.max_potions_used)
    });
    let internal_no_win_rescue_enabled = !options.search.disable_no_win_rescue;
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
                internal_no_win_rescue_enabled,
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
        internal_no_win_rescue_enabled,
        committed,
        auto_stop_kind,
        applied_operations,
    })
}

pub(super) fn combat_search_summaries(
    attempt: &CombatSearchLaneAttempt,
) -> Vec<CombatSearchTraceSummary> {
    let Some(outcome) = attempt.outcome.as_ref() else {
        return Vec::new();
    };
    let mut summaries =
        sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
            .collect::<Vec<_>>();
    for summary in &mut summaries {
        summary.lane = Some(attempt.label.to_string());
        summary.profile_id = Some(attempt.label.to_string());
        summary.profile_max_nodes = Some(attempt.max_nodes);
        summary.profile_wall_ms = Some(attempt.wall_ms);
        summary.profile_potion_policy =
            Some(potion_policy_label(attempt.potion_policy).to_string());
        summary.profile_max_potions_used = attempt.max_potions_used;
        summary.profile_internal_no_win_rescue_enabled =
            Some(attempt.internal_no_win_rescue_enabled);
    }
    summaries
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

fn potion_policy_label(
    policy: Option<sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy>,
) -> &'static str {
    match policy {
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never) => "never",
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::All) => "all",
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted) => {
            "semantic"
        }
        None => "default",
    }
}
