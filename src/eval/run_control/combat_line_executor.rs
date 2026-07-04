use crate::ai::combat_search_v2::{
    CombatSearchV2ActionTrace, CombatSearchV2Config, CombatSearchV2Report,
    CombatSearchV2TurnSegmentReport,
};
use crate::sim::combat::{CombatPosition, CombatTerminal};
use crate::state::core::ClientInput;

use super::combat_candidate_line::{replay_candidate_line, CombatCandidateLine};
use super::combat_line_trace::{
    combat_automation_step_state_v1, combat_line_performance_trace_annotation,
    combat_search_performance_trace_annotation, current_run_apply_status,
    CombatCandidateLinePerformance,
};
use super::combat_search_render::{
    render_complete_line_solver_application, render_saved_evidence_note, render_search_application,
    render_segment_application,
};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::{
    CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
};
use super::transition_report::{
    action_result_from_transition, render_action_result, ActionResult, ActionResultChange,
    CardSnapshot, RunVisibleSnapshot, TransitionAction,
};

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
    session.mark_current_combat_search_resolved();
    let automation_actions = apply_combat_action_traces(session, &applied)?;
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
    session.mark_current_combat_search_resolved();
    let automation_actions = apply_combat_action_traces(session, &applied)?;
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

fn active_combat_is_waiting_for_smoke_escape_turn_end(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|active| active.combat_state.turn.counters.player_escaping)
}

fn apply_combat_action_traces(
    session: &mut RunControlSession,
    actions: &[CombatSearchV2ActionTrace],
) -> Result<Vec<CombatAutomationActionV1>, String> {
    actions
        .iter()
        .map(|action| {
            let outcome = session.apply_input(action.input.clone())?;
            Ok(CombatAutomationActionV1 {
                step_index: action.step_index,
                action_key: action.action_key.clone(),
                input: action.input.clone(),
                drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
                combat_after: combat_automation_step_state_v1(session),
            })
        })
        .collect()
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
