//! Exact, resumable planning over complete player-turn options.
//!
//! It turns the simulator's exact legal input surface into replayable options
//! ending at the next supported combat boundary. Optional policies guide work
//! order but never change legality or terminal truth. Partial action prefixes
//! remain private generator work.
//! The crate boundary also keeps planner iteration out of the core unit-test
//! harness; production integration belongs to the control layer.

mod atomic_witness;
mod depth_beam_turn;
mod generator;
mod local_turn_graph_search;
mod policy;
mod policy_discrepancy_search;
mod replay;
mod selection_transaction;
mod types;
mod witness;

pub use atomic_witness::ExactAtomicWitness;
pub use depth_beam_turn::{
    generate_depth_beam_turn_options, search_depth_beam_agenda_witness, DepthBeamAgendaBudget,
    DepthBeamAgendaConfig, DepthBeamAgendaCounters, DepthBeamAgendaInterruption,
    DepthBeamAgendaReport, DepthBeamAgendaStatus, DepthBeamAgendaWitness, DepthBeamTurnBudget,
    DepthBeamTurnConfig, DepthBeamTurnCounters, DepthBeamTurnInterruption,
    DepthBeamTurnLayerReport, DepthBeamTurnReport, DepthBeamTurnStatus,
};
pub use generator::{
    LiveActionTransitionSnapshot, TurnOptionGeneratorSession, DETAIL_TIMING_SAMPLE_INTERVAL,
};
pub use local_turn_graph_search::{
    LocalTurnGraphEdgeSnapshot, LocalTurnGraphGuideServiceSnapshot,
    LocalTurnGraphPlanAnnotationEnableError, LocalTurnGraphPlanTransitionEdgeSnapshot,
    LocalTurnGraphPolicyLineReport, LocalTurnGraphRetainedGuidePromiseSnapshot,
    LocalTurnGraphRootActionFamilySnapshot, LocalTurnGraphStateSnapshot,
    LocalTurnGraphSuffixProbeAttempt, LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessCounters,
    LocalTurnGraphWitnessInterruption, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessReport,
    LocalTurnGraphWitnessSession, LocalTurnGraphWitnessStatus,
};
pub use policy::{
    combat_plan_selection_timing_policy_v1, combat_plan_state_guide_policy_v1, CombatActionPolicy,
    CombatGuideLaneId, CombatLookaheadEvaluation, CombatLookaheadEvaluator,
    CombatPlanSelectionTimingPolicyV1, CombatPlanStateGuidePolicyV1, CombatPolicyChoice,
    CombatPolicyWitnessProposal, CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy,
    SharedCombatLookaheadEvaluator, UniformCombatActionPolicy, COMBAT_PLAN_STATE_GUIDE_LANE_V1,
};
pub use policy_discrepancy_search::{
    PolicyDiscrepancyConfig, PolicyDiscrepancyCounters, PolicyDiscrepancyInterruption,
    PolicyDiscrepancyQuantum, PolicyDiscrepancyReport, PolicyDiscrepancySession,
    PolicyDiscrepancyStateDiagnostic, PolicyDiscrepancyStatus, PolicyDiscrepancyTrajectoryAudit,
    PolicyDiscrepancyTrajectoryDeviation, PolicyDiscrepancyTurnMacroConfig,
};
pub use replay::{
    replay_turn_option, ReplayError, ReplayFailure, ReplayLimits, VerifiedTurnOptionReplay,
};
pub use types::{
    CombatDecisionRoot, CombatDecisionRootError, CombatPlanningCounters, CombatPlanningQuantum,
    CompleteTurnOption, CompleteTurnOptionBoundary, GenerationInterruption, ReplaySuccessorHash,
    TurnOptionAction, TurnOptionGenerationDiagnostics, TurnOptionGenerationGap,
    TurnOptionGenerationGapKind, TurnOptionGenerationReport, TurnOptionGenerationStatus,
    TurnOptionGeneratorConfig,
};
pub use witness::{
    OracleCombatDeepStateSnapshot, OracleCombatGuideQueueSnapshot, OracleCombatGuideRankSnapshot,
    OracleCombatWitness, OracleCombatWitnessDiscoverySource, OracleCombatWitnessProgressSnapshot,
    OracleCombatWitnessReplayError, OracleCombatWitnessSatisfaction,
    OracleCombatWitnessStateProgressSnapshot,
};

#[cfg(test)]
mod tests;
