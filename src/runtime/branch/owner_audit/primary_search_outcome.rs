use sts_simulator::eval::run_control::{
    CombatLineAdjudicationV1, CombatLineCleanlinessV1, CombatSearchTerminalLineSummary,
    CombatSearchTraceSummary,
};
use sts_simulator::runtime::branch::{
    PrimarySearchLineSummary, PrimarySearchOutcomeSummary, PrimarySearchProfileSummary,
    PrimarySearchTelemetrySummary,
};

use super::branch_model::Branch;
use super::combat_search_report::CombatSearchSessionReport;

pub(super) fn primary_search_outcome_from_branch(branch: &Branch) -> PrimarySearchOutcomeSummary {
    primary_search_outcome(&branch.combat_search, branch.combat_portfolio.as_ref())
}

pub(super) fn primary_search_outcome_value(
    attempts: &[CombatSearchTraceSummary],
    portfolio: Option<&CombatSearchSessionReport>,
) -> serde_json::Value {
    serde_json::to_value(primary_search_outcome(attempts, portfolio))
        .unwrap_or(serde_json::Value::Null)
}

fn primary_search_outcome(
    attempts: &[CombatSearchTraceSummary],
    portfolio: Option<&CombatSearchSessionReport>,
) -> PrimarySearchOutcomeSummary {
    let primary_attempt = primary_attempt(attempts);
    let profile_attempt = primary_profile_attempt(portfolio);
    let execution_adjudication =
        primary_attempt.and_then(|attempt| attempt.execution_adjudication.clone());
    let accepted_line = matches!(
        &execution_adjudication,
        Some(CombatLineAdjudicationV1::Accepted { .. })
    )
    .then(|| {
        primary_attempt
            .and_then(|attempt| attempt.best_win.as_ref())
            .map(primary_line_summary)
    })
    .flatten();
    PrimarySearchOutcomeSummary {
        status: primary_search_status(primary_attempt).to_string(),
        profile: PrimarySearchProfileSummary {
            profile_id: profile_attempt
                .map(|profile| profile.profile_id.to_string())
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
                .map(|_| false)
                .or_else(|| {
                    primary_attempt
                        .and_then(|attempt| attempt.profile_internal_no_win_rescue_enabled)
                })
                .unwrap_or(false),
        },
        telemetry: primary_search_telemetry(primary_attempt, profile_attempt),
        accepted_line,
        best_complete_line: primary_attempt
            .and_then(|attempt| attempt.best_complete.as_ref())
            .map(primary_line_summary),
        best_partial_line: None,
        execution_adjudication,
    }
}

pub(super) fn latest_execution_adjudication(
    attempts: &[CombatSearchTraceSummary],
) -> Option<CombatLineAdjudicationV1> {
    attempts
        .iter()
        .rev()
        .find_map(|attempt| attempt.execution_adjudication.clone())
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
    search_session: Option<&CombatSearchSessionReport>,
) -> Option<&CombatSearchSessionReport> {
    search_session
}

fn primary_search_status(attempt: Option<&CombatSearchTraceSummary>) -> &'static str {
    match attempt.and_then(|attempt| attempt.execution_adjudication.as_ref()) {
        Some(CombatLineAdjudicationV1::Accepted {
            cleanliness: CombatLineCleanlinessV1::Clean,
            ..
        }) => "accepted_win",
        Some(CombatLineAdjudicationV1::Accepted {
            cleanliness: CombatLineCleanlinessV1::Dirty,
            ..
        }) => "accepted_dirty_win",
        Some(CombatLineAdjudicationV1::Rejected { .. }) => "no_accepted_line",
        Some(CombatLineAdjudicationV1::ReplayFailed { .. }) => "search_internal_error",
        None if attempt
            .and_then(|attempt| attempt.best_win.as_ref())
            .is_some() =>
        {
            "legacy_unknown"
        }
        None => "no_accepted_line",
    }
}

fn primary_search_telemetry(
    attempt: Option<&CombatSearchTraceSummary>,
    profile: Option<&CombatSearchSessionReport>,
) -> PrimarySearchTelemetrySummary {
    let total_us = attempt.map(|attempt| attempt.total_us).unwrap_or(0);
    PrimarySearchTelemetrySummary {
        elapsed_ms: attempt.map(|attempt| attempt.total_us / 1_000),
        deadline_hit: attempt.map(|attempt| attempt.deadline_hit),
        expanded_nodes: attempt.map(|attempt| attempt.nodes_expanded),
        terminal_wins: attempt.map(|attempt| attempt.terminal_wins),
        us_per_node: attempt
            .and_then(|attempt| us_per_node(attempt.total_us, attempt.nodes_expanded)),
        first_win_node: attempt.and_then(|attempt| attempt.nodes_to_first_win),
        first_win_ms: None,
        first_accepted_node: None,
        first_accepted_ms: None,
        rollout_us: attempt.map(|attempt| attempt.rollout_us),
        expansion_us: attempt.map(|attempt| attempt.expansion_us),
        transition_us: attempt.map(|attempt| attempt.engine_step_us),
        rollout_pct: attempt.and_then(|attempt| percent_of_total(attempt.rollout_us, total_us)),
        expansion_pct: attempt.and_then(|attempt| percent_of_total(attempt.expansion_us, total_us)),
        transition_pct: attempt
            .and_then(|attempt| percent_of_total(attempt.engine_step_us, total_us)),
        diagnostic_pct: attempt.and_then(|attempt| {
            percent_of_total(
                attempt
                    .shadow_audit_us
                    .saturating_add(attempt.root_turn_plan_diag_us),
                total_us,
            )
        }),
        unattributed_pct: attempt
            .and_then(|attempt| percent_of_total(attempt.unattributed_us, total_us)),
        selected_first_action: profile.and_then(|profile| profile.action_keys.first().cloned()),
        top_root_actions: profile
            .map(|profile| profile.action_keys.clone())
            .unwrap_or_default(),
    }
}

fn us_per_node(total_us: u64, nodes_expanded: u64) -> Option<u64> {
    if nodes_expanded == 0 {
        None
    } else {
        Some(total_us / nodes_expanded)
    }
}

fn percent_of_total(part_us: u64, total_us: u64) -> Option<u64> {
    if total_us == 0 {
        None
    } else {
        Some(part_us.saturating_mul(100).saturating_add(total_us / 2) / total_us)
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, SearchTerminalLabel,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{
        CombatLineAdjudicationV1, CombatLineCleanlinessV1, CombatLineObservedOutcomeV1,
        RunActionCardSnapshotV1,
    };
    use sts_simulator::sim::combat::CombatTerminal;

    fn search_attempt_fixture() -> CombatSearchTraceSummary {
        let line = CombatSearchTerminalLineSummary {
            terminal: SearchTerminalLabel::Win,
            final_hp: 44,
            hp_loss: 0,
            turns: 7,
            cards_played: 25,
            potions_used: 0,
            potions_discarded: 0,
            action_count: 32,
        };
        CombatSearchTraceSummary {
            source: "search_combat".to_string(),
            lane: Some("primary".to_string()),
            profile_id: Some("primary".to_string()),
            combat_kind: "hallway".to_string(),
            complete_trajectory_found: true,
            complete_win_found: true,
            best_complete: Some(line.clone()),
            best_win: Some(line),
            ..CombatSearchTraceSummary::default()
        }
    }

    fn dirty_accepted_adjudication() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Accepted {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            cleanliness: CombatLineCleanlinessV1::Dirty,
            observed_outcome: CombatLineObservedOutcomeV1 {
                terminal: CombatTerminal::Win,
                final_hp: 44,
                hp_loss: 0,
                potions_used: 0,
                action_count: 32,
                gold_delta: 0,
                ritual_dagger_growth: 0,
                gained_curses: vec![RunActionCardSnapshotV1 {
                    id: CardId::Parasite,
                    uuid: 9001,
                    upgrades: 0,
                }],
            },
        }
    }

    #[test]
    fn primary_search_distinguishes_execution_acceptance_from_legacy_raw_win() {
        let mut accepted = search_attempt_fixture();
        accepted.execution_adjudication = Some(dirty_accepted_adjudication());
        let accepted_value = primary_search_outcome_value(&[accepted], None);
        assert_eq!(accepted_value["status"], "accepted_dirty_win");
        assert_eq!(
            accepted_value["execution_adjudication"]["observed_outcome"]["gained_curses"][0]["id"],
            "Parasite"
        );

        let mut legacy = search_attempt_fixture();
        legacy.execution_adjudication = None;
        let legacy_value = primary_search_outcome_value(&[legacy], None);
        assert_eq!(legacy_value["status"], "legacy_unknown");
        assert!(legacy_value["accepted_line"].is_null());
    }
}
