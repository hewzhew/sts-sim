use crate::ai::combat_search_v2::{
    filter_combat_search_legal_actions, has_external_payoff_opportunity,
    plan_combat_turn_segment_v1, run_combat_search_v2, CombatSearchV2ActionTrace,
    CombatSearchV2Config, CombatSearchV2Report, CombatSearchV2TrajectoryReport,
    CombatSearchV2TurnSegmentReport, SearchTerminalLabel,
};
use crate::content::monsters::EnemyId;
use crate::content::potions::PotionId;
use crate::content::powers::{store, PowerId};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_legal_actions::engine_local_moves;
use crate::state::core::{EngineState, RunResult};

use super::combat_candidate_line::{replay_candidate_line, CombatCandidateLine};
use super::combat_line_outcome::{
    evaluate_combat_candidate_line_outcome, find_accepted_alternative_in_report,
    find_clean_no_potion_alternative, render_combat_line_outcome_detail,
    CombatLineAcceptancePolicy,
};
use super::commands::{
    RunControlCombatSegmentMode, RunControlHpLossLimit, RunControlSearchCombatOptions,
    RunControlSearchEvidenceTarget,
};
use super::registry::BenchmarkCasePaths;
use super::search_evidence::{save_combat_search_evidence_v1, CombatSearchEvidenceContextV1};
use super::session::{
    RunControlCombatSearchRejection, RunControlCommandOutcome, RunControlSession,
};
use super::trace_annotation::{
    CombatAutomationActionV1, CombatAutomationMonsterStateV1, CombatAutomationStepStateV1,
    CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
    CombatSearchPerformanceSnapshotV1, CombatSearchTerminalLineSummary,
    RunControlTraceAnnotationV1,
};
use super::transition_report::{
    action_result_from_transition, render_action_result, ActionResult, ActionResultChange,
    CardSnapshot, RunApplyStatus, RunVisibleSnapshot, TransitionAction,
};
use super::view_model::client_input_hint;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunControlCommandOutcome, String> {
    let options = high_stakes_search_options(session, options);
    let start = session.current_active_combat_position()?;
    let config = search_config(session, options.clone());
    let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
    let saved_evidence =
        save_search_evidence_if_requested(session, options.evidence.as_ref(), &report)?;
    if search_report_has_invalid_card_identity(&report) {
        let mut outcome = RunControlCommandOutcome::message(format!(
            "{}{}\n\n{}",
            render_search_rejection(&report, "invalid_card_identity", None),
            render_saved_evidence_note(saved_evidence.as_deref()),
            super::render::render_run_control_state(session)
        ))
        .with_combat_search_rejection(RunControlCombatSearchRejection::InvalidCardIdentity);
        outcome
            .trace_annotations
            .push(combat_search_performance_trace_annotation(
                "search_combat_rejected",
                session,
                &start,
                &report,
            ));
        outcome.search_evidence_path = saved_evidence;
        return Ok(outcome);
    }
    let Some(trajectory) = report.best_win_trajectory.as_ref() else {
        if let Some(solution) =
            super::combat_complete_line_solver::try_solve_complete_line(&start, &config)
        {
            if effective_hp_loss_limit(session, &options)
                .is_none_or(|limit| solution.line.hp_loss <= limit as i32)
            {
                let summary = format!(
                    "complete_line_solver actions={}/{} delta={} hp_loss={}/{} saved={} budget=base:{}/{}ms repair:{}x{}/{}ms stops={}/{} nodes={} generated={} base_nodes={}/{} repair_nodes={}/{} repair={}/{}/{} elapsed_ms={}",
                    solution.final_action_count,
                    solution.base_action_count,
                    solution.repair_action_count_delta,
                    solution.final_hp_loss,
                    solution.base_hp_loss,
                    solution.repair_hp_loss_saved,
                    solution.base_node_budget,
                    solution.base_ms_budget,
                    solution.repair_cut_budget,
                    solution.repair_node_budget_per_cut,
                    solution.repair_ms_budget_per_cut,
                    solution.base_stop_reason,
                    solution.last_repair_stop_reason.unwrap_or("none"),
                    solution.nodes_expanded,
                    solution.nodes_generated,
                    solution.base_nodes_expanded,
                    solution.base_nodes_generated,
                    solution.repair_nodes_expanded,
                    solution.repair_nodes_generated,
                    solution.repair_attempts,
                    solution.repair_wins,
                    solution.repair_improvements,
                    solution.elapsed_ms
                );
                return apply_selected_combat_candidate_line(
                    session,
                    &start,
                    &config,
                    &report,
                    saved_evidence.as_deref(),
                    solution.line,
                    CombatAutomationTrajectorySource::CompleteLineSolver,
                    summary,
                    Some(CombatCandidateLinePerformance {
                        nodes_expanded: solution.nodes_expanded as u64,
                        nodes_generated: solution.nodes_generated as u64,
                        total_us: millis_to_micros_u64(solution.elapsed_ms),
                    }),
                );
            }
        }
        if let Some(outcome) = try_apply_turn_segment_after_rejection(
            session,
            &start,
            &config,
            &options,
            &report,
            saved_evidence.as_deref(),
            "no_complete_winning_candidate",
        )? {
            return Ok(outcome);
        }
        if let Some(outcome) = try_apply_smoke_bomb_survival_fallback_after_rejection(
            session,
            saved_evidence.as_deref(),
            "no_complete_winning_candidate",
        )? {
            return Ok(outcome);
        }
        let mut outcome = RunControlCommandOutcome::message(format!(
            "{}{}\n\n{}",
            render_search_rejection(&report, "no_complete_winning_candidate", None),
            render_saved_evidence_note(saved_evidence.as_deref()),
            super::render::render_run_control_state(session)
        ))
        .with_combat_search_rejection(RunControlCombatSearchRejection::NoCompleteWinningCandidate);
        outcome
            .trace_annotations
            .push(combat_search_performance_trace_annotation(
                "search_combat_rejected",
                session,
                &start,
                &report,
            ));
        outcome.search_evidence_path = saved_evidence;
        return Ok(outcome);
    };
    let original_line = CombatCandidateLine::from_search_trajectory(trajectory);
    let mut selected_line = original_line.clone();
    let mut selected_report_owned: Option<CombatSearchV2Report> = None;
    let mut repair_summary: Option<String> = None;
    if let Some(repair) =
        super::combat_line_repair::try_repair_winning_trajectory(&start, trajectory, &config)
    {
        selected_line = repair.line;
        repair_summary = Some(format!(
            "line_repair attempts={} wins={} improvements={} elapsed_ms={} original_hp_loss={} repaired_hp_loss={}",
            repair.attempts,
            repair.wins,
            repair.improvements,
            repair.elapsed_ms,
            trajectory.hp_loss,
            selected_line.hp_loss
        ));
    }
    let policy = CombatLineAcceptancePolicy::default();
    let selected_eval =
        evaluate_combat_candidate_line_outcome(session, &start, &config, selected_line.clone())?;
    if policy.classify(&selected_eval.outcome).is_rejected() {
        if let Some(alternative) =
            find_accepted_alternative_in_report(session, &start, &config, &report, policy)?
        {
            selected_line = alternative.line;
            append_repair_summary(
                &mut repair_summary,
                format!(
                    "same_report_clean_alternative replaced dirty_win gained_curses={} original_final_hp={} clean_final_hp={}",
                    selected_eval.outcome.gained_curse_count(),
                    selected_eval.outcome.final_hp,
                    alternative.outcome.final_hp
                ),
            );
        } else if let Some(alternative) =
            find_clean_no_potion_alternative(session, &start, &config, policy)?
        {
            selected_line = alternative.line;
            selected_report_owned = Some(alternative.report);
            append_repair_summary(
                &mut repair_summary,
                format!(
                    "clean_no_potion_alternative replaced dirty_win gained_curses={} original_final_hp={} clean_final_hp={}",
                    selected_eval.outcome.gained_curse_count(),
                    selected_eval.outcome.final_hp,
                    alternative.outcome.final_hp
                ),
            );
        } else {
            let mut outcome = RunControlCommandOutcome::message(format!(
                "{}{}\n\n{}",
                render_search_rejection(
                    &report,
                    "dirty_winning_candidate_rejected",
                    Some(render_combat_line_outcome_detail(&selected_eval.outcome)),
                ),
                render_saved_evidence_note(saved_evidence.as_deref()),
                super::render::render_run_control_state(session)
            ))
            .with_combat_search_rejection(
                RunControlCombatSearchRejection::DirtyWinningCandidateRejected,
            );
            outcome
                .trace_annotations
                .push(combat_search_performance_trace_annotation(
                    "search_combat_rejected_dirty_win",
                    session,
                    &start,
                    &report,
                ));
            outcome.search_evidence_path = saved_evidence;
            return Ok(outcome);
        }
    }

    if let Some(max_hp_loss) = effective_hp_loss_limit(session, &options) {
        if selected_line.hp_loss > max_hp_loss as i32 {
            if let Some(outcome) = try_apply_turn_segment_after_rejection(
                session,
                &start,
                &config,
                &options,
                &report,
                saved_evidence.as_deref(),
                "complete_winning_candidate_exceeds_hp_loss_limit",
            )? {
                return Ok(outcome);
            }
            let mut outcome = RunControlCommandOutcome::message(format!(
                "{}{}\n\n{}",
                render_search_rejection(
                    &report,
                    "complete_winning_candidate_exceeds_hp_loss_limit",
                    Some(format!(
                        "candidate_hp_loss={} max_hp_loss={max_hp_loss}",
                        selected_line.hp_loss
                    )),
                ),
                render_saved_evidence_note(saved_evidence.as_deref()),
                super::render::render_run_control_state(session)
            ))
            .with_combat_search_rejection(RunControlCombatSearchRejection::HpLossLimitExceeded);
            outcome
                .trace_annotations
                .push(combat_search_performance_trace_annotation(
                    "search_combat_rejected",
                    session,
                    &start,
                    &report,
                ));
            outcome.search_evidence_path = saved_evidence;
            return Ok(outcome);
        }
    }

    let mut summary = format!(
        "search-combat applied {} actions",
        selected_line.actions.len()
    );
    if let Some(repair_summary) = repair_summary.as_ref() {
        summary.push_str(&format!(" {repair_summary}"));
    }
    if let Some(path) = saved_evidence.as_ref() {
        summary.push_str(&format!(" saved_search={}", path.display()));
    }
    apply_selected_combat_candidate_line(
        session,
        &start,
        &config,
        selected_report_owned.as_ref().unwrap_or(&report),
        saved_evidence.as_deref(),
        selected_line,
        CombatAutomationTrajectorySource::SearchCombat,
        summary,
        None,
    )
}

#[derive(Clone, Copy)]
struct CombatCandidateLinePerformance {
    nodes_expanded: u64,
    nodes_generated: u64,
    total_us: u64,
}

fn append_repair_summary(summary: &mut Option<String>, item: String) {
    match summary {
        Some(summary) => {
            summary.push(' ');
            summary.push_str(&item);
        }
        None => *summary = Some(item),
    }
}

fn apply_selected_combat_candidate_line(
    session: &mut RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
    saved_evidence: Option<&std::path::Path>,
    mut selected_line: CombatCandidateLine,
    trajectory_source: CombatAutomationTrajectorySource,
    transition_label: String,
    line_performance: Option<CombatCandidateLinePerformance>,
) -> Result<RunControlCommandOutcome, String> {
    let replay =
        replay_candidate_line(start, selected_line.source, &selected_line.actions, config)?;
    if replay.line.terminal != CombatTerminal::Win {
        return Err(format!(
            "selected combat candidate line did not replay to win; source={} terminal={:?}",
            replay.line.source.label(),
            replay.line.terminal
        ));
    }
    selected_line = replay.line;
    let before_snapshot = RunVisibleSnapshot::capture(session);
    let applied = selected_line.actions.clone();
    let mut automation_actions = Vec::new();
    session.mark_current_combat_search_resolved();
    for action in &applied {
        let outcome = session.apply_input(action.input.clone())?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: action.step_index,
            action_key: action.action_key.clone(),
            input: action.input.clone(),
            drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
            combat_after: combat_automation_step_state_v1(session),
        });
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let action_result = action_result_from_transition(
        TransitionAction {
            label: transition_label,
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let application = if line_performance.is_some() {
        render_complete_line_solver_application(
            report,
            &applied,
            &selected_line,
            replay.applied_count,
        )
    } else {
        render_search_application(report, &applied, &selected_line, replay.applied_count)
    };
    let message = format!(
        "{}{}\n{}\n{}",
        application,
        render_saved_evidence_note(saved_evidence),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    let automation_record =
        CombatAutomationTrajectoryRecordV1::new(trajectory_source, automation_actions);
    session.remember_combat_automation_trajectory(automation_record.clone());
    let mut outcome = RunControlCommandOutcome::action(message, action_result)
        .with_trace_annotations(vec![
            automation_record.into_annotation(),
            combat_line_performance_trace_annotation(
                trajectory_source.label(),
                session,
                start,
                report,
                &selected_line,
                line_performance,
            ),
        ]);
    outcome.search_evidence_path = saved_evidence.map(std::path::Path::to_path_buf);
    Ok(outcome)
}

fn try_apply_turn_segment_after_rejection(
    session: &mut RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    options: &RunControlSearchCombatOptions,
    search_report: &CombatSearchV2Report,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<Option<RunControlCommandOutcome>, String> {
    if !segment_mode_allows_turn_segment(options.segment_mode, start) {
        return Ok(None);
    }

    let segment_report = plan_combat_turn_segment_v1(&start.engine, &start.combat, config);
    let Some(trajectory) = segment_report.selected.as_ref() else {
        return Ok(None);
    };
    verify_segment_trajectory_replays(start, &trajectory.actions, config)?;

    let before_snapshot = RunVisibleSnapshot::capture(session);
    let applied = trajectory.actions.clone();
    let mut automation_actions = Vec::new();
    session.mark_current_combat_search_resolved();
    for action in &applied {
        let outcome = session.apply_input(action.input.clone())?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: action.step_index,
            action_key: action.action_key.clone(),
            input: action.input.clone(),
            drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
            combat_after: combat_automation_step_state_v1(session),
        });
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let mut transition_label = format!(
        "search-combat segment applied {} actions (partial turn; not terminal claim)",
        applied.len()
    );
    if let Some(path) = saved_evidence.as_ref() {
        transition_label.push_str(&format!(" saved_search={}", path.display()));
    }
    let action_result = action_result_from_transition(
        TransitionAction {
            label: transition_label,
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let message = format!(
        "{}{}\n{}\n{}",
        render_segment_application(search_report, &segment_report, rejection_result),
        render_saved_evidence_note(saved_evidence),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    let automation_record = CombatAutomationTrajectoryRecordV1::new(
        CombatAutomationTrajectorySource::SearchCombatTurnSegment,
        automation_actions,
    );
    session.remember_combat_automation_trajectory(automation_record.clone());
    let mut outcome = RunControlCommandOutcome::action(message, action_result)
        .with_trace_annotations(vec![
            automation_record.into_annotation(),
            combat_search_performance_trace_annotation(
                "search_combat_turn_segment",
                session,
                start,
                search_report,
            ),
        ]);
    outcome.search_evidence_path = saved_evidence.map(|path| path.to_path_buf());
    Ok(Some(outcome))
}

fn try_apply_smoke_bomb_survival_fallback_after_rejection(
    session: &mut RunControlSession,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<Option<RunControlCommandOutcome>, String> {
    let Some(active) = session.active_combat.as_ref() else {
        return Ok(None);
    };
    let smoke_input = engine_local_moves(&active.engine_state, &active.combat_state)
        .into_iter()
        .find(|input| match input {
            crate::state::core::ClientInput::UsePotion { potion_index, .. } => active
                .combat_state
                .entities
                .potions
                .get(*potion_index)
                .and_then(|potion| potion.as_ref())
                .is_some_and(|potion| potion.id == PotionId::SmokeBomb),
            _ => false,
        });
    let Some(smoke_input) = smoke_input else {
        return Ok(None);
    };

    let before_snapshot = RunVisibleSnapshot::capture(session);
    let mut automation_actions = Vec::new();
    let outcome = session.apply_input(smoke_input.clone())?;
    automation_actions.push(CombatAutomationActionV1 {
        step_index: 0,
        action_key: "combat/use_smoke_bomb_survival".to_string(),
        input: smoke_input,
        drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
        combat_after: combat_automation_step_state_v1(session),
    });
    if active_combat_is_waiting_for_smoke_escape_turn_end(session) {
        let end_turn_outcome = session.apply_input(crate::state::core::ClientInput::EndTurn)?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: 1,
            action_key: "combat/end_turn_after_smoke_bomb".to_string(),
            input: crate::state::core::ClientInput::EndTurn,
            drawn_cards: drawn_cards_from_action_result(end_turn_outcome.action_result.as_ref()),
            combat_after: combat_automation_step_state_v1(session),
        });
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let mut transition_label = format!(
        "Smoke Bomb survival fallback after {rejection_result} (not a combat victory claim)"
    );
    if let Some(path) = saved_evidence.as_ref() {
        transition_label.push_str(&format!(" saved_search={}", path.display()));
    }
    let action_result = action_result_from_transition(
        TransitionAction {
            label: transition_label,
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let message = format!(
        "Search combat did not find a complete win; used Smoke Bomb as a survival fallback after {rejection_result}.{}\n{}\n{}",
        render_saved_evidence_note(saved_evidence),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    let automation_record = CombatAutomationTrajectoryRecordV1::new(
        CombatAutomationTrajectorySource::SearchCombatSmokeBombSurvival,
        automation_actions,
    );
    session.remember_combat_automation_trajectory(automation_record.clone());
    let mut outcome = RunControlCommandOutcome::action(message, action_result)
        .with_trace_annotations(vec![automation_record.into_annotation()]);
    outcome.search_evidence_path = saved_evidence.map(|path| path.to_path_buf());
    Ok(Some(outcome))
}

fn active_combat_is_waiting_for_smoke_escape_turn_end(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|active| active.combat_state.turn.counters.player_escaping)
}

fn segment_mode_allows_turn_segment(
    mode: Option<RunControlCombatSegmentMode>,
    start: &CombatPosition,
) -> bool {
    match mode {
        Some(RunControlCombatSegmentMode::TurnBoundary) => true,
        Some(RunControlCombatSegmentMode::NonBossTurnBoundary) => !start.combat.meta.is_boss_fight,
        None => false,
    }
}

fn combat_automation_step_state_v1(
    session: &RunControlSession,
) -> Option<CombatAutomationStepStateV1> {
    let combat = &session.active_combat.as_ref()?.combat_state;
    Some(CombatAutomationStepStateV1 {
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        cards_played_this_turn: combat.turn.counters.cards_played_this_turn,
        early_end_turn_pending: combat.turn.counters.early_end_turn_pending,
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| CombatAutomationMonsterStateV1 {
                id: monster.id,
                label: EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| format!("monster#{}", monster.monster_type)),
                hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: monster.is_alive_for_action(),
                time_warp: store::power_amount(combat, monster.id, PowerId::TimeWarp),
                strength: store::power_amount(combat, monster.id, PowerId::Strength),
            })
            .collect(),
    })
}

fn combat_search_performance_trace_annotation(
    source: impl Into<String>,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
) -> RunControlTraceAnnotationV1 {
    RunControlTraceAnnotationV1::CombatSearchPerformance {
        snapshot: combat_search_performance_snapshot(source.into(), session, start, report),
    }
}

fn combat_line_performance_trace_annotation(
    source: impl Into<String>,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
    selected_line: &CombatCandidateLine,
    line_performance: Option<CombatCandidateLinePerformance>,
) -> RunControlTraceAnnotationV1 {
    let Some(performance) = line_performance else {
        return combat_search_performance_trace_annotation(source, session, start, report);
    };
    let mut snapshot = combat_search_performance_snapshot(source.into(), session, start, report);
    let line_summary = combat_candidate_line_summary(selected_line);
    snapshot.coverage_status = "CompleteLineSolverApplied".to_string();
    snapshot.complete_trajectory_found = true;
    snapshot.complete_win_found = selected_line.terminal == CombatTerminal::Win;
    snapshot.best_complete = Some(line_summary.clone());
    snapshot.best_win = (selected_line.terminal == CombatTerminal::Win).then_some(line_summary);
    snapshot.best_hp_loss =
        (selected_line.terminal == CombatTerminal::Win).then_some(selected_line.hp_loss);
    snapshot.nodes_expanded = performance.nodes_expanded;
    snapshot.nodes_generated = performance.nodes_generated;
    snapshot.terminal_wins = u64::from(selected_line.terminal == CombatTerminal::Win);
    snapshot.total_us = performance.total_us;
    RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot }
}

fn combat_search_performance_snapshot(
    source: String,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
) -> CombatSearchPerformanceSnapshotV1 {
    let combat = &start.combat;
    CombatSearchPerformanceSnapshotV1 {
        source,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        turn: combat.turn.turn_count,
        combat_kind: combat_kind_label(combat),
        enemies: combat_enemy_names(combat),
        boss: session
            .run_state
            .boss_key
            .map(|boss| format!("{boss:?}"))
            .unwrap_or_else(|| "unknown".to_string()),
        external_payoff_opportunity: has_external_payoff_opportunity(combat),
        coverage_status: format!("{:?}", report.outcome.coverage_status),
        complete_trajectory_found: report.outcome.complete_trajectory_found,
        complete_win_found: report.outcome.complete_win_found,
        best_complete: report
            .best_complete_trajectory
            .as_ref()
            .map(combat_search_line_summary),
        best_win: report
            .best_win_trajectory
            .as_ref()
            .map(combat_search_line_summary),
        best_hp_loss: report
            .best_win_trajectory
            .as_ref()
            .map(|trajectory| trajectory.hp_loss),
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        terminal_wins: report.stats.terminal_wins,
        total_us: micros_to_u64(report.performance.total_elapsed_us),
        unattributed_us: micros_to_u64(report.performance.unattributed_elapsed_us),
        rollout_calls: report.performance.rollout_estimate_calls,
        root_rollout_calls: report.performance.root_rollout_estimate_calls,
        child_rollout_calls: report.performance.child_rollout_estimate_calls,
        deferred_child_rollout_calls: report.performance.deferred_child_rollout_estimate_calls,
        turn_plan_seed_rollout_calls: report.performance.turn_plan_seed_rollout_estimate_calls,
        deferred_child_rollout_nodes: report.performance.deferred_child_rollout_nodes,
        deferred_child_rollout_requeues: report.performance.deferred_child_rollout_requeues,
        rollout_cache_hits: report.rollout.cache_hits,
        rollout_cache_queries: report.rollout.cache_queries,
        rollout_cache_misses: report.rollout.cache_misses,
        rollout_cache_inserts: report.rollout.cache_inserts,
        rollout_budget_skips: report.rollout.budget_skips,
        rollout_max_evaluation_budget_skips: report.rollout.max_evaluation_budget_skips,
        rollout_deadline_budget_skips: report.rollout.deadline_budget_skips,
        rollout_truncated: report.rollout.truncated_rollouts,
        rollout_terminal_wins: report.rollout.terminal_wins,
        rollout_cache_lookup_us: micros_to_u64(report.rollout.performance.cache_lookup_us),
        rollout_policy_dispatch_us: micros_to_u64(report.rollout.performance.policy_dispatch_us),
        rollout_no_potion_iterations: report.rollout.performance.no_potion_iterations,
        rollout_no_potion_phase_profile_us: micros_to_u64(
            report.rollout.performance.no_potion_phase_profile_us,
        ),
        rollout_no_potion_legal_actions_us: micros_to_u64(
            report.rollout.performance.no_potion_legal_actions_us,
        ),
        rollout_no_potion_choose_action_us: micros_to_u64(
            report.rollout.performance.no_potion_choose_action_us,
        ),
        rollout_no_potion_choose_ordering_us: micros_to_u64(
            report.rollout.performance.no_potion_choose_ordering_us,
        ),
        rollout_no_potion_probe_us: micros_to_u64(report.rollout.performance.no_potion_probe_us),
        rollout_no_potion_probe_score_calls: report.rollout.performance.no_potion_probe_score_calls,
        rollout_no_potion_probe_actions_evaluated: report
            .rollout
            .performance
            .no_potion_probe_actions_evaluated,
        rollout_no_potion_probe_step_reuses: report.rollout.performance.no_potion_probe_step_reuses,
        rollout_no_potion_probe_engine_step_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_engine_step_us,
        ),
        rollout_no_potion_probe_phase_profile_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_phase_profile_us,
        ),
        rollout_no_potion_probe_action_facts_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_action_facts_us,
        ),
        rollout_no_potion_engine_step_us: micros_to_u64(
            report.rollout.performance.no_potion_engine_step_us,
        ),
        rollout_no_potion_child_build_us: micros_to_u64(
            report.rollout.performance.no_potion_child_build_us,
        ),
        terminal_child_rollout_skips: report.performance.terminal_child_rollout_skips,
        terminal_turn_plan_seed_rollout_skips: report
            .performance
            .terminal_turn_plan_seed_rollout_skips,
        turn_local_dominance_rollout_skips: report.performance.turn_local_dominance_rollout_skips,
        rollout_us: micros_to_u64(report.performance.rollout_estimate_elapsed_us),
        expansion_us: micros_to_u64(report.performance.expansion_elapsed_us),
        child_bookkeeping_us: micros_to_u64(report.performance.child_bookkeeping_elapsed_us),
        engine_step_us: micros_to_u64(report.performance.engine_step_elapsed_us),
        pre_expand_us: micros_to_u64(report.performance.pre_expand_elapsed_us),
        frontier_pop_us: micros_to_u64(report.performance.frontier_pop_elapsed_us),
        turn_plan_seed_us: micros_to_u64(report.performance.turn_plan_frontier_seed_elapsed_us),
        shadow_audit_us: micros_to_u64(report.performance.shadow_audit_elapsed_us),
        root_turn_plan_diag_us: micros_to_u64(
            report.performance.root_turn_plan_diagnostics_elapsed_us,
        ),
    }
}

fn combat_kind_label(combat: &crate::runtime::combat::CombatState) -> String {
    if combat.meta.is_boss_fight {
        "boss".to_string()
    } else if combat.meta.is_elite_fight {
        "elite".to_string()
    } else {
        "hallway".to_string()
    }
}

fn combat_enemy_names(combat: &crate::runtime::combat::CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_escaped)
        .map(|monster| {
            EnemyId::from_id(monster.monster_type)
                .map(|enemy| enemy.get_name().to_string())
                .unwrap_or_else(|| format!("monster#{}", monster.monster_type))
        })
        .collect()
}

fn combat_search_line_summary(
    trajectory: &CombatSearchV2TrajectoryReport,
) -> CombatSearchTerminalLineSummary {
    CombatSearchTerminalLineSummary {
        terminal: trajectory.terminal,
        final_hp: trajectory.final_hp,
        hp_loss: trajectory.hp_loss,
        turns: trajectory.turns,
        cards_played: trajectory.cards_played,
        potions_used: trajectory.potions_used,
        potions_discarded: trajectory.potions_discarded,
        action_count: trajectory.actions.len(),
    }
}

fn combat_candidate_line_summary(line: &CombatCandidateLine) -> CombatSearchTerminalLineSummary {
    CombatSearchTerminalLineSummary {
        terminal: match line.terminal {
            CombatTerminal::Win => SearchTerminalLabel::Win,
            CombatTerminal::Loss => SearchTerminalLabel::Loss,
            CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
        },
        final_hp: line.final_hp,
        hp_loss: line.hp_loss,
        turns: line.turns,
        cards_played: line.cards_played,
        potions_used: line.potions_used,
        potions_discarded: line.potions_discarded,
        action_count: line.actions.len(),
    }
}

fn micros_to_u64(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn millis_to_micros_u64(value: u128) -> u64 {
    micros_to_u64(value.saturating_mul(1_000))
}

fn drawn_cards_from_action_result(action_result: Option<&ActionResult>) -> Vec<CardSnapshot> {
    action_result
        .into_iter()
        .flat_map(|result| result.changes.iter())
        .filter_map(|change| match change {
            ActionResultChange::CombatCardDrawn { card } => Some(card.clone()),
            _ => None,
        })
        .collect()
}

fn save_search_evidence_if_requested(
    session: &RunControlSession,
    target: Option<&RunControlSearchEvidenceTarget>,
    report: &CombatSearchV2Report,
) -> Result<Option<std::path::PathBuf>, String> {
    let Some(target) = target else {
        return Ok(None);
    };
    let (path, capture_case_id, capture_root, capture_path) = match target {
        RunControlSearchEvidenceTarget::Path(path) => {
            (next_available_evidence_path(path), None, None, None)
        }
        RunControlSearchEvidenceTarget::LastCaptureCase => {
            let case = session.active_capture_case().ok_or_else(|| {
                "search evidence save=case requires the current combat to have a matching cap <case_id>"
                    .to_string()
            })?;
            let paths = BenchmarkCasePaths::for_case(&case.root, &case.case_id);
            let base_path = case.root.join("search_evidence").join(format!(
                "{}.step{}.search.json",
                case.case_id, session.decision_step
            ));
            (
                next_available_evidence_path(&base_path),
                Some(case.case_id.clone()),
                Some(case.root.display().to_string()),
                Some(paths.capture_path.display().to_string()),
            )
        }
    };
    save_combat_search_evidence_v1(
        &path,
        CombatSearchEvidenceContextV1 {
            source_kind: "run_control_search_combat",
            decision_step: session.decision_step,
            capture_case_id,
            capture_root,
            capture_path,
        },
        report,
    )?;
    Ok(Some(path))
}

fn effective_hp_loss_limit(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> Option<u32> {
    match options.max_hp_loss {
        Some(RunControlHpLossLimit::Limit(limit)) => Some(limit),
        Some(RunControlHpLossLimit::Unlimited) => None,
        None => session.search_max_hp_loss,
    }
}

pub(in crate::eval::run_control) fn high_stakes_search_options(
    session: &RunControlSession,
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, &options);
    if options.potion_policy.is_none() && session.search_potion_policy.is_none() {
        options.potion_policy = plan.primary_potion_policy;
    }
    if options.max_potions_used.is_none() && session.search_max_potions_used.is_none() {
        options.max_potions_used = plan.primary_max_potions_used;
    }
    options
}

fn search_report_has_invalid_card_identity(report: &CombatSearchV2Report) -> bool {
    report
        .diagnostics
        .card_identity
        .states_with_uuid_card_id_conflict
        > 0
}

fn next_available_evidence_path(path: &std::path::Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("search_evidence");
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    for idx in 2..10_000 {
        let candidate = parent.join(format!("{stem}.{idx}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}.overflow.{ext}"))
}

fn render_saved_evidence_note(path: Option<&std::path::Path>) -> String {
    path.map(|path| format!("\nSearch evidence saved: {}", path.display()))
        .unwrap_or_default()
}

fn search_config(
    session: &RunControlSession,
    options: RunControlSearchCombatOptions,
) -> CombatSearchV2Config {
    let defaults = CombatSearchV2Config::default();
    let stop_on_win_hp_loss_at_most = effective_hp_loss_limit(session, &options);
    CombatSearchV2Config {
        max_nodes: options
            .max_nodes
            .or(session.search_max_nodes)
            .unwrap_or(defaults.max_nodes),
        max_actions_per_line: options
            .max_actions_per_line
            .unwrap_or(defaults.max_actions_per_line),
        max_engine_steps_per_action: options
            .max_engine_steps_per_action
            .unwrap_or(defaults.max_engine_steps_per_action),
        wall_time: options
            .wall_ms
            .or(session.search_wall_ms)
            .map(std::time::Duration::from_millis),
        stop_on_win_hp_loss_at_most,
        min_win_candidates_before_stop: defaults.min_win_candidates_before_stop,
        input_label: Some(format!(
            "run_play_driver:search_combat:step{}",
            session.decision_step
        )),
        potion_policy: options
            .potion_policy
            .or(session.search_potion_policy)
            .unwrap_or(defaults.potion_policy),
        max_potions_used: options
            .max_potions_used
            .or(session.search_max_potions_used)
            .or(defaults.max_potions_used),
        rollout_policy: options.rollout_policy.unwrap_or(defaults.rollout_policy),
        child_rollout_policy: options
            .child_rollout_policy
            .unwrap_or(defaults.child_rollout_policy),
        rollout_max_evaluations: options
            .rollout_max_evaluations
            .unwrap_or(defaults.rollout_max_evaluations),
        rollout_max_actions: options
            .rollout_max_actions
            .unwrap_or(defaults.rollout_max_actions),
        rollout_beam_width: options
            .rollout_beam_width
            .unwrap_or(defaults.rollout_beam_width),
        turn_plan_policy: options
            .turn_plan_policy
            .unwrap_or(defaults.turn_plan_policy),
        frontier_policy: options.frontier_policy.unwrap_or(defaults.frontier_policy),
        phase_guard_policy: defaults.phase_guard_policy,
        turn_plan_probe_max_inner_nodes: defaults.turn_plan_probe_max_inner_nodes,
        turn_plan_probe_max_end_states: defaults.turn_plan_probe_max_end_states,
        turn_plan_probe_per_bucket_limit: defaults.turn_plan_probe_per_bucket_limit,
        root_action_prior: None,
        turn_plan_prior: None,
    }
}

fn verify_segment_trajectory_replays(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    config: &CombatSearchV2Config,
) -> Result<(), String> {
    if actions.is_empty() {
        return Err("search-combat segment dry-run refused empty action list".to_string());
    }
    let stepper = EngineCombatStepper;
    let mut position = start.clone();
    for action in actions {
        let choices = filter_combat_search_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &position.combat,
        );
        let Some(choice) = choices
            .iter()
            .find(|choice| choice.input == action.input && choice.action_key == action.action_key)
        else {
            return Err(format!(
                "search-combat segment dry-run drift at step {}: expected {} ({})",
                action.step_index,
                action.action_key,
                client_input_hint(&action.input)
            ));
        };
        let step = stepper.apply_to_stable(
            &position,
            choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated {
            return Err(format!(
                "search-combat segment dry-run truncated at step {} after {} engine steps",
                action.step_index, step.engine_steps
            ));
        }
        position = step.position;
    }
    match combat_terminal(&position.engine, &position.combat) {
        CombatTerminal::Loss => Err("search-combat segment dry-run ended in loss".to_string()),
        CombatTerminal::Win | CombatTerminal::Unresolved => Ok(()),
    }
}

fn render_search_rejection(
    report: &CombatSearchV2Report,
    result: &'static str,
    detail: Option<String>,
) -> String {
    let mut lines = vec![
        "Search combat did not modify state.".to_string(),
        format!("  result={result}"),
    ];
    if let Some(detail) = detail {
        lines.push(format!("  detail={detail}"));
    }
    if let Some(candidate) = report.best_complete_trajectory.as_ref() {
        lines.push(format!(
            "  best_complete_candidate terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} potions_used={} actions={}",
            candidate.terminal,
            candidate.final_hp,
            candidate.hp_loss,
            candidate.turns,
            candidate.cards_played,
            candidate.potions_used,
            candidate.actions.len()
        ));
    } else {
        lines.push("  best_complete_candidate=none".to_string());
    }
    lines.extend([
        format!("  coverage_status={:?}", report.outcome.coverage_status),
        render_search_policy_summary(report),
        render_search_diagnostics_summary(report),
        render_search_performance_summary(report),
        render_policy_evidence_summary(report),
        format!(
            "  complete_trajectory_found={}",
            report.outcome.complete_trajectory_found
        ),
        format!("  terminal_wins={}", report.stats.terminal_wins),
        format!("  nodes_expanded={}", report.stats.nodes_expanded),
        format!("  nodes_generated={}", report.stats.nodes_generated),
        format!(
            "  rollouts={} rollout_wins={} rollout_skips={}",
            report.rollout.evaluations, report.rollout.terminal_wins, report.rollout.budget_skips
        ),
        format!("  reliability={}", report.evidence_reliability.reliability),
        format!("  coverage_reason={}", report.outcome.coverage_reason),
    ]);
    lines.join("\n")
}

fn render_search_application(
    report: &CombatSearchV2Report,
    actions: &[CombatSearchV2ActionTrace],
    selected_line: &CombatCandidateLine,
    replay_applied_count: usize,
) -> String {
    let trajectory = report
        .best_win_trajectory
        .as_ref()
        .expect("caller only renders after selecting a complete trajectory");
    let mut lines = vec![
        "Search combat applied complete winning candidate.".to_string(),
        format!(
            "  coverage_status={:?} reliability={}",
            report.outcome.coverage_status, report.evidence_reliability.reliability
        ),
        render_search_policy_summary(report),
        render_search_diagnostics_summary(report),
        render_search_performance_summary(report),
        render_policy_evidence_summary(report),
        format!("  coverage_reason={}", report.outcome.coverage_reason),
        format!(
            "  selected_line source={} terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} potions_used={} potions_discarded={} replay_applied={}",
            selected_line.source.label(),
            selected_line.terminal,
            selected_line.final_hp,
            selected_line.hp_loss,
            selected_line.turns,
            selected_line.cards_played,
            selected_line.potions_used,
            selected_line.potions_discarded,
            replay_applied_count
        ),
        format!(
            "  selected_assumptions={}",
            selected_line.assumption_labels().join(",")
        ),
        format!(
            "  original_final_hp={} original_hp_loss={} original_turns={} original_cards_played={} original_potions_used={} original_potions_discarded={}",
            trajectory.final_hp,
            trajectory.hp_loss,
            trajectory.turns,
            trajectory.cards_played,
            trajectory.potions_used,
            trajectory.potions_discarded
        ),
        format!(
            "  nodes_expanded={} nodes_generated={} nodes_to_first_win={:?}",
            report.stats.nodes_expanded,
            report.stats.nodes_generated,
            report.stats.nodes_to_first_win
        ),
        format!(
            "  rollout_policy={} rollouts={} rollout_wins={} rollout_skips={}",
            report.rollout.policy,
            report.rollout.evaluations,
            report.rollout.terminal_wins,
            report.rollout.budget_skips
        ),
        format!(
            "  action_count={} potion_policy={}",
            actions.len(),
            report.search_policy.potion_policy
        ),
    ];
    for action in actions.iter().take(12) {
        lines.push(format!(
            "    {} | {} | {}",
            action.step_index,
            client_input_hint(&action.input),
            action.action_key
        ));
    }
    if actions.len() > 12 {
        lines.push(format!("    ... {} more actions", actions.len() - 12));
    }
    lines.join("\n")
}

fn render_complete_line_solver_application(
    report: &CombatSearchV2Report,
    actions: &[CombatSearchV2ActionTrace],
    selected_line: &CombatCandidateLine,
    replay_applied_count: usize,
) -> String {
    let mut lines = vec![
        "Complete line solver applied winning candidate.".to_string(),
        format!(
            "  previous_search coverage_status={:?} reliability={}",
            report.outcome.coverage_status, report.evidence_reliability.reliability
        ),
        format!(
            "  selected_line source={} terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} replay_applied={}",
            selected_line.source.label(),
            selected_line.terminal,
            selected_line.final_hp,
            selected_line.hp_loss,
            selected_line.turns,
            selected_line.cards_played,
            replay_applied_count
        ),
    ];
    for action in actions.iter().take(12) {
        lines.push(format!(
            "    {} | {} | {}",
            action.step_index,
            client_input_hint(&action.input),
            action.action_key
        ));
    }
    if actions.len() > 12 {
        lines.push(format!("    ... {} more actions", actions.len() - 12));
    }
    lines.join("\n")
}

fn render_segment_application(
    search_report: &CombatSearchV2Report,
    segment_report: &CombatSearchV2TurnSegmentReport,
    rejection_result: &'static str,
) -> String {
    let trajectory = segment_report
        .selected
        .as_ref()
        .expect("caller only renders after selecting a segment");
    let mut lines = vec![
        "Search combat applied partial turn segment.".to_string(),
        format!("  behavior_label={}", segment_report.behavior_label),
        format!("  source={}", segment_report.source),
        format!("  original_search_result={rejection_result}"),
        format!(
            "  segment_bucket={} stop_reason={} candidate_count={} nodes_expanded={} nodes_generated={}",
            segment_report.selected_bucket.unwrap_or("unknown"),
            segment_report.selected_stop_reason.unwrap_or("unknown"),
            segment_report.candidate_count,
            segment_report.nodes_expanded,
            segment_report.nodes_generated
        ),
        format!(
            "  segment_terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} potions_used={}",
            trajectory.terminal,
            trajectory.final_hp,
            trajectory.hp_loss,
            trajectory.turns,
            trajectory.cards_played,
            trajectory.potions_used
        ),
        format!(
            "  search_coverage={:?} reliability={}",
            search_report.outcome.coverage_status, search_report.evidence_reliability.reliability
        ),
        render_search_policy_summary(search_report),
        render_search_performance_summary(search_report),
        render_policy_evidence_summary(search_report),
        "  terminal_claim=none; this is an exact applied prefix, not a complete-win proof"
            .to_string(),
        format!("  action_count={}", trajectory.actions.len()),
    ];
    for action in trajectory.actions.iter().take(12) {
        lines.push(format!(
            "    {} | {} | {}",
            action.step_index,
            client_input_hint(&action.input),
            action.action_key
        ));
    }
    if trajectory.actions.len() > 12 {
        lines.push(format!(
            "    ... {} more actions",
            trajectory.actions.len() - 12
        ));
    }
    lines.join("\n")
}

fn render_search_policy_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  frontier_policy={} turn_plan_policy={} rollout_policy={}",
        report.search_policy.frontier_policy,
        report.search_policy.turn_plan_policy,
        report.rollout.policy
    )
}

fn render_search_diagnostics_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  search_diagnostics=frontier_remaining={} unresolved_leaf={} max_actions_cut={} engine_step_cut={} potion_budget_cut={} turn_plan_observed={} turn_plan_seeded={} pending_states={} pending_high_fanout={} rollout_budget_skips={}",
        report.frontier.remaining_states,
        report.frontier.unresolved_leaf_count,
        report.frontier.max_actions_cut_count,
        report.frontier.engine_step_limit_count,
        report.frontier.potion_budget_cut_count,
        report.diagnostics.turn_plan.root_states_observed,
        report.diagnostics.turn_plan.frontier_seeded_nodes,
        report.diagnostics.pending_choice.pending_choice_states,
        report.diagnostics.pending_choice.high_fanout_states,
        report.rollout.budget_skips,
    )
}

fn render_search_performance_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  search_performance=elapsed_ms={} total_us={} unattributed_us={} frontier_pop_calls={} frontier_pop_us={} pre_expand_us={} expansion_us={} child_bookkeeping_us={} engine_step_calls={} engine_step_us={} rollout_calls={} root_rollout_calls={} child_rollout_calls={} deferred_child_rollout_calls={} turn_plan_seed_rollout_calls={} deferred_child_nodes={} deferred_child_requeues={} rollout_cache=hits/queries/misses/inserts:{}/{}/{}/{} rollout_budget_skips={} max_eval_budget_skips={} deadline_budget_skips={} rollout_truncated={} rollout_terminal_wins={} rollout_inner_us=iters:{} cache_lookup:{} policy_total:{} phase:{} legal:{} choose:{} order:{} probe:{} probe_calls:{} probe_eval:{} probe_reuse:{} probe_engine:{} probe_phase:{} probe_facts:{} engine:{} build:{} terminal_child_rollout_skips={} terminal_turn_plan_seed_rollout_skips={} turn_local_dominance_rollout_skips={} rollout_us={} turn_plan_seed_calls={} turn_plan_seed_us={} shadow_audit_us={} root_turn_plan_diag_us={}",
        report.stats.elapsed_ms,
        report.performance.total_elapsed_us,
        report.performance.unattributed_elapsed_us,
        report.performance.frontier_pop_calls,
        report.performance.frontier_pop_elapsed_us,
        report.performance.pre_expand_elapsed_us,
        report.performance.expansion_elapsed_us,
        report.performance.child_bookkeeping_elapsed_us,
        report.performance.engine_step_calls,
        report.performance.engine_step_elapsed_us,
        report.performance.rollout_estimate_calls,
        report.performance.root_rollout_estimate_calls,
        report.performance.child_rollout_estimate_calls,
        report.performance.deferred_child_rollout_estimate_calls,
        report.performance.turn_plan_seed_rollout_estimate_calls,
        report.performance.deferred_child_rollout_nodes,
        report.performance.deferred_child_rollout_requeues,
        report.rollout.cache_hits,
        report.rollout.cache_queries,
        report.rollout.cache_misses,
        report.rollout.cache_inserts,
        report.rollout.budget_skips,
        report.rollout.max_evaluation_budget_skips,
        report.rollout.deadline_budget_skips,
        report.rollout.truncated_rollouts,
        report.rollout.terminal_wins,
        report.rollout.performance.no_potion_iterations,
        report.rollout.performance.cache_lookup_us,
        report.rollout.performance.policy_dispatch_us,
        report.rollout.performance.no_potion_phase_profile_us,
        report.rollout.performance.no_potion_legal_actions_us,
        report.rollout.performance.no_potion_choose_action_us,
        report.rollout.performance.no_potion_choose_ordering_us,
        report.rollout.performance.no_potion_probe_us,
        report.rollout.performance.no_potion_probe_score_calls,
        report.rollout.performance.no_potion_probe_actions_evaluated,
        report.rollout.performance.no_potion_probe_step_reuses,
        report.rollout.performance.no_potion_probe_engine_step_us,
        report.rollout.performance.no_potion_probe_phase_profile_us,
        report.rollout.performance.no_potion_probe_action_facts_us,
        report.rollout.performance.no_potion_engine_step_us,
        report.rollout.performance.no_potion_child_build_us,
        report.performance.terminal_child_rollout_skips,
        report.performance.terminal_turn_plan_seed_rollout_skips,
        report.performance.turn_local_dominance_rollout_skips,
        report.performance.rollout_estimate_elapsed_us,
        report.performance.turn_plan_frontier_seed_calls,
        report.performance.turn_plan_frontier_seed_elapsed_us,
        report.performance.shadow_audit_elapsed_us,
        report.performance.root_turn_plan_diagnostics_elapsed_us,
    )
}

fn render_policy_evidence_summary(report: &CombatSearchV2Report) -> String {
    format!("  {}", report.policy_evidence.machine_summary())
}

fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{
        combat_automation_step_state_v1, effective_hp_loss_limit, high_stakes_search_options,
        next_available_evidence_path, search_config, segment_mode_allows_turn_segment,
    };
    use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::powers::{store, PowerId};
    use crate::eval::run_control::trace_annotation::{
        CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1,
        CombatAutomationTrajectorySource, RunControlTraceAnnotationV1,
    };
    use crate::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    };
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, ClientInput, CombatContext, EngineState, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardScreenContext;

    fn session_with_active_combat(
        mut combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            {
                combat.entities.monsters = vec![crate::test_support::test_monster(
                    crate::content::monsters::EnemyId::JawWorm,
                )];
                combat
            },
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn combat_automation_step_state_records_time_warp_counter_and_forced_end_state() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(
            crate::content::monsters::EnemyId::TimeEater,
        )];
        let monster_id = combat.entities.monsters[0].id;
        store::set_powers_for(
            &mut combat,
            monster_id,
            vec![
                crate::runtime::combat::Power {
                    power_type: PowerId::TimeWarp,
                    instance_id: None,
                    amount: 11,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
                crate::runtime::combat::Power {
                    power_type: PowerId::Strength,
                    instance_id: None,
                    amount: 2,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
            ],
        );
        combat.turn.counters.cards_played_this_turn = 11;
        combat.turn.mark_early_end_turn_pending();
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let snapshot = combat_automation_step_state_v1(&session)
            .expect("active combat should produce automation step state");

        assert_eq!(snapshot.cards_played_this_turn, 11);
        assert!(snapshot.early_end_turn_pending);
        assert_eq!(snapshot.monsters.len(), 1);
        assert_eq!(snapshot.monsters[0].label, "Time Eater");
        assert_eq!(snapshot.monsters[0].time_warp, 11);
        assert_eq!(snapshot.monsters[0].strength, 2);
    }

    fn session_with_combat_flags(is_boss_fight: bool, is_elite_fight: bool) -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss_fight;
        combat.meta.is_elite_fight = is_elite_fight;
        session_with_active_combat(combat)
    }

    fn options_with_hp_loss(max_hp_loss: RunControlHpLossLimit) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            max_hp_loss: Some(max_hp_loss),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn options_with_potion_budget(
        potion_policy: CombatSearchV2PotionPolicy,
        max_potions_used: u32,
    ) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            potion_policy: Some(potion_policy),
            max_potions_used: Some(max_potions_used),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn assert_potion_budget(
        options: RunControlSearchCombatOptions,
        expected_policy: Option<CombatSearchV2PotionPolicy>,
        expected_max_used: Option<u32>,
    ) {
        assert_eq!(options.potion_policy, expected_policy);
        assert_eq!(options.max_potions_used, expected_max_used);
    }

    #[test]
    fn search_combat_uses_smoke_bomb_as_survival_fallback_when_no_win_exists() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 1;
        combat.entities.player.max_hp = 80;
        combat.turn.energy = 0;
        combat.meta.is_boss_fight = false;
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        jaw_worm.current_hp = 40;
        jaw_worm.max_hp = 40;
        let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
            crate::runtime::monster_move::AttackSpec {
                base_damage: 10,
                hits: 1,
                damage_kind: crate::runtime::monster_move::DamageKind::Normal,
            },
        );
        jaw_worm.set_planned_steps(attack.to_steps());
        jaw_worm.set_planned_visible_spec(Some(attack));
        combat.entities.monsters = vec![jaw_worm];
        combat.zones.hand = vec![CombatCard::new(crate::content::cards::CardId::Defend, 1)];
        combat.update_hand_cards();
        combat.entities.potions = vec![Some(Potion::new(PotionId::SmokeBomb, 1))];
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let outcome = super::try_apply_smoke_bomb_survival_fallback_after_rejection(
            &mut session,
            None,
            "no_complete_winning_candidate",
        )
        .expect("fallback should not error")
        .expect("search combat should allow smoke bomb survival fallback");

        let EngineState::RewardScreen(rewards) = &session.engine_state else {
            panic!(
                "smoke bomb fallback should leave combat at reward screen, got {:?}",
                session.engine_state
            );
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::SmokedCombat);
        assert!(
            outcome.message.contains("Smoke Bomb"),
            "fallback outcome should be explicit, got: {}",
            outcome.message
        );
    }

    #[test]
    fn search_evidence_path_does_not_overwrite_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "sts_search_evidence_path_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir should be created");
        let base = root.join("case.step1.search.json");
        fs::write(&base, "{}").expect("base file should be written");

        let next = next_available_evidence_path(&base);

        assert_ne!(next, base);
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("case.step1.search.2.json")
        );
        assert!(!next.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn combat_automation_trace_annotation_preserves_action_inputs() {
        let annotation = CombatAutomationTrajectoryRecordV1::new(
            CombatAutomationTrajectorySource::SearchCombat,
            vec![CombatAutomationActionV1 {
                step_index: 7,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
        )
        .into_annotation();

        let RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source,
            action_count,
            actions,
            label_role,
        } = annotation
        else {
            panic!("expected combat automation trajectory annotation")
        };
        assert_eq!(source, CombatAutomationTrajectorySource::SearchCombat);
        assert_eq!(action_count, 1);
        assert_eq!(actions[0].step_index, 7);
        assert_eq!(actions[0].action_key, "combat/end_turn");
        assert_eq!(actions[0].input, ClientInput::EndTurn);
        assert_eq!(label_role, "simulator_generated_not_teacher_label");
    }

    #[test]
    fn hp_loss_limit_uses_session_default_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(12),
            ..RunControlConfig::default()
        });

        assert_eq!(
            effective_hp_loss_limit(&session, &RunControlSearchCombatOptions::default()),
            Some(12)
        );
        assert_eq!(
            search_config(&session, RunControlSearchCombatOptions::default())
                .stop_on_win_hp_loss_at_most,
            Some(12)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            ),
            Some(4)
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            )
            .stop_on_win_hp_loss_at_most,
            Some(4)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            ),
            None
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            )
            .stop_on_win_hp_loss_at_most,
            None
        );
    }

    #[test]
    fn search_config_uses_session_budget_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_nodes: Some(1234),
            search_wall_ms: Some(5678),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(config.max_nodes, 1234);
        assert_eq!(config.wall_time, Some(Duration::from_millis(5678)));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(90),
                wall_ms: Some(12),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 90);
        assert_eq!(config.wall_time, Some(Duration::from_millis(12)));
    }

    #[test]
    fn search_config_uses_session_potion_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            search_max_potions_used: Some(2),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn high_stakes_search_options_enables_semantic_potions_for_boss_manual_search() {
        let session = session_with_combat_flags(true, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(2),
        );
    }

    #[test]
    fn high_stakes_search_options_enables_single_semantic_potion_for_elite_manual_search() {
        let session = session_with_combat_flags(false, true);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(1),
        );
    }

    #[test]
    fn non_boss_segment_mode_allows_hallway_partial_turns_but_blocks_boss_partial_turns() {
        let hallway = session_with_combat_flags(false, false);
        let hallway_start = hallway
            .current_active_combat_position()
            .expect("hallway combat position");
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &hallway_start
        ));

        let boss = session_with_combat_flags(true, false);
        let boss_start = boss
            .current_active_combat_position()
            .expect("boss combat position");
        assert!(!segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &boss_start
        ));
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::TurnBoundary),
            &boss_start
        ));
    }

    #[test]
    fn high_stakes_search_options_keeps_ordinary_manual_search_no_potion_default() {
        let session = session_with_combat_flags(false, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(options, None, None);
    }

    #[test]
    fn high_stakes_search_options_respects_user_potion_override() {
        let session = session_with_combat_flags(true, false);

        let options = high_stakes_search_options(
            &session,
            options_with_potion_budget(CombatSearchV2PotionPolicy::Never, 0),
        );

        assert_potion_budget(options, Some(CombatSearchV2PotionPolicy::Never), Some(0));
    }
}
