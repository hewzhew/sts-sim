use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::diff::replay::{
    compare_states_from_snapshots, continue_deferred_pending_choice, tick_until_stable,
    ActionContext, DiffResult,
};
use crate::diff::state_sync::{build_combat_state_from_snapshots, sync_state_from_snapshots};
use crate::protocol::java::{build_live_observation_snapshot, build_live_truth_snapshot};
use crate::state::core::{ClientInput, EngineState, PendingChoice};

const LIVE_REPLAY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveCommandKind {
    Start,
    Play,
    End,
    Potion,
    Choose,
    Proceed,
    Click,
    Key,
    Wait,
    Handoff,
    State,
    Cancel,
    Other,
}

impl LiveCommandKind {
    fn from_protocol(value: Option<&str>, command_text: Option<&str>) -> Self {
        match value.unwrap_or_default().to_ascii_lowercase().as_str() {
            "start" => Self::Start,
            "play" => Self::Play,
            "end" | "end_turn" => Self::End,
            "potion" => Self::Potion,
            "choose" => Self::Choose,
            "proceed" => Self::Proceed,
            "click" => Self::Click,
            "key" => Self::Key,
            "wait" => Self::Wait,
            "handoff" => Self::Handoff,
            "state" => Self::State,
            "cancel" => Self::Cancel,
            _ => {
                let text = command_text.unwrap_or_default().to_ascii_uppercase();
                if text == "END" {
                    Self::End
                } else if text.starts_with("PLAY ") {
                    Self::Play
                } else if text.starts_with("POTION USE ") {
                    Self::Potion
                } else if text.starts_with("CHOOSE ") {
                    Self::Choose
                } else if text == "PROCEED" {
                    Self::Proceed
                } else if text == "CANCEL" {
                    Self::Cancel
                } else if text.starts_with("CLICK ") {
                    Self::Click
                } else if text.starts_with("KEY ") {
                    Self::Key
                } else if text == "WAIT" {
                    Self::Wait
                } else if text == "HANDOFF" {
                    Self::Handoff
                } else if text == "STATE" {
                    Self::State
                } else {
                    Self::Other
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSessionReplay {
    pub schema_version: u32,
    #[serde(default)]
    pub source_path: Option<String>,
    pub total_frames: usize,
    pub steps: Vec<LiveReplayStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveReplayStep {
    pub command_id: u64,
    pub command_text: String,
    pub command_kind: LiveCommandKind,
    pub before_root: Value,
    pub after_root: Value,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub state_frame_id: Option<u64>,
    #[serde(default)]
    pub room_phase: Option<String>,
    #[serde(default)]
    pub screen_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatReplayView {
    #[serde(default)]
    pub source_path: Option<String>,
    pub steps: Vec<CombatReplayStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatReplayStep {
    pub command_id: u64,
    pub command_text: String,
    pub command_kind: LiveCommandKind,
    pub status: CombatReplayStepStatus,
    #[serde(default)]
    pub skip_reason: Option<String>,
    #[serde(default)]
    pub mapped_command: Option<CombatMappedCommand>,
    pub before_root: Value,
    pub after_root: Value,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub state_frame_id: Option<u64>,
    #[serde(default)]
    pub room_phase: Option<String>,
    #[serde(default)]
    pub screen_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CombatReplayStepStatus {
    Executable,
    SkippedNoncombat,
    Unsupported,
    InsufficientContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatMappedCommand {
    Play {
        card_index: usize,
        #[serde(default)]
        target_index: Option<usize>,
    },
    End,
    PotionUse {
        slot: usize,
        #[serde(default)]
        target_index: Option<usize>,
    },
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatVerificationReport {
    #[serde(default)]
    pub source_path: Option<String>,
    pub total_steps: usize,
    pub executable_steps: usize,
    pub skipped_noncombat_steps: usize,
    pub unsupported_steps: usize,
    pub insufficient_context_steps: usize,
    pub failures: Vec<CombatVerificationFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatVerificationFailure {
    pub step_index: usize,
    pub command_id: u64,
    pub command_text: String,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub state_frame_id: Option<u64>,
    pub diffs: Vec<SerializableDiffResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatStepInspection {
    pub step_index: usize,
    pub command_id: u64,
    pub command_text: String,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub state_frame_id: Option<u64>,
    pub rust_after: CombatStateSummary,
    pub java_after: CombatStateSummary,
    pub diffs: Vec<SerializableDiffResult>,
}

#[derive(Debug, Clone)]
pub struct CombatReconstructedStep {
    pub step_index: usize,
    pub command_id: u64,
    pub command_text: String,
    pub response_id: Option<u64>,
    pub state_frame_id: Option<u64>,
    pub before_response_id: Option<u64>,
    pub before_state_frame_id: Option<u64>,
    pub before_root: Value,
    pub mapped_command: CombatMappedCommand,
    pub before_engine: EngineState,
    pub before_combat: crate::runtime::combat::CombatState,
    pub after_engine: EngineState,
    pub after_combat: crate::runtime::combat::CombatState,
    pub java_after_truth_snapshot: Value,
    pub java_after_observation_snapshot: Value,
    pub diffs: Vec<SerializableDiffResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatStateSummary {
    pub player_hp: i32,
    pub player_block: i32,
    pub player_energy: u8,
    pub hand: Vec<String>,
    pub draw_pile: Vec<String>,
    pub discard_pile: Vec<String>,
    pub exhaust_pile: Vec<String>,
    pub player_powers: Vec<String>,
    pub player_relics: Vec<String>,
    pub monsters: Vec<CombatMonsterSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatMonsterSummary {
    pub id: usize,
    pub hp: i32,
    pub block: i32,
    pub intent: String,
    pub powers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDiffResult {
    pub field: String,
    pub rust_val: String,
    pub java_val: String,
    pub category: String,
}

impl From<&DiffResult> for SerializableDiffResult {
    fn from(value: &DiffResult) -> Self {
        Self {
            field: value.field.clone(),
            rust_val: value.rust_val.clone(),
            java_val: value.java_val.clone(),
            category: value.category.to_string(),
        }
    }
}

pub fn load_live_session_replay_path(path: &Path) -> Result<LiveSessionReplay, String> {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
    {
        return build_live_session_replay_from_raw_path(path);
    }

    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read structured replay '{}': {err}",
            path.display()
        )
    })?;
    let mut replay: LiveSessionReplay = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse structured replay '{}': {err}",
            path.display()
        )
    })?;
    if replay.source_path.is_none() {
        replay.source_path = Some(path.display().to_string());
    }
    Ok(replay)
}

pub fn build_live_session_replay_from_raw_path(path: &Path) -> Result<LiveSessionReplay, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read raw livecomm log '{}': {err}",
            path.display()
        )
    })?;
    let mut frames = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse raw livecomm line {} from '{}': {err}",
                line_idx + 1,
                path.display()
            )
        })?;
        frames.push(value);
    }
    build_live_session_replay_from_frames(&frames, Some(path.display().to_string()))
}

pub fn build_live_session_replay_from_frames(
    frames: &[Value],
    source_path: Option<String>,
) -> Result<LiveSessionReplay, String> {
    let mut steps = Vec::new();
    let mut seen_command_ids = HashSet::new();
    let mut previous_root: Option<&Value> = None;

    for root in frames {
        let Some(meta) = root.get("protocol_meta").and_then(Value::as_object) else {
            previous_root = Some(root);
            continue;
        };
        let Some(command_id) = meta
            .get("emitted_for_command_id")
            .and_then(json_u64)
            .or_else(|| meta.get("last_command_id").and_then(json_u64))
        else {
            previous_root = Some(root);
            continue;
        };
        if !seen_command_ids.insert(command_id) {
            previous_root = Some(root);
            continue;
        }

        let command_text = meta
            .get("emitted_for_command")
            .or_else(|| meta.get("last_command"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let command_kind = LiveCommandKind::from_protocol(
            meta.get("emitted_for_command_kind")
                .or_else(|| meta.get("last_command_kind"))
                .and_then(Value::as_str),
            Some(&command_text),
        );
        let after_game_state = root.get("game_state");
        let before_root = previous_root.cloned().unwrap_or_else(|| root.clone());

        steps.push(LiveReplayStep {
            command_id,
            command_text,
            command_kind,
            before_root,
            after_root: root.clone(),
            response_id: meta.get("response_id").and_then(json_u64),
            state_frame_id: meta.get("state_frame_id").and_then(json_u64),
            room_phase: after_game_state
                .and_then(|gs| gs.get("room_phase"))
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
            screen_type: after_game_state
                .and_then(|gs| gs.get("screen_type"))
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
        });
        previous_root = Some(root);
    }

    Ok(LiveSessionReplay {
        schema_version: LIVE_REPLAY_SCHEMA_VERSION,
        source_path,
        total_frames: frames.len(),
        steps,
    })
}

pub fn write_live_session_replay_to_path(
    replay: &LiveSessionReplay,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create replay directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(replay)
        .map_err(|err| format!("failed to serialize structured replay: {err}"))?;
    std::fs::write(path, text).map_err(|err| {
        format!(
            "failed to write structured replay '{}': {err}",
            path.display()
        )
    })
}

pub fn generate_live_session_replay_sidecar(
    raw_path: &Path,
    replay_path: &Path,
) -> Result<LiveSessionReplay, String> {
    let replay = build_live_session_replay_from_raw_path(raw_path)?;
    write_live_session_replay_to_path(&replay, replay_path)?;
    Ok(replay)
}

pub fn derive_combat_replay_view(session: &LiveSessionReplay) -> CombatReplayView {
    let steps = session
        .steps
        .iter()
        .map(|step| {
            let combat_related = step_is_combat_related(step);
            let (status, skip_reason, mapped_command) = if !combat_related {
                (
                    CombatReplayStepStatus::SkippedNoncombat,
                    Some("step did not occur in combat room phase".to_string()),
                    None,
                )
            } else {
                classify_combat_step(step)
            };

            CombatReplayStep {
                command_id: step.command_id,
                command_text: step.command_text.clone(),
                command_kind: step.command_kind.clone(),
                status,
                skip_reason,
                mapped_command,
                before_root: step.before_root.clone(),
                after_root: step.after_root.clone(),
                response_id: step.response_id,
                state_frame_id: step.state_frame_id,
                room_phase: step.room_phase.clone(),
                screen_type: step.screen_type.clone(),
            }
        })
        .collect();

    CombatReplayView {
        source_path: session.source_path.clone(),
        steps,
    }
}

pub fn verify_combat_replay_view(
    view: &CombatReplayView,
    first_fail_only: bool,
) -> Result<CombatVerificationReport, String> {
    let total_steps = view.steps.len();
    let mut report = CombatVerificationReport {
        source_path: view.source_path.clone(),
        total_steps,
        executable_steps: 0,
        skipped_noncombat_steps: 0,
        unsupported_steps: 0,
        insufficient_context_steps: 0,
        failures: Vec::new(),
    };

    for step in &view.steps {
        match step.status {
            CombatReplayStepStatus::Executable => report.executable_steps += 1,
            CombatReplayStepStatus::SkippedNoncombat => report.skipped_noncombat_steps += 1,
            CombatReplayStepStatus::Unsupported => report.unsupported_steps += 1,
            CombatReplayStepStatus::InsufficientContext => report.insufficient_context_steps += 1,
        }
    }

    let Some(first_executable) = view
        .steps
        .iter()
        .find(|step| step.status == CombatReplayStepStatus::Executable)
    else {
        return Ok(report);
    };

    let initial_game_state = first_executable
        .before_root
        .get("game_state")
        .ok_or_else(|| "first executable step missing before_root.game_state".to_string())?;
    let (initial_truth_snapshot, initial_observation_snapshot) =
        build_live_split_combat_snapshots_from_root(&first_executable.before_root)?;
    let relics = initial_game_state
        .get("relics")
        .cloned()
        .unwrap_or(Value::Null);

    let mut combat = build_combat_state_from_snapshots(
        &initial_truth_snapshot,
        &initial_observation_snapshot,
        &relics,
    );
    let mut previous_truth_snapshot = initial_truth_snapshot;
    let mut previous_observation_snapshot = initial_observation_snapshot;
    let mut carried_pending: Option<PendingChoice> = None;
    let mut last_executed_response_id = root_response_id(&first_executable.before_root);

    for (step_index, step) in view.steps.iter().enumerate() {
        if step.status != CombatReplayStepStatus::Executable {
            continue;
        }

        let before_response_id = root_response_id(&step.before_root);
        let continuity_intact =
            last_executed_response_id.is_some() && before_response_id == last_executed_response_id;
        if continuity_intact {
            sync_state_from_snapshots(
                &mut combat,
                &previous_truth_snapshot,
                &previous_observation_snapshot,
            );
        } else {
            let before_game_state = step.before_root.get("game_state").ok_or_else(|| {
                format!(
                    "step command_id={} missing before_root.game_state",
                    step.command_id
                )
            })?;
            let (before_truth_snapshot, before_observation_snapshot) =
                build_live_split_combat_snapshots_from_root(&step.before_root)?;
            let before_relics = before_game_state
                .get("relics")
                .cloned()
                .unwrap_or(Value::Null);
            combat = build_combat_state_from_snapshots(
                &before_truth_snapshot,
                &before_observation_snapshot,
                &before_relics,
            );
            carried_pending = None;
        }
        let mut engine_state = EngineState::CombatPlayerTurn;

        if matches!(
            step.mapped_command,
            Some(CombatMappedCommand::PotionUse { .. })
        ) {
            if let Some(pending) = carried_pending.take() {
                continue_deferred_pending_choice(&pending, &mut combat, &step.after_root).map_err(
                    |err| {
                        format!(
                            "step command_id={} deferred continuation replay failed: {err}",
                            step.command_id
                        )
                    },
                )?;
            }
        }

        let input = mapped_command_to_input(
            step.mapped_command.as_ref().ok_or_else(|| {
                format!("executable step {} had no mapped command", step.command_id)
            })?,
            &combat,
        )?;

        let is_end_turn = matches!(step.mapped_command, Some(CombatMappedCommand::End));
        let _alive = tick_until_stable(&mut engine_state, &mut combat, input);
        carried_pending = match &engine_state {
            EngineState::PendingChoice(choice) => Some(choice.clone()),
            _ => None,
        };

        let (java_after_truth_snapshot, java_after_observation_snapshot) =
            extract_split_combat_snapshots(&step.after_root).ok_or_else(|| {
                format!(
                    "step command_id={} missing after_root.game_state.combat payload",
                    step.command_id
                )
            })?;
        let context = action_context_for_step(
            step,
            &java_after_truth_snapshot,
            &java_after_observation_snapshot,
        );
        let diffs = compare_states_from_snapshots(
            &combat,
            &java_after_truth_snapshot,
            &java_after_observation_snapshot,
            is_end_turn,
            &context,
        );
        if !diffs.is_empty() {
            report.failures.push(CombatVerificationFailure {
                step_index,
                command_id: step.command_id,
                command_text: step.command_text.clone(),
                response_id: step.response_id,
                state_frame_id: step.state_frame_id,
                diffs: diffs.iter().map(SerializableDiffResult::from).collect(),
            });
            if first_fail_only {
                break;
            }
        }

        previous_truth_snapshot = java_after_truth_snapshot;
        previous_observation_snapshot = java_after_observation_snapshot;
        last_executed_response_id = step.response_id;
    }

    Ok(report)
}

pub fn inspect_combat_replay_step(
    view: &CombatReplayView,
    target_step_index: usize,
) -> Result<CombatStepInspection, String> {
    let reconstructed = reconstruct_combat_replay_step(view, target_step_index)?;
    Ok(CombatStepInspection {
        step_index: reconstructed.step_index,
        command_id: reconstructed.command_id,
        command_text: reconstructed.command_text,
        response_id: reconstructed.response_id,
        state_frame_id: reconstructed.state_frame_id,
        rust_after: summarize_combat_state(&reconstructed.after_combat),
        java_after: summarize_java_snapshots(
            &reconstructed.java_after_truth_snapshot,
            &reconstructed.java_after_observation_snapshot,
        ),
        diffs: reconstructed.diffs,
    })
}

pub fn find_combat_step_index_by_before_frame_id(
    view: &CombatReplayView,
    frame_id: u64,
) -> Option<usize> {
    view.steps
        .iter()
        .enumerate()
        .find_map(|(step_index, step)| {
            (step.status == CombatReplayStepStatus::Executable
                && root_state_frame_id(&step.before_root) == Some(frame_id))
            .then_some(step_index)
        })
}

pub fn reconstruct_combat_replay_step(
    view: &CombatReplayView,
    target_step_index: usize,
) -> Result<CombatReconstructedStep, String> {
    let Some(first_executable) = view
        .steps
        .iter()
        .find(|step| step.status == CombatReplayStepStatus::Executable)
    else {
        return Err("combat replay had no executable steps".into());
    };

    let initial_game_state = first_executable
        .before_root
        .get("game_state")
        .ok_or_else(|| "first executable step missing before_root.game_state".to_string())?;
    let (initial_truth_snapshot, initial_observation_snapshot) =
        build_live_split_combat_snapshots_from_root(&first_executable.before_root)?;
    let relics = initial_game_state
        .get("relics")
        .cloned()
        .unwrap_or(Value::Null);

    let mut combat = build_combat_state_from_snapshots(
        &initial_truth_snapshot,
        &initial_observation_snapshot,
        &relics,
    );
    let mut previous_truth_snapshot = initial_truth_snapshot;
    let mut previous_observation_snapshot = initial_observation_snapshot;
    let mut carried_pending: Option<PendingChoice> = None;
    let mut last_executed_response_id = root_response_id(&first_executable.before_root);

    for (step_index, step) in view.steps.iter().enumerate() {
        if step.status != CombatReplayStepStatus::Executable {
            continue;
        }

        let before_response_id = root_response_id(&step.before_root);
        let continuity_intact =
            last_executed_response_id.is_some() && before_response_id == last_executed_response_id;
        if continuity_intact {
            sync_state_from_snapshots(
                &mut combat,
                &previous_truth_snapshot,
                &previous_observation_snapshot,
            );
        } else {
            let before_game_state = step.before_root.get("game_state").ok_or_else(|| {
                format!(
                    "step command_id={} missing before_root.game_state",
                    step.command_id
                )
            })?;
            let (before_truth_snapshot, before_observation_snapshot) =
                build_live_split_combat_snapshots_from_root(&step.before_root)?;
            let before_relics = before_game_state
                .get("relics")
                .cloned()
                .unwrap_or(Value::Null);
            combat = build_combat_state_from_snapshots(
                &before_truth_snapshot,
                &before_observation_snapshot,
                &before_relics,
            );
            carried_pending = None;
        }
        let mut engine_state = EngineState::CombatPlayerTurn;
        let before_engine = engine_state.clone();
        let before_combat = combat.clone();

        if matches!(
            step.mapped_command,
            Some(CombatMappedCommand::PotionUse { .. })
        ) {
            if let Some(pending) = carried_pending.take() {
                continue_deferred_pending_choice(&pending, &mut combat, &step.after_root).map_err(
                    |err| {
                        format!(
                            "step command_id={} deferred continuation replay failed: {err}",
                            step.command_id
                        )
                    },
                )?;
            }
        }

        let input = mapped_command_to_input(
            step.mapped_command.as_ref().ok_or_else(|| {
                format!("executable step {} had no mapped command", step.command_id)
            })?,
            &before_combat,
        )?;

        let is_end_turn = matches!(step.mapped_command, Some(CombatMappedCommand::End));
        let _alive = tick_until_stable(&mut engine_state, &mut combat, input);
        carried_pending = match &engine_state {
            EngineState::PendingChoice(choice) => Some(choice.clone()),
            _ => None,
        };

        let (java_after_truth_snapshot, java_after_observation_snapshot) =
            extract_split_combat_snapshots(&step.after_root).ok_or_else(|| {
                format!(
                    "step command_id={} missing after_root.game_state.combat payload",
                    step.command_id
                )
            })?;
        let context = action_context_for_step(
            step,
            &java_after_truth_snapshot,
            &java_after_observation_snapshot,
        );
        let diffs = compare_states_from_snapshots(
            &combat,
            &java_after_truth_snapshot,
            &java_after_observation_snapshot,
            is_end_turn,
            &context,
        );

        if step_index == target_step_index {
            return Ok(CombatReconstructedStep {
                step_index,
                command_id: step.command_id,
                command_text: step.command_text.clone(),
                response_id: step.response_id,
                state_frame_id: step.state_frame_id,
                before_response_id: root_response_id(&step.before_root),
                before_state_frame_id: root_state_frame_id(&step.before_root),
                before_root: step.before_root.clone(),
                mapped_command: step.mapped_command.clone().ok_or_else(|| {
                    format!("executable step {} had no mapped command", step.command_id)
                })?,
                before_engine,
                before_combat,
                after_engine: engine_state.clone(),
                after_combat: combat.clone(),
                java_after_truth_snapshot,
                java_after_observation_snapshot,
                diffs: diffs.iter().map(SerializableDiffResult::from).collect(),
            });
        }

        previous_truth_snapshot = java_after_truth_snapshot;
        previous_observation_snapshot = java_after_observation_snapshot;
        last_executed_response_id = step.response_id;
    }

    Err(format!(
        "target step_index={} not found among executable combat steps",
        target_step_index
    ))
}

pub fn full_run_command_kind_counts(session: &LiveSessionReplay) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for step in &session.steps {
        let key = match step.command_kind {
            LiveCommandKind::Start => "start",
            LiveCommandKind::Play => "play",
            LiveCommandKind::End => "end",
            LiveCommandKind::Potion => "potion",
            LiveCommandKind::Choose => "choose",
            LiveCommandKind::Proceed => "proceed",
            LiveCommandKind::Click => "click",
            LiveCommandKind::Key => "key",
            LiveCommandKind::Wait => "wait",
            LiveCommandKind::Handoff => "handoff",
            LiveCommandKind::State => "state",
            LiveCommandKind::Cancel => "cancel",
            LiveCommandKind::Other => "other",
        };
        *counts.entry(key.to_string()).or_insert(0) += 1;
    }
    counts
}

fn step_is_combat_related(step: &LiveReplayStep) -> bool {
    root_is_combat_related(&step.before_root) || root_is_combat_related(&step.after_root)
}

fn summarize_combat_state(combat: &crate::runtime::combat::CombatState) -> CombatStateSummary {
    CombatStateSummary {
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        player_energy: combat.turn.energy,
        hand: combat
            .zones
            .hand
            .iter()
            .map(|card| crate::content::cards::java_id(card.id).to_string())
            .collect(),
        draw_pile: combat
            .zones
            .draw_pile
            .iter()
            .map(|card| crate::content::cards::java_id(card.id).to_string())
            .collect(),
        discard_pile: combat
            .zones
            .discard_pile
            .iter()
            .map(|card| crate::content::cards::java_id(card.id).to_string())
            .collect(),
        exhaust_pile: combat
            .zones
            .exhaust_pile
            .iter()
            .map(|card| crate::content::cards::java_id(card.id).to_string())
            .collect(),
        player_powers: crate::content::powers::store::powers_for(combat, 0)
            .map(|powers| {
                powers
                    .iter()
                    .map(|power| {
                        format!(
                            "{}={}",
                            crate::content::powers::get_power_definition(power.power_type).name,
                            power.amount
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        player_relics: combat
            .entities
            .player
            .relics
            .iter()
            .map(|relic| {
                format!(
                    "{:?}:counter={},used_up={}",
                    relic.id, relic.counter, relic.used_up
                )
            })
            .collect(),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| CombatMonsterSummary {
                id: monster.id,
                hp: monster.current_hp,
                block: monster.block,
                intent: format!(
                    "{:?}",
                    crate::content::monsters::resolve_monster_turn_plan(combat, monster)
                        .summary_spec()
                ),
                powers: crate::content::powers::store::powers_for(combat, monster.id)
                    .map(|powers| {
                        powers
                            .iter()
                            .map(|power| {
                                format!(
                                    "{}={}",
                                    crate::content::powers::get_power_definition(power.power_type)
                                        .name,
                                    power.amount
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

fn summarize_java_snapshots(
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) -> CombatStateSummary {
    let player = &truth_snapshot["player"];
    let truth_monsters = truth_snapshot["monsters"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let observation_monsters = observation_snapshot
        .get("monsters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    CombatStateSummary {
        player_hp: player["current_hp"]
            .as_i64()
            .or_else(|| player["hp"].as_i64())
            .unwrap_or(0) as i32,
        player_block: player["block"].as_i64().unwrap_or(0) as i32,
        player_energy: player["energy"].as_u64().unwrap_or(0) as u8,
        hand: truth_snapshot["hand"]
            .as_array()
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| card["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        draw_pile: truth_snapshot["draw_pile"]
            .as_array()
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| card["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        discard_pile: truth_snapshot["discard_pile"]
            .as_array()
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| card["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        exhaust_pile: truth_snapshot["exhaust_pile"]
            .as_array()
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| card["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        player_powers: player["powers"]
            .as_array()
            .map(|powers| {
                powers
                    .iter()
                    .filter_map(|power| {
                        let id = power["id"].as_str()?;
                        let amount = power["amount"].as_i64().unwrap_or(0);
                        Some(format!("{id}={amount}"))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        player_relics: truth_snapshot["relics"]
            .as_array()
            .map(|relics| {
                relics
                    .iter()
                    .filter_map(|relic| {
                        let id = relic["id"].as_str()?;
                        let runtime_state = relic.get("runtime_state").unwrap_or_else(|| {
                            panic!("strict live_comm_replay: relic.runtime_state missing for {id}")
                        });
                        let counter = runtime_state
                            .get("counter")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| {
                                panic!("strict live_comm_replay: relic.runtime_state.counter missing for {id}")
                            });
                        let used_up = runtime_state
                            .get("used_up")
                            .and_then(|value| value.as_bool())
                            .unwrap_or_else(|| {
                                panic!("strict live_comm_replay: relic.runtime_state.used_up missing for {id}")
                            });
                        Some(format!("{id}:counter={counter},used_up={used_up}"))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        monsters: truth_monsters
            .iter()
            .enumerate()
            .map(|(index, monster)| CombatMonsterSummary {
                id: index + 1,
                hp: monster["current_hp"].as_i64().unwrap_or(0) as i32,
                block: monster["block"].as_i64().unwrap_or(0) as i32,
                intent: observation_monsters
                    .get(index)
                    .and_then(|observation| observation.get("intent"))
                    .map(Value::to_string)
                    .unwrap_or_else(|| "null".to_string()),
                powers: monster["powers"]
                    .as_array()
                    .map(|powers| {
                        powers
                            .iter()
                            .filter_map(|power| {
                                let id = power["id"].as_str()?;
                                let amount = power["amount"].as_i64().unwrap_or(0);
                                Some(format!("{id}={amount}"))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

fn root_is_combat_related(root: &Value) -> bool {
    let Some(game_state) = root.get("game_state") else {
        return false;
    };
    has_live_combat_payload(game_state)
        || game_state
            .get("room_phase")
            .and_then(Value::as_str)
            .is_some_and(|phase| phase == "COMBAT")
}

fn classify_combat_step(
    step: &LiveReplayStep,
) -> (
    CombatReplayStepStatus,
    Option<String>,
    Option<CombatMappedCommand>,
) {
    let has_before_snapshot = extract_split_combat_snapshots(&step.before_root).is_some();
    let has_after_snapshot = extract_split_combat_snapshots(&step.after_root).is_some();
    if !has_before_snapshot || !has_after_snapshot {
        return if step.room_phase.as_deref() == Some("COMBAT") {
            (
                CombatReplayStepStatus::InsufficientContext,
                Some("combat-adapter requires both before and after combat snapshots".to_string()),
                None,
            )
        } else {
            (
                CombatReplayStepStatus::SkippedNoncombat,
                Some("step crossed out of combat before a comparable snapshot existed".to_string()),
                None,
            )
        };
    }

    match parse_mapped_command(&step.command_text) {
        Some(mapped) => {
            if opens_intermediate_combat_choice(&step.after_root) {
                (
                    CombatReplayStepStatus::InsufficientContext,
                    Some(
                        "combat step opens an intermediate pending-choice screen; compare the resolved follow-up step instead"
                            .to_string(),
                    ),
                    None,
                )
            } else {
                (CombatReplayStepStatus::Executable, None, Some(mapped))
            }
        }
        None => match step.command_kind {
            LiveCommandKind::Choose | LiveCommandKind::Proceed | LiveCommandKind::Click => (
                CombatReplayStepStatus::InsufficientContext,
                Some(
                    "combat step requires pending-choice context to map into ClientInput"
                        .to_string(),
                ),
                None,
            ),
            LiveCommandKind::Key
            | LiveCommandKind::Wait
            | LiveCommandKind::Handoff
            | LiveCommandKind::State
            | LiveCommandKind::Other
            | LiveCommandKind::Start => (
                CombatReplayStepStatus::Unsupported,
                Some("combat-adapter does not yet execute this command kind".to_string()),
                None,
            ),
            LiveCommandKind::Play
            | LiveCommandKind::End
            | LiveCommandKind::Potion
            | LiveCommandKind::Cancel => (
                CombatReplayStepStatus::Unsupported,
                Some("command text did not match a supported combat input pattern".to_string()),
                None,
            ),
        },
    }
}

fn opens_intermediate_combat_choice(root: &Value) -> bool {
    let Some(game_state) = root.get("game_state") else {
        return false;
    };
    let screen_type = game_state
        .get("screen_type")
        .and_then(Value::as_str)
        .unwrap_or("NONE");
    if screen_type.eq_ignore_ascii_case("NONE") {
        return false;
    }
    root.get("available_commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| {
            commands
                .iter()
                .filter_map(Value::as_str)
                .any(|command| command.eq_ignore_ascii_case("choose"))
        })
}

fn parse_mapped_command(command_text: &str) -> Option<CombatMappedCommand> {
    let parts: Vec<&str> = command_text.split_whitespace().collect();
    match parts.as_slice() {
        ["END"] => Some(CombatMappedCommand::End),
        ["CANCEL"] => Some(CombatMappedCommand::Cancel),
        ["PLAY", card_idx] => Some(CombatMappedCommand::Play {
            card_index: card_idx.parse::<usize>().ok()?.saturating_sub(1),
            target_index: None,
        }),
        ["PLAY", card_idx, target] => Some(CombatMappedCommand::Play {
            card_index: card_idx.parse::<usize>().ok()?.saturating_sub(1),
            target_index: Some(target.parse::<usize>().ok()?),
        }),
        ["POTION", "USE", slot] => Some(CombatMappedCommand::PotionUse {
            slot: slot.parse::<usize>().ok()?,
            target_index: None,
        }),
        ["POTION", "USE", slot, target] => Some(CombatMappedCommand::PotionUse {
            slot: slot.parse::<usize>().ok()?,
            target_index: Some(target.parse::<usize>().ok()?),
        }),
        _ => None,
    }
}

pub fn mapped_command_to_input(
    command: &CombatMappedCommand,
    combat: &crate::runtime::combat::CombatState,
) -> Result<ClientInput, String> {
    match command {
        CombatMappedCommand::Play {
            card_index,
            target_index,
        } => {
            let target = target_index.map(|idx| {
                combat
                    .entities
                    .monsters
                    .get(idx)
                    .map(|monster| monster.id)
                    .unwrap_or(idx + 1)
            });
            Ok(ClientInput::PlayCard {
                card_index: *card_index,
                target,
            })
        }
        CombatMappedCommand::End => Ok(ClientInput::EndTurn),
        CombatMappedCommand::PotionUse { slot, target_index } => {
            let target = target_index.map(|idx| {
                combat
                    .entities
                    .monsters
                    .get(idx)
                    .map(|monster| monster.id)
                    .unwrap_or(idx + 1)
            });
            Ok(ClientInput::UsePotion {
                potion_index: *slot,
                target,
            })
        }
        CombatMappedCommand::Cancel => Ok(ClientInput::Cancel),
    }
}

fn extract_split_combat_snapshots(root: &Value) -> Option<(Value, Value)> {
    build_live_split_combat_snapshots_from_root(root).ok()
}

fn has_live_combat_payload(game_state: &Value) -> bool {
    game_state.get("combat_truth").is_some_and(|v| !v.is_null())
        && game_state
            .get("combat_observation")
            .is_some_and(|v| !v.is_null())
}

pub fn build_live_split_combat_snapshots_from_root(root: &Value) -> Result<(Value, Value), String> {
    let game_state = root
        .get("game_state")
        .ok_or_else(|| "root missing game_state".to_string())?;
    if !has_live_combat_payload(game_state) {
        return Err("game_state missing combat payload".to_string());
    }
    Ok((
        build_live_truth_snapshot(game_state),
        build_live_observation_snapshot(game_state),
    ))
}

fn action_context_for_step(
    step: &CombatReplayStep,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) -> ActionContext {
    let mut context = ActionContext {
        last_command: step.command_text.clone(),
        was_end_turn: matches!(step.mapped_command, Some(CombatMappedCommand::End)),
        has_rng_state: truth_snapshot.get("rng_state").is_some()
            || step.after_root.get("rng_state").is_some(),
        ..Default::default()
    };
    if let Some(monsters) = observation_snapshot
        .get("monsters")
        .or_else(|| truth_snapshot.get("monsters"))
        .and_then(Value::as_array)
    {
        context.monster_intents = monsters
            .iter()
            .map(|monster| monster["intent"].as_str().unwrap_or("?").to_string())
            .collect();
        context.monster_names = monsters
            .iter()
            .map(|monster| monster["id"].as_str().unwrap_or("?").to_string())
            .collect();
    }
    context
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
}

fn root_response_id(root: &Value) -> Option<u64> {
    root.get("protocol_meta")
        .and_then(|meta| meta.get("response_id"))
        .and_then(json_u64)
}

pub fn root_state_frame_id(root: &Value) -> Option<u64> {
    root.get("protocol_meta")
        .and_then(|meta| meta.get("state_frame_id").or_else(|| meta.get("frame_id")))
        .and_then(json_u64)
}
