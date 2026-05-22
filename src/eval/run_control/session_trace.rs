use std::fs;
use std::path::{Path, PathBuf};

use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};

use super::commands::RunControlCommand;
use super::registry::BenchmarkCasePaths;
use super::session::RunControlSession;
use super::transition_report::ActionResult;
use super::view_model::{build_run_control_view_model, CandidateResolution, DecisionCandidate};

pub const SESSION_TRACE_SCHEMA_NAME: &str = "SessionTraceV1";
pub const SESSION_TRACE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub run_config: SessionTraceRunConfigV1,
    pub steps: Vec<SessionTraceStepV1>,
    pub artifact_refs: Vec<SessionTraceArtifactRefV1>,
}

impl SessionTraceV1 {
    pub fn new(session: &RunControlSession) -> Self {
        Self {
            schema_name: SESSION_TRACE_SCHEMA_NAME.to_string(),
            schema_version: SESSION_TRACE_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            run_config: SessionTraceRunConfigV1::from_session(session),
            steps: Vec::new(),
            artifact_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceRunConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: String,
    pub final_act: bool,
    pub skip_neow: bool,
}

impl SessionTraceRunConfigV1 {
    fn from_session(session: &RunControlSession) -> Self {
        Self {
            seed: session.run_state.seed,
            ascension_level: session.run_state.ascension_level,
            player_class: session.run_state.player_class.to_string(),
            final_act: session.run_state.is_final_act_available,
            skip_neow: session.run_state.event_state.is_none() && session.run_state.floor_num == 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceStepV1 {
    pub step_index: usize,
    pub raw_command_line: String,
    pub decision_step_before: u64,
    pub decision_step_after: u64,
    pub screen_title: String,
    pub decision_kind: String,
    pub before: SessionTraceBoundaryFingerprintV1,
    pub after: SessionTraceBoundaryFingerprintV1,
    pub visible_candidates: Vec<SessionTraceCandidateV1>,
    pub selected_candidate: Option<SessionTraceCandidateV1>,
    pub selection_resolution: SessionTraceSelectionResolution,
    pub action_result: ActionResult,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceBoundaryFingerprintV1 {
    pub decision_step: u64,
    pub engine_state: String,
    pub active_combat_engine_state: Option<String>,
    pub screen_title: String,
    pub decision_kind: String,
    pub decision_label: String,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub boss: String,
    pub candidate_count: usize,
    pub candidate_set_hash: String,
    pub candidate_order_hash: String,
    pub combat: Option<SessionTraceCombatFingerprintV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceCombatFingerprintV1 {
    pub public_observation_hash: String,
    pub legal_candidate_set_hash: String,
    pub legal_candidate_order_hash: String,
    pub exact_state_hash: String,
    pub stable_outcome_hash: Option<String>,
    pub rng_boundary_status: crate::eval::fingerprint::RngFingerprintStatus,
    pub rng_boundary_stream_count: usize,
    pub rng_boundary_digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceCandidateV1 {
    pub id: String,
    pub label: String,
    pub command: String,
    pub note: Option<String>,
    pub executable: bool,
    pub resolution: Option<CandidateResolution>,
}

impl From<DecisionCandidate> for SessionTraceCandidateV1 {
    fn from(candidate: DecisionCandidate) -> Self {
        Self {
            id: candidate.id,
            label: candidate.label,
            command: candidate.action.command_hint(),
            note: candidate.note,
            executable: candidate.action.executable_input().is_some(),
            resolution: candidate.resolution,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTraceSelectionResolution {
    ResolvedByVisibleId,
    ResolvedSingleVisibleCandidate,
    ResolvedByUniqueLabel,
    AmbiguousLabel,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTraceArtifactRefV1 {
    pub raw_command_line: String,
    pub decision_step: u64,
    pub artifact_kind: SessionTraceArtifactKind,
    pub capture_path: Option<String>,
    pub baseline_path: Option<String>,
    pub benchmark_manifest_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTraceArtifactKind {
    CombatCaptureCase,
    CombatBaselineCase,
}

#[derive(Clone, Debug)]
pub struct SessionTracePendingStep {
    raw_command_line: String,
    decision_step_before: u64,
    screen_title: String,
    decision_kind: String,
    before: SessionTraceBoundaryFingerprintV1,
    visible_candidates: Vec<SessionTraceCandidateV1>,
    selected_candidate: Option<SessionTraceCandidateV1>,
    selection_resolution: SessionTraceSelectionResolution,
}

#[derive(Debug)]
pub struct SessionTraceRecorder {
    path: PathBuf,
    trace: SessionTraceV1,
}

impl SessionTraceRecorder {
    pub fn new(path: PathBuf, session: &RunControlSession) -> Self {
        Self {
            path,
            trace: SessionTraceV1::new(session),
        }
    }

    pub fn prepare_step(
        session: &RunControlSession,
        raw_command_line: impl Into<String>,
        command: &RunControlCommand,
    ) -> SessionTracePendingStep {
        let view = build_run_control_view_model(session);
        let candidates = view
            .candidates
            .clone()
            .into_iter()
            .map(SessionTraceCandidateV1::from)
            .collect::<Vec<_>>();
        let (selected_candidate, selection_resolution) =
            resolve_selected_candidate(command, &candidates, None);
        SessionTracePendingStep {
            raw_command_line: raw_command_line.into(),
            decision_step_before: session.decision_step,
            screen_title: view.header.title.clone(),
            decision_kind: decision_kind_from_title(&view.header.title),
            before: boundary_fingerprint(session),
            visible_candidates: candidates,
            selected_candidate,
            selection_resolution,
        }
    }

    pub fn record_action_step(
        &mut self,
        pending: SessionTracePendingStep,
        session_after: &RunControlSession,
        action_result: &ActionResult,
    ) -> Result<(), String> {
        let (selected_candidate, selection_resolution) = if pending.selected_candidate.is_some() {
            (pending.selected_candidate, pending.selection_resolution)
        } else {
            resolve_selected_candidate_by_label(&pending.visible_candidates, action_result)
        };
        let step = SessionTraceStepV1 {
            step_index: self.trace.steps.len(),
            raw_command_line: pending.raw_command_line,
            decision_step_before: pending.decision_step_before,
            decision_step_after: session_after.decision_step,
            screen_title: pending.screen_title,
            decision_kind: pending.decision_kind,
            before: pending.before,
            after: boundary_fingerprint(session_after),
            visible_candidates: pending.visible_candidates,
            selected_candidate,
            selection_resolution,
            action_result: action_result.clone(),
        };
        self.trace.steps.push(step);
        self.save()
    }

    pub fn record_artifact_command(
        &mut self,
        raw_command_line: impl Into<String>,
        session: &RunControlSession,
        command: &RunControlCommand,
    ) -> Result<bool, String> {
        let raw_command_line = raw_command_line.into();
        let artifact = match command {
            RunControlCommand::CaptureCase { root, case_id, .. } => {
                let paths = BenchmarkCasePaths::for_case(root, case_id);
                Some(SessionTraceArtifactRefV1 {
                    raw_command_line,
                    decision_step: session.decision_step,
                    artifact_kind: SessionTraceArtifactKind::CombatCaptureCase,
                    capture_path: Some(path_string(&paths.capture_path)),
                    baseline_path: paths
                        .baseline_path
                        .exists()
                        .then(|| path_string(&paths.baseline_path)),
                    benchmark_manifest_path: Some(path_string(&paths.benchmark_manifest)),
                })
            }
            RunControlCommand::SaveBaselineCase { root, case_id } => {
                let paths = BenchmarkCasePaths::for_case(root, case_id);
                Some(SessionTraceArtifactRefV1 {
                    raw_command_line,
                    decision_step: session.decision_step,
                    artifact_kind: SessionTraceArtifactKind::CombatBaselineCase,
                    capture_path: paths
                        .capture_path
                        .exists()
                        .then(|| path_string(&paths.capture_path)),
                    baseline_path: Some(path_string(&paths.baseline_path)),
                    benchmark_manifest_path: paths
                        .benchmark_manifest
                        .exists()
                        .then(|| path_string(&paths.benchmark_manifest)),
                })
            }
            _ => None,
        };
        let Some(artifact) = artifact else {
            return Ok(false);
        };
        self.trace.artifact_refs.push(artifact);
        self.save()?;
        Ok(true)
    }

    pub fn trace(&self) -> &SessionTraceV1 {
        &self.trace
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let payload = serde_json::to_string_pretty(&self.trace).map_err(|err| err.to_string())?;
        fs::write(&self.path, payload).map_err(|err| err.to_string())
    }
}

fn boundary_fingerprint(session: &RunControlSession) -> SessionTraceBoundaryFingerprintV1 {
    let view = build_run_control_view_model(session);
    let candidates = view
        .candidates
        .clone()
        .into_iter()
        .map(SessionTraceCandidateV1::from)
        .collect::<Vec<_>>();
    let (candidate_set_hash, candidate_order_hash) = candidate_hashes(&candidates);
    let (current_hp, max_hp) = session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp));
    SessionTraceBoundaryFingerprintV1 {
        decision_step: session.decision_step,
        engine_state: format!("{:?}", session.engine_state),
        active_combat_engine_state: session
            .active_combat
            .as_ref()
            .map(|active| format!("{:?}", active.engine_state)),
        screen_title: view.header.title.clone(),
        decision_kind: decision_kind_from_title(&view.header.title),
        decision_label: view.decision.label,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        current_hp,
        max_hp,
        gold: session.run_state.gold,
        boss: super::view_model::boss_label(&session.run_state),
        candidate_count: candidates.len(),
        candidate_set_hash,
        candidate_order_hash,
        combat: combat_fingerprint(session),
    }
}

fn combat_fingerprint(session: &RunControlSession) -> Option<SessionTraceCombatFingerprintV1> {
    let position = session.current_active_combat_position().ok()?;
    let fingerprint = crate::eval::fingerprint::combat_state_fingerprint_v1(&position);
    Some(SessionTraceCombatFingerprintV1 {
        public_observation_hash: fingerprint.public_observation_hash,
        legal_candidate_set_hash: fingerprint.legal_candidate_set_hash,
        legal_candidate_order_hash: fingerprint.legal_candidate_order_hash,
        exact_state_hash: fingerprint.exact_state_hash,
        stable_outcome_hash: fingerprint.stable_outcome_hash,
        rng_boundary_status: fingerprint.rng_boundary.status,
        rng_boundary_stream_count: fingerprint.rng_boundary.stream_count,
        rng_boundary_digest: fingerprint.rng_boundary.digest,
    })
}

fn resolve_selected_candidate(
    command: &RunControlCommand,
    candidates: &[SessionTraceCandidateV1],
    action_result: Option<&ActionResult>,
) -> (
    Option<SessionTraceCandidateV1>,
    SessionTraceSelectionResolution,
) {
    match command {
        RunControlCommand::Candidate(id) => {
            let candidate = candidates
                .iter()
                .find(|candidate| &candidate.id == id)
                .cloned();
            if candidate.is_some() {
                (
                    candidate,
                    SessionTraceSelectionResolution::ResolvedByVisibleId,
                )
            } else {
                (None, SessionTraceSelectionResolution::Unresolved)
            }
        }
        RunControlCommand::DefaultCandidate if candidates.len() == 1 => (
            candidates.first().cloned(),
            SessionTraceSelectionResolution::ResolvedSingleVisibleCandidate,
        ),
        _ => action_result
            .map(|result| resolve_selected_candidate_by_label(candidates, result))
            .unwrap_or((None, SessionTraceSelectionResolution::Unresolved)),
    }
}

fn resolve_selected_candidate_by_label(
    candidates: &[SessionTraceCandidateV1],
    action_result: &ActionResult,
) -> (
    Option<SessionTraceCandidateV1>,
    SessionTraceSelectionResolution,
) {
    let matches = candidates
        .iter()
        .filter(|candidate| candidate.label == action_result.chosen_label)
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => (
            Some(candidate.clone()),
            SessionTraceSelectionResolution::ResolvedByUniqueLabel,
        ),
        [] => (None, SessionTraceSelectionResolution::Unresolved),
        _ => (None, SessionTraceSelectionResolution::AmbiguousLabel),
    }
}

fn candidate_hashes(candidates: &[SessionTraceCandidateV1]) -> (String, String) {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|left, right| candidate_stable_key(left).cmp(&candidate_stable_key(right)));
    (hash_serializable(&sorted), hash_serializable(candidates))
}

fn candidate_stable_key(candidate: &SessionTraceCandidateV1) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        candidate.id, candidate.command, candidate.label
    )
}

fn decision_kind_from_title(title: &str) -> String {
    title
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn hash_serializable<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).expect("session trace fingerprint should serialize");
    let mut hasher = Blake2b512::new();
    hasher.update(&payload);
    let digest = hasher.finalize();
    hex_lower(&digest[..32])
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::{RunControlCommand, RunControlConfig};
    use crate::state::core::ClientInput;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_trace_serializes_diagnostic_schema_fields() {
        let session = RunControlSession::new(RunControlConfig::default());
        let trace = SessionTraceV1::new(&session);
        let json = serde_json::to_string_pretty(&trace).expect("trace should serialize");

        assert!(json.contains("\"schema_name\": \"SessionTraceV1\""));
        assert!(json.contains("\"label_role\": \"diagnostic_not_teacher_label\""));
        assert!(json.contains("\"trainable_as_action_label\": false"));
        assert!(json.contains("\"policy_quality_claim\": false"));
    }

    #[test]
    fn recorder_appends_successful_action_step() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let path = unique_temp_dir("session_trace_action").join("trace.json");
        let mut recorder = SessionTraceRecorder::new(path.clone(), &session);
        let command = RunControlCommand::DefaultCandidate;
        let pending = SessionTraceRecorder::prepare_step(&session, "", &command);
        let outcome = session
            .apply_command(command)
            .expect("default candidate should advance Neow intro");
        let action_result = outcome
            .action_result
            .as_ref()
            .expect("state-changing command should return action result");

        recorder
            .record_action_step(pending, &session, action_result)
            .expect("trace step should save");

        assert_eq!(recorder.trace().steps.len(), 1);
        let step = &recorder.trace().steps[0];
        assert_eq!(step.decision_step_before, 0);
        assert_eq!(step.decision_step_after, 1);
        assert_eq!(
            step.selection_resolution,
            SessionTraceSelectionResolution::ResolvedSingleVisibleCandidate
        );
        assert!(step.selected_candidate.is_some());
        assert!(path.exists());

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn recorder_does_not_record_read_only_artifact_command() {
        let session = RunControlSession::new(RunControlConfig::default());
        let path = unique_temp_dir("session_trace_read_only").join("trace.json");
        let mut recorder = SessionTraceRecorder::new(path.clone(), &session);

        let recorded = recorder
            .record_artifact_command("help", &session, &RunControlCommand::Help)
            .expect("non-artifact command should not fail");

        assert!(!recorded);
        assert!(recorder.trace().steps.is_empty());
        assert!(recorder.trace().artifact_refs.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn candidate_hashes_distinguish_order_and_set() {
        let first = SessionTraceCandidateV1 {
            id: "0".to_string(),
            label: "A".to_string(),
            command: "event 0".to_string(),
            note: None,
            executable: true,
            resolution: None,
        };
        let second = SessionTraceCandidateV1 {
            id: "1".to_string(),
            label: "B".to_string(),
            command: "event 1".to_string(),
            note: None,
            executable: true,
            resolution: None,
        };

        let (set_a, order_a) = candidate_hashes(&[first.clone(), second.clone()]);
        let (set_b, order_b) = candidate_hashes(&[second.clone(), first.clone()]);
        let (set_c, _) = candidate_hashes(&[first]);

        assert_eq!(set_a, set_b);
        assert_ne!(order_a, order_b);
        assert_ne!(set_a, set_c);
    }

    #[test]
    fn recorder_records_capture_case_artifact_ref() {
        let mut session = RunControlSession::new(RunControlConfig {
            skip_neow: true,
            ..RunControlConfig::default()
        });
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(1)))
            .expect("map input should enter combat for default seed");
        let path = unique_temp_dir("session_trace_artifact").join("trace.json");
        let root = path.parent().unwrap().join("bench");
        let mut recorder = SessionTraceRecorder::new(path.clone(), &session);

        session
            .apply_command(RunControlCommand::CaptureCase {
                root: root.clone(),
                case_id: "first_fight".to_string(),
                label: None,
            })
            .expect("capture-case should save");
        let recorded = recorder
            .record_artifact_command(
                "capture-case bench first_fight",
                &session,
                &RunControlCommand::CaptureCase {
                    root,
                    case_id: "first_fight".to_string(),
                    label: None,
                },
            )
            .expect("artifact ref should save");

        assert!(recorded);
        assert_eq!(recorder.trace().artifact_refs.len(), 1);
        assert_eq!(
            recorder.trace().artifact_refs[0].artifact_kind,
            SessionTraceArtifactKind::CombatCaptureCase
        );
        assert!(recorder.trace().artifact_refs[0].capture_path.is_some());
        assert!(recorder.trace().artifact_refs[0]
            .benchmark_manifest_path
            .is_some());

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }
}
