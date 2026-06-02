use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::commands::parse_run_control_command;
use super::render::render_run_control_state;
use super::session::RunControlSession;
use super::session_trace::{
    session_trace_boundary_fingerprint, SessionTraceBoundaryFingerprintV1, SessionTraceCandidateV1,
    SessionTraceSelectionResolution, SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME,
    SESSION_TRACE_SCHEMA_VERSION,
};
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
    if trace.schema_version != SESSION_TRACE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported trace schema version {}, expected {SESSION_TRACE_SCHEMA_VERSION}",
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
    let max_steps = options.max_steps.unwrap_or(trace.steps.len());
    let mut applied_steps = Vec::new();
    let mut order_drift_steps = Vec::new();

    for step in trace.steps.iter().take(max_steps) {
        let before = session_trace_boundary_fingerprint(session);
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
                    stop: SessionTraceReplayStop::CommandFailed {
                        step_index: step.step_index,
                        command_line,
                        error,
                    },
                };
            }
        };

        if let Err(error) = session.apply_command(command) {
            return SessionTraceReplayReport {
                trace_step_count: trace.steps.len(),
                applied_steps,
                order_drift_steps,
                stop: SessionTraceReplayStop::CommandFailed {
                    step_index: step.step_index,
                    command_line,
                    error,
                },
            };
        }

        applied_steps.push(SessionTraceReplayAppliedStep {
            step_index: step.step_index,
            command_line,
            selected_label: step
                .selected_candidate
                .as_ref()
                .map(|candidate| candidate.label.clone()),
        });

        let after = session_trace_boundary_fingerprint(session);
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
                stop: SessionTraceReplayStop::Drift(drift),
            };
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
            "decision_step",
            expected.decision_step.to_string(),
            actual.decision_step.to_string(),
        ),
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
        (
            "candidate_set_hash",
            expected.candidate_set_hash.clone(),
            actual.candidate_set_hash.clone(),
        ),
    ];
    fields
        .into_iter()
        .find(|(_, expected, actual)| expected != actual)
        .map(|(field, expected, actual)| SessionTraceReplayDrift {
            step_index,
            phase,
            field: field.to_string(),
            expected,
            actual,
        })
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::eval::run_control::{
        parse_run_control_command, RunControlConfig, SessionTraceRecorder,
    };

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
}
