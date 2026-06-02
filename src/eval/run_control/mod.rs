mod artifact_commands;
mod auto_capture;
mod auto_step;
mod combat_search;
mod combat_start;
mod commands;
mod decision_case;
mod decision_surface;
#[cfg(test)]
mod decision_surface_tests;
mod input_gate;
pub mod outcome;
mod panels;
#[cfg(test)]
mod pending_choice_card_contract_tests;
pub mod registry;
mod render;
mod reward_auto;
mod route_policy;
mod search_defaults;
mod search_evidence;
mod selection_surface;
mod session;
mod session_trace;
mod trace_annotation;
mod transition_report;
mod view_model;

pub use auto_capture::AutoCombatCaptureConfig;
pub use commands::{
    parse_run_control_command, run_control_help, run_control_short_hint, RunControlAutoStepOptions,
    RunControlCommand, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSearchDefaultsCommand, RunControlSearchEvidenceTarget,
};
pub use decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
    RUN_DECISION_CASE_SCHEMA_NAME, RUN_DECISION_CASE_SCHEMA_VERSION,
};
pub use decision_surface::{build_decision_surface, DecisionSurface};
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
pub use search_evidence::{
    load_combat_search_evidence_v1, validate_combat_search_evidence_v1,
    COMBAT_SEARCH_EVIDENCE_SCHEMA_NAME, COMBAT_SEARCH_EVIDENCE_SCHEMA_VERSION,
};
pub use session::{
    canonical_player_class, RunControlCommandOutcome, RunControlConfig, RunControlSession,
};
pub use session_trace::{
    SessionTraceArtifactKind, SessionTraceArtifactRefV1, SessionTraceBoundaryFingerprintV1,
    SessionTraceCandidateV1, SessionTraceCombatFingerprintV1, SessionTraceRecorder,
    SessionTraceRewardAutomationV1, SessionTraceRunConfigV1, SessionTraceSelectionResolution,
    SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME, SESSION_TRACE_SCHEMA_VERSION,
};
pub use trace_annotation::RunControlTraceAnnotationV1;
pub use transition_report::{
    ActionResult as RunActionResultV1, ActionResultChange as RunActionResultChangeV1,
    CardSnapshot as RunActionCardSnapshotV1, CombatPlayerResult as RunActionCombatPlayerResultV1,
    MonsterSnapshot as RunActionMonsterSnapshotV1, PileCounts as RunActionPileCountsV1,
    RunApplyStatus as RunActionApplyStatusV1, RunEndResult as RunActionEndResultV1,
    RunKey as RunActionKeyV1, ValueChange as RunActionValueChangeV1,
};
