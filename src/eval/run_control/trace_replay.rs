use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::commands::{parse_run_control_command, RunControlCommand};
use super::render::render_run_control_state;
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::session_trace::{
    session_trace_boundary_fingerprint, SessionTraceBoundaryFingerprintV1, SessionTraceCandidateV1,
    SessionTraceRecorder, SessionTraceSelectionResolution, SessionTraceStepSourceV1,
    SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME, SESSION_TRACE_SCHEMA_VERSION,
};
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::view_model::build_run_control_view_model;

#[derive(Clone, Debug, Default)]
pub struct SessionTraceReplayOptions {
    pub max_steps: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionTraceReplayReport {
    pub trace_step_count: usize,
    pub applied_steps: Vec<SessionTraceReplayAppliedStep>,
    pub order_drift_steps: Vec<usize>,
    pub non_blocking_drifts: Vec<SessionTraceReplayDrift>,
    pub stop: SessionTraceReplayStop,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTraceReplayAppliedStep {
    pub step_index: usize,
    pub command_line: String,
    pub selected_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTraceReplayStop {
    TraceEnd,
    MaxSteps {
        max_steps: usize,
    },
    Drift(SessionTraceReplayDrift),
    UnresolvedSelection {
        step_index: usize,
        reason: String,
    },
    CommandFailed {
        step_index: usize,
        command_line: String,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTraceReplayDrift {
    pub step_index: usize,
    pub phase: SessionTraceReplayDriftPhase,
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTraceReplayDriftPhase {
    Before,
    After,
}

pub fn load_session_trace_v1(path: &Path) -> Result<SessionTraceV1, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let trace = serde_json::from_str::<SessionTraceV1>(&payload).map_err(|err| err.to_string())?;
    if trace.schema_name != SESSION_TRACE_SCHEMA_NAME {
        return Err(format!(
            "unsupported trace schema '{}', expected {SESSION_TRACE_SCHEMA_NAME}",
            trace.schema_name
        ));
    }
    if !(6..=SESSION_TRACE_SCHEMA_VERSION).contains(&trace.schema_version) {
        return Err(format!(
            "unsupported trace schema version {}, expected 6..={SESSION_TRACE_SCHEMA_VERSION}",
            trace.schema_version
        ));
    }
    Ok(trace)
}

pub fn replay_session_trace(
    session: &mut RunControlSession,
    trace: &SessionTraceV1,
    options: SessionTraceReplayOptions,
) -> SessionTraceReplayReport {
    replay_session_trace_with_recorder(session, trace, options, None)
}

pub fn replay_session_trace_with_recorder(
    session: &mut RunControlSession,
    trace: &SessionTraceV1,
    options: SessionTraceReplayOptions,
    mut recorder: Option<&mut SessionTraceRecorder>,
) -> SessionTraceReplayReport {
    let max_steps = options.max_steps.unwrap_or(trace.steps.len());
    let mut applied_steps = Vec::new();
    let mut order_drift_steps = Vec::new();
    let mut non_blocking_drifts = Vec::new();

    for step in trace.steps.iter().take(max_steps) {
        let before = session_trace_boundary_fingerprint(session);
        non_blocking_drifts.extend(non_blocking_boundary_drifts(
            step.step_index,
            SessionTraceReplayDriftPhase::Before,
            &step.before,
            &before,
        ));
        if let Some(drift) = first_boundary_drift(
            step.step_index,
            SessionTraceReplayDriftPhase::Before,
            &step.before,
            &before,
        ) {
            return SessionTraceReplayReport {
                trace_step_count: trace.steps.len(),
                applied_steps,
                order_drift_steps,
                non_blocking_drifts,
                stop: SessionTraceReplayStop::Drift(drift),
            };
        }
        if step.before.candidate_order_hash != before.candidate_order_hash {
            order_drift_steps.push(step.step_index);
        }

        let command_line = match replay_command_for_step(session, step) {
            Ok(command_line) => command_line,
            Err(stop) => {
                return SessionTraceReplayReport {
                    trace_step_count: trace.steps.len(),
                    applied_steps,
                    order_drift_steps,
                    non_blocking_drifts,
                    stop,
                };
            }
        };

        let command = match parse_run_control_command(&command_line) {
            Ok(command) => command,
            Err(error) => {
                return SessionTraceReplayReport {
                    trace_step_count: trace.steps.len(),
                    applied_steps,
                    order_drift_steps,
                    non_blocking_drifts,
                    stop: SessionTraceReplayStop::CommandFailed {
                        step_index: step.step_index,
                        command_line,
                        error,
                    },
                };
            }
        };
        let pending_recording = recorder
            .as_ref()
            .map(|_| SessionTraceRecorder::prepare_step(session, &command_line, &command));

        let outcome = match try_replay_recorded_combat_trajectory(session, step, &command) {
            Some(outcome) => outcome,
            None => match session.apply_command(command) {
                Ok(outcome) => outcome,
                Err(error) => {
                    return SessionTraceReplayReport {
                        trace_step_count: trace.steps.len(),
                        applied_steps,
                        order_drift_steps,
                        non_blocking_drifts,
                        stop: SessionTraceReplayStop::CommandFailed {
                            step_index: step.step_index,
                            command_line,
                            error,
                        },
                    };
                }
            },
        };

        applied_steps.push(SessionTraceReplayAppliedStep {
            step_index: step.step_index,
            command_line: command_line.clone(),
            selected_label: step
                .selected_candidate
                .as_ref()
                .map(|candidate| candidate.label.clone()),
        });

        let after = session_trace_boundary_fingerprint(session);
        non_blocking_drifts.extend(non_blocking_boundary_drifts(
            step.step_index,
            SessionTraceReplayDriftPhase::After,
            &step.after,
            &after,
        ));
        if let Some(drift) = first_boundary_drift(
            step.step_index,
            SessionTraceReplayDriftPhase::After,
            &step.after,
            &after,
        ) {
            return SessionTraceReplayReport {
                trace_step_count: trace.steps.len(),
                applied_steps,
                order_drift_steps,
                non_blocking_drifts,
                stop: SessionTraceReplayStop::Drift(drift),
            };
        }

        if let Some(recorder) = recorder.as_deref_mut() {
            let Some(action_result) = outcome.action_result.as_ref() else {
                return SessionTraceReplayReport {
                    trace_step_count: trace.steps.len(),
                    applied_steps,
                    order_drift_steps,
                    non_blocking_drifts,
                    stop: SessionTraceReplayStop::CommandFailed {
                        step_index: step.step_index,
                        command_line,
                        error: "recorded replay step did not produce an action result".to_string(),
                    },
                };
            };
            if let Some(pending) = pending_recording {
                if let Err(error) = recorder.record_action_step_with_source(
                    pending,
                    session,
                    action_result,
                    &outcome.trace_annotations,
                    SessionTraceStepSourceV1::ReplayVerified {
                        source_trace_step_index: step.step_index,
                    },
                ) {
                    return SessionTraceReplayReport {
                        trace_step_count: trace.steps.len(),
                        applied_steps,
                        order_drift_steps,
                        non_blocking_drifts,
                        stop: SessionTraceReplayStop::CommandFailed {
                            step_index: step.step_index,
                            command_line,
                            error,
                        },
                    };
                }
            }
        }
    }

    let stop = if max_steps < trace.steps.len() {
        SessionTraceReplayStop::MaxSteps { max_steps }
    } else {
        SessionTraceReplayStop::TraceEnd
    };
    SessionTraceReplayReport {
        trace_step_count: trace.steps.len(),
        applied_steps,
        order_drift_steps,
        non_blocking_drifts,
        stop,
    }
}

pub fn render_session_trace_replay_report(
    report: &SessionTraceReplayReport,
    session: &RunControlSession,
) -> String {
    let mut lines = vec![format!(
        "Replay trace: applied {}/{} recorded step(s)",
        report.applied_steps.len(),
        report.trace_step_count
    )];
    if !report.order_drift_steps.is_empty() {
        lines.push(format!(
            "  candidate order drift on step(s): {}; selected candidates were matched by stable descriptor",
            report
                .order_drift_steps
                .iter()
                .map(|step| step.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.non_blocking_drifts.is_empty() {
        lines.extend(render_non_blocking_drifts(&report.non_blocking_drifts));
    }
    lines.push(format!("  stop: {}", replay_stop_summary(&report.stop)));
    match &report.stop {
        SessionTraceReplayStop::Drift(drift) => {
            lines.push(format!(
                "  drift: step={} phase={:?} field={}",
                drift.step_index, drift.phase, drift.field
            ));
            lines.push(format!("    expected: {}", drift.expected));
            lines.push(format!("    actual:   {}", drift.actual));
        }
        SessionTraceReplayStop::UnresolvedSelection { reason, .. } => {
            lines.push(format!("  reason: {reason}"));
        }
        SessionTraceReplayStop::CommandFailed { error, .. } => {
            lines.push(format!("  error: {error}"));
        }
        SessionTraceReplayStop::TraceEnd | SessionTraceReplayStop::MaxSteps { .. } => {}
    }
    lines.push(String::new());
    lines.push(render_run_control_state(session));
    lines.join("\n")
}

fn replay_command_for_step(
    session: &RunControlSession,
    step: &SessionTraceStepV1,
) -> Result<String, SessionTraceReplayStop> {
    if let Some(recorded) = step.selected_candidate.as_ref() {
        let current = current_candidates(session);
        let matches = current
            .iter()
            .filter(|candidate| candidate_signature(candidate) == candidate_signature(recorded))
            .collect::<Vec<_>>();
        return match matches.as_slice() {
            [candidate] => Ok(candidate.command.clone()),
            [] => Err(SessionTraceReplayStop::UnresolvedSelection {
                step_index: step.step_index,
                reason: format!(
                    "recorded selected candidate is no longer visible: {}",
                    candidate_signature(recorded)
                ),
            }),
            _ => Err(SessionTraceReplayStop::UnresolvedSelection {
                step_index: step.step_index,
                reason: format!(
                    "recorded selected candidate is ambiguous in current candidates: {}",
                    candidate_signature(recorded)
                ),
            }),
        };
    }

    if matches!(
        step.selection_resolution,
        SessionTraceSelectionResolution::Unresolved
            | SessionTraceSelectionResolution::AmbiguousLabel
    ) && step.raw_command_line.trim().is_empty()
    {
        return Err(SessionTraceReplayStop::UnresolvedSelection {
            step_index: step.step_index,
            reason: "recorded step has no resolved selected candidate and no raw command line"
                .to_string(),
        });
    }

    Ok(step.raw_command_line.clone())
}

fn current_candidates(session: &RunControlSession) -> Vec<SessionTraceCandidateV1> {
    build_run_control_view_model(session)
        .candidates
        .into_iter()
        .map(SessionTraceCandidateV1::from)
        .collect()
}

fn candidate_signature(candidate: &SessionTraceCandidateV1) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        candidate.id, candidate.command, candidate.label
    )
}

fn first_boundary_drift(
    step_index: usize,
    phase: SessionTraceReplayDriftPhase,
    expected: &SessionTraceBoundaryFingerprintV1,
    actual: &SessionTraceBoundaryFingerprintV1,
) -> Option<SessionTraceReplayDrift> {
    let fields = [
        (
            "engine_state",
            expected.engine_state.clone(),
            actual.engine_state.clone(),
        ),
        (
            "active_combat_engine_state",
            format!("{:?}", expected.active_combat_engine_state),
            format!("{:?}", actual.active_combat_engine_state),
        ),
        (
            "screen_title",
            expected.screen_title.clone(),
            actual.screen_title.clone(),
        ),
        (
            "decision_kind",
            expected.decision_kind.clone(),
            actual.decision_kind.clone(),
        ),
        ("act", expected.act.to_string(), actual.act.to_string()),
        (
            "floor",
            expected.floor.to_string(),
            actual.floor.to_string(),
        ),
        (
            "current_hp",
            expected.current_hp.to_string(),
            actual.current_hp.to_string(),
        ),
        (
            "max_hp",
            expected.max_hp.to_string(),
            actual.max_hp.to_string(),
        ),
        ("gold", expected.gold.to_string(), actual.gold.to_string()),
        ("boss", expected.boss.clone(), actual.boss.clone()),
    ];
    let structural_drift = fields
        .into_iter()
        .find(|(_, expected, actual)| expected != actual)
        .map(|(field, expected, actual)| SessionTraceReplayDrift {
            step_index,
            phase,
            field: field.to_string(),
            expected,
            actual,
        });
    if structural_drift.is_some() {
        return structural_drift;
    }

    if phase == SessionTraceReplayDriftPhase::Before
        && expected.candidate_set_hash != actual.candidate_set_hash
    {
        return Some(SessionTraceReplayDrift {
            step_index,
            phase,
            field: "candidate_set_hash".to_string(),
            expected: expected.candidate_set_hash.clone(),
            actual: actual.candidate_set_hash.clone(),
        });
    }

    None
}

fn non_blocking_boundary_drifts(
    step_index: usize,
    phase: SessionTraceReplayDriftPhase,
    expected: &SessionTraceBoundaryFingerprintV1,
    actual: &SessionTraceBoundaryFingerprintV1,
) -> Vec<SessionTraceReplayDrift> {
    let mut fields = vec![(
        "decision_step",
        expected.decision_step.to_string(),
        actual.decision_step.to_string(),
    )];
    if phase == SessionTraceReplayDriftPhase::After {
        fields.push((
            "candidate_set_hash",
            expected.candidate_set_hash.clone(),
            actual.candidate_set_hash.clone(),
        ));
    }
    fields
        .into_iter()
        .filter(|(_, expected, actual)| expected != actual)
        .map(|(field, expected, actual)| SessionTraceReplayDrift {
            step_index,
            phase,
            field: field.to_string(),
            expected,
            actual,
        })
        .collect()
}

fn render_non_blocking_drifts(drifts: &[SessionTraceReplayDrift]) -> Vec<String> {
    let mut lines = vec!["  non-blocking drift(s):".to_string()];
    let mut by_field = BTreeMap::<&str, usize>::new();
    for drift in drifts {
        *by_field.entry(&drift.field).or_insert(0) += 1;
    }
    let summary = by_field
        .into_iter()
        .map(|(field, count)| format!("{field}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!("    summary: {summary}"));

    let max_examples = 4;
    for drift in drifts.iter().take(max_examples) {
        lines.push(format!(
            "    example: step={} phase={:?} field={} expected={} actual={}",
            drift.step_index, drift.phase, drift.field, drift.expected, drift.actual
        ));
    }
    if drifts.len() > max_examples {
        lines.push(format!("    ... {} more", drifts.len() - max_examples));
    }
    lines
}

fn replay_stop_summary(stop: &SessionTraceReplayStop) -> String {
    match stop {
        SessionTraceReplayStop::TraceEnd => "trace_end".to_string(),
        SessionTraceReplayStop::MaxSteps { max_steps } => {
            format!("max_steps {max_steps}")
        }
        SessionTraceReplayStop::Drift(drift) => {
            format!(
                "drift at step {} before/after {:?}",
                drift.step_index, drift.phase
            )
        }
        SessionTraceReplayStop::UnresolvedSelection { step_index, .. } => {
            format!("unresolved_selection at step {step_index}")
        }
        SessionTraceReplayStop::CommandFailed {
            step_index,
            command_line,
            ..
        } => format!("command_failed at step {step_index}: {command_line}"),
    }
}

fn try_replay_recorded_combat_trajectory(
    session: &mut RunControlSession,
    step: &SessionTraceStepV1,
    command: &RunControlCommand,
) -> Option<RunControlCommandOutcome> {
    if !matches!(
        command,
        RunControlCommand::AutoStep(_) | RunControlCommand::SearchCombat(_)
    ) {
        return None;
    }

    let (source, action_count, actions) =
        step.annotations
            .iter()
            .find_map(|annotation| match annotation {
                RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                    source,
                    action_count,
                    actions,
                    ..
                } => Some((source, action_count, actions)),
                _ => None,
            })?;
    if actions.is_empty() || *action_count != actions.len() {
        return None;
    }

    let mut trial = session.clone();
    trial.mark_current_combat_search_resolved();
    for action in actions {
        if trial.apply_input(action.input.clone()).is_err() {
            return None;
        }
    }

    let after = session_trace_boundary_fingerprint(&trial);
    if first_boundary_drift(
        step.step_index,
        SessionTraceReplayDriftPhase::After,
        &step.after,
        &after,
    )
    .is_some()
    {
        return None;
    }

    *session = trial;
    let outcome = RunControlCommandOutcome::action(
        format!(
            "replayed recorded combat automation: {source} applied {} action(s)",
            actions.len()
        ),
        step.action_result.clone(),
    )
    .with_trace_annotations(step.annotations.clone());
    Some(outcome)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::eval::run_control::trace_annotation::{
        CombatAutomationActionV1, RunControlTraceAnnotationV1,
    };
    use crate::eval::run_control::{
        parse_run_control_command, RunControlAutoStepOptions, RunControlCommand, RunControlConfig,
        SessionTraceRecorder, SessionTraceStepSourceV1,
    };
    use crate::state::core::{ClientInput, EngineState};

    fn one_step_trace(command_line: &str) -> SessionTraceV1 {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let command = parse_run_control_command(command_line).expect("command parses");
        let path = unique_temp_path("trace_replay").join("trace.json");
        let mut recorder = SessionTraceRecorder::new(path, &session);
        let pending = SessionTraceRecorder::prepare_step(&session, command_line, &command);
        let outcome = session.apply_command(command).expect("command applies");
        let action_result = outcome
            .action_result
            .as_ref()
            .expect("command should change state");
        recorder
            .record_action_step(pending, &session, action_result, &outcome.trace_annotations)
            .expect("trace records");
        recorder.trace().clone()
    }

    #[test]
    fn replay_trace_applies_recorded_visible_candidate() {
        let trace = one_step_trace("0");
        let mut session = RunControlSession::new(RunControlConfig::default());

        let report =
            replay_session_trace(&mut session, &trace, SessionTraceReplayOptions::default());

        assert_eq!(report.applied_steps.len(), 1);
        assert!(matches!(report.stop, SessionTraceReplayStop::TraceEnd));
        assert_eq!(session.decision_step, 1);
    }

    #[test]
    fn replay_trace_stops_before_candidate_set_drift() {
        let mut trace = one_step_trace("0");
        trace.steps[0].before.candidate_set_hash = "not-the-current-candidate-set".to_string();
        let mut session = RunControlSession::new(RunControlConfig::default());

        let report =
            replay_session_trace(&mut session, &trace, SessionTraceReplayOptions::default());

        assert!(report.applied_steps.is_empty());
        assert!(matches!(
            report.stop,
            SessionTraceReplayStop::Drift(SessionTraceReplayDrift {
                phase: SessionTraceReplayDriftPhase::Before,
                ..
            })
        ));
        assert_eq!(session.decision_step, 0);
    }

    #[test]
    fn replay_trace_warns_but_continues_on_decision_step_drift() {
        let mut trace = one_step_trace("0");
        trace.steps[0].before.decision_step += 100;
        trace.steps[0].after.decision_step += 100;
        let mut session = RunControlSession::new(RunControlConfig::default());

        let report =
            replay_session_trace(&mut session, &trace, SessionTraceReplayOptions::default());

        assert_eq!(report.applied_steps.len(), 1);
        assert!(matches!(report.stop, SessionTraceReplayStop::TraceEnd));
        assert_eq!(report.non_blocking_drifts.len(), 2);
        assert!(report
            .non_blocking_drifts
            .iter()
            .all(|drift| drift.field == "decision_step"));
        assert_eq!(session.decision_step, 1);
    }

    #[test]
    fn replay_trace_warns_but_continues_on_after_candidate_set_drift() {
        let mut trace = one_step_trace("0");
        trace.steps[0].after.candidate_set_hash = "stale-after-candidate-set".to_string();
        let mut session = RunControlSession::new(RunControlConfig::default());

        let report =
            replay_session_trace(&mut session, &trace, SessionTraceReplayOptions::default());

        assert_eq!(report.applied_steps.len(), 1);
        assert!(matches!(report.stop, SessionTraceReplayStop::TraceEnd));
        assert!(report
            .non_blocking_drifts
            .iter()
            .any(|drift| drift.phase == SessionTraceReplayDriftPhase::After
                && drift.field == "candidate_set_hash"));
        assert_eq!(session.decision_step, 1);
    }

    #[test]
    fn replay_trace_can_record_replayed_prefix_into_new_trace() {
        let trace = one_step_trace("0");
        let mut session = RunControlSession::new(RunControlConfig::default());
        let path = unique_temp_path("trace_replay_recording").join("trace.json");
        let mut recorder = SessionTraceRecorder::new(path.clone(), &session);

        let report = replay_session_trace_with_recorder(
            &mut session,
            &trace,
            SessionTraceReplayOptions::default(),
            Some(&mut recorder),
        );

        assert!(matches!(report.stop, SessionTraceReplayStop::TraceEnd));
        assert_eq!(recorder.trace().steps.len(), 1);
        assert_eq!(
            recorder.trace().steps[0].step_source,
            SessionTraceStepSourceV1::ReplayVerified {
                source_trace_step_index: 0
            }
        );
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn replay_trace_uses_recorded_combat_trajectory_for_auto_step() {
        let mut recording_session = test_session_at_first_combat();
        let command = RunControlCommand::AutoStep(RunControlAutoStepOptions {
            max_operations: Some(0),
            ..Default::default()
        });
        let path = unique_temp_path("trace_replay_combat_trajectory").join("trace.json");
        let mut recorder = SessionTraceRecorder::new(path.clone(), &recording_session);
        let pending =
            SessionTraceRecorder::prepare_step(&recording_session, "n max_ops=0", &command);
        let outcome = recording_session
            .apply_input(ClientInput::EndTurn)
            .expect("recorded combat action should apply");
        let action_result = outcome
            .action_result
            .as_ref()
            .expect("combat input should produce an action result");
        let annotations = vec![RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "test".to_string(),
            action_count: 1,
            actions: vec![CombatAutomationActionV1 {
                step_index: 0,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
            }],
            label_role: "simulator_generated_not_teacher_label".to_string(),
        }];
        recorder
            .record_action_step(pending, &recording_session, action_result, &annotations)
            .expect("trace step should save combat automation annotation");
        let mut trace = recorder.trace().clone();
        trace.steps[0].selected_candidate = None;
        trace.steps[0].selection_resolution = SessionTraceSelectionResolution::Unresolved;
        let expected_after = trace.steps[0].after.clone();
        let mut replay_session = test_session_at_first_combat();

        let report = replay_session_trace(
            &mut replay_session,
            &trace,
            SessionTraceReplayOptions::default(),
        );

        assert!(matches!(report.stop, SessionTraceReplayStop::TraceEnd));
        assert_eq!(report.applied_steps.len(), 1);
        assert_eq!(
            first_boundary_drift(
                0,
                SessionTraceReplayDriftPhase::After,
                &expected_after,
                &session_trace_boundary_fingerprint(&replay_session),
            ),
            None
        );
        assert_eq!(
            combat_turn_count(&replay_session),
            combat_turn_count(&recording_session),
            "replay should apply the recorded combat input, not only accept the compact boundary"
        );

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn non_blocking_drift_render_is_summarized() {
        let drifts = (0..8)
            .map(|index| SessionTraceReplayDrift {
                step_index: index,
                phase: SessionTraceReplayDriftPhase::After,
                field: "decision_step".to_string(),
                expected: index.to_string(),
                actual: (index + 1).to_string(),
            })
            .collect::<Vec<_>>();

        let lines = render_non_blocking_drifts(&drifts);

        assert!(lines[1].contains("summary: decision_step=8"));
        assert!(lines.iter().any(|line| line.contains("4 more")));
        assert!(lines.len() < drifts.len());
    }

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "sts_simulator_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock works")
                .as_nanos()
        ));
        path
    }

    fn test_session_at_first_combat() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(1)))
            .expect("map input should enter combat");
        session
    }

    fn combat_turn_count(session: &RunControlSession) -> Option<u32> {
        session
            .active_combat
            .as_ref()
            .map(|active| active.combat_state.turn.turn_count)
    }
}
