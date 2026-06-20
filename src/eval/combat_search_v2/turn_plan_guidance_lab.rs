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
    pub baseline_search: CombatSearchGuidanceLabChildSearchV1,
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
    pub cases_guided_prefix_better_than_baseline: usize,
    pub cases_guided_prefix_tied_with_baseline: usize,
    pub cases_guided_prefix_worse_than_baseline: usize,
    pub cases_without_guided_prefix_baseline_comparison: usize,
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
    pub tactical: CombatTurnPlanTacticalTraceV1,
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
    pub current_first_vs_best_target: Option<CombatTurnPlanGuidanceSelectedComparisonV1>,
    pub baseline_vs_best_guided_prefix: Option<CombatTurnPlanGuidanceBaselineComparisonV1>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatTurnPlanTacticalTraceV1 {
    pub action_count: usize,
    pub cards_played: usize,
    pub potions_used: usize,
    pub end_turns: usize,
    pub powers_played: usize,
    pub attacks_played: usize,
    pub skills_played: usize,
    pub zero_cost_cards_played: usize,
    pub damage_done: i32,
    pub block_gained_proxy: i32,
    pub visible_attack_mitigation_hint: i32,
    pub enemy_debuff_pressure_hint: i32,
    pub player_hp_delta: i32,
    pub player_hp_lost: i32,
    pub energy_delta: i32,
    pub energy_spent_proxy: i32,
    pub hand_delta: i32,
    pub draw_delta: i32,
    pub discard_delta: i32,
    pub exhaust_delta: i32,
    pub limbo_delta: i32,
    pub queued_cards_delta: i32,
    pub enemy_block_delta: i32,
    pub player_strength_gain: i32,
    pub player_temporary_strength_gain: i32,
    pub reactive_player_hp_loss: i32,
    pub reactive_player_block: i32,
    pub reactive_enemy_damage: i32,
    pub reactive_bad_draw_cards: i32,
    pub forced_turn_end_actions: usize,
    pub pending_choice_steps: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceSelectedComparisonV1 {
    pub same_plan: bool,
    pub current_first: CombatTurnPlanGuidancePlanSnapshotV1,
    pub best_by_child_target: CombatTurnPlanGuidancePlanSnapshotV1,
    pub delta_best_minus_current_first: CombatTurnPlanGuidanceOutcomeDeltaV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidancePlanSnapshotV1 {
    pub plan_index: usize,
    pub first_action_key: Option<String>,
    pub target_source: &'static str,
    pub terminal: SearchTerminalLabel,
    pub complete_win: bool,
    pub final_hp: Option<i32>,
    pub hp_loss: Option<i32>,
    pub turns: Option<u32>,
    pub potions_used: Option<u32>,
    pub cards_played: Option<u32>,
    pub action_count: Option<usize>,
    pub nodes_expanded: Option<u64>,
    pub tactical: CombatTurnPlanTacticalTraceV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceOutcomeDeltaV1 {
    pub final_hp_delta: Option<i32>,
    pub hp_loss_delta: Option<i32>,
    pub turn_delta: Option<i32>,
    pub potions_used_delta: Option<i32>,
    pub cards_played_delta: Option<i32>,
    pub action_count_delta: Option<i32>,
    pub nodes_expanded_delta: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceBaselineComparisonV1 {
    pub verdict: &'static str,
    pub baseline: CombatTurnPlanGuidanceSearchSnapshotV1,
    pub best_guided_prefix: CombatTurnPlanGuidancePlanSnapshotV1,
    pub delta_guided_minus_baseline: CombatTurnPlanGuidanceOutcomeDeltaV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanGuidanceSearchSnapshotV1 {
    pub source: &'static str,
    pub terminal: SearchTerminalLabel,
    pub complete_win: bool,
    pub final_hp: Option<i32>,
    pub hp_loss: Option<i32>,
    pub turns: Option<u32>,
    pub potions_used: Option<u32>,
    pub cards_played: Option<u32>,
    pub action_count: Option<usize>,
    pub first_action_key: Option<String>,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub terminal_wins: u64,
    pub elapsed_ms: u128,
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
    let baseline_search = child_search_report(&run_combat_search_v2(
        &loaded.position.engine,
        &loaded.position.combat,
        root_config.clone(),
    ));
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
                tactical: tactical_trace_for_plan_report(&candidate.report),
                plan: candidate.report.clone(),
                end_fingerprints: fingerprint_report_for_position(&candidate.position),
                child_search,
                target,
            }
        })
        .collect::<Vec<_>>();
    let summary = summarize_candidates(&candidates, &baseline_search);

    CombatTurnPlanGuidanceLabV1Report {
        schema_name: "CombatTurnPlanGuidanceLabV1Report",
        schema_version: 4,
        label_role: "oracle_turn_plan_guidance_lab_not_human_policy",
        policy_quality_claim: false,
        input_label: loaded.label.clone(),
        root_fingerprints: loaded
            .fingerprints
            .as_ref()
            .map(CombatSearchV2InputFingerprintReport::from)
            .unwrap_or_else(|| fingerprint_report_for_position(&loaded.position)),
        baseline_search,
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
    baseline_search: &CombatSearchGuidanceLabChildSearchV1,
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
    summary.current_first_vs_best_target = selected_vs_best_target_report(candidates);
    summary.baseline_vs_best_guided_prefix =
        baseline_vs_best_guided_prefix_report(baseline_search, candidates);

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

fn tactical_trace_for_plan_report(
    plan: &CombatSearchV2TurnPlanProbeCandidateReport,
) -> CombatTurnPlanTacticalTraceV1 {
    let mut trace = CombatTurnPlanTacticalTraceV1 {
        action_count: plan.actions.len(),
        ..CombatTurnPlanTacticalTraceV1::default()
    };
    for step in &plan.steps {
        match &step.action.input {
            crate::state::core::ClientInput::PlayCard { .. } => trace.cards_played += 1,
            crate::state::core::ClientInput::UsePotion { .. } => trace.potions_used += 1,
            crate::state::core::ClientInput::EndTurn => trace.end_turns += 1,
            _ => {}
        }

        if let Some(card) = step.action_facts.card.as_ref() {
            match card.card_type {
                crate::content::cards::CardType::Attack => trace.attacks_played += 1,
                crate::content::cards::CardType::Skill => trace.skills_played += 1,
                crate::content::cards::CardType::Power => trace.powers_played += 1,
                crate::content::cards::CardType::Status
                | crate::content::cards::CardType::Curse => {}
            }
            if card.cost_for_turn == 0 {
                trace.zero_cost_cards_played += 1;
            }
        }

        let exact = &step.action_facts.exact_one_step_delta;
        trace.player_hp_delta += exact.player_hp_delta;
        trace.player_hp_lost += (-exact.player_hp_delta).max(0);
        trace.energy_delta += exact.energy_delta;
        trace.energy_spent_proxy += (-exact.energy_delta).max(0);
        trace.hand_delta += exact.hand_delta;
        trace.draw_delta += exact.draw_delta;
        trace.discard_delta += exact.discard_delta;
        trace.exhaust_delta += exact.exhaust_delta;
        trace.limbo_delta += exact.limbo_delta;
        trace.queued_cards_delta += exact.queued_cards_delta;
        trace.damage_done += (-exact.total_enemy_hp_delta).max(0);
        trace.enemy_block_delta += exact.total_enemy_block_delta;
        trace.block_gained_proxy += exact.player_block_delta.max(0);
        if exact.pending_choice_present {
            trace.pending_choice_steps += 1;
        }

        let mechanics = &step.action_facts.mechanics;
        trace.visible_attack_mitigation_hint += mechanics.visible_attack_mitigation_hint;
        trace.enemy_debuff_pressure_hint += mechanics.enemy_weak
            + mechanics.enemy_vulnerable
            + mechanics.persistent_enemy_strength_down
            + mechanics.temporary_enemy_strength_down;
        trace.player_strength_gain += mechanics.player_strength_gain;
        trace.player_temporary_strength_gain += mechanics.player_temporary_strength_gain;
        trace.reactive_player_hp_loss += mechanics.reactive_player_hp_loss;
        trace.reactive_player_block += mechanics.reactive_player_block;
        trace.reactive_enemy_damage += mechanics.reactive_enemy_damage;
        trace.reactive_bad_draw_cards += mechanics.reactive_bad_draw_cards;
        if mechanics.reactive_forced_turn_end {
            trace.forced_turn_end_actions += 1;
        }
    }
    trace
}

fn selected_vs_best_target_report(
    candidates: &[CombatTurnPlanGuidanceLabCandidateV1],
) -> Option<CombatTurnPlanGuidanceSelectedComparisonV1> {
    let current_first = candidates.first()?;
    let best = candidates.iter().max_by(|left, right| {
        compare_targets(&left.target, &right.target)
            .then_with(|| right.plan.plan_index.cmp(&left.plan.plan_index))
    })?;
    let current_first_snapshot = plan_snapshot(current_first);
    let best_snapshot = plan_snapshot(best);
    Some(CombatTurnPlanGuidanceSelectedComparisonV1 {
        same_plan: current_first.plan.plan_index == best.plan.plan_index,
        delta_best_minus_current_first: outcome_delta(&best_snapshot, &current_first_snapshot),
        current_first: current_first_snapshot,
        best_by_child_target: best_snapshot,
    })
}

fn baseline_vs_best_guided_prefix_report(
    baseline_search: &CombatSearchGuidanceLabChildSearchV1,
    candidates: &[CombatTurnPlanGuidanceLabCandidateV1],
) -> Option<CombatTurnPlanGuidanceBaselineComparisonV1> {
    let baseline = search_snapshot(baseline_search);
    let best = candidates.iter().max_by(|left, right| {
        compare_targets(&left.target, &right.target)
            .then_with(|| right.plan.plan_index.cmp(&left.plan.plan_index))
    })?;
    let best_guided_prefix = plan_snapshot(best);
    let delta = outcome_delta_plan_minus_search(&best_guided_prefix, &baseline);
    Some(CombatTurnPlanGuidanceBaselineComparisonV1 {
        verdict: guided_vs_baseline_verdict(&best_guided_prefix, &baseline),
        baseline,
        best_guided_prefix,
        delta_guided_minus_baseline: delta,
    })
}

fn search_snapshot(
    search: &CombatSearchGuidanceLabChildSearchV1,
) -> CombatTurnPlanGuidanceSearchSnapshotV1 {
    let best_complete = search.best_complete.as_ref();
    CombatTurnPlanGuidanceSearchSnapshotV1 {
        source: "baseline_whole_combat_search",
        terminal: best_complete
            .map(|trajectory| trajectory.terminal)
            .unwrap_or(SearchTerminalLabel::Unresolved),
        complete_win: best_complete
            .is_some_and(|trajectory| trajectory.terminal == SearchTerminalLabel::Win),
        final_hp: best_complete.map(|trajectory| trajectory.final_hp),
        hp_loss: best_complete.map(|trajectory| trajectory.hp_loss),
        turns: best_complete.map(|trajectory| trajectory.turns),
        potions_used: best_complete.map(|trajectory| trajectory.potions_used),
        cards_played: best_complete.map(|trajectory| trajectory.cards_played),
        action_count: best_complete.map(|trajectory| trajectory.action_count),
        first_action_key: None,
        nodes_expanded: search.nodes_expanded,
        nodes_generated: search.nodes_generated,
        terminal_wins: search.terminal_wins,
        elapsed_ms: search.elapsed_ms,
    }
}

fn plan_snapshot(
    candidate: &CombatTurnPlanGuidanceLabCandidateV1,
) -> CombatTurnPlanGuidancePlanSnapshotV1 {
    let best_complete = candidate
        .child_search
        .as_ref()
        .and_then(|child| child.best_complete.as_ref());
    let final_hp = candidate.target.final_hp;
    let root_hp = candidate
        .plan
        .steps
        .first()
        .map(|step| step.state_before.player_hp);
    CombatTurnPlanGuidancePlanSnapshotV1 {
        plan_index: candidate.plan.plan_index,
        first_action_key: candidate.plan.first_action_key.clone(),
        target_source: candidate.target.source,
        terminal: candidate.target.terminal,
        complete_win: candidate.target.complete_win,
        final_hp,
        hp_loss: root_total_hp_loss(root_hp, final_hp).or(candidate.target.child_search_hp_loss),
        turns: best_complete.map(|trajectory| trajectory.turns),
        potions_used: best_complete.map(|trajectory| trajectory.potions_used),
        cards_played: best_complete.map(|trajectory| trajectory.cards_played),
        action_count: best_complete.map(|trajectory| trajectory.action_count),
        nodes_expanded: candidate.target.nodes_expanded,
        tactical: candidate.tactical.clone(),
    }
}

fn root_total_hp_loss(root_hp: Option<i32>, final_hp: Option<i32>) -> Option<i32> {
    Some(root_hp? - final_hp?)
}

fn outcome_delta(
    best: &CombatTurnPlanGuidancePlanSnapshotV1,
    current_first: &CombatTurnPlanGuidancePlanSnapshotV1,
) -> CombatTurnPlanGuidanceOutcomeDeltaV1 {
    CombatTurnPlanGuidanceOutcomeDeltaV1 {
        final_hp_delta: option_i32_delta(best.final_hp, current_first.final_hp),
        hp_loss_delta: option_i32_delta(best.hp_loss, current_first.hp_loss),
        turn_delta: option_u32_i32_delta(best.turns, current_first.turns),
        potions_used_delta: option_u32_i32_delta(best.potions_used, current_first.potions_used),
        cards_played_delta: option_u32_i32_delta(best.cards_played, current_first.cards_played),
        action_count_delta: option_usize_i32_delta(best.action_count, current_first.action_count),
        nodes_expanded_delta: option_u64_i64_delta(
            best.nodes_expanded,
            current_first.nodes_expanded,
        ),
    }
}

fn outcome_delta_plan_minus_search(
    best: &CombatTurnPlanGuidancePlanSnapshotV1,
    baseline: &CombatTurnPlanGuidanceSearchSnapshotV1,
) -> CombatTurnPlanGuidanceOutcomeDeltaV1 {
    CombatTurnPlanGuidanceOutcomeDeltaV1 {
        final_hp_delta: option_i32_delta(best.final_hp, baseline.final_hp),
        hp_loss_delta: option_i32_delta(best.hp_loss, baseline.hp_loss),
        turn_delta: option_u32_i32_delta(best.turns, baseline.turns),
        potions_used_delta: option_u32_i32_delta(best.potions_used, baseline.potions_used),
        cards_played_delta: option_u32_i32_delta(best.cards_played, baseline.cards_played),
        action_count_delta: option_usize_i32_delta(best.action_count, baseline.action_count),
        nodes_expanded_delta: option_u64_i64_delta(
            best.nodes_expanded,
            Some(baseline.nodes_expanded),
        ),
    }
}

fn guided_vs_baseline_verdict(
    guided: &CombatTurnPlanGuidancePlanSnapshotV1,
    baseline: &CombatTurnPlanGuidanceSearchSnapshotV1,
) -> &'static str {
    match compare_plan_snapshot_to_search(guided, baseline) {
        Ordering::Greater => "guided_better",
        Ordering::Equal => "guided_tied",
        Ordering::Less => "guided_worse",
    }
}

fn compare_plan_snapshot_to_search(
    guided: &CombatTurnPlanGuidancePlanSnapshotV1,
    baseline: &CombatTurnPlanGuidanceSearchSnapshotV1,
) -> Ordering {
    guided
        .complete_win
        .cmp(&baseline.complete_win)
        .then_with(|| terminal_tier(guided.terminal).cmp(&terminal_tier(baseline.terminal)))
        .then_with(|| {
            guided
                .final_hp
                .unwrap_or(i32::MIN)
                .cmp(&baseline.final_hp.unwrap_or(i32::MIN))
        })
        .then_with(|| {
            baseline
                .hp_loss
                .unwrap_or(i32::MAX)
                .cmp(&guided.hp_loss.unwrap_or(i32::MAX))
        })
        .then_with(|| {
            baseline
                .potions_used
                .unwrap_or(u32::MAX)
                .cmp(&guided.potions_used.unwrap_or(u32::MAX))
        })
        .then_with(|| {
            baseline
                .turns
                .unwrap_or(u32::MAX)
                .cmp(&guided.turns.unwrap_or(u32::MAX))
        })
        .then_with(|| {
            baseline
                .cards_played
                .unwrap_or(u32::MAX)
                .cmp(&guided.cards_played.unwrap_or(u32::MAX))
        })
        .then_with(|| {
            baseline
                .nodes_expanded
                .cmp(&guided.nodes_expanded.unwrap_or(u64::MAX))
        })
}

fn terminal_tier(terminal: SearchTerminalLabel) -> u8 {
    match terminal {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

fn option_i32_delta(left: Option<i32>, right: Option<i32>) -> Option<i32> {
    Some(left? - right?)
}

fn option_u32_i32_delta(left: Option<u32>, right: Option<u32>) -> Option<i32> {
    Some(left? as i32 - right? as i32)
}

fn option_usize_i32_delta(left: Option<usize>, right: Option<usize>) -> Option<i32> {
    Some(left? as i32 - right? as i32)
}

fn option_u64_i64_delta(left: Option<u64>, right: Option<u64>) -> Option<i64> {
    Some(left? as i64 - right? as i64)
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
        record_guided_prefix_verdict_count(&mut summary, lab);
    }
    summary
}

fn record_guided_prefix_verdict_count(
    summary: &mut CombatTurnPlanGuidanceLabBenchmarkSummaryV1,
    lab: &CombatTurnPlanGuidanceLabSummaryV1,
) {
    match lab
        .baseline_vs_best_guided_prefix
        .as_ref()
        .map(|comparison| comparison.verdict)
    {
        Some("guided_better") => summary.cases_guided_prefix_better_than_baseline += 1,
        Some("guided_tied") => summary.cases_guided_prefix_tied_with_baseline += 1,
        Some("guided_worse") => summary.cases_guided_prefix_worse_than_baseline += 1,
        Some(_) | None => summary.cases_without_guided_prefix_baseline_comparison += 1,
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::combat_search_v2::{
        CombatSearchV2ActionCardFacts, CombatSearchV2ActionExactDeltaFacts,
        CombatSearchV2ActionFacts, CombatSearchV2ActionImmediateFacts,
        CombatSearchV2ActionMechanicsFacts, CombatSearchV2ActionTargetFacts,
        CombatSearchV2ActionTrace, CombatSearchV2EnemySummary, CombatSearchV2StateSummary,
        CombatSearchV2TurnPlanProbeCandidateReport, CombatSearchV2TurnPlanProbeStepReport,
        SearchTerminalLabel,
    };
    use crate::content::cards::{CardTarget, CardType};
    use crate::state::core::ClientInput;

    use super::*;

    #[test]
    fn tactical_trace_summarizes_mechanical_turn_plan_deltas() {
        let plan = probe_plan(vec![
            probe_step(
                0,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
                Some(card_facts("Feel No Pain", "FeelNoPain", CardType::Power, 1)),
                exact_delta(0, 0, -1, -1, 0, 1, 0, -12, 0),
                mechanics_delta(0, 0, 0, 0, 0),
            ),
            probe_step(
                1,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
                Some(card_facts("True Grit", "TrueGrit", CardType::Skill, 1)),
                exact_delta(0, 9, -1, -1, 0, 0, 1, 0, 0),
                mechanics_delta(0, 0, 0, 0, 0),
            ),
            probe_step(
                2,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: Some(1),
                },
                None,
                exact_delta(-5, 0, 0, 0, 0, 0, 0, -20, 0),
                mechanics_delta(0, 0, 0, 0, 0),
            ),
        ]);

        let trace = tactical_trace_for_plan_report(&plan);

        assert_eq!(trace.action_count, 3);
        assert_eq!(trace.cards_played, 2);
        assert_eq!(trace.potions_used, 1);
        assert_eq!(trace.powers_played, 1);
        assert_eq!(trace.damage_done, 32);
        assert_eq!(trace.block_gained_proxy, 9);
        assert_eq!(trace.energy_spent_proxy, 2);
        assert_eq!(trace.exhaust_delta, 1);
        assert_eq!(trace.player_hp_lost, 5);
    }

    #[test]
    fn selected_vs_best_target_reports_current_ordering_gap() {
        let first = lab_candidate(
            0,
            "first",
            tactical_plan_with_damage(0, "Strike", 6),
            target_with_complete_win(30, 10, 7, 0, 12, 10),
        );
        let best = lab_candidate(
            1,
            "best",
            tactical_plan_with_damage(1, "Bash", 9),
            target_with_complete_win(40, 0, 5, 0, 9, 8),
        );
        let candidates = vec![first, best];

        let comparison =
            selected_vs_best_target_report(&candidates).expect("comparison should exist");

        assert!(!comparison.same_plan);
        assert_eq!(comparison.current_first.plan_index, 0);
        assert_eq!(comparison.best_by_child_target.plan_index, 1);
        assert_eq!(
            comparison.delta_best_minus_current_first.final_hp_delta,
            Some(10)
        );
        assert_eq!(
            comparison.delta_best_minus_current_first.hp_loss_delta,
            Some(-10)
        );
        assert_eq!(
            comparison.delta_best_minus_current_first.turn_delta,
            Some(-2)
        );
        assert_eq!(
            comparison
                .delta_best_minus_current_first
                .nodes_expanded_delta,
            Some(-20)
        );
        assert_eq!(comparison.best_by_child_target.tactical.damage_done, 9);
    }

    #[test]
    fn baseline_vs_best_guided_prefix_reports_search_outcome_delta() {
        let baseline =
            child_search_with_best_complete(target_with_complete_win(35, 8, 5, 0, 12, 12));
        let weaker = lab_candidate(
            0,
            "weaker",
            tactical_plan_with_damage(0, "Strike", 6),
            target_with_complete_win(30, 13, 5, 0, 12, 12),
        );
        let guided = lab_candidate(
            1,
            "guided",
            tactical_plan_with_damage(1, "Defend", 0),
            target_with_complete_win(41, 2, 6, 0, 15, 16),
        );
        let candidates = vec![weaker, guided];

        let comparison = baseline_vs_best_guided_prefix_report(&baseline, &candidates)
            .expect("comparison should exist");

        assert_eq!(comparison.verdict, "guided_better");
        assert_eq!(comparison.baseline.final_hp, Some(35));
        assert_eq!(comparison.best_guided_prefix.plan_index, 1);
        assert_eq!(
            comparison.delta_guided_minus_baseline.final_hp_delta,
            Some(6)
        );
        assert_eq!(
            comparison.delta_guided_minus_baseline.hp_loss_delta,
            Some(1)
        );
        assert_eq!(comparison.delta_guided_minus_baseline.turn_delta, Some(1));
        assert_eq!(
            comparison.delta_guided_minus_baseline.action_count_delta,
            Some(4)
        );
    }

    #[test]
    fn benchmark_summary_counts_guided_prefix_verdicts() {
        let mut summary = CombatTurnPlanGuidanceLabBenchmarkSummaryV1::default();
        record_guided_prefix_verdict_count(
            &mut summary,
            &lab_summary_with_guided_verdict("guided_better"),
        );
        record_guided_prefix_verdict_count(
            &mut summary,
            &lab_summary_with_guided_verdict("guided_tied"),
        );
        record_guided_prefix_verdict_count(
            &mut summary,
            &lab_summary_with_guided_verdict("guided_worse"),
        );
        record_guided_prefix_verdict_count(
            &mut summary,
            &CombatTurnPlanGuidanceLabSummaryV1::default(),
        );

        assert_eq!(summary.cases_guided_prefix_better_than_baseline, 1);
        assert_eq!(summary.cases_guided_prefix_tied_with_baseline, 1);
        assert_eq!(summary.cases_guided_prefix_worse_than_baseline, 1);
        assert_eq!(summary.cases_without_guided_prefix_baseline_comparison, 1);
    }

    fn lab_candidate(
        plan_index: usize,
        _action_key: &str,
        plan: CombatSearchV2TurnPlanProbeCandidateReport,
        child_best: CombatSearchGuidanceLabTrajectoryV1,
    ) -> CombatTurnPlanGuidanceLabCandidateV1 {
        CombatTurnPlanGuidanceLabCandidateV1 {
            tactical: tactical_trace_for_plan_report(&plan),
            plan,
            end_fingerprints: fingerprint_report(plan_index),
            child_search: Some(CombatSearchGuidanceLabChildSearchV1 {
                outcome: crate::ai::combat_search_v2::CombatSearchV2OutcomeReport {
                    coverage_status: crate::ai::combat_search_v2::SearchCoverageStatus::Exhaustive,
                    coverage_reason: "test".to_string(),
                    complete_trajectory_found: true,
                    exhaustive: true,
                },
                best_complete: Some(child_best.clone()),
                best_frontier: Some(child_best.clone()),
                final_state: None,
                nodes_expanded: child_best.action_count as u64 * 10,
                nodes_generated: child_best.action_count as u64 * 20,
                terminal_wins: 1,
                elapsed_ms: 0,
            }),
            target: CombatSearchGuidanceLabTargetV1 {
                target_kind: "root_turn_plan_child_search_rank",
                source: "bounded_child_search_best_complete",
                terminal: SearchTerminalLabel::Win,
                complete_win: true,
                post_root_player_hp: 50,
                child_search_hp_loss: Some(child_best.hp_loss),
                final_hp: Some(child_best.final_hp),
                nodes_expanded: Some(child_best.action_count as u64 * 10),
                limitations: vec![],
            },
        }
    }

    fn child_search_with_best_complete(
        best_complete: CombatSearchGuidanceLabTrajectoryV1,
    ) -> CombatSearchGuidanceLabChildSearchV1 {
        CombatSearchGuidanceLabChildSearchV1 {
            outcome: crate::ai::combat_search_v2::CombatSearchV2OutcomeReport {
                coverage_status: crate::ai::combat_search_v2::SearchCoverageStatus::Exhaustive,
                coverage_reason: "test".to_string(),
                complete_trajectory_found: true,
                exhaustive: true,
            },
            best_complete: Some(best_complete.clone()),
            best_frontier: Some(best_complete.clone()),
            final_state: None,
            nodes_expanded: best_complete.action_count as u64 * 10,
            nodes_generated: best_complete.action_count as u64 * 20,
            terminal_wins: 1,
            elapsed_ms: 0,
        }
    }

    fn lab_summary_with_guided_verdict(
        verdict: &'static str,
    ) -> CombatTurnPlanGuidanceLabSummaryV1 {
        CombatTurnPlanGuidanceLabSummaryV1 {
            baseline_vs_best_guided_prefix: Some(CombatTurnPlanGuidanceBaselineComparisonV1 {
                verdict,
                baseline: CombatTurnPlanGuidanceSearchSnapshotV1 {
                    source: "baseline_whole_combat_search",
                    terminal: SearchTerminalLabel::Win,
                    complete_win: true,
                    final_hp: Some(40),
                    hp_loss: Some(0),
                    turns: Some(1),
                    potions_used: Some(0),
                    cards_played: Some(1),
                    action_count: Some(1),
                    first_action_key: None,
                    nodes_expanded: 1,
                    nodes_generated: 1,
                    terminal_wins: 1,
                    elapsed_ms: 0,
                },
                best_guided_prefix: CombatTurnPlanGuidancePlanSnapshotV1 {
                    plan_index: 0,
                    first_action_key: None,
                    target_source: "bounded_child_search_best_complete",
                    terminal: SearchTerminalLabel::Win,
                    complete_win: true,
                    final_hp: Some(40),
                    hp_loss: Some(0),
                    turns: Some(1),
                    potions_used: Some(0),
                    cards_played: Some(1),
                    action_count: Some(1),
                    nodes_expanded: Some(1),
                    tactical: CombatTurnPlanTacticalTraceV1::default(),
                },
                delta_guided_minus_baseline: CombatTurnPlanGuidanceOutcomeDeltaV1 {
                    final_hp_delta: Some(0),
                    hp_loss_delta: Some(0),
                    turn_delta: Some(0),
                    potions_used_delta: Some(0),
                    cards_played_delta: Some(0),
                    action_count_delta: Some(0),
                    nodes_expanded_delta: Some(0),
                },
            }),
            ..CombatTurnPlanGuidanceLabSummaryV1::default()
        }
    }

    fn fingerprint_report(plan_index: usize) -> CombatSearchV2InputFingerprintReport {
        CombatSearchV2InputFingerprintReport {
            boundary: crate::eval::fingerprint::DecisionBoundaryFingerprintV1 {
                engine_state: "CombatPlayerTurn".to_string(),
                decision_kind: "combat".to_string(),
                terminal: crate::sim::combat::CombatTerminal::Unresolved,
                stable_boundary: true,
                turn_count: 1,
            },
            public_observation_hash: format!("public-{plan_index}"),
            legal_candidate_set_hash: format!("set-{plan_index}"),
            legal_candidate_order_hash: format!("order-{plan_index}"),
            exact_state_hash: format!("hash-{plan_index}"),
            stable_outcome_hash: Some(format!("stable-{plan_index}")),
            rng_boundary_status: crate::eval::fingerprint::RngFingerprintStatus::Complete,
            rng_boundary_stream_count: 0,
            rng_boundary_digest: "empty".to_string(),
        }
    }

    fn target_with_complete_win(
        final_hp: i32,
        hp_loss: i32,
        turns: u32,
        potions_used: u32,
        cards_played: u32,
        action_count: usize,
    ) -> CombatSearchGuidanceLabTrajectoryV1 {
        CombatSearchGuidanceLabTrajectoryV1 {
            terminal: SearchTerminalLabel::Win,
            estimated: false,
            final_hp,
            hp_loss,
            turns,
            potions_used,
            potions_discarded: 0,
            cards_played,
            action_count,
        }
    }

    fn tactical_plan_with_damage(
        plan_index: usize,
        card_name: &'static str,
        damage: i32,
    ) -> CombatSearchV2TurnPlanProbeCandidateReport {
        probe_plan_with_index(
            plan_index,
            vec![probe_step(
                0,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
                Some(card_facts(card_name, card_name, CardType::Attack, 1)),
                exact_delta(0, 0, -1, -1, 0, 1, 0, -damage, 0),
                mechanics_delta(0, 0, 0, 0, 0),
            )],
        )
    }

    fn probe_plan(
        steps: Vec<CombatSearchV2TurnPlanProbeStepReport>,
    ) -> CombatSearchV2TurnPlanProbeCandidateReport {
        probe_plan_with_index(0, steps)
    }

    fn probe_plan_with_index(
        plan_index: usize,
        steps: Vec<CombatSearchV2TurnPlanProbeStepReport>,
    ) -> CombatSearchV2TurnPlanProbeCandidateReport {
        CombatSearchV2TurnPlanProbeCandidateReport {
            plan_index,
            bucket: "balanced",
            stop_reason: "next_turn",
            outcome_class: "unresolved",
            survival_bucket: "safe",
            progress_bucket: "race_even",
            action_count: steps.len(),
            first_action_key: steps.first().map(|step| step.action.action_key.clone()),
            action_keys: steps
                .iter()
                .map(|step| step.action.action_key.clone())
                .collect(),
            actions: steps.iter().map(|step| step.action.clone()).collect(),
            action_facts: steps.iter().map(|step| step.action_facts.clone()).collect(),
            steps,
            eval_final_hp: 50,
            eval_risk_margin: 0,
            eval_enemy_progress: 0,
            end_state: state_summary(50, 0, 3, 100),
        }
    }

    fn probe_step(
        step_index: usize,
        input: ClientInput,
        card: Option<CombatSearchV2ActionCardFacts>,
        exact_one_step_delta: CombatSearchV2ActionExactDeltaFacts,
        mechanics: CombatSearchV2ActionMechanicsFacts,
    ) -> CombatSearchV2TurnPlanProbeStepReport {
        CombatSearchV2TurnPlanProbeStepReport {
            step_index,
            action: CombatSearchV2ActionTrace {
                step_index,
                action_id: step_index,
                action_key: format!("action-{step_index}"),
                action_debug: format!("action {step_index}"),
                input,
            },
            action_facts: CombatSearchV2ActionFacts {
                action_kind: "test",
                card,
                target: Some(CombatSearchV2ActionTargetFacts {
                    target_slot: 0,
                    entity_id: 1,
                    enemy_id: "Cultist".to_string(),
                    hp: 100,
                    block: 0,
                    visible_incoming_damage: 6,
                    vulnerable: 0,
                    weak: 0,
                    strength: 0,
                }),
                immediate: CombatSearchV2ActionImmediateFacts::default(),
                mechanics,
                exact_one_step_delta,
            },
            exact_state_hash_kind: "exact",
            state_before_exact_state_hash: format!("before-{step_index}"),
            state_after_exact_state_hash: format!("after-{step_index}"),
            state_before: state_summary(50, 0, 3, 100),
            state_after: state_summary(50, 0, 2, 100),
        }
    }

    fn card_facts(
        name: &'static str,
        card_id: &str,
        card_type: CardType,
        cost_for_turn: i32,
    ) -> CombatSearchV2ActionCardFacts {
        CombatSearchV2ActionCardFacts {
            hand_index: 0,
            uuid: 1,
            card_id: card_id.to_string(),
            name,
            upgraded: false,
            card_type,
            definition_target: CardTarget::Enemy,
            effective_target: CardTarget::Enemy,
            cost_for_turn,
            base_cost: cost_for_turn as i8,
            evaluated_damage: 0,
            evaluated_block: 0,
            evaluated_magic: 0,
            exhaust: false,
            ethereal: false,
            innate: false,
        }
    }

    fn exact_delta(
        player_hp_delta: i32,
        player_block_delta: i32,
        energy_delta: i32,
        hand_delta: i32,
        draw_delta: i32,
        discard_delta: i32,
        exhaust_delta: i32,
        total_enemy_hp_delta: i32,
        total_enemy_block_delta: i32,
    ) -> CombatSearchV2ActionExactDeltaFacts {
        CombatSearchV2ActionExactDeltaFacts {
            status: "ok",
            terminal: SearchTerminalLabel::Unresolved,
            engine_steps: 1,
            player_hp_delta,
            player_block_delta,
            energy_delta,
            hand_delta,
            draw_delta,
            discard_delta,
            exhaust_delta,
            limbo_delta: 0,
            queued_cards_delta: 0,
            total_enemy_hp_delta,
            total_enemy_block_delta,
            pending_choice_present: false,
            pending_choice_estimated_action_fanout: 0,
        }
    }

    fn mechanics_delta(
        visible_attack_mitigation_hint: i32,
        player_strength_gain: i32,
        player_temporary_strength_gain: i32,
        reactive_player_hp_loss: i32,
        reactive_bad_draw_cards: i32,
    ) -> CombatSearchV2ActionMechanicsFacts {
        CombatSearchV2ActionMechanicsFacts {
            visible_attack_mitigation_hint,
            player_strength_gain,
            player_temporary_strength_gain,
            reactive_player_hp_loss,
            reactive_bad_draw_cards,
            ..CombatSearchV2ActionMechanicsFacts::default()
        }
    }

    fn state_summary(
        player_hp: i32,
        player_block: i32,
        energy: u8,
        total_enemy_hp: i32,
    ) -> CombatSearchV2StateSummary {
        CombatSearchV2StateSummary {
            engine_state: "CombatPlayerTurn".to_string(),
            terminal: SearchTerminalLabel::Unresolved,
            player_hp,
            player_block,
            energy,
            turn_count: 1,
            living_enemy_count: 1,
            total_enemy_hp,
            visible_incoming_damage: 6,
            enemy_slots: vec![CombatSearchV2EnemySummary {
                slot: 0,
                entity_id: 1,
                enemy_id: "Cultist".to_string(),
                hp: total_enemy_hp,
                max_hp: 100,
                block: 0,
                alive: true,
                escaped: false,
                dying: false,
                half_dead: false,
                planned_move_id: 0,
                visible_intent: "attack".to_string(),
                visible_incoming_damage: 6,
            }],
            hand_count: 5,
            draw_count: 5,
            discard_count: 0,
            exhaust_count: 0,
            limbo_count: 0,
            queued_cards_count: 0,
        }
    }
}
