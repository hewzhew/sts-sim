mod artifact_commands;
mod auto_capture;
mod auto_run;
mod auto_step;
mod bookmarks;
mod boss_relic_policy;
mod campfire_policy;
mod card_reward_auto;
mod combat_auto_policy;
mod combat_search;
mod combat_start;
mod commands;
mod decision_case;
mod decision_surface;
#[cfg(test)]
mod decision_surface_tests;
mod event_policy;
mod input_gate;
mod next_hint;
mod noncombat_auto;
mod noncombat_boundary;
mod noncombat_policy_annotation;
pub mod outcome;
mod panels;
#[cfg(test)]
mod pending_choice_card_contract_tests;
pub mod registry;
mod render;
mod reward_auto;
mod route_policy;
mod run_choice_policy;
mod search_defaults;
mod search_evidence;
mod selection_surface;
mod session;
mod session_trace;
mod session_trace_outcome;
mod shop_legal;
mod shop_policy;
mod trace_annotation;
mod trace_replay;
mod transition_report;
mod view_model;

pub use auto_capture::AutoCombatCaptureConfig;
pub(crate) use auto_run::apply_branch_experiment_auto_run;
pub use bookmarks::{
    default_bookmark_registry_path, load_bookmark_registry, mark_current_boundary,
    render_bookmarks, resolve_goto_bookmark, validate_bookmark_name, GotoBookmarkPlan,
    RunPlayBookmarkRegistryV1, RunPlayBookmarkV1, BOOKMARK_REGISTRY_SCHEMA_NAME,
    BOOKMARK_REGISTRY_SCHEMA_VERSION,
};
pub use commands::{
    parse_run_control_command, run_control_help, run_control_short_hint, RunControlAutoStepOptions,
    RunControlCombatSegmentMode, RunControlCommand, RunControlHpLossLimit,
    RunControlRouteAutomationMode, RunControlSearchCombatOptions, RunControlSearchDefaultsCommand,
    RunControlSearchEvidenceTarget,
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
    RunControlSessionCheckpointV1,
};
pub use session_trace::{
    SessionTraceArtifactKind, SessionTraceArtifactRefV1, SessionTraceBoundaryFingerprintV1,
    SessionTraceBoundaryRecordV1, SessionTraceCandidateV1, SessionTraceCombatFingerprintV1,
    SessionTraceLineageRoleV1, SessionTraceLineageV1, SessionTraceRecorder,
    SessionTraceRewardAutomationV1, SessionTraceRunConfigV1, SessionTraceSelectionResolution,
    SessionTraceStepSourceV1, SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME,
    SESSION_TRACE_SCHEMA_VERSION,
};
pub(crate) use shop_legal::shop_potion_purchase_block_reason_v1;
pub(crate) use trace_annotation::combat_automation_trajectories_v1;
pub use trace_annotation::{
    annotations_have_combat_automation_trajectory_v1, CombatAutomationActionV1,
    CombatAutomationMonsterStateV1, CombatAutomationStepStateV1,
    CombatAutomationTrajectoryRecordV1, CombatSearchPerformanceSnapshotV1,
    RunControlTraceAnnotationV1,
};
pub use trace_replay::{
    load_session_trace_v1, render_session_trace_replay_report, replay_session_trace,
    replay_session_trace_with_recorder, SessionTraceReplayAppliedStep, SessionTraceReplayDrift,
    SessionTraceReplayDriftPhase, SessionTraceReplayOptions, SessionTraceReplayReport,
    SessionTraceReplayStop,
};
pub use transition_report::{
    ActionResult as RunActionResultV1, ActionResultChange as RunActionResultChangeV1,
    CardSnapshot as RunActionCardSnapshotV1, CombatPlayerResult as RunActionCombatPlayerResultV1,
    MonsterSnapshot as RunActionMonsterSnapshotV1, PileCounts as RunActionPileCountsV1,
    RunApplyStatus as RunActionApplyStatusV1, RunEndResult as RunActionEndResultV1,
    RunKey as RunActionKeyV1, ValueChange as RunActionValueChangeV1,
};
