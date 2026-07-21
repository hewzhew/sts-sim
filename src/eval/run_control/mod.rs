mod accepted_combat_line_evidence;
mod auto_capture;
mod auto_step;
mod bounded_run_driver;
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
mod combat_line_selector;
mod combat_line_trace;
mod combat_no_win_fallback;
#[cfg(test)]
mod combat_planner_cutover_harness;
mod combat_resolution;
mod combat_search;
mod combat_search_rejection;
mod combat_search_render;
mod combat_search_setup;
mod combat_start;
mod decision_action;
mod decision_case;
mod decision_surface;
#[cfg(test)]
mod decision_surface_tests;
mod decision_transaction;
mod forced_transition;
mod input_gate;
mod next_hint;
mod noncombat_boundary;
mod noncombat_policy_annotation;
mod oracle_analysis_session;
mod oracle_combat_policy;
mod oracle_combat_work;
mod oracle_neow;
mod oracle_run_explorer;
pub mod outcome;
mod panels;
#[cfg(test)]
mod pending_choice_card_contract_tests;
mod persistent_burden_cutpoint_probe;
mod planner_boundary_capture;
mod planner_capture;
mod progress_journal;
mod progress_options;
mod progress_replay;
mod progress_step;
pub mod registry;
mod render;
mod reward_auto;
mod route_policy;
mod selection_surface;
mod session;
mod session_trace;
mod shop_legal;
mod strategic_checkpoint_probe;
mod strategic_encounter_probe;
mod strategic_mechanism_probe;
mod strategic_probe_calibration;
mod trace_annotation;
mod transition_report;
mod view_model;

pub use accepted_combat_line_evidence::{
    accepted_combat_line_evidence_v1, AcceptedCombatLineEvidenceV1,
};
pub use auto_capture::AutoCombatCaptureConfig;
pub use bounded_run_driver::{
    BoundedRunDriveErrorV1, BoundedRunDriveResultV1, BoundedRunDriveStopV1, BoundedRunDriver,
    BoundedRunResultV1, BoundedRunStepContextV1, BoundedRunStepControlV1,
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
pub use combat_resolution::{
    RunCombatResolutionBoundaryV1, RunCombatResolutionKindV1, RunCombatResolutionV1,
    RUN_COMBAT_RESOLUTION_SCHEMA_NAME, RUN_COMBAT_RESOLUTION_SCHEMA_VERSION,
};
pub use combat_search::{RunControlCombatWorkAdvanceV1, RunControlCombatWorkV1};
pub use decision_action::RunDecisionAction;
pub use decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
    RUN_DECISION_CASE_SCHEMA_NAME, RUN_DECISION_CASE_SCHEMA_VERSION,
};
pub use decision_surface::{build_decision_surface, DecisionSurface};
pub use decision_transaction::{
    RunDecisionBoundaryV1, RunDecisionCandidateSnapshotV1, RunDecisionSelectionSourceV1,
    RunDecisionSelectionV1, RunDecisionTransactionV1, RUN_DECISION_TRANSACTION_SCHEMA_NAME,
    RUN_DECISION_TRANSACTION_SCHEMA_VERSION,
};
pub use forced_transition::{
    RunForcedTransitionKindV1, RunForcedTransitionV1, RUN_FORCED_TRANSITION_SCHEMA_NAME,
    RUN_FORCED_TRANSITION_SCHEMA_VERSION,
};
pub use oracle_analysis_session::{
    OracleAnalysisAdvanceReportV1, OracleAnalysisAdvanceRequestV1, OracleAnalysisAdvanceStatusV1,
    OracleAnalysisChildViewV1, OracleAnalysisChoiceViewV1, OracleAnalysisCombatJobCheckpointV1,
    OracleAnalysisCombatProgressV1, OracleAnalysisEdgeKindV1, OracleAnalysisEdgeV1,
    OracleAnalysisNodeSummaryV1, OracleAnalysisNodeViewV1, OracleAnalysisSessionCheckpointV1,
    OracleAnalysisSessionV1, OracleAnalysisTreeViewV1, ORACLE_ANALYSIS_SESSION_SCHEMA_NAME,
    ORACLE_ANALYSIS_SESSION_SCHEMA_VERSION,
};
pub use oracle_combat_policy::{
    existing_combat_knowledge_policy_v1, existing_combat_knowledge_policy_with_rollout_guide_v1,
    ExistingCombatKnowledgeAdvisorAdvanceV1, ExistingCombatKnowledgeAdvisorV1,
};
pub use oracle_combat_work::OracleRunCombatWorkCheckpointV1;
pub use oracle_neow::{
    expand_oracle_neow_candidates_v1, CompletedNeowCandidateV1, NeowOracleExpansionV1,
    NeowOracleReplayStepV1, UnresolvedNeowCandidateV1,
};
pub use oracle_run_explorer::{
    drive_oracle_run_explorer_v1, seed_oracle_run_explorer_from_checkpoint_v1,
    seed_oracle_run_explorer_from_session_v1, seed_oracle_run_explorer_v1,
    ExactDuplicateOracleRunBranchV1, LazyOracleRunDecisionV1, OracleCombatSearchResumeKindV1,
    OraclePendingCombatEnemyV1, OraclePendingCombatSummaryV1, OracleRunActiveCombatCheckpointV1,
    OracleRunBoundaryV1, OracleRunBranchCheckpointV1, OracleRunBranchV1, OracleRunCombatBudgetsV1,
    OracleRunCombatEdgeOrderFnV1, OracleRunCombatEdgeProbeV1, OracleRunDecisionAnnotationFnV1,
    OracleRunDecisionOrderFnV1, OracleRunExploreBudgetV1, OracleRunExploreResultV1,
    OracleRunExploreStopV1, OracleRunExplorerCheckpointV1, OracleRunExplorerV1,
    OracleRunJournalNodeCheckpointV1, OracleRunReplayStepV1, OracleRunUnresolvedCombatV1,
    OracleRunWorkKindV1,
};
pub use outcome::{
    load_combat_baseline_outcome_v1, save_combat_baseline_outcome_v1, CombatBaselineOutcomeV1,
    COMBAT_BASELINE_OUTCOME_SCHEMA_NAME, COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION,
};
pub use persistent_burden_cutpoint_probe::{
    probe_combat_case_persistent_burden_cutpoints_v1, CombatCasePersistentBurdenCutpointProbeV1,
    PersistentBurdenCutpointActionDomainV1, PersistentBurdenCutpointAggregateV1,
    PersistentBurdenCutpointConclusionV1, PersistentBurdenCutpointInputOutcomeKindV1,
    PersistentBurdenCutpointInputOutcomeV1, PersistentBurdenCutpointSummaryV1,
    PersistentBurdenEnemyPlanChangeV1, PersistentBurdenGainedCurseCountV1,
    PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1,
};
pub use planner_boundary_capture::{
    build_planner_boundary_capture_coverage_report_v1, capture_planner_boundary_ticket_v1,
    capture_planner_boundary_yield_v1, PlannerBoundaryCandidateLinkV1,
    PlannerBoundaryCaptureCoverageReportV1, PlannerBoundaryCaptureSegmentV1,
    PlannerBoundaryCaptureTicketV1, PlannerBoundaryMutationKindV1, PlannerBoundarySiteCoverageV1,
    PlannerBoundaryVisitOutcomeV1, PlannerBoundaryVisitV1, PlannerBoundaryYieldKindV1,
    PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_NAME, PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_VERSION,
};
pub use planner_capture::{
    build_planner_capture_coverage_report, build_planner_capture_dataset,
    PlannerCaptureCoverageReport, PlannerCaptureDataset, PlannerDecisionSiteCoverage,
};
pub use progress_journal::{
    RunProgressJournalV1, RUN_PROGRESS_JOURNAL_SCHEMA_NAME, RUN_PROGRESS_JOURNAL_SCHEMA_VERSION,
};
pub use progress_options::{
    RunControlAutoStepOptions, RunControlCombatSearchQuantum, RunControlCombatSegmentMode,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
};
pub use progress_replay::{exact_replay_run_progress_journal_v1, ExactRunProgressReplayReportV1};
pub use progress_step::{RunControlAutoStopKind, RunControlAutoStopV1, RunProgressStepV1};
pub use registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
pub use render::{
    render_auto_applied_step_compact_v1, render_progress_step_compact_v1,
    render_run_control_details, render_run_control_raw, render_run_control_state,
};
pub use reward_auto::{
    apply_reward_policy_step, reward_surface_has_only_unclaimable_potions, RewardAutomationConfig,
};
pub use session::{
    canonical_player_class, RunControlAutoAppliedKindV1, RunControlAutoAppliedStepV1,
    RunControlCombatSearchRejection, RunControlConfig, RunControlSession,
    RunControlSessionCheckpointV1, RunProgressOutcome, ShopVisitContextV1,
};
pub use session_trace::{
    load_session_trace_v1, SessionTraceArtifactKind, SessionTraceArtifactRefV1,
    SessionTraceBoundaryFingerprintV1, SessionTraceBoundaryRecordV1, SessionTraceCandidateV1,
    SessionTraceCombatFingerprintV1, SessionTraceLineageRoleV1, SessionTraceLineageV1,
    SessionTraceRewardAutomationV1, SessionTraceRunConfigV1, SessionTraceSelectionResolution,
    SessionTraceStepSourceV1, SessionTraceStepV1, SessionTraceV1, SESSION_TRACE_SCHEMA_NAME,
    SESSION_TRACE_SCHEMA_VERSION,
};
pub(crate) use shop_legal::shop_potion_purchase_block_reason_v1;
pub use strategic_checkpoint_probe::{
    run_strategic_checkpoint_probe_decomposition_v1, StrategicCheckpointProbeDecompositionV1,
    StrategicCheckpointProbeOmissionV1, StrategicCheckpointProbeStateSummaryV1,
    StrategicCheckpointProbeVariantKindV1, StrategicCheckpointProbeVariantV1,
    StrategicCheckpointReferenceRelationV1, STRATEGIC_CHECKPOINT_PROBE_SCHEMA_NAME,
    STRATEGIC_CHECKPOINT_PROBE_SCHEMA_VERSION,
};
pub use strategic_encounter_probe::{
    run_strategic_encounter_probe_suite_v1, run_strategic_encounter_probes_v1,
    strategic_encounter_probe_plan_v1, StrategicCapabilityPredictionV1,
    StrategicEncounterFrontierObservationV1, StrategicEncounterHeuristicEvidenceV1,
    StrategicEncounterPrimaryEvidenceV1, StrategicEncounterProbeBudgetReportV1,
    StrategicEncounterProbeBudgetV1, StrategicEncounterProbeHpBasisV1,
    StrategicEncounterProbeObservationV1, StrategicEncounterProbePotionUseV1,
    StrategicEncounterProbeReportV1, StrategicEncounterProbeSpecV1,
    StrategicEncounterRolloutObservationV1, StrategicEncounterWinObservationV1,
    STRATEGIC_ENCOUNTER_PROBE_SCHEMA_NAME, STRATEGIC_ENCOUNTER_PROBE_SCHEMA_VERSION,
};
pub use strategic_mechanism_probe::{
    run_strategic_mechanism_probes_v1, strategic_mechanism_probe_plan_v1, StrategicMechanismKindV1,
    StrategicMechanismProbeObservationV1, StrategicMechanismProbeOutcomeV1,
    StrategicMechanismProbeReportV1, StrategicMechanismProbeSpecV1,
    STRATEGIC_MECHANISM_PROBE_SCHEMA_NAME, STRATEGIC_MECHANISM_PROBE_SCHEMA_VERSION,
};
pub use strategic_probe_calibration::{
    run_strategic_probe_calibration_v1, strategic_combat_edge_shadow_order_v1,
    strategic_probe_resolved_label_v1, strategic_probe_shadow_order_key_v1,
    validate_strategic_probe_shadow_ordering_v1, StrategicProbeCalibrationObservationV1,
    StrategicProbeCalibrationPartitionV1, StrategicProbeCalibrationReportV1,
    StrategicProbeFidelityConsistencyV1, StrategicProbeFidelityV1,
    StrategicProbeHeldOutOrderingValidationV1, StrategicProbeOrderingCalibrationCaseV1,
    StrategicProbeOwnerAuthorityV1, StrategicProbeResolvedLabelV1,
    StrategicProbeSchedulingAuthorityV1, StrategicProbeShadowFidelityV1,
    StrategicProbeShadowObservationV1, StrategicProbeShadowOrderKeyV1,
    STRATEGIC_PROBE_CALIBRATION_SCHEMA_NAME, STRATEGIC_PROBE_CALIBRATION_SCHEMA_VERSION,
};
pub(crate) use trace_annotation::combat_automation_trajectories_v1;
pub use trace_annotation::{
    annotations_have_combat_automation_trajectory_v1, combat_search_trace_summaries,
    CardRewardFunctionV1, CardRewardObligationDeltaV1, CardRewardObligationSourceV1,
    CardRewardOwnerProvenanceV1, CombatAutomationActionV1, CombatAutomationAnswerClaimV1,
    CombatAutomationAnswerSourceV1, CombatAutomationCardOriginV1, CombatAutomationMonsterStateV1,
    CombatAutomationOpportunityStateV1, CombatAutomationPotionStateV1, CombatAutomationStepStateV1,
    CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
    CombatSearchPerformanceSnapshotV1, CombatSearchTerminalLineSummary, CombatSearchTraceSummary,
    RunControlTraceAnnotationV1,
};
pub use transition_report::{
    ActionResult as RunActionResultV1, ActionResultChange as RunActionResultChangeV1,
    CardSnapshot as RunActionCardSnapshotV1, CombatPlayerResult as RunActionCombatPlayerResultV1,
    MonsterSnapshot as RunActionMonsterSnapshotV1, PileCounts as RunActionPileCountsV1,
    RunApplyStatus as RunActionApplyStatusV1, RunEndResult as RunActionEndResultV1,
    RunKey as RunActionKeyV1, ValueChange as RunActionValueChangeV1,
};
pub use view_model::DecisionCandidateKey;
