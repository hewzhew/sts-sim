mod artifact_commands;
mod auto_step;
mod combat_search;
mod combat_start;
mod commands;
mod decision_case;
mod input_gate;
pub mod outcome;
mod panels;
pub mod registry;
mod render;
mod reward_auto;
mod search_evidence;
mod session;
mod session_trace;
mod transition_report;
mod view_model;

pub use commands::{
    parse_run_control_command, run_control_help, run_control_short_hint, RunControlAutoStepOptions,
    RunControlCommand, RunControlSearchCombatOptions, RunControlSearchEvidenceTarget,
};
pub use decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
    RUN_DECISION_CASE_SCHEMA_NAME, RUN_DECISION_CASE_SCHEMA_VERSION,
};
pub use outcome::{
    load_combat_baseline_outcome_v1, save_combat_baseline_outcome_v1, CombatBaselineOutcomeV1,
    COMBAT_BASELINE_OUTCOME_SCHEMA_NAME, COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION,
};
pub use registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
pub use render::{
    render_combat_actions, render_run_control_details, render_run_control_raw,
    render_run_control_state,
};
pub use reward_auto::{RewardAutomationConfig, RewardAutomationTarget};
pub use session::{
    canonical_player_class, RunControlCommandOutcome, RunControlConfig, RunControlSession,
};
pub use session_trace::{
    SessionTraceArtifactKind, SessionTraceArtifactRefV1, SessionTraceBoundaryFingerprintV1,
    SessionTraceCandidateV1, SessionTraceCombatFingerprintV1, SessionTraceRecorder,
    SessionTraceRewardAutomationV1, SessionTraceRunConfigV1, SessionTraceSelectionResolution,
    SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME, SESSION_TRACE_SCHEMA_VERSION,
};
pub use transition_report::{
    ActionResult as RunActionResultV1, ActionResultChange as RunActionResultChangeV1,
    CardSnapshot as RunActionCardSnapshotV1, CombatPlayerResult as RunActionCombatPlayerResultV1,
    MonsterSnapshot as RunActionMonsterSnapshotV1, PileCounts as RunActionPileCountsV1,
    RunApplyStatus as RunActionApplyStatusV1, RunEndResult as RunActionEndResultV1,
    RunKey as RunActionKeyV1, ValueChange as RunActionValueChangeV1,
};
