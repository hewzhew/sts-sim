//! Replay execution, continuation, and diff comparison surfaces.

#[path = "../comparator.rs"]
mod comparator;
#[path = "../live_comm_replay.rs"]
mod live_comm_replay;
#[path = "../replay_support.rs"]
mod replay_support;

pub use comparator::{compare_states, ActionContext, DiffCategory, DiffResult};
pub use live_comm_replay::{
    build_live_combat_snapshot_from_root, build_live_session_replay_from_frames,
    build_live_session_replay_from_raw_path, derive_combat_replay_view,
    find_combat_step_index_by_before_frame_id, full_run_command_kind_counts,
    generate_live_session_replay_sidecar, inspect_combat_replay_step,
    load_live_session_replay_path, mapped_command_to_input, reconstruct_combat_replay_step,
    root_state_frame_id, verify_combat_replay_view, write_live_session_replay_to_path,
    CombatMappedCommand, CombatMonsterSummary, CombatReconstructedStep, CombatReplayStep,
    CombatReplayStepStatus, CombatReplayView, CombatStateSummary, CombatStepInspection,
    CombatVerificationFailure, CombatVerificationReport, LiveCommandKind, LiveReplayStep,
    LiveSessionReplay, SerializableDiffResult,
};
pub use replay_support::{continue_deferred_pending_choice, drain_to_stable, tick_until_stable};
