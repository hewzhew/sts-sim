use sts_simulator::eval::run_control::{CombatSearchTerminalLineSummary, CombatSearchTraceSummary};
use sts_simulator::runtime::branch::{
    PrimarySearchLineSummary, PrimarySearchOutcomeSummary, PrimarySearchProfileSummary,
    PrimarySearchTelemetrySummary,
};

use super::branch_model::Branch;
use super::combat_search_report::{
    CombatSearchLaneReport, CombatSearchPortfolioReport, CombatSearchPortfolioStatus,
};

pub(super) fn primary_search_outcome_from_branch(branch: &Branch) -> PrimarySearchOutcomeSummary {
    primary_search_outcome(&branch.combat_search, branch.combat_portfolio.as_ref())
}

pub(super) fn primary_search_outcome_value(
    attempts: &[CombatSearchTraceSummary],
    portfolio: Option<&CombatSearchPortfolioReport>,
) -> serde_json::Value {
    serde_json::to_value(primary_search_outcome(attempts, portfolio))
        .unwrap_or(serde_json::Value::Null)
}

fn primary_search_outcome(
    attempts: &[CombatSearchTraceSummary],
    portfolio: Option<&CombatSearchPortfolioReport>,
) -> PrimarySearchOutcomeSummary {
    let primary_attempt = primary_attempt(attempts);
    let profile_attempt = primary_profile_attempt(portfolio);
    PrimarySearchOutcomeSummary {
        status: primary_search_status(primary_attempt, portfolio).to_string(),
        profile: PrimarySearchProfileSummary {
            profile_id: profile_attempt
                .map(|profile| profile.label.to_string())
                .or_else(|| primary_attempt.and_then(|attempt| attempt.profile_id.clone())),
            stakes: primary_attempt.map(|attempt| attempt.combat_kind.clone()),
            max_nodes: profile_attempt
                .map(|profile| profile.max_nodes)
                .or_else(|| primary_attempt.and_then(|attempt| attempt.profile_max_nodes)),
            wall_ms: profile_attempt
                .map(|profile| profile.wall_ms)
                .or_else(|| primary_attempt.and_then(|attempt| attempt.profile_wall_ms)),
            potion_policy: profile_attempt
                .map(|profile| profile.potion_policy.to_string())
                .or_else(|| {
                    primary_attempt.and_then(|attempt| attempt.profile_potion_policy.clone())
                }),
            max_potions_used: profile_attempt
                .and_then(|profile| profile.max_potions_used)
                .or_else(|| primary_attempt.and_then(|attempt| attempt.profile_max_potions_used)),
            internal_no_win_rescue_enabled: profile_attempt
                .map(|profile| profile.label == "diagnostic_rescue")
                .or_else(|| {
                    primary_attempt
                        .and_then(|attempt| attempt.profile_internal_no_win_rescue_enabled)
                })
                .unwrap_or(false),
        },
        telemetry: primary_search_telemetry(primary_attempt, profile_attempt),
        accepted_line: primary_attempt
            .and_then(|attempt| attempt.best_win.as_ref())
            .map(primary_line_summary),
        best_complete_line: primary_attempt
            .and_then(|attempt| attempt.best_complete.as_ref())
            .map(primary_line_summary),
        best_partial_line: None,
    }
}

fn primary_attempt(attempts: &[CombatSearchTraceSummary]) -> Option<&CombatSearchTraceSummary> {
    attempts
        .iter()
        .find(|attempt| attempt.lane.as_deref() == Some("primary"))
        .or_else(|| {
            attempts
                .iter()
                .find(|attempt| attempt.source == "search_combat")
        })
}

fn primary_profile_attempt(
    portfolio: Option<&CombatSearchPortfolioReport>,
) -> Option<&CombatSearchLaneReport> {
    portfolio.and_then(|report| {
        report
            .attempts
            .iter()
            .find(|attempt| attempt.label == "primary")
            .or_else(|| report.attempts.first())
    })
}

fn primary_search_status(
    attempt: Option<&CombatSearchTraceSummary>,
    portfolio: Option<&CombatSearchPortfolioReport>,
) -> &'static str {
    if attempt
        .and_then(|attempt| attempt.best_win.as_ref())
        .is_some()
    {
        return "accepted_win";
    }
    match portfolio.map(|report| &report.status) {
        Some(CombatSearchPortfolioStatus::Terminal(_)) => "accepted_win",
        Some(CombatSearchPortfolioStatus::Advanced(_)) => "accepted_win",
        Some(CombatSearchPortfolioStatus::Failed(_)) | None => "no_accepted_line",
    }
}

fn primary_search_telemetry(
    attempt: Option<&CombatSearchTraceSummary>,
    profile: Option<&CombatSearchLaneReport>,
) -> PrimarySearchTelemetrySummary {
    PrimarySearchTelemetrySummary {
        elapsed_ms: attempt.map(|attempt| attempt.total_us / 1_000),
        deadline_hit: attempt.map(|attempt| attempt.deadline_hit),
        expanded_nodes: attempt.map(|attempt| attempt.nodes_expanded),
        terminal_wins: attempt.map(|attempt| attempt.terminal_wins),
        first_win_node: attempt.and_then(|attempt| attempt.nodes_to_first_win),
        first_win_ms: None,
        first_accepted_node: None,
        first_accepted_ms: None,
        rollout_us: attempt.map(|attempt| attempt.rollout_us),
        expansion_us: attempt.map(|attempt| attempt.expansion_us),
        transition_us: attempt.map(|attempt| attempt.engine_step_us),
        selected_first_action: profile.and_then(|profile| profile.action_keys.first().cloned()),
        top_root_actions: profile
            .map(|profile| profile.action_keys.clone())
            .unwrap_or_default(),
    }
}

fn primary_line_summary(line: &CombatSearchTerminalLineSummary) -> PrimarySearchLineSummary {
    PrimarySearchLineSummary {
        terminal: format!("{:?}", line.terminal),
        line_len: line.action_count,
        final_player_hp: line.final_hp,
        hp_delta: -line.hp_loss,
        potions_used: line.potions_used,
        first_action_label: None,
        first_action_kind: None,
    }
}
