use crate::ai::combat_search_v2::{
    CombatSearchV2ActionTrace, CombatSearchV2Config, CombatSearchV2Report,
    CombatSearchV2TurnSegmentReport, SearchTerminalLabel,
};
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::sim::combat::{CombatPosition, CombatTerminal};
use crate::state::core::{ClientInput, EngineState, RunResult};

use super::combat_candidate_line::{replay_candidate_line, CombatCandidateLine};
use super::session::{RunControlCommandOutcome, RunControlSession};
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

#[derive(Clone, Copy)]
pub(super) struct CombatCandidateLinePerformance {
    pub(super) nodes_expanded: u64,
    pub(super) nodes_generated: u64,
    pub(super) total_us: u64,
}

pub(super) fn apply_selected_combat_candidate_line(
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

pub(super) fn apply_combat_turn_segment(
    session: &mut RunControlSession,
    start: &CombatPosition,
    search_report: &CombatSearchV2Report,
    segment_report: &CombatSearchV2TurnSegmentReport,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<RunControlCommandOutcome, String> {
    let trajectory = segment_report
        .selected
        .as_ref()
        .expect("caller only applies after selecting a segment");
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
        render_segment_application(search_report, segment_report, rejection_result),
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
    outcome.search_evidence_path = saved_evidence.map(std::path::Path::to_path_buf);
    Ok(outcome)
}

pub(super) fn apply_smoke_bomb_survival_fallback(
    session: &mut RunControlSession,
    smoke_input: ClientInput,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<RunControlCommandOutcome, String> {
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
        let end_turn_outcome = session.apply_input(ClientInput::EndTurn)?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: 1,
            action_key: "combat/end_turn_after_smoke_bomb".to_string(),
            input: ClientInput::EndTurn,
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
    outcome.search_evidence_path = saved_evidence.map(std::path::Path::to_path_buf);
    Ok(outcome)
}

pub(super) fn combat_automation_step_state_v1(
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

fn active_combat_is_waiting_for_smoke_escape_turn_end(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|active| active.combat_state.turn.counters.player_escaping)
}

pub(super) fn combat_search_performance_trace_annotation(
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
        external_payoff_opportunity: crate::ai::combat_search_v2::has_external_payoff_opportunity(
            combat,
        ),
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
    trajectory: &crate::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
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

pub(super) fn drawn_cards_from_action_result(
    action_result: Option<&ActionResult>,
) -> Vec<CardSnapshot> {
    action_result
        .into_iter()
        .flat_map(|result| result.changes.iter())
        .filter_map(|change| match change {
            ActionResultChange::CombatCardDrawn { card } => Some(card.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn render_saved_evidence_note(path: Option<&std::path::Path>) -> String {
    path.map(|path| format!("\nSearch evidence saved: {}", path.display()))
        .unwrap_or_default()
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

pub(super) fn render_search_policy_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  frontier_policy={} turn_plan_policy={} rollout_policy={}",
        report.search_policy.frontier_policy,
        report.search_policy.turn_plan_policy,
        report.rollout.policy
    )
}

pub(super) fn render_search_diagnostics_summary(report: &CombatSearchV2Report) -> String {
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

pub(super) fn render_search_performance_summary(report: &CombatSearchV2Report) -> String {
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

pub(super) fn render_policy_evidence_summary(report: &CombatSearchV2Report) -> String {
    format!("  {}", report.policy_evidence.machine_summary())
}

pub(super) fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

pub(super) fn millis_to_micros_u64(value: u128) -> u64 {
    micros_to_u64(value.saturating_mul(1_000))
}

fn micros_to_u64(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}
