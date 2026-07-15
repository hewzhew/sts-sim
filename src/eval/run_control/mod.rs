mod accepted_combat_line_evidence;
mod artifact_commands;
mod auto_capture;
mod auto_run;
mod auto_step;
mod bookmarks;
mod card_reward_auto;
mod combat_auto_policy;
mod combat_candidate_line;
mod combat_case_adjudication;
mod combat_case_candidate_census;
mod combat_case_retained_candidates;
mod combat_complete_line_repair;
mod combat_complete_line_scoring;
mod combat_complete_line_search;
mod combat_complete_line_solver;
mod combat_line_adjudication;
mod combat_line_executor;
mod combat_line_outcome;
mod combat_line_repair;
mod combat_line_selector;
mod combat_line_trace;
mod combat_no_win_fallback;
mod combat_search;
mod combat_search_rejection;
mod combat_search_render;
mod combat_search_setup;
mod combat_start;
mod commands;
mod decision_case;
mod decision_surface;
#[cfg(test)]
mod decision_surface_tests;
mod input_gate;
mod next_hint;
mod noncombat_boundary;
mod noncombat_policy_annotation;
pub mod outcome;
mod panels;
#[cfg(test)]
mod pending_choice_card_contract_tests;
mod persistent_burden_cutpoint_probe;
pub mod registry;
mod render;
mod reward_auto;
mod route_policy;
mod search_defaults;
mod selection_surface;
mod session;
mod session_trace;
mod session_trace_outcome;
mod shop_legal;
mod trace_annotation;
mod trace_replay;
mod transition_report;
mod view_model;

pub use accepted_combat_line_evidence::{
    accepted_combat_line_evidence_v1, AcceptedCombatLineEvidenceV1,
};
pub use auto_capture::AutoCombatCaptureConfig;
pub use auto_run::apply_owner_audit_auto_run;
pub use bookmarks::{
    default_bookmark_registry_path, load_bookmark_registry, mark_current_boundary,
    render_bookmarks, resolve_goto_bookmark, validate_bookmark_name, GotoBookmarkPlan,
    RunPlayBookmarkRegistryV1, RunPlayBookmarkV1, BOOKMARK_REGISTRY_SCHEMA_NAME,
    BOOKMARK_REGISTRY_SCHEMA_VERSION,
};
pub use combat_case_adjudication::{
    adjudicate_combat_case_line_v1, CombatCaseAdjudicationProbeV1, COMBAT_CASE_PROJECTION_TRUST_V1,
};
pub use combat_case_candidate_census::{
    adjudicate_combat_case_candidates_v1, CombatCaseCandidateAdjudicationCensusV1,
    CombatCaseCandidateCensusConclusionV1, CombatCaseCandidateOutcomeSummaryV1,
    CombatCaseCandidateReplayFailureV1, CombatCaseGainedCurseCountV1,
};
pub use combat_line_adjudication::{
    CombatLineAdjudicationV1, CombatLineCleanlinessV1, CombatLineObservedOutcomeV1,
    CombatLineRejectionReasonV1,
};
pub use commands::{
    parse_run_control_command, run_control_help, run_control_short_hint, RunControlAutoStepOptions,
    RunControlCombatSegmentMode, RunControlCommand, RunControlHpLossLimit,
    RunControlRouteAutomationMode, RunControlSearchCombatOptions, RunControlSearchDefaultsCommand,
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
pub use persistent_burden_cutpoint_probe::{
    probe_combat_case_persistent_burden_cutpoints_v1, CombatCasePersistentBurdenCutpointProbeV1,
    PersistentBurdenCutpointAggregateV1, PersistentBurdenCutpointConclusionV1,
    PersistentBurdenCutpointInputOutcomeKindV1, PersistentBurdenCutpointInputOutcomeV1,
    PersistentBurdenCutpointSummaryV1, PersistentBurdenEnemyPlanChangeV1,
    PersistentBurdenGainedCurseCountV1, PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1,
};
pub use registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
pub use render::{
    render_auto_applied_step_compact_v1, render_combat_actions, render_run_control_details,
    render_run_control_raw, render_run_control_state,
};
pub use reward_auto::{
    apply_reward_tiny_automation, RewardAutomationConfig, RewardAutomationTarget,
};
pub use session::{
    canonical_player_class, RunControlAutoAppliedKindV1, RunControlAutoAppliedStepV1,
    RunControlAutoStopKind, RunControlAutoStopV1, RunControlCommandOutcome, RunControlConfig,
    RunControlSession, RunControlSessionCheckpointV1, ShopVisitContextV1,
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
    annotations_have_combat_automation_trajectory_v1, combat_search_trace_summaries,
    CombatAutomationActionV1, CombatAutomationMonsterStateV1, CombatAutomationStepStateV1,
    CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
    CombatSearchPerformanceSnapshotV1, CombatSearchTerminalLineSummary, CombatSearchTraceSummary,
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
pub use view_model::DecisionCandidateKey;
