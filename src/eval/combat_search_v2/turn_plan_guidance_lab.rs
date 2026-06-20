use std::cmp::Ordering;

use serde::Serialize;

use crate::ai::combat_search_v2::{
    enumerate_combat_search_v2_turn_plan_probe_candidates, run_combat_search_v2,
    CombatSearchV2Report, CombatSearchV2TurnPlanProbeCandidateReport,
    CombatSearchV2TurnPlanProbeRootReport, SearchTerminalLabel,
};
use crate::eval::fingerprint::combat_state_fingerprint_v1;
use crate::sim::combat::CombatPosition;

use super::{
    CombatSearchGuidanceLabChildSearchV1, CombatSearchGuidanceLabTargetV1,
    CombatSearchGuidanceLabTrajectoryV1, CombatSearchV2BenchmarkInputKind,
    CombatSearchV2InputFingerprintReport, CombatSearchV2LoadedBenchmark, CombatSearchV2LoadedStart,
    CombatSearchV2RunOptions,
};

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceLabV1Report {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub label_role: &'static str,
    pub policy_quality_claim: bool,
    pub input_label: String,
    pub root_fingerprints: CombatSearchV2InputFingerprintReport,
    pub root: CombatSearchV2TurnPlanProbeRootReport,
    pub candidates: Vec<CombatTurnPlanGuidanceLabCandidateV1>,
    pub summary: CombatTurnPlanGuidanceLabSummaryV1,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceLabBenchmarkV1Report {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub label_role: &'static str,
    pub policy_quality_claim: bool,
    pub benchmark_name: String,
    pub requested_case_limit: Option<usize>,
    pub effective_case_limit: usize,
    pub summary: CombatTurnPlanGuidanceLabBenchmarkSummaryV1,
    pub cases: Vec<CombatTurnPlanGuidanceLabBenchmarkCaseV1>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatTurnPlanGuidanceLabBenchmarkSummaryV1 {
    pub cases_run: usize,
    pub cases_available: usize,
    pub candidate_count: usize,
    pub child_searches_run: usize,
    pub child_complete_wins: usize,
    pub cases_best_target_not_first_plan: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceLabBenchmarkCaseV1 {
    pub id: String,
    pub input_kind: CombatSearchV2BenchmarkInputKind,
    pub input_path: String,
    pub lab: CombatTurnPlanGuidanceLabV1Report,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceLabCandidateV1 {
    pub plan: CombatSearchV2TurnPlanProbeCandidateReport,
    pub end_fingerprints: CombatSearchV2InputFingerprintReport,
    pub child_search: Option<CombatSearchGuidanceLabChildSearchV1>,
    pub target: CombatSearchGuidanceLabTargetV1,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatTurnPlanGuidanceLabSummaryV1 {
    pub candidate_count: usize,
    pub child_searches_run: usize,
    pub child_complete_wins: usize,
    pub child_losses: usize,
    pub child_unresolved: usize,
    pub best_target_plan_index: Option<usize>,
    pub first_plan_rank_by_target: Option<usize>,
}

pub fn run_combat_turn_plan_guidance_lab_benchmark_v1(
    loaded: &CombatSearchV2LoadedBenchmark,
    root_options: CombatSearchV2RunOptions,
    child_options: CombatSearchV2RunOptions,
    max_cases: Option<usize>,
) -> CombatTurnPlanGuidanceLabBenchmarkV1Report {
    let limit = max_cases.unwrap_or(4);
    let cases = loaded
        .cases
        .iter()
        .take(limit)
        .map(|case| CombatTurnPlanGuidanceLabBenchmarkCaseV1 {
            id: case.id.clone(),
            input_kind: case.input.kind,
            input_path: case.input.path.display().to_string(),
            lab: run_combat_turn_plan_guidance_lab_v1(
                &case.start,
                root_options.clone(),
                child_options.clone(),
            ),
        })
        .collect::<Vec<_>>();
    let summary = summarize_benchmark(&cases, loaded.cases.len());
    CombatTurnPlanGuidanceLabBenchmarkV1Report {
        schema_name: "CombatTurnPlanGuidanceLabBenchmarkV1Report",
        schema_version: 1,
        label_role: "oracle_turn_plan_guidance_lab_not_human_policy",
        policy_quality_claim: false,
        benchmark_name: loaded.name.clone(),
        requested_case_limit: max_cases,
        effective_case_limit: limit,
        summary,
        cases,
        notes: vec![
            "offline turn-plan lab only; does not alter combat search ordering",
            "turn plans are exact same-turn candidates ending at a stable boundary",
            "targets come from bounded child search after each root turn plan",
        ],
    }
}

pub fn run_combat_turn_plan_guidance_lab_v1(
    loaded: &CombatSearchV2LoadedStart,
    root_options: CombatSearchV2RunOptions,
    child_options: CombatSearchV2RunOptions,
) -> CombatTurnPlanGuidanceLabV1Report {
    let root_config =
        root_options.to_search_config_for_position(loaded.label.clone(), &loaded.position);
    let enumeration = enumerate_combat_search_v2_turn_plan_probe_candidates(
        &loaded.position.engine,
        &loaded.position.combat,
        &root_config,
    );
    let candidates = enumeration
        .candidates
        .iter()
        .map(|candidate| {
            let child_search =
                if candidate.report.end_state.terminal == SearchTerminalLabel::Unresolved {
                    let child_config = child_options.to_search_config_for_position(
                        format!(
                            "{}:turn-plan-child:{}",
                            loaded.label, candidate.report.plan_index
                        ),
                        &candidate.position,
                    );
                    let report = run_combat_search_v2(
                        &candidate.position.engine,
                        &candidate.position.combat,
                        child_config,
                    );
                    Some(child_search_report(&report))
                } else {
                    None
                };
            let target = plan_target(&candidate.report, child_search.as_ref());
            CombatTurnPlanGuidanceLabCandidateV1 {
                plan: candidate.report.clone(),
                end_fingerprints: fingerprint_report_for_position(&candidate.position),
                child_search,
                target,
            }
        })
        .collect::<Vec<_>>();
    let summary = summarize_candidates(&candidates);

    CombatTurnPlanGuidanceLabV1Report {
        schema_name: "CombatTurnPlanGuidanceLabV1Report",
        schema_version: 2,
        label_role: "oracle_turn_plan_guidance_lab_not_human_policy",
        policy_quality_claim: false,
        input_label: loaded.label.clone(),
        root_fingerprints: loaded
            .fingerprints
            .as_ref()
            .map(CombatSearchV2InputFingerprintReport::from)
            .unwrap_or_else(|| fingerprint_report_for_position(&loaded.position)),
        root: enumeration.report,
        candidates,
        summary,
        notes: vec![
            "offline lab only; does not alter combat search ordering",
            "labels are oracle-under-current-simulator-budget, not human-optimal plans",
            "plan candidates are bounded by root turn-plan enumeration limits",
        ],
    }
}

fn fingerprint_report_for_position(
    position: &CombatPosition,
) -> CombatSearchV2InputFingerprintReport {
    CombatSearchV2InputFingerprintReport::from(&combat_state_fingerprint_v1(position))
}

fn child_search_report(report: &CombatSearchV2Report) -> CombatSearchGuidanceLabChildSearchV1 {
    CombatSearchGuidanceLabChildSearchV1 {
        outcome: report.outcome.clone(),
        best_complete: report
            .best_complete_trajectory
            .as_ref()
            .map(trajectory_summary),
        best_frontier: report
            .best_frontier_trajectory
            .as_ref()
            .map(trajectory_summary),
        final_state: report
            .best_complete_trajectory
            .as_ref()
            .map(|trajectory| trajectory.final_state.clone()),
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        terminal_wins: report.stats.terminal_wins,
        elapsed_ms: report.stats.elapsed_ms,
    }
}

fn trajectory_summary(
    trajectory: &crate::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> CombatSearchGuidanceLabTrajectoryV1 {
    CombatSearchGuidanceLabTrajectoryV1 {
        terminal: trajectory.terminal,
        estimated: trajectory.estimated,
        final_hp: trajectory.final_hp,
        hp_loss: trajectory.hp_loss,
        turns: trajectory.turns,
        potions_used: trajectory.potions_used,
        potions_discarded: trajectory.potions_discarded,
        cards_played: trajectory.cards_played,
        action_count: trajectory.actions.len(),
    }
}

fn plan_target(
    plan: &CombatSearchV2TurnPlanProbeCandidateReport,
    child_search: Option<&CombatSearchGuidanceLabChildSearchV1>,
) -> CombatSearchGuidanceLabTargetV1 {
    match plan.end_state.terminal {
        SearchTerminalLabel::Win => CombatSearchGuidanceLabTargetV1 {
            target_kind: "root_turn_plan_child_search_rank",
            source: "turn_plan_terminal",
            terminal: SearchTerminalLabel::Win,
            complete_win: true,
            post_root_player_hp: plan.end_state.player_hp,
            child_search_hp_loss: Some(0),
            final_hp: Some(plan.end_state.player_hp),
            nodes_expanded: Some(0),
            limitations: vec!["turn_plan_terminal_win_no_child_search_needed"],
        },
        SearchTerminalLabel::Loss => CombatSearchGuidanceLabTargetV1 {
            target_kind: "root_turn_plan_child_search_rank",
            source: "turn_plan_terminal",
            terminal: SearchTerminalLabel::Loss,
            complete_win: false,
            post_root_player_hp: plan.end_state.player_hp,
            child_search_hp_loss: None,
            final_hp: Some(plan.end_state.player_hp),
            nodes_expanded: Some(0),
            limitations: vec!["turn_plan_terminal_loss_no_child_search_run"],
        },
        SearchTerminalLabel::Unresolved => {
            if let Some(child) = child_search {
                if let Some(best) = child.best_complete.as_ref() {
                    CombatSearchGuidanceLabTargetV1 {
                        target_kind: "root_turn_plan_child_search_rank",
                        source: "bounded_child_search_best_complete",
                        terminal: best.terminal,
                        complete_win: best.terminal == SearchTerminalLabel::Win,
                        post_root_player_hp: plan.end_state.player_hp,
                        child_search_hp_loss: Some(best.hp_loss),
                        final_hp: Some(best.final_hp),
                        nodes_expanded: Some(child.nodes_expanded),
                        limitations: vec![
                            "bounded_child_search_not_exhaustive",
                            "target_terms_are_diagnostic_not_policy",
                        ],
                    }
                } else {
                    CombatSearchGuidanceLabTargetV1 {
                        target_kind: "root_turn_plan_child_search_rank",
                        source: "bounded_child_search_no_complete",
                        terminal: SearchTerminalLabel::Unresolved,
                        complete_win: false,
                        post_root_player_hp: plan.end_state.player_hp,
                        child_search_hp_loss: None,
                        final_hp: None,
                        nodes_expanded: Some(child.nodes_expanded),
                        limitations: vec![
                            "no_complete_child_candidate_under_budget",
                            "unresolved_does_not_prove_bad_plan",
                        ],
                    }
                }
            } else {
                CombatSearchGuidanceLabTargetV1 {
                    target_kind: "root_turn_plan_child_search_rank",
                    source: "turn_plan_unsearched",
                    terminal: SearchTerminalLabel::Unresolved,
                    complete_win: false,
                    post_root_player_hp: plan.end_state.player_hp,
                    child_search_hp_loss: None,
                    final_hp: None,
                    nodes_expanded: None,
                    limitations: vec!["unresolved_plan_without_child_search"],
                }
            }
        }
    }
}

fn summarize_candidates(
    candidates: &[CombatTurnPlanGuidanceLabCandidateV1],
) -> CombatTurnPlanGuidanceLabSummaryV1 {
    let mut summary = CombatTurnPlanGuidanceLabSummaryV1 {
        candidate_count: candidates.len(),
        ..CombatTurnPlanGuidanceLabSummaryV1::default()
    };
    let mut ranked = candidates.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        compare_targets(&right.target, &left.target)
            .then_with(|| left.plan.plan_index.cmp(&right.plan.plan_index))
    });
    summary.best_target_plan_index = ranked.first().map(|candidate| candidate.plan.plan_index);
    summary.first_plan_rank_by_target = ranked
        .iter()
        .position(|candidate| candidate.plan.plan_index == 0)
        .map(|index| index + 1);

    for candidate in candidates {
        if candidate.child_search.is_some() {
            summary.child_searches_run += 1;
        }
        match candidate.target.terminal {
            SearchTerminalLabel::Win if candidate.target.complete_win => {
                summary.child_complete_wins += 1;
            }
            SearchTerminalLabel::Loss => summary.child_losses += 1,
            SearchTerminalLabel::Unresolved | SearchTerminalLabel::Win => {
                summary.child_unresolved += 1;
            }
        }
    }
    summary
}

fn compare_targets(
    left: &CombatSearchGuidanceLabTargetV1,
    right: &CombatSearchGuidanceLabTargetV1,
) -> Ordering {
    target_terminal_tier(left)
        .cmp(&target_terminal_tier(right))
        .then_with(|| {
            left.final_hp
                .unwrap_or(i32::MIN)
                .cmp(&right.final_hp.unwrap_or(i32::MIN))
        })
        .then_with(|| {
            right
                .child_search_hp_loss
                .unwrap_or(i32::MAX)
                .cmp(&left.child_search_hp_loss.unwrap_or(i32::MAX))
        })
        .then_with(|| {
            right
                .nodes_expanded
                .unwrap_or(u64::MAX)
                .cmp(&left.nodes_expanded.unwrap_or(u64::MAX))
        })
}

fn target_terminal_tier(target: &CombatSearchGuidanceLabTargetV1) -> u8 {
    match (target.complete_win, target.terminal) {
        (true, SearchTerminalLabel::Win) => 3,
        (false, SearchTerminalLabel::Win) => 2,
        (_, SearchTerminalLabel::Unresolved) => 1,
        (_, SearchTerminalLabel::Loss) => 0,
    }
}

fn summarize_benchmark(
    cases: &[CombatTurnPlanGuidanceLabBenchmarkCaseV1],
    cases_available: usize,
) -> CombatTurnPlanGuidanceLabBenchmarkSummaryV1 {
    let mut summary = CombatTurnPlanGuidanceLabBenchmarkSummaryV1 {
        cases_run: cases.len(),
        cases_available,
        ..CombatTurnPlanGuidanceLabBenchmarkSummaryV1::default()
    };
    for case in cases {
        let lab = &case.lab.summary;
        summary.candidate_count += lab.candidate_count;
        summary.child_searches_run += lab.child_searches_run;
        summary.child_complete_wins += lab.child_complete_wins;
        if lab.best_target_plan_index.is_some_and(|index| index != 0) {
            summary.cases_best_target_not_first_plan += 1;
        }
    }
    summary
}
