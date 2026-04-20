use serde_json::Value;

use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;

pub use crate::diff::replay::{
    ActionContext, CombatMappedCommand, CombatReconstructedStep, CombatReplayView,
    CombatVerificationReport, DiffCategory, DiffResult, LiveSessionReplay,
};

pub fn build_combat_state_from_snapshots(
    truth_snapshot: &Value,
    observation_snapshot: &Value,
    relics: &Value,
) -> CombatState {
    crate::diff::state_sync::build_combat_state_from_snapshots(
        truth_snapshot,
        observation_snapshot,
        relics,
    )
}

pub fn build_live_split_combat_snapshots_from_root(root: &Value) -> Result<(Value, Value), String> {
    crate::diff::replay::build_live_split_combat_snapshots_from_root(root)
}

pub fn mapped_command_to_input(
    command: &CombatMappedCommand,
    combat: &CombatState,
) -> Result<ClientInput, String> {
    crate::diff::replay::mapped_command_to_input(command, combat)
}

pub fn compare_combat_states_from_snapshots(
    expected: &CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
    was_end_turn: bool,
    action_context: &ActionContext,
) -> Vec<DiffResult> {
    crate::diff::replay::compare_states_from_snapshots(
        expected,
        truth_snapshot,
        observation_snapshot,
        was_end_turn,
        action_context,
    )
}

pub fn load_live_session_replay(path: &std::path::Path) -> Result<LiveSessionReplay, String> {
    crate::diff::replay::load_live_session_replay_path(path)
}

pub fn derive_replay_view(session: &LiveSessionReplay) -> CombatReplayView {
    crate::diff::replay::derive_combat_replay_view(session)
}

pub fn verify_replay_view(
    view: &CombatReplayView,
    fail_fast: bool,
) -> Result<CombatVerificationReport, String> {
    crate::diff::replay::verify_combat_replay_view(view, fail_fast)
}

pub fn build_live_session_replay_from_raw(
    path: &std::path::Path,
) -> Result<LiveSessionReplay, String> {
    crate::diff::replay::build_live_session_replay_from_raw_path(path)
}

pub fn write_live_session_replay(
    replay: &LiveSessionReplay,
    path: &std::path::Path,
) -> Result<(), String> {
    crate::diff::replay::write_live_session_replay_to_path(replay, path)
}

pub fn generate_live_session_replay(
    raw_path: &std::path::Path,
    replay_path: &std::path::Path,
) -> Result<LiveSessionReplay, String> {
    crate::diff::replay::generate_live_session_replay_sidecar(raw_path, replay_path)
}
